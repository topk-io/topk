use std::borrow::Cow;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
};
use tokio::net::TcpListener;
use url::Url;

use super::callback;
use super::session::Session;
use super::{AccessTokenClaims, Auth};

// Configured callback URLs (127.0.0.1) ports
const PORTS: [u16; 3] = [38123, 38124, 38125];
const SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];
const TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub struct Login<'a> {
    auth: &'a Auth,
    listener: TcpListener,
    url: Url,
    redirect_uri: RedirectUrl,
    verifier: PkceCodeVerifier,
    state: CsrfToken,
}

impl<'a> Login<'a> {
    pub(super) async fn new(auth: &'a Auth) -> Result<Self> {
        let addresses = PORTS.map(|port| SocketAddr::from(([127, 0, 0, 1], port)));
        let listener = TcpListener::bind(addresses.as_slice())
            .await
            .with_context(|| format!("cannot listen on 127.0.0.1 ports {PORTS:?}"))?;
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let redirect_uri = RedirectUrl::new(format!(
            "http://127.0.0.1:{}/callback",
            listener.local_addr()?.port()
        ))?;
        let (url, state) = auth
            .client
            .authorize_url(CsrfToken::new_random)
            .set_redirect_uri(Cow::Borrowed(&redirect_uri))
            .add_scopes(SCOPES.map(|scope| Scope::new(scope.to_string())))
            .add_extra_param("audience", &auth.oauth_config.audience)
            .add_extra_param("prompt", "login")
            .set_pkce_challenge(challenge)
            .url();
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
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(self.verifier)
                .set_redirect_uri(Cow::Owned(self.redirect_uri))
                .request_async(&self.auth.http)
                .await
                .context("exchanging the authorization code")?;
            let session = Session::from_response(res, None)?;
            let claims =
                AccessTokenClaims::parse(&session.access_token, &self.auth.oauth_config.audience);
            self.auth.store.lock().await?.save(session)?;
            Ok(claims)
        })
        .await
    }
}
