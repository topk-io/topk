use anyhow::Result;
use chrono::Utc;
use http::header::{HeaderValue, AUTHORIZATION};
use http::Request;
use serde::{Deserialize, Serialize};
use tonic::body::Body;
use tonic::Code;
use tracing::debug;

use topk_rs::client::AsyncInterceptor;

use crate::client::{Error, ManagementClient};
use crate::config::Config;
use crate::ProjectId;

const REFRESH_EARLY_SECS: u64 = 60;

/// Mints the project's access tokens and caches them with the login session.
pub struct AccessTokenProvider {
    mgmt: ManagementClient,
    config: Config,
    project_id: ProjectId,
}

impl AccessTokenProvider {
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
        debug!(project_id = %self.project_id, "minting data access token");
        self.mgmt
            .tokens()
            .mint(&self.project_id)
            .await
            .map_err(|error| {
                let context = match &error {
                    Error::Status(status) if status.code() == Code::Unauthenticated => {
                        "session rejected. Run `topk login`."
                    }
                    _ => "minting data access token",
                };
                anyhow::Error::new(error).context(context)
            })
    }
}

#[tonic::async_trait]
impl AsyncInterceptor for AccessTokenProvider {
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

impl AccessToken {
    fn is_fresh(&self) -> bool {
        self.expires_at > (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
