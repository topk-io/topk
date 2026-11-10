use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::{self, Config};
use crate::endpoint::DataEndpoint;
use crate::import::error::Error;
use crate::import::source::{Cursor, Uri};
use crate::import::spec::{Spec, Target};
use crate::ProjectId;

/// Names a run in its header, its state file and `--resume`: eight lowercase hex digits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RunId(u32);

impl FromStr for RunId {
    type Err = Error;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        if id.len() != 8
            || !id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::InvalidArgument(
                "run id must be eight lowercase hex digits".into(),
            ));
        }
        u32::from_str_radix(id, 16)
            .map(RunId)
            .map_err(|_| Error::InvalidArgument("run id must be eight lowercase hex digits".into()))
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

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

#[derive(Serialize, Deserialize)]
pub struct Destination {
    pub project_id: ProjectId,
    pub region: String,
}

impl Destination {
    pub fn build(config: &Config, args: &DataEndpoint) -> Result<Self, Error> {
        Ok(Self {
            region: args.region()?.to_string(),
            project_id: config.project_id()?,
        })
    }
}

/// Created at confirmation, rewritten at every checkpoint, deleted on success:
/// a file exists only for a run that stopped.
#[derive(Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    pub id: RunId,
    /// Passwords are redacted.
    pub source: String,
    /// The whole plan, done collections included.
    pub spec: Spec,
    #[serde(flatten)]
    pub destination: Destination,
    #[serde(default)]
    pub cursors: BTreeMap<String, Mark>,
}

impl State {
    pub fn start(source: &str, destination: Destination, spec: Spec) -> Result<State, Error> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        Ok(State {
            id: RunId(nanos as u32 ^ std::process::id()),
            source: source.to_string(),
            spec,
            destination,
            cursors: BTreeMap::new(),
        })
    }

    pub fn pending(&self) -> impl Iterator<Item = (&String, &Target)> {
        self.spec
            .collections
            .iter()
            .filter(|(name, target)| match self.cursors.get(*name) {
                Some(Mark::Done) => false,
                Some(Mark::After(checkpoint)) => target.limit != Some(checkpoint.consumed),
                None => true,
            })
    }

    fn path(id: RunId) -> Result<PathBuf, Error> {
        // Tests that fail on purpose must not litter the real one.
        let dir = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => Config::dir()?.join("import"),
        };
        Ok(dir.join(format!("{id}.toml")))
    }

    pub fn save(&self) -> Result<(), Error> {
        config::write_toml(&Self::path(self.id)?, self)?;
        Ok(())
    }

    /// Best effort: a leftover file is a stale `--resume` target, not a failed import.
    pub fn remove(&self) {
        if let Ok(path) = Self::path(self.id) {
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != ErrorKind::NotFound {
                    tracing::warn!(%e, path = %path.display(), "cannot remove run state");
                }
            }
        }
    }
}

pub struct Run {
    state: State,
    _lock: File,
}

impl Run {
    pub async fn start(source: &str, destination: Destination, spec: Spec) -> Result<Run, Error> {
        let state = State::start(source, destination, spec)?;
        Ok(Run {
            _lock: Self::lock(state.id).await?,
            state,
        })
    }

    pub async fn load(id: RunId) -> Result<Run, Error> {
        let lock = Self::lock(id).await?;
        let path = State::path(id)?;
        let mut state: State = config::read_toml(&path)
            .map_err(|error| {
                Error::Setup(error.context(format!(
                    "run {id} cannot be resumed: invalid or older saved state; start a new import"
                )))
            })?
            .ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "no run {id} to resume ({}) — the id is in the header of the run that stopped",
                    path.display()
                ))
            })?;
        state.id = id;
        for (name, mark) in &state.cursors {
            let target = state.spec.collections.get(name).ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "run {id} has a checkpoint for unknown collection {name:?}"
                ))
            })?;
            if let (Mark::After(checkpoint), Some(limit)) = (mark, target.limit) {
                if checkpoint.consumed > limit {
                    return Err(Error::InvalidArgument(format!(
                        "{name}: checkpoint consumed {} rows, exceeding limit {limit}",
                        checkpoint.consumed
                    )));
                }
            }
        }
        Ok(Run { state, _lock: lock })
    }

    async fn lock(id: RunId) -> Result<File, Error> {
        Ok(config::lock(State::path(id)?.with_extension("lock")).await?)
    }

    pub fn source(&self, supplied: Option<&Uri>) -> Result<Uri, Error> {
        let mut uri = match supplied {
            Some(uri) => {
                if uri.to_string() != self.state.source {
                    return Err(Error::InvalidArgument(format!(
                        "run {} reads {:?}, not {}",
                        self.id, self.state.source, uri
                    )));
                }
                uri.clone()
            }
            None if self.state.source.is_empty() => Uri::default(),
            None => self.state.source.parse()?,
        };
        if uri.password() == Some("***") {
            return Err(Error::InvalidArgument(format!(
                "run {} needs SOURCE again to reconnect; its saved URL omits the password",
                self.id
            )));
        }
        if let Uri::Topk(topk) = &mut uri {
            if topk.project_id.is_none() {
                topk.project_id = Some(self.destination.project_id.clone());
            }
        }
        Ok(uri)
    }

    pub fn mark(&mut self, name: &str, mark: Mark) -> bool {
        self.state.cursors.insert(name.to_string(), mark);
        if let Err(e) = self.state.save() {
            tracing::warn!(%e, "cannot save run state");
            return false;
        }
        true
    }
}

impl Deref for Run {
    type Target = State;

    fn deref(&self) -> &State {
        &self.state
    }
}
