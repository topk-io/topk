#[cfg(unix)]
use std::fs::Permissions;
use std::fs::{create_dir_all, File, OpenOptions, TryLockError};
use std::io::{ErrorKind, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use tempfile::{NamedTempFile, TempPath};
use tokio::time::{sleep, timeout};

const LOCK_TIMEOUT: Duration = Duration::from_secs(60);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Poll the OS lock so cancellation never leaves a blocking worker behind.
pub(crate) async fn lock(path: PathBuf) -> Result<File> {
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

pub(crate) fn read(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

pub(crate) fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}

/// Replace a file atomically so readers cannot observe partial data.
pub(crate) fn write_secret_file(path: &Path, content: &str) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use std::io::Read;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn replacement_preserves_open_readers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.toml");
        write_secret_file(&path, "old session").unwrap();
        let mut reader = File::open(&path).unwrap();

        write_secret_file(&path, "new session").unwrap();

        let mut old = String::new();
        reader.read_to_string(&mut old).unwrap();
        assert_eq!(old, "old session");
        assert_eq!(read(&path).unwrap().as_deref(), Some("new session"));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replacement_cleans_up_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.toml");
        create_dir_all(&path).unwrap();
        std::fs::write(path.join("existing"), "preserve me").unwrap();

        assert!(write_secret_file(&path, "new session").is_err());

        assert_eq!(
            std::fs::read_to_string(path.join("existing")).unwrap(),
            "preserve me"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn atomic_replacement_never_exposes_partial_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        write_secret_file(&path, "{\"value\":0}").unwrap();
        std::thread::scope(|scope| {
            let path = &path;
            let writer = scope.spawn(move || {
                for n in 1..50 {
                    write_secret_file(
                        path,
                        &serde_json::json!({"value": n, "data": "x".repeat(4096)}).to_string(),
                    )
                    .unwrap();
                }
            });
            for _ in 0..100 {
                let value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
                assert!(value["value"].is_number());
            }
            writer.join().unwrap();
        });
    }

    // Invoked in a separate test process by the parent below.
    #[tokio::test]
    async fn lock_child() {
        let Some(dir) = std::env::var_os("TOPK_TEST_LOCK_DIR") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        std::fs::write(dir.join("ready"), "").unwrap();
        let _guard = lock(dir.join("shared.lock")).await.unwrap();
        std::fs::write(dir.join("acquired"), "").unwrap();
    }

    #[tokio::test]
    async fn lock_coordinates_separate_processes() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock(dir.path().join("shared.lock")).await.unwrap();
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "file::tests::lock_child"])
            .env("TOPK_TEST_LOCK_DIR", dir.path())
            .stdout(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !dir.path().join("ready").exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!dir.path().join("acquired").exists());
        drop(guard);
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        assert!(dir.path().join("acquired").exists());
    }

    #[cfg(unix)]
    #[test]
    fn write_secret_file_uses_restrictive_permissions() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("credentials.json");

        write_secret_file(&path, "{}").expect("write file");

        let mode = std::fs::metadata(&path)
            .expect("stat file")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
