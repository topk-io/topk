//! The CLI's configuration directory: the login session, and the project tokens minted with it.

#[cfg(unix)]
use std::fs::Permissions;
use std::fs::{create_dir_all, File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempPath};
use tokio::time::{sleep, timeout};
use toml::Table;

use crate::auth::{OAuthConfig, Session};
use crate::management::ProjectAccessToken;

const LOCK_TIMEOUT: Duration = Duration::from_secs(60);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Clone)]
pub struct Config {
    dir: PathBuf,
    oauth: OAuthConfig,
}

impl Config {
    pub fn new(oauth: OAuthConfig, dir: PathBuf) -> Self {
        Self { oauth, dir }
    }

    pub async fn session(&self) -> Result<LockedSession<'_>> {
        Ok(LockedSession {
            config: self,
            _lock: lock(self.session_lock_file()).await?,
        })
    }

    pub fn project_tokens(&self) -> ProjectTokens {
        ProjectTokens {
            dir: self.tenant_dir().join("projects"),
        }
    }

    pub fn dir() -> Result<PathBuf> {
        std::env::var_os("TOPK_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("topk")))
            .context("no config directory")
    }

    pub fn oauth(&self) -> &OAuthConfig {
        &self.oauth
    }

    fn config_file(&self) -> PathBuf {
        self.dir.join("config.toml")
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
        let Some(raw) = read(&self.config.credentials_file())? else {
            return Ok(None);
        };
        let credentials: Credentials =
            toml::from_str(&raw).context("stored credentials are corrupt")?;
        Ok(Some(credentials.session(&self.config.oauth)?))
    }

    pub fn save(&self, session: Session) -> Result<()> {
        let mut config: Table = match read(&self.config.config_file())? {
            Some(raw) => toml::from_str(&raw).context("invalid config.toml")?,
            None => Table::new(),
        };
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

/// The project access tokens minted with this session, cached per project.
pub struct ProjectTokens {
    dir: PathBuf,
}

impl ProjectTokens {
    /// The project's cached token; a missing or corrupt file reads as `None`.
    pub fn load(&self, project_id: &str) -> Result<Option<ProjectAccessToken>> {
        Ok(read(&self.token_file(project_id))?.and_then(|raw| toml::from_str(&raw).ok()))
    }

    /// Wait for the project token lock.
    pub async fn lock(&self, project_id: &str) -> Result<File> {
        lock(self.lock_file(project_id)).await
    }

    pub fn save(&self, project_id: &str, token: &ProjectAccessToken) -> Result<()> {
        write_toml(&self.token_file(project_id), token)
    }

    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_dir_all(self.tokens_dir()) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("clearing project token cache")
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

/// Poll the OS lock so cancellation never leaves a blocking worker behind.
pub async fn lock(path: PathBuf) -> Result<File> {
    create_dir_all(path.parent().context("lock has no parent directory")?)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    timeout(LOCK_TIMEOUT, async {
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) => sleep(LOCK_POLL_INTERVAL).await,
                Err(TryLockError::Error(e)) => return Err(e.into()),
            }
        }
    })
    .await
    .context("timed out waiting for the lock")?
}

pub fn read(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_secret_file(path, &toml::to_string_pretty(value)?)
}

/// Replace a file atomically so readers cannot observe partial data.
pub fn write_secret_file(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().context("file has no parent")?;
    create_dir_all(parent)?;
    let mut file = NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        file.as_file()
            .set_permissions(Permissions::from_mode(0o600))?;
    }
    file.write_all(content.as_bytes())?;
    file.as_file().sync_all()?;
    // Clear the Windows temporary attribute, then restore cleanup on failure.
    let (_, temp_path) = file.keep()?;
    let temp_path = TempPath::try_from_path(temp_path)?;
    // Unlike tempfile::persist, std supports replacing open files on Windows.
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
