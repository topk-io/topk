use anyhow::{bail, Context, Result};
use tracing::info;

use crate::auth::oauth::{Authorization, OAuthClient};
use crate::config::Config;

mod callback;
mod login;
mod oauth;
mod session;

pub use crate::auth::login::{BrowserLogin, CodeLogin};
pub use crate::auth::oauth::{AccessTokenClaims, OAuthConfig};
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

    pub async fn browser_login(&self, ports: &[u16]) -> Result<BrowserLogin<'_>> {
        BrowserLogin::new(self, ports).await
    }

    pub fn browserless_login(&self) -> Result<CodeLogin<'_>> {
        CodeLogin::new(self)
    }

    async fn exchange_code(
        &self,
        authorization: Authorization,
        code: String,
    ) -> Result<AccessTokenClaims> {
        let (session, claims) = self.client.exchange_code(authorization, code).await?;
        self.config.session().await?.save(session)?;
        // Clear cached access tokens
        self.config.access_tokens().clear()?;
        Ok(claims)
    }

    pub async fn logout(&self) -> Result<()> {
        self.config.session().await?.delete()?;
        self.config.access_tokens().clear()
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
