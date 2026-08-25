use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::import::error::Error;

/// Where a collection's import stands; the mark is the source's own cursor.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cursor {
    Done,
    After(String),
}

/// Created at confirmation, rewritten at every checkpoint, deleted on success:
/// a file exists only for a run that stopped.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// Redacted; never used to connect.
    pub source: String,
    pub started: chrono::DateTime<chrono::Utc>,
    /// The whole plan as TOML, done collections included.
    pub spec: String,
    #[serde(default)]
    pub cursors: BTreeMap<String, Cursor>,
}

impl State {
    pub fn new(source: String, spec: String) -> State {
        State {
            source,
            started: chrono::Utc::now(),
            spec,
            cursors: BTreeMap::new(),
        }
    }

    pub fn id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        format!("{:08x}", (nanos ^ (std::process::id() as u64) << 32) as u32)
    }

    fn path(id: &str) -> Result<PathBuf, Error> {
        // Tests that fail on purpose must not litter the real one.
        let dir = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => dirs::config_dir()
                .ok_or_else(|| Error::InvalidArgument("no config directory".to_string()))?
                .join("topk")
                .join("import"),
        };
        Ok(dir.join(format!("{id}.toml")))
    }

    pub fn load(id: &str) -> Result<State, Error> {
        let path = Self::path(id)?;
        let text = std::fs::read_to_string(&path).map_err(|_| {
            Error::InvalidArgument(format!(
                "no run {id} to resume ({}) — the id is in the header of the run that stopped",
                path.display()
            ))
        })?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self, id: &str) -> Result<(), Error> {
        let path = Self::path(id)?;
        std::fs::create_dir_all(path.parent().expect("state path has a parent"))?;
        let text = toml::to_string_pretty(self)
            .map_err(|e| Error::InvalidArgument(format!("cannot serialize run state: {e}")))?;
        // Rename is atomic: a crash mid-write leaves the previous checkpoint.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// Best effort: a leftover file is a stale `--resume` target, not a failed import.
    pub fn remove(id: &str) {
        if let Ok(path) = Self::path(id) {
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(%e, path = %path.display(), "cannot remove run state");
                }
            }
        }
    }
}
