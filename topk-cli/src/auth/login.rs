use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use oauth2::RedirectUrl;
use tokio::net::TcpListener;
use url::Url;

use super::callback;
use super::oauth::Authorization;
use super::{AccessTokenClaims, Auth};

const TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub struct Login<'a> {
    auth: &'a Auth,
    listener: TcpListener,
    authorization: Authorization,
}

impl<'a> Login<'a> {
    pub(super) async fn new(auth: &'a Auth, ports: &[u16]) -> Result<Self> {
        let addresses: Vec<_> = ports
            .iter()
            .map(|&port| SocketAddr::from(([127, 0, 0, 1], port)))
            .collect();
        let listener = TcpListener::bind(addresses.as_slice())
            .await
            .context("listening for the login callback")?;
        let redirect_uri = RedirectUrl::new(format!("http://{}/callback", listener.local_addr()?))?;
        Ok(Self {
            auth,
            listener,
            authorization: auth.client.authorize(redirect_uri),
        })
    }

    pub fn url(&self) -> &Url {
        &self.authorization.url
    }

    pub async fn finish(self) -> Result<Option<AccessTokenClaims>> {
        callback::run(
            self.listener,
            self.authorization.state.clone(),
            TIMEOUT,
            |code| async move {
                let (session, claims) = self
                    .auth
                    .client
                    .exchange_code(self.authorization, code)
                    .await?;
                self.auth.store.lock().await?.save(session)?;
                Ok(claims)
            },
        )
        .await
    }
}
