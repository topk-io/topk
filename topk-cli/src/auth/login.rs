use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use url::Url;

use super::callback;
use super::session::Session;
use super::util::{random_urlsafe, s256};
use super::{AccessTokenClaims, Auth};

// Configured callback URLs (127.0.0.1) ports
const PORTS: [u16; 3] = [38123, 38124, 38125];
// OAuth scope for the login
const SCOPE: &str = "openid profile email offline_access";
// Timeout for the login
const TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub struct Login<'a> {
    auth: &'a Auth,
    listener: TcpListener,
    url: Url,
    redirect_uri: String,
    verifier: String,
    state: String,
}

impl<'a> Login<'a> {
    pub(super) async fn new(auth: &'a Auth) -> Result<Self> {
        let addresses = PORTS.map(|port| SocketAddr::from(([127, 0, 0, 1], port)));
        let listener = TcpListener::bind(addresses.as_slice())
            .await
            .with_context(|| format!("cannot listen on 127.0.0.1 ports {PORTS:?}"))?;
        let verifier = random_urlsafe();
        let state = random_urlsafe();
        let redirect_uri = format!(
            "http://127.0.0.1:{}/callback",
            listener.local_addr()?.port()
        );
        let mut url = auth.client.identity.issuer.join("authorize")?;
        url.query_pairs_mut().extend_pairs([
            ("response_type", "code"),
            ("client_id", auth.client.identity.client_id.as_str()),
            ("redirect_uri", &redirect_uri),
            ("scope", SCOPE),
            ("audience", &auth.client.identity.audience),
            ("code_challenge", &s256(&verifier)),
            ("code_challenge_method", "S256"),
            ("state", &state),
            ("prompt", "login"),
        ]);
        Ok(Self {
            auth,
            listener,
            url,
            redirect_uri,
            verifier,
            state,
        })
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub async fn finish(self) -> Result<Option<AccessTokenClaims>> {
        callback::run(self.listener, self.state, TIMEOUT, |code| async move {
            let res = self
                .auth
                .client
                .post_token(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", &self.auth.client.identity.client_id),
                    ("code", &code),
                    ("code_verifier", &self.verifier),
                    ("redirect_uri", &self.redirect_uri),
                ])
                .await
                .context("exchanging the authorization code")?;
            let session = Session::from_response(res, None);
            let claims = AccessTokenClaims::parse(&session.access_token, self.auth.audience());
            self.auth.store.lock().await?.save(session)?;
            Ok(claims)
        })
        .await
    }
}

#[cfg(test)]
#[path = "../../tests/auth/login.rs"]
mod tests;
