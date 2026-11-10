use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::import::error::Error;
use crate::import::source::Cursor;
use crate::import::spec::Spec;
use crate::ProjectId;

/// Where a collection's import stands.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Done,
    After(Checkpoint),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(flatten)]
    pub cursor: Cursor,
    #[serde(default)]
    pub consumed: u64,
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
    /// The project the run writes to.
    pub project_id: ProjectId,
    #[serde(default)]
    pub cursors: BTreeMap<String, Mark>,
}

impl State {
    /// A fresh run of the whole `spec`.
    pub fn new(source: &str, project_id: ProjectId, spec: &Spec) -> Result<State, Error> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        Ok(State {
            id: format!("{:08x}", nanos as u32 ^ std::process::id()),
            source: source.to_string(),
            started: Utc::now(),
            spec: toml::to_string_pretty(spec)
                .map_err(|e| Error::InvalidArgument(format!("cannot serialize spec: {e}")))?,
            project_id,
            cursors: BTreeMap::new(),
        })
    }

    /// Checks that this invocation continues the stopped run, drops its done
    /// collections from `spec`, and returns how many were done and where the
    /// rest stopped.
    pub fn resume(
        &mut self,
        source: &str,
        project_id: &ProjectId,
        spec: &mut Spec,
    ) -> Result<(usize, BTreeMap<String, Checkpoint>), Error> {
        let plan = toml::to_string_pretty(spec)
            .map_err(|e| Error::InvalidArgument(format!("cannot serialize spec: {e}")))?;
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
        if *project_id != self.project_id {
            return Err(Error::InvalidArgument(format!(
                "run {} was started for project {started}; resume it with --project-id {started}",
                self.id,
                started = self.project_id,
            )));
        }
        let mut after: BTreeMap<String, Checkpoint> = BTreeMap::new();
        // A cursor only holds for an unchanged target.
        let stored: Spec = toml::from_str(&self.spec)?;
        self.cursors.retain(|name, cursor| {
            let (target, was) = match (spec.collections.get(name), stored.collections.get(name)) {
                (Some(target), Some(was)) => (target, was),
                _ => return false,
            };
            if target != was {
                crate::import::note(format!("# {name}: spec changed, starting over"));
                return false;
            }
            if let Mark::After(cursor) = cursor {
                after.insert(name.clone(), cursor.clone());
            }
            true
        });
        for (name, checkpoint) in &after {
            if let Some(limit) = spec.collections[name].limit {
                if checkpoint.consumed > limit {
                    return Err(Error::InvalidArgument(format!(
                        "{name}: checkpoint consumed {} rows, exceeding limit {limit}",
                        checkpoint.consumed
                    )));
                }
            }
        }
        let done = self
            .cursors
            .values()
            .filter(|c| matches!(c, Mark::Done))
            .count();
        spec.collections
            .retain(|name, _| !matches!(self.cursors.get(name), Some(Mark::Done)));
        self.spec = plan;
        Ok((done, after))
    }

    fn path(id: &str) -> Result<PathBuf, Error> {
        // Tests that fail on purpose must not litter the real one.
        let dir = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => Config::dir()
                .map_err(|e| Error::InvalidArgument(e.to_string()))?
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
        let mut state: State = toml::from_str(&text).map_err(|e| {
            Error::InvalidArgument(format!(
                "run {id} cannot be resumed ({}): saved by an older topk? \
                 Start the import again; upserts are idempotent",
                e.message()
            ))
        })?;
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
