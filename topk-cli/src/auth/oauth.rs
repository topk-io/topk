use std::borrow::Cow;
use std::time::Duration;

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use oauth2::basic::{BasicClient, BasicErrorResponseType, BasicTokenResponse};
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, RequestTokenError, Scope,
    TokenResponse, TokenUrl,
};
use reqwest::{redirect::Policy, Client as HttpClient};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::auth::session::Session;

mod claims;
pub use claims::AccessTokenClaims;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];

/// Resolved issuer, OAuth client, and API audience used for authentication.
#[derive(Clone)]
pub(crate) struct OAuthConfig {
    pub issuer: Url,
    pub client_id: String,
    pub audience: String,
}

impl OAuthConfig {
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
    ) -> Result<(Session, Option<AccessTokenClaims>)> {
        let res = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(authorization.verifier)
            .set_redirect_uri(Cow::Owned(authorization.redirect_uri))
            .request_async(&self.http)
            .await
            .context("exchanging the authorization code")?;
        let session = session_from_response(res)?;
        let claims = AccessTokenClaims::parse(&session.access_token, &self.audience);
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
