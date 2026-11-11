use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use oauth2::{CsrfToken, RedirectUrl};
use tokio::net::TcpListener;
use url::Url;

use crate::auth::oauth::Authorization;
use crate::auth::{callback, AccessTokenClaims, Auth};

const TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub struct CodeLogin<'a> {
    auth: &'a Auth,
    authorization: Authorization,
}

impl<'a> CodeLogin<'a> {
    fn with_redirect(auth: &'a Auth, redirect_uri: RedirectUrl) -> Self {
        Self {
            auth,
            authorization: auth.client.authorize(redirect_uri),
        }
    }

    pub fn new(auth: &'a Auth) -> Result<Self> {
        Ok(Self::with_redirect(
            auth,
            RedirectUrl::new(auth.config.api_url() + "/auth/code")?,
        ))
    }

    pub fn url(&self) -> &Url {
        &self.authorization.url
    }

    fn state(&self) -> &CsrfToken {
        &self.authorization.state
    }

    pub async fn finish(self, code: String) -> Result<Option<AccessTokenClaims>> {
        let (session, claims) = self
            .auth
            .client
            .exchange_code(self.authorization, code)
            .await?
            .context("login rejected or expired. Run `topk login` again.")?;
        self.auth.config.session().await?.save(session)?;
        // Clear cached access tokens
        self.auth.config.access_tokens().clear()?;
        Ok(claims)
    }
}

pub struct BrowserLogin<'a> {
    login: CodeLogin<'a>,
    listener: TcpListener,
}

impl<'a> BrowserLogin<'a> {
    pub async fn new(auth: &'a Auth, ports: &[u16]) -> Result<Self> {
        let (listener, redirect_uri) = callback_listener(ports).await?;
        Ok(Self {
            login: CodeLogin::with_redirect(auth, redirect_uri),
            listener,
        })
    }

    pub fn url(&self) -> &Url {
        self.login.url()
    }

    pub async fn finish(self) -> Result<Option<AccessTokenClaims>> {
        callback::run(self.listener, self.login.state().clone(), TIMEOUT, |code| {
            self.login.finish(code)
        })
        .await
    }
}

async fn callback_listener(ports: &[u16]) -> Result<(TcpListener, RedirectUrl)> {
    let addresses: Vec<_> = ports
        .iter()
        .map(|&port| SocketAddr::from(([127, 0, 0, 1], port)))
        .collect();
    let listener = TcpListener::bind(addresses.as_slice())
        .await
        .context("listening for the login callback")?;
    let redirect_uri = RedirectUrl::new(format!("http://{}/callback", listener.local_addr()?))?;
    Ok((listener, redirect_uri))
}
