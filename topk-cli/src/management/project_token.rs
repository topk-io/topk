use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Context, Error, Result};
use chrono::Utc;
use http::header::{HeaderValue, AUTHORIZATION};
use http::Request;
use serde::{Deserialize, Serialize};
use tonic::body::Body;
use tonic::Code;
use tracing::info;

use topk_rs::client::AsyncInterceptor;

use crate::auth::store::{lock, read, write_secret_file, SessionStore};
use crate::endpoint::ProjectId;
use crate::management::proto::{MintAccessTokenRequest, MintAccessTokenResponse};
use crate::management::Client as ManagementClient;

const REFRESH_EARLY_SECS: u64 = 60;

/// Mints project access tokens and caches them with the login session.
pub struct ProjectToken {
    client: ManagementClient,
    session: SessionStore,
    project_id: ProjectId,
}

impl ProjectToken {
    pub fn new(mgmt: ManagementClient, sessions: SessionStore, project_id: ProjectId) -> Self {
        Self {
            client: mgmt,
            session: sessions,
            project_id,
        }
    }

    /// The cached token, or a newly minted one when it is missing or expiring.
    pub async fn token(&self) -> Result<ProjectAccessToken> {
        if let Some(token) = self.load()? {
            if !token.needs_refresh() {
                return Ok(token);
            }
        }
        let _lock = self.lock().await?;
        // Another task or process may have minted while we waited for the lock.
        if let Some(token) = self.load()? {
            if !token.needs_refresh() {
                return Ok(token);
            }
        }
        info!(project_id = %self.project_id, "minting data access token");
        let response = self
            .client
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
            .into_inner();
        let token = ProjectAccessToken::try_from(response)?;
        write_secret_file(&self.token_file(), &toml::to_string_pretty(&token)?)?;
        Ok(token)
    }

    /// A missing or corrupt file reads as no token, so the token is minted again.
    fn load(&self) -> Result<Option<ProjectAccessToken>> {
        Ok(read(&self.token_file())?.and_then(|raw| toml::from_str(&raw).ok()))
    }

    async fn lock(&self) -> Result<File> {
        lock(self.lock_file()).await
    }

    fn token_file(&self) -> PathBuf {
        Self::tokens_dir(&self.session).join(format!("{}.toml", self.project_id))
    }

    /// Locks live apart from tokens so clearing tokens never removes a held lock.
    fn lock_file(&self) -> PathBuf {
        self.session
            .tenant_dir()
            .join("projects/locks")
            .join(format!("{}.lock", self.project_id))
    }

    /// Removes every project's cached token for `auth`'s session. Lock files stay, so a lock
    /// held by another process is never removed.
    pub fn clear_all(sessions: &SessionStore) -> Result<()> {
        match std::fs::remove_dir_all(Self::tokens_dir(sessions)) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("clearing project token cache")
            }
            _ => Ok(()),
        }
    }

    /// Where `session`'s project tokens live; `clear_all` removes exactly this directory.
    fn tokens_dir(session: &SessionStore) -> PathBuf {
        session.tenant_dir().join("projects/tokens")
    }
}

/// Authorizes each request with the project's access token.
#[tonic::async_trait]
impl AsyncInterceptor for ProjectToken {
    async fn call(&self, request: &mut Request<Body>) -> Result<()> {
        let mut header = HeaderValue::from_str(&format!("Bearer {}", self.token().await?.token))?;
        header.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, header);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProjectAccessToken {
    pub token: String,
    /// Unix timestamp in seconds.
    pub expires_at: u64,
}

impl TryFrom<MintAccessTokenResponse> for ProjectAccessToken {
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

impl ProjectAccessToken {
    fn needs_refresh(&self) -> bool {
        self.expires_at <= (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
