#[cfg(unix)]
use std::fs::Permissions;
use std::fs::{create_dir_all, File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tempfile::{NamedTempFile, TempPath};
use tokio::time::{sleep, timeout};

const LOCK_TIMEOUT: Duration = Duration::from_secs(60);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(25);

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

pub fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    read(path)?
        .map(|raw| toml::from_str(&raw).with_context(|| format!("parsing {}", path.display())))
        .transpose()
}

pub fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
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
