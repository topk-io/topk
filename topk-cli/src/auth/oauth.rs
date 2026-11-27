use std::borrow::Cow;
use std::time::Duration;

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use clap::builder::NonEmptyStringValueParser;
use oauth2::basic::{BasicClient, BasicErrorResponseType, BasicTokenResponse};
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RequestTokenError, Scope,
    TokenResponse, TokenUrl,
};
use reqwest::{redirect::Policy, Client as HttpClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::auth::session::Session;
use crate::host::{Host, DEFAULT_HOST};

mod claims;
pub use claims::AccessTokenClaims;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];

const AUTH_ISSUER: &str = "https://topk-prod.us.auth0.com/";
const AUTH_CLIENT_ID: &str = "2LqddiN2N5fQplfMP2MIYPHM6ttFNeaG";
const AUTH_AUDIENCE: &str = "https://api.topk.io";

#[derive(clap::Args, Clone, Debug)]
pub struct OAuthArgs {
    #[arg(
        long = "auth-issuer",
        env = "TOPK_AUTH_ISSUER",
        requires_all = ["client_id", "audience"],
        hide = true,
        global = true
    )]
    issuer: Option<Url>,
    #[arg(
        long = "auth-client-id",
        env = "TOPK_AUTH_CLIENT_ID",
        value_parser = NonEmptyStringValueParser::new(),
        requires_all = ["issuer", "audience"],
        hide = true,
        global = true
    )]
    client_id: Option<String>,
    #[arg(
        long = "auth-audience",
        env = "TOPK_AUTH_AUDIENCE",
        requires_all = ["issuer", "client_id"],
        hide = true,
        global = true
    )]
    audience: Option<String>,
}

impl OAuthArgs {
    pub fn config(self) -> Option<OAuthConfig> {
        Some(OAuthConfig {
            issuer: self.issuer?,
            client_id: self.client_id?,
            audience: self.audience?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub issuer: Url,
    #[serde(rename = "cli_client_id")]
    pub client_id: String,
    pub audience: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            issuer: AUTH_ISSUER.parse().expect("valid issuer URL"),
            client_id: AUTH_CLIENT_ID.into(),
            audience: AUTH_AUDIENCE.into(),
        }
    }
}

impl OAuthConfig {
    pub async fn resolve(host: &Host) -> Result<Self> {
        match host.host.as_str() {
            DEFAULT_HOST => Ok(Self::default()),
            _ => {
                let url = format!("{}/auth/config.json", host.api_url());
                Self::fetch(&url)
                    .await
                    .with_context(|| format!("cannot load login settings from {url}"))
            }
        }
    }

    async fn fetch(url: &str) -> reqwest::Result<Self> {
        HttpClient::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    pub fn issuer_key(&self) -> String {
        format!("{:x}", Sha256::digest(self.issuer.as_str().as_bytes()))
    }
}

pub(crate) struct OAuthClient {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    http: HttpClient,
    audience: String,
}

pub struct Authorization {
    pub url: Url,
    pub state: CsrfToken,
    redirect_uri: RedirectUrl,
    verifier: PkceCodeVerifier,
}

impl OAuthClient {
    pub fn new(config: OAuthConfig) -> Result<Self> {
        Ok(Self {
            client: BasicClient::new(ClientId::new(config.client_id))
                .set_auth_type(AuthType::RequestBody)
                .set_auth_uri(AuthUrl::from_url(config.issuer.join("authorize")?))
                .set_token_uri(TokenUrl::from_url(config.issuer.join("oauth/token")?)),
            http: HttpClient::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .redirect(Policy::none())
                .build()?,
            audience: config.audience,
        })
    }

    pub fn authorize(&self, redirect_uri: RedirectUrl) -> Authorization {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .set_redirect_uri(Cow::Borrowed(&redirect_uri))
            .add_scopes(SCOPES.map(|scope| Scope::new(scope.to_string())))
            .add_extra_param("audience", &self.audience)
            .add_extra_param("prompt", "login")
            .set_pkce_challenge(challenge)
            .url();
        Authorization {
            url,
            state,
            redirect_uri,
            verifier,
        }
    }

    pub async fn exchange_code(
        &self,
        authorization: Authorization,
        code: String,
    ) -> Result<(Session, AccessTokenClaims)> {
        let res = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(authorization.verifier)
            .set_redirect_uri(Cow::Owned(authorization.redirect_uri))
            .request_async(&self.http)
            .await
            .context("exchanging the authorization code failed. Run `topk login` again.")?;
        let session = session_from_response(res)?;
        let claims = AccessTokenClaims::parse(&session.access_token, &self.audience)?;
        Ok((session, claims))
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<Option<Session>> {
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
                return Ok(None)
            }
            Err(e) => return Err(e).context("refreshing the access token"),
        };
        let mut session = session_from_response(res)?;
        session.refresh_token = session.refresh_token.or(Some(refresh_token));
        Ok(Some(session))
    }
}

fn session_from_response(res: BasicTokenResponse) -> Result<Session> {
    Ok(Session {
        access_token: res.access_token().secret().clone(),
        refresh_token: res.refresh_token().map(|token| token.secret().clone()),
        expires_at: res
            .expires_in()
            .context("token response is missing expires_in")?
            .as_secs()
            .saturating_add(Utc::now().timestamp() as u64),
    })
}

/// Decodes claims (unverified)
fn jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
}
