use std::error::Error as StdError;
use std::fmt;
use std::iter::successors;
use std::sync::Arc;

use tonic::Status;

#[derive(Debug)]
pub enum Error {
    AccessToken(Arc<anyhow::Error>),
    Status(Status),
}

impl From<Status> for Error {
    fn from(status: Status) -> Self {
        match status.source().and_then(|e| e.downcast_ref::<Error>()) {
            Some(Error::AccessToken(source)) => Error::AccessToken(source.clone()),
            _ => Error::Status(status),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AccessToken(source) => write!(f, "{source:#}"),
            Error::Status(status) => match status.source() {
                // Tonic made this status from a local failure (e.g. a refused connection).
                // The reason is in its causes, not its message, so print those.
                Some(cause) => {
                    write!(f, "{cause}")?;
                    successors(cause.source(), |e| (*e).source())
                        .try_for_each(|cause| write!(f, ": {cause}"))
                }
                None => f.write_str(status.message()),
            },
        }
    }
}

impl StdError for Error {}
