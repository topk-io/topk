use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use tracing::info;

use oauth::OAuthClient;
use store::SessionStore;

mod callback;
mod config;
mod login;
mod oauth;
mod session;
mod store;

pub use config::Config;
pub use login::Login;
pub use oauth::AccessTokenClaims;

const SESSION_EXPIRED_MSG: &str = "session expired. Run `topk login`.";

pub struct Auth {
    client: OAuthClient,
    store: SessionStore,
}

impl Auth {
    pub fn new(config: &Config, config_dir: PathBuf) -> Result<Self> {
        let oauth_config = config.oauth();
        Ok(Self {
            store: SessionStore::new(oauth_config.clone(), config_dir),
            client: OAuthClient::new(oauth_config)?,
        })
    }

    pub async fn login(&self, ports: &[u16]) -> Result<Login<'_>> {
        self.store.lock().await?.prepare()?;
        Login::new(self, ports).await
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
        let Some(session) = self.client.refresh(refresh_token).await? else {
            store.delete()?;
            bail!(SESSION_EXPIRED_MSG);
        };
        let access_token = session.access_token.clone();
        store.save(session)?;
        Ok(access_token)
    }
}
