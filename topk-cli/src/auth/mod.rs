use anyhow::{bail, Context, Result};
use tracing::info;

use crate::auth::oauth::OAuthClient;
use crate::config::Config;

mod callback;
mod login;
mod oauth;
mod oauth_config;
mod session;

pub use crate::auth::login::Login;
pub use crate::auth::oauth::AccessTokenClaims;
pub use crate::auth::oauth_config::OAuthConfig;
pub use crate::auth::session::Session;

const SESSION_EXPIRED_MSG: &str = "session expired. Run `topk login`.";

pub struct Auth {
    client: OAuthClient,
    config: Config,
}

impl Auth {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            client: OAuthClient::new(config.oauth().clone())?,
            config,
        })
    }

    pub async fn login(&self, ports: &[u16]) -> Result<Login<'_>> {
        Login::new(self, ports).await
    }

    pub async fn logout(&self) -> Result<()> {
        self.config.session().await?.delete()?;
        self.config.project_tokens().clear()
    }

    pub async fn access_token(&self) -> Result<String> {
        let store = self.config.session().await?;
        let session = store.load()?.context("not logged in. Run `topk login`.")?;
        if !session.needs_refresh() {
            return Ok(session.access_token);
        }
        let refresh_token = session.refresh_token.context(SESSION_EXPIRED_MSG)?;
        info!("refreshing access token");
        let Some(session) = self.client.refresh(refresh_token).await? else {
            store.delete()?;
            bail!(SESSION_EXPIRED_MSG);
        };
        let access_token = session.access_token.clone();
        store.save(session)?;
        Ok(access_token)
    }
}
