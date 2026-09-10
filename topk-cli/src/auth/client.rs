use std::time::Duration;

use anyhow::Result;
use reqwest::{redirect::Policy, Client as HttpClient, StatusCode};
use serde::Deserialize;

use super::config::OAuthConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct Client {
    pub identity: OAuthConfig,
    http: HttpClient,
}

impl Client {
    pub fn new(identity: OAuthConfig) -> Result<Self> {
        Ok(Self {
            identity,
            http: HttpClient::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .redirect(Policy::none())
                .build()?,
        })
    }

    pub async fn post_token(&self, form: &[(&str, &str)]) -> Result<TokenResponse, Error> {
        let res = self
            .http
            .post(
                self.identity
                    .issuer
                    .join("oauth/token")
                    .expect("issuer URL"),
            )
            .form(form)
            .send()
            .await?;
        let status = res.status();
        if status.is_success() {
            return Ok(res.json().await?);
        }
        // Retain the status even when a proxy returns HTML or an empty body.
        let body = res.bytes().await?;
        match serde_json::from_slice(&body) {
            Ok(error) => Err(Error::OAuth { status, error }),
            Err(_) => Err(Error::Status(status)),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize, thiserror::Error)]
#[error("{error}: {error_description}")]
pub(super) struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: String,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("token endpoint returned {status}: {error}")]
    OAuth {
        status: StatusCode,
        error: OAuthError,
    },
    #[error("token endpoint returned {0}")]
    Status(StatusCode),
}

impl Error {
    pub fn is_invalid_grant(&self) -> bool {
        matches!(self, Self::OAuth { error, .. } if error.error == "invalid_grant")
    }
}
