use anyhow::{Context, Error, Result};
use chrono::Utc;
use http::header::{HeaderValue, AUTHORIZATION};
use http::Request;
use serde::{Deserialize, Serialize};
use tonic::body::Body;
use tonic::Code;
use tracing::info;

use topk_rs::client::AsyncInterceptor;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::management::proto::{MintAccessTokenRequest, MintAccessTokenResponse};
use crate::ProjectId;

const REFRESH_EARLY_SECS: u64 = 60;

/// Mints the project's access tokens and caches them with the login session.
pub struct AccessTokenInterceptor {
    mgmt: ManagementClient,
    config: Config,
    project_id: ProjectId,
}

impl AccessTokenInterceptor {
    pub fn new(mgmt: ManagementClient, config: Config, project_id: ProjectId) -> Self {
        Self {
            mgmt,
            config,
            project_id,
        }
    }

    /// The cached token, or a newly minted one when it is missing or expiring.
    pub async fn token(&self) -> Result<AccessToken> {
        let tokens = self.config.access_tokens();
        let project_id = self.project_id.as_str();
        // The common case reads the cache without the lock.
        if let Some(token) = tokens.load(project_id)?.filter(AccessToken::is_fresh) {
            return Ok(token);
        }
        let _lock = tokens.lock(project_id).await?;
        // Whoever held the lock may have minted for everyone waiting on it.
        if let Some(token) = tokens.load(project_id)?.filter(AccessToken::is_fresh) {
            return Ok(token);
        }
        let token = self.mint().await?;
        tokens.save(project_id, &token)?;
        Ok(token)
    }

    async fn mint(&self) -> Result<AccessToken> {
        info!(project_id = %self.project_id, "minting data access token");
        self.mgmt
            .tokens
            .clone()
            .mint_access_token(MintAccessTokenRequest {
                project_id: self.project_id.to_string(),
            })
            .await
            .map_err(|status| {
                let context = match status.code() {
                    Code::Unauthenticated => "session rejected. Run `topk login`.",
                    _ => "minting data access token",
                };
                Error::new(status).context(context)
            })?
            .into_inner()
            .try_into()
    }
}

/// Authorizes each request with the project's access token.
#[tonic::async_trait]
impl AsyncInterceptor for AccessTokenInterceptor {
    async fn call(&self, request: &mut Request<Body>) -> Result<()> {
        let mut header = HeaderValue::from_str(&format!("Bearer {}", self.token().await?.token))?;
        header.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, header);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    /// Unix timestamp in seconds.
    pub expires_at: u64,
}

impl TryFrom<MintAccessTokenResponse> for AccessToken {
    type Error = Error;

    fn try_from(response: MintAccessTokenResponse) -> Result<Self> {
        Ok(Self {
            token: response.token,
            expires_at: response
                .expires_at
                .try_into()
                .context("invalid token expiry")?,
        })
    }
}

impl AccessToken {
    fn is_fresh(&self) -> bool {
        self.expires_at > (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
