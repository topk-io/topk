use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::import::error::Error;
use crate::import::source::{Checkpoint, Uri};
use crate::import::spec::{Spec, Target};
use crate::scope::Scope;
use crate::storage;

/// Names a run in its header, its state file and `--resume`: eight lowercase hex digits.
#[derive(Clone, Copy, Debug, PartialEq)]
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
pub enum Progress {
    Done,
    After(Checkpoint),
}

/// Created at confirmation, rewritten at every checkpoint, deleted on success:
/// a file exists only for a run that stopped.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// Passwords are redacted.
    pub source: String,
    /// The whole plan, done collections included.
    pub spec: Spec,
    #[serde(flatten)]
    pub destination: Scope,
    #[serde(default, rename = "cursors")]
    pub progress: BTreeMap<String, Progress>,
}

impl State {
    pub fn new(source: String, destination: Scope, spec: Spec) -> State {
        State {
            source,
            spec,
            destination,
            progress: BTreeMap::new(),
        }
    }

    pub fn pending(&self) -> impl Iterator<Item = (&String, &Target)> {
        self.spec
            .collections
            .iter()
            .filter(|(name, target)| match self.progress.get(*name) {
                Some(Progress::Done) => false,
                Some(Progress::After(checkpoint)) => target.limit != Some(checkpoint.consumed),
                None => true,
            })
    }

    pub fn source(&self, supplied: Option<&Uri>) -> Result<Uri, Error> {
        let mut uri = match supplied {
            Some(uri) => {
                if uri.to_string() == self.source {
                    uri.clone()
                } else {
                    let saved: Uri = self.source.parse()?;
                    match (&saved, uri) {
                        (Uri::Files(Some(a)), Uri::Files(Some(b)))
                            if a.store.is_none()
                                && b.store.is_none()
                                && matches!((std::fs::canonicalize(&a.path), std::fs::canonicalize(&b.path)),
                                (Ok(a), Ok(b)) if a == b) =>
                        {
                            saved
                        }
                        _ => {
                            return Err(Error::InvalidArgument(format!(
                                "saved run reads {:?}, not {}",
                                self.source, uri
                            )))
                        }
                    }
                }
            }
            None if self.source.is_empty() => Uri::Files(None),
            None => self.source.parse()?,
        };
        if uri.password() == Some("***") {
            return Err(Error::InvalidArgument(
                "saved run needs source credentials again; its saved URL omits the password".into(),
            ));
        }
        if let Uri::Topk { project, .. } = &mut uri {
            if project.is_none() {
                *project = Some(self.destination.project_id.clone());
            }
        }
        Ok(uri)
    }

    pub fn validate(&self) -> Result<(), Error> {
        self.spec.validate()?;
        for (name, mark) in &self.progress {
            let target = self.spec.collections.get(name).ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "saved run has a checkpoint for unknown collection {name:?}"
                ))
            })?;
            if let (Progress::After(checkpoint), Some(limit)) = (mark, target.limit) {
                if checkpoint.consumed > limit {
                    return Err(Error::InvalidArgument(format!(
                        "{name}: checkpoint consumed {} rows, exceeding limit {limit}",
                        checkpoint.consumed
                    )));
                }
            }
        }
        Ok(())
    }
}

pub struct Run {
    pub id: RunId,
    state: State,
    path: PathBuf,
    _lock: File,
}

impl Run {
    pub async fn start(directory: &Path, state: State) -> Result<Run, Error> {
        state.validate()?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        let id = RunId(nanos as u32 ^ std::process::id());
        let path = directory.join(format!("{id}.toml"));
        Ok(Run {
            id,
            _lock: storage::lock(path.with_extension("lock")).await?,
            path,
            state,
        })
    }

    pub async fn load(directory: &Path, id: RunId) -> Result<Run, Error> {
        let path = directory.join(format!("{id}.toml"));
        let lock = storage::lock(path.with_extension("lock")).await?;
        let state: State = storage::read_toml(&path)
            .map_err(|error| {
                Error::Setup(error.context(format!(
                    "run {id} cannot be resumed: invalid or older saved state; start a new import"
                )))
            })?
            .ok_or_else(|| {
                Error::InvalidArgument(format!("no run {id} to resume ({})", path.display()))
            })?;
        state.validate()?;
        Ok(Run {
            id,
            state,
            path,
            _lock: lock,
        })
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn save(&self) -> Result<(), Error> {
        storage::write_toml(&self.path, &self.state)?;
        Ok(())
    }

    pub(super) fn mark(&mut self, name: &str, mark: Progress) -> Result<(), Error> {
        self.state.progress.insert(name.to_string(), mark);
        self.save()
    }

    pub fn remove(&self) -> Result<(), Error> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
