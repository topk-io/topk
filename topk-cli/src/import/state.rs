use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::import::error::Error;
use crate::import::spec::Spec;

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
    #[serde(skip)]
    pub id: String,
    /// Redacted; never used to connect.
    pub source: String,
    pub started: DateTime<Utc>,
    /// The whole plan as TOML, done collections included.
    pub spec: String,
    #[serde(default)]
    pub cursors: BTreeMap<String, Cursor>,
}

impl State {
    pub fn new(id: String, source: String, spec: String) -> State {
        State {
            id,
            source,
            started: Utc::now(),
            spec,
            cursors: BTreeMap::new(),
        }
    }

    pub fn reconcile(
        &mut self,
        source: &str,
        spec: &mut Spec,
        plan: String,
    ) -> Result<(usize, BTreeMap<String, String>), Error> {
        if self.source != source {
            return Err(Error::InvalidArgument(format!(
                "run {} reads {}, not {source:?}",
                self.id,
                match self.source.is_empty() {
                    true => "files".to_string(),
                    false => format!("{:?}", self.source),
                }
            )));
        }
        let mut after: BTreeMap<String, String> = BTreeMap::new();
        // A cursor only holds for an unchanged target.
        let stored: Spec = toml::from_str(&self.spec)?;
        self.cursors.retain(|name, cursor| {
            let (Some(target), Some(was)) =
                (spec.collections.get(name), stored.collections.get(name))
            else {
                return false;
            };
            if target != was {
                eprintln!("# {name}: spec changed, starting over");
                return false;
            }
            if let Cursor::After(mark) = cursor {
                after.insert(name.clone(), mark.clone());
            }
            true
        });
        let done = self
            .cursors
            .values()
            .filter(|c| matches!(c, Cursor::Done))
            .count();
        spec.collections
            .retain(|name, _| !matches!(self.cursors.get(name), Some(Cursor::Done)));
        self.spec = plan;
        Ok((done, after))
    }

    pub fn id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        format!("{:08x}", (nanos ^ (std::process::id() as u64) << 32) as u32)
    }

    fn path(id: &str) -> Result<PathBuf, Error> {
        // Tests that fail on purpose must not litter the real one.
        let dir = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => crate::config::dir()
                .ok_or_else(|| Error::InvalidArgument("no config directory".to_string()))?
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
        let mut state: State = toml::from_str(&text)?;
        state.id = id.to_string();
        Ok(state)
    }

    pub fn save(&self) -> Result<(), Error> {
        let path = Self::path(&self.id)?;
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
                if e.kind() != ErrorKind::NotFound {
                    tracing::warn!(%e, path = %path.display(), "cannot remove run state");
                }
            }
        }
    }
}
