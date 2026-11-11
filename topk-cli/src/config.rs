//! The CLI's configuration: which API to talk to, the login session, and the
//! access tokens minted with it.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use toml::Table;

use crate::auth::{OAuthConfig, Session};
use crate::client::AccessToken;
use crate::host::Host;
use crate::ProjectId;

pub use crate::storage::{lock, read, read_toml, write_secret_file, write_toml};

#[derive(Clone)]
pub struct Config {
    dir: PathBuf,
    host: Host,
    oauth: OAuthConfig,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Defaults {
    pub project_id: Option<ProjectId>,
}

impl Config {
    pub fn new(host: Host, oauth: OAuthConfig, dir: PathBuf) -> Self {
        Self { dir, host, oauth }
    }

    pub async fn session(&self) -> Result<LockedSession<'_>> {
        Ok(LockedSession {
            config: self,
            _lock: lock(self.session_lock_file()).await?,
        })
    }

    pub fn access_tokens(&self) -> AccessTokens {
        AccessTokens {
            dir: self.tenant_dir().join("projects"),
        }
    }

    pub fn dir() -> Result<PathBuf> {
        std::env::var_os("TOPK_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("topk")))
            .context("no config directory")
    }

    pub fn host(&self) -> &str {
        &self.host.host
    }

    pub fn hostname(&self) -> &str {
        self.host
            .host
            .rsplit_once(':')
            .map_or(&self.host.host, |(name, _)| name)
    }

    pub fn port(&self) -> Option<u16> {
        self.host
            .host
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse().ok())
    }

    pub fn https(&self) -> bool {
        self.host.https
    }

    pub fn api_url(&self) -> String {
        let protocol = if self.host.https { "https" } else { "http" };
        format!("{protocol}://api.{}", self.host.host)
    }

    pub fn oauth(&self) -> &OAuthConfig {
        &self.oauth
    }

    pub fn defaults(&self) -> Result<Defaults> {
        Ok(read_toml(&self.defaults_file())
            .context("invalid saved defaults")?
            .unwrap_or_default())
    }

    pub fn default_project(&self) -> Result<ProjectId> {
        self.defaults()?.project_id.context(
            "--project-id (or TOPK_PROJECT_ID environment variable) is required, \
             or select a default project with `topk project select`",
        )
    }

    pub fn set_project_id(&self, project_id: Option<ProjectId>) -> Result<()> {
        write_toml(
            &self.defaults_file(),
            &Defaults {
                project_id,
                ..self.defaults()?
            },
        )
    }

    fn config_file(&self) -> PathBuf {
        self.dir.join("config.toml")
    }

    fn defaults_file(&self) -> PathBuf {
        let key = Sha256::digest(format!("{}:{}", self.host.https, self.host.host).as_bytes());
        self.tenant_dir()
            .join("defaults")
            .join(format!("{key:x}.toml"))
    }

    /// Everything scoped to this issuer's session lives here.
    fn tenant_dir(&self) -> PathBuf {
        self.dir.join("tenants").join(self.oauth.issuer_key())
    }

    fn credentials_file(&self) -> PathBuf {
        self.tenant_dir().join("credentials.toml")
    }

    fn session_lock_file(&self) -> PathBuf {
        self.tenant_dir().join("session.lock")
    }
}

pub struct LockedSession<'a> {
    config: &'a Config,
    _lock: File,
}

impl LockedSession<'_> {
    pub fn load(&self) -> Result<Option<Session>> {
        let credentials: Option<Credentials> =
            read_toml(&self.config.credentials_file()).context("stored credentials are corrupt")?;
        credentials
            .map(|credentials| credentials.session(&self.config.oauth))
            .transpose()
    }

    pub fn save(&self, session: Session) -> Result<()> {
        let mut config: Table = read_toml(&self.config.config_file())
            .context("invalid config.toml")?
            .unwrap_or_default();
        write_toml(
            &self.config.credentials_file(),
            &Credentials::new(&self.config.oauth, session),
        )?;
        if config.remove("api_key").is_some() {
            write_toml(&self.config.config_file(), &config)?;
        }
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        remove(&self.config.credentials_file())?;
        remove(&self.config.config_file())
    }
}

/// The access tokens minted with this session, cached per project.
pub struct AccessTokens {
    dir: PathBuf,
}

impl AccessTokens {
    /// The project's cached token; a missing or corrupt file reads as `None`.
    pub fn load(&self, project_id: &str) -> Result<Option<AccessToken>> {
        Ok(read(&self.token_file(project_id))?.and_then(|raw| toml::from_str(&raw).ok()))
    }

    /// Wait for the project's token lock.
    pub async fn lock(&self, project_id: &str) -> Result<File> {
        lock(self.lock_file(project_id)).await
    }

    pub fn save(&self, project_id: &str, token: &AccessToken) -> Result<()> {
        write_toml(&self.token_file(project_id), token)
    }

    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_dir_all(self.tokens_dir()) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("clearing access token cache")
            }
            _ => Ok(()),
        }
    }

    fn tokens_dir(&self) -> PathBuf {
        self.dir.join("tokens")
    }

    fn token_file(&self, project_id: &str) -> PathBuf {
        self.tokens_dir().join(format!("{project_id}.toml"))
    }

    fn lock_file(&self, project_id: &str) -> PathBuf {
        self.dir.join("locks").join(format!("{project_id}.lock"))
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    audience: String,
    #[serde(flatten)]
    session: Session,
}

impl Credentials {
    fn new(oauth: &OAuthConfig, session: Session) -> Self {
        Self {
            client_id: oauth.client_id.clone(),
            audience: oauth.audience.clone(),
            session,
        }
    }

    fn session(self, oauth: &OAuthConfig) -> Result<Session> {
        ensure!(
            self.client_id == oauth.client_id && self.audience == oauth.audience,
            "authentication configuration mismatch"
        );
        Ok(self.session)
    }
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}
