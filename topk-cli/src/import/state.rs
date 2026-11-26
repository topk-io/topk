use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::import::error::Error;
use crate::import::execution::{ReadOptions, WriteOptions};
use crate::import::plan::Plan;
use crate::import::source::{Resume, Uri};
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

/// Where the import stands.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Progress {
    Done,
    After(Resume),
}

/// Created at confirmation, rewritten at every checkpoint, deleted on success:
/// a file exists only for a run that stopped.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// Passwords are redacted.
    pub source: String,
    pub plan: Plan,
    pub read: ReadOptions,
    pub write: WriteOptions,
    #[serde(flatten)]
    pub destination: Scope,
    #[serde(default)]
    pub progress: Option<Progress>,
}

impl State {
    pub fn new(
        source: String,
        destination: Scope,
        plan: Plan,
        read: ReadOptions,
        write: WriteOptions,
    ) -> State {
        State {
            source,
            plan,
            read,
            write,
            destination,
            progress: None,
        }
    }

    pub fn done(&self) -> bool {
        match &self.progress {
            Some(Progress::Done) => true,
            Some(Progress::After(resume)) => self.plan.target.limit == Some(resume.consumed),
            None => false,
        }
    }

    pub fn source(&self, supplied: Option<&Uri>) -> Result<Uri, Error> {
        let saved = || {
            self.source.parse::<Uri>().map_err(|error| {
                Error::InvalidArgument(format!("{error}; old saved runs need a new import"))
            })
        };
        let mut uri = match supplied {
            Some(uri) => {
                if uri.to_string() == self.source {
                    uri.clone()
                } else {
                    let saved = saved()?;
                    match (&saved, uri) {
                        (Uri::Files(a), Uri::Files(b))
                            if a == b
                                || a.store.is_none()
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
            None => saved()?,
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
        self.plan.validate()?;
        if let (Some(Progress::After(resume)), Some(limit)) =
            (&self.progress, self.plan.target.limit)
        {
            if resume.consumed > limit {
                return Err(Error::InvalidArgument(format!(
                    "checkpoint consumed {} rows, exceeding limit {limit}",
                    resume.consumed
                )));
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
                let _ = std::fs::remove_file(path.with_extension("lock"));
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

    pub fn set_options(&mut self, read: ReadOptions, write: WriteOptions) -> Result<(), Error> {
        if read.shard != self.state.read.shard {
            return Err(Error::InvalidArgument(
                "--shard cannot change when resuming; start a new import".into(),
            ));
        }
        self.state.read = read;
        self.state.write = write;
        Ok(())
    }

    pub fn save(&self) -> Result<(), Error> {
        storage::write_toml(&self.path, &self.state)?;
        Ok(())
    }

    pub(super) fn mark(&mut self, mark: Progress) -> Result<(), Error> {
        self.state.progress = Some(mark);
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

impl Drop for Run {
    /// A lock is kept only next to a state file, for the resume that will take it.
    fn drop(&mut self) {
        if self.path.exists() {
            return;
        }
        match std::fs::remove_file(self.path.with_extension("lock")) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                warn!(%error, "cannot remove run lock")
            }
            _ => {}
        }
    }
}
