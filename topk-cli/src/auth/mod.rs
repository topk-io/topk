//! Browser login and stored access tokens, refreshed under a cross-process lock.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Value;
use tracing::info;

use self::client::Client;
use self::session::Session;
use self::store::SessionStore;
use crate::config as cli_config;

mod callback;
mod claims;
mod client;
mod config;
mod login;
mod session;
mod store;
mod util;

pub use claims::AccessTokenClaims;
pub use config::Config;
pub use login::Login;
pub use store::CredentialsStore;

const SESSION_EXPIRED: &str = "session expired. Run `topk login`.";
pub struct Auth {
    client: Client,
    store: SessionStore,
}

impl Auth {
    pub fn new(config: &Config) -> Result<Self> {
        let oauth_config = config.oauth();

        Ok(Self {
            store: SessionStore::new(
                oauth_config.key()?,
                config.store,
                cli_config::dir().context("no config directory")?,
            ),
            client: Client::new(oauth_config)?,
        })
    }

    pub async fn login(&self) -> Result<Login<'_>> {
        self.store.lock().await?.prepare()?;
        Login::new(self).await
    }

    pub async fn logout(&self) -> Result<()> {
        self.store.lock().await?.delete()
    }

    pub fn audience(&self) -> &str {
        &self.client.identity.audience
    }

    pub async fn access_token(&self) -> Result<String> {
        let mut store = self.store.lock().await?;
        let session = store.load()?.context("not logged in. Run `topk login`.")?;
        if !session.needs_refresh() {
            return Ok(session.access_token);
        }

        let refresh_token = session.refresh_token.context(SESSION_EXPIRED)?;

        info!("refreshing access token");

        let res = match self
            .client
            .post_token(&[
                ("grant_type", "refresh_token"),
                ("client_id", &self.client.identity.client_id),
                ("refresh_token", &refresh_token),
            ])
            .await
        {
            Ok(res) => res,
            Err(e) if e.is_invalid_grant() => {
                store.delete()?;
                bail!(SESSION_EXPIRED);
            }
            Err(e) => return Err(e).context("refreshing the access token"),
        };

        let session = Session::from_response(res, Some(refresh_token));
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

#[cfg(test)]
#[path = "../../tests/auth/session.rs"]
mod tests;
