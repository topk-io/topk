use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{ensure, Context, Error, Result};
use chrono::Utc;
use http::header::{HeaderValue, AUTHORIZATION};
use http::Request;
use serde::{Deserialize, Serialize};
use tonic::body::Body;
use tonic::Code;
use tracing::info;

use topk_rs::client::AsyncInterceptor;

use crate::auth::store::{lock, read, write_secret_file, SessionStore};
use crate::auth::Auth;
use crate::management::proto::MintAccessTokenRequest;
use crate::management::Client as ManagementClient;

const PROJECTS_DIR: &str = "projects";
const TOKENS_DIR: &str = "tokens";
const LOCKS_DIR: &str = "locks";
const REFRESH_EARLY_SECS: u64 = 60;

/// Mints project access tokens and caches them with the login session.
pub struct ProjectTokens {
    client: ManagementClient,
    session: SessionStore,
}

impl ProjectTokens {
    /// Mints with `client` and caches the tokens next to `auth`'s session.
    pub fn new(client: ManagementClient, auth: &Auth) -> Self {
        Self {
            client,
            session: auth.store().clone(),
        }
    }

    /// Removes every project's cached token for `auth`'s session. Lock files stay, so a lock
    /// held by another process is never removed.
    pub fn clear(auth: &Auth) -> Result<()> {
        match std::fs::remove_dir_all(tokens_dir(auth.store())) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("clearing project token cache")
            }
            _ => Ok(()),
        }
    }

    /// The cached token for `project_id`, or a newly minted one when it is missing or expiring.
    pub async fn token(&self, project_id: &str) -> Result<ProjectAccessToken> {
        // Project IDs name the cache files directly, so reject anything that could leave the directory.
        ensure!(
            !project_id.is_empty()
                && project_id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "invalid project ID: {project_id:?}"
        );
        if let Some(token) = self.load(project_id)? {
            if !token.needs_refresh() {
                return Ok(token);
            }
        }
        let _lock = self.lock(project_id).await?;
        // Another task or process may have minted while we waited for the lock.
        if let Some(token) = self.load(project_id)? {
            if !token.needs_refresh() {
                return Ok(token);
            }
        }
        info!(project_id, "minting data access token");
        let response = self
            .client
            .tokens
            .clone()
            .mint_access_token(MintAccessTokenRequest {
                project_id: project_id.to_owned(),
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
        let token = ProjectAccessToken {
            token: response.token,
            expires_at: response
                .expires_at
                .try_into()
                .context("invalid token expiry")?,
        };
        write_secret_file(
            &self.token_file(project_id),
            &toml::to_string_pretty(&token)?,
        )?;
        Ok(token)
    }

    /// A missing or corrupt file reads as no token, so the token is minted again.
    fn load(&self, project_id: &str) -> Result<Option<ProjectAccessToken>> {
        Ok(read(&self.token_file(project_id))?.and_then(|raw| toml::from_str(&raw).ok()))
    }

    /// Locks live apart from tokens so clearing tokens never removes a held lock.
    async fn lock(&self, project_id: &str) -> Result<File> {
        lock(self.lock_file(project_id)).await
    }

    fn token_file(&self, project_id: &str) -> PathBuf {
        tokens_dir(&self.session).join(format!("{project_id}.toml"))
    }

    fn lock_file(&self, project_id: &str) -> PathBuf {
        locks_dir(&self.session).join(format!("{project_id}.lock"))
    }
}

/// Where `session`'s project tokens and their locks live.
fn projects_dir(session: &SessionStore) -> PathBuf {
    session.tenant_dir().join(PROJECTS_DIR)
}

fn tokens_dir(session: &SessionStore) -> PathBuf {
    projects_dir(session).join(TOKENS_DIR)
}

fn locks_dir(session: &SessionStore) -> PathBuf {
    projects_dir(session).join(LOCKS_DIR)
}

/// Authorizes each request with one project's access token.
pub struct ProjectTokenInterceptor {
    tokens: Arc<ProjectTokens>,
    project_id: String,
}

impl ProjectTokenInterceptor {
    pub fn new(tokens: Arc<ProjectTokens>, project_id: String) -> Self {
        Self { tokens, project_id }
    }
}

#[tonic::async_trait]
impl AsyncInterceptor for ProjectTokenInterceptor {
    async fn call(&self, request: &mut Request<Body>) -> Result<()> {
        let token = self.tokens.token(&self.project_id).await?;
        let mut header = HeaderValue::from_str(&format!("Bearer {}", token.token))?;
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

impl ProjectAccessToken {
    fn needs_refresh(&self) -> bool {
        self.expires_at <= (Utc::now().timestamp() as u64).saturating_add(REFRESH_EARLY_SECS)
    }
}
