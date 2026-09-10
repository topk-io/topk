use std::fs::File;
use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Result};
use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};

use super::session::Session;
use crate::config::Config;

mod file;

const KEYRING_SERVICE: &str = "TopK CLI";
const KEYRING_USERNAME: &str = "session";

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialsStore {
    /// Keyring when available, otherwise file. macOS and Windows default to file.
    #[default]
    #[serde(skip)]
    Auto,
    File,
    Keyring,
}

impl CredentialsStore {
    fn resolve(self) -> CredentialsStore {
        match self {
            Self::Auto if cfg!(any(target_os = "macos", windows)) => Self::File,
            Self::Auto => match Storage::Keyring.read() {
                Ok(_) => Self::Keyring,
                Err(_) => Self::File,
            },
            store => store,
        }
    }
}

pub(super) struct SessionStore {
    oauth_config_key: String,
    default_store: CredentialsStore,
    config_dir: PathBuf,
}

impl SessionStore {
    pub fn new(
        oauth_config_key: String,
        default_store: CredentialsStore,
        config_dir: PathBuf,
    ) -> Self {
        Self {
            oauth_config_key,
            default_store,
            config_dir,
        }
    }

    pub async fn lock(&self) -> Result<LockedSessionStore<'_>> {
        let lock = file::lock(self.config_dir.join("session.lock")).await?;
        let config = match file::read(&self.config_dir.join("config.toml"))? {
            Some(raw) => toml::from_str(&raw).context("invalid config.toml")?,
            None => Config::default(),
        };
        Ok(LockedSessionStore {
            store: self,
            config,
            _lock: lock,
        })
    }
}

pub(super) struct LockedSessionStore<'a> {
    store: &'a SessionStore,
    config: Config,
    _lock: File,
}

impl LockedSessionStore<'_> {
    /// Resolve and persist the credentials store before asking the user to authenticate.
    pub fn prepare(&mut self) -> Result<()> {
        let storage = self.get_storage()?;
        // Check readability before persisting the choice and starting browser login.
        storage.read()?;
        if self.config.store.is_none() {
            self.config.store = Some(storage.kind());
            self.write_config()?;
        }
        Ok(())
    }

    pub fn load(&self) -> Result<Option<Session>> {
        let storage = self.get_storage()?;
        let Some(raw) = storage.read()? else {
            return Ok(None);
        };
        let credentials: Credentials =
            toml::from_str(&raw).context("stored credentials are corrupt")?;
        ensure!(
            credentials.store_key == self.store.oauth_config_key,
            "authentication configuration mismatch"
        );
        Ok(Some(credentials.session))
    }

    pub fn save(&mut self, session: Session) -> Result<()> {
        if self.config.store.is_none() {
            self.prepare()?;
        }
        let raw = toml::to_string_pretty(&Credentials {
            store_key: self.store.oauth_config_key.clone(),
            session,
        })?;
        self.get_storage()?.write(&raw)?;
        if self.config.extra.remove("api_key").is_some() {
            self.write_config()?;
        }
        Ok(())
    }

    pub fn delete(&mut self) -> Result<()> {
        if let Some(store) = self.config.store {
            self.storage(store)?.delete()?;
        }
        // Only clear the stored backend choice after credentials are deleted.
        self.config.store = None;
        // Remove legacy API key from config
        self.config.extra.remove("api_key");
        self.write_config()
    }

    fn get_storage(&self) -> Result<Storage> {
        self.storage(
            self.config
                .store
                .unwrap_or_else(|| self.store.default_store.resolve()),
        )
    }

    fn storage(&self, store: CredentialsStore) -> Result<Storage> {
        match store {
            CredentialsStore::File => Ok(Storage::File(
                self.store.config_dir.join("credentials.toml"),
            )),
            CredentialsStore::Keyring => Ok(Storage::Keyring),
            CredentialsStore::Auto => bail!("credentials store must be resolved before use"),
        }
    }

    fn write_config(&self) -> Result<()> {
        let path = self.store.config_dir.join("config.toml");
        if self.config.store.is_none() && self.config.extra.is_empty() {
            file::remove(&path)
        } else {
            file::write_secret_file(&path, &toml::to_string_pretty(&self.config)?)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    store_key: String,
    #[serde(flatten)]
    session: Session,
}

enum Storage {
    File(PathBuf),
    Keyring,
}

impl Storage {
    fn kind(&self) -> CredentialsStore {
        match self {
            Self::File(_) => CredentialsStore::File,
            Self::Keyring => CredentialsStore::Keyring,
        }
    }

    fn read(&self) -> Result<Option<String>> {
        match self {
            Self::File(path) => file::read(path),
            Self::Keyring => {
                ensure!(
                    !cfg!(windows),
                    "keyring storage cannot accommodate token sizes on Windows; use file storage"
                );
                match Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?.get_password() {
                    Ok(raw) => Ok(Some(raw)),
                    Err(KeyringError::NoEntry) => Ok(None),
                    Err(e) => Err(e).context("reading the keyring"),
                }
            }
        }
    }

    fn write(&self, raw: &str) -> Result<()> {
        match self {
            Self::File(path) => file::write_secret_file(path, raw),
            Self::Keyring => Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?
                .set_password(raw)
                .context("writing the keyring"),
        }
    }

    fn delete(&self) -> Result<()> {
        match self {
            Self::File(path) => file::remove(path),
            Self::Keyring => {
                match Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)?.delete_credential() {
                    Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                    Err(e) => Err(e).context("deleting the keyring credentials"),
                }
            }
        }
    }
}
