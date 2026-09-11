use std::fs::File;
use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use toml::Table;

use super::session::Session;
use crate::file;

pub(super) struct SessionStore {
    oauth_config_key: String,
    config_file: PathBuf,
    credentials_file: PathBuf,
    lock_file: PathBuf,
}

impl SessionStore {
    pub fn new(oauth_config_key: String, config_dir: PathBuf) -> Self {
        Self {
            oauth_config_key,
            config_file: config_dir.join("config.toml"),
            credentials_file: config_dir.join("credentials.toml"),
            lock_file: config_dir.join("session.lock"),
        }
    }

    pub async fn lock(&self) -> Result<LockedSessionStore<'_>> {
        let lock = file::lock(self.lock_file.clone()).await?;
        Ok(LockedSessionStore {
            store: self,
            _lock: lock,
        })
    }
}

pub(super) struct LockedSessionStore<'a> {
    store: &'a SessionStore,
    _lock: File,
}

impl LockedSessionStore<'_> {
    /// Check readability before starting browser login.
    pub fn prepare(&self) -> Result<()> {
        file::read(&self.store.credentials_file)?;
        Ok(())
    }

    pub fn load(&self) -> Result<Option<Session>> {
        let Some(raw) = file::read(&self.store.credentials_file)? else {
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

    pub fn save(&self, session: Session) -> Result<()> {
        let mut config: Table = match file::read(&self.store.config_file)? {
            Some(raw) => toml::from_str(&raw).context("invalid config.toml")?,
            None => Table::new(),
        };
        let raw = toml::to_string_pretty(&Credentials {
            store_key: self.store.oauth_config_key.clone(),
            session,
        })?;
        file::write_secret_file(&self.store.credentials_file, &raw)?;
        if config.remove("api_key").is_some() {
            file::write_secret_file(&self.store.config_file, &toml::to_string_pretty(&config)?)?;
        }
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        file::remove(&self.store.credentials_file)?;
        file::remove(&self.store.config_file)
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    store_key: String,
    #[serde(flatten)]
    session: Session,
}
