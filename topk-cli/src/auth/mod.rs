use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use oauth2::basic::{BasicClient, BasicErrorResponseType};
use oauth2::{
    AuthType, AuthUrl, ClientId, EndpointNotSet, EndpointSet, RefreshToken, RequestTokenError,
    TokenUrl,
};
use reqwest::{redirect::Policy, Client as HttpClient};
use serde_json::Value;
use tracing::info;

use self::config::OAuthConfig;
use self::session::Session;
use self::store::SessionStore;

mod callback;
mod claims;
mod config;
mod login;
mod session;
mod store;

pub use claims::AccessTokenClaims;
pub use config::Config;
pub use login::Login;

const SESSION_EXPIRED_MSG: &str = "session expired. Run `topk login`.";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Auth {
    oauth_config: OAuthConfig,
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    http: HttpClient,
    store: SessionStore,
}

impl Auth {
    pub fn new(config: &Config, config_dir: PathBuf) -> Result<Self> {
        let oauth_config = config.oauth();
        Ok(Self {
            store: SessionStore::new(oauth_config.key()?, config_dir),
            client: BasicClient::new(ClientId::new(oauth_config.client_id.clone()))
                .set_auth_type(AuthType::RequestBody)
                .set_auth_uri(AuthUrl::from_url(oauth_config.issuer.join("authorize")?))
                .set_token_uri(TokenUrl::from_url(oauth_config.issuer.join("oauth/token")?)),
            http: HttpClient::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .redirect(Policy::none())
                .build()?,
            oauth_config,
        })
    }

    pub async fn login(&self) -> Result<Login<'_>> {
        self.store.lock().await?.prepare()?;
        Login::new(self).await
    }

    pub async fn logout(&self) -> Result<()> {
        self.store.lock().await?.delete()
    }

    pub async fn access_token(&self) -> Result<String> {
        let store = self.store.lock().await?;
        let session = store.load()?.context("not logged in. Run `topk login`.")?;
        if !session.needs_refresh() {
            return Ok(session.access_token);
        }
        let refresh_token = session.refresh_token.context(SESSION_EXPIRED_MSG)?;
        info!("refreshing access token");
        let res = match self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request_async(&self.http)
            .await
        {
            Ok(res) => res,
            Err(RequestTokenError::ServerResponse(error))
                if *error.error() == BasicErrorResponseType::InvalidGrant =>
            {
                store.delete()?;
                bail!(SESSION_EXPIRED_MSG);
            }
            Err(e) => return Err(e).context("refreshing the access token"),
        };
        let session = Session::from_response(res, Some(refresh_token))?;
        let access_token = session.access_token.clone();
        store.save(session)?;
        Ok(access_token)
    }
}

/// Decodes claims (unverified)
fn jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
}
