use thiserror::Error as ThisError;

use crate::import::spec::Type;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("duckdb error: {0}")]
    Duck(#[from] duckdb::Error),

    #[error("elasticsearch error: {0}")]
    Es(#[from] elasticsearch::Error),

    #[error("mongodb error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    #[error("worker task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Topk(#[from] topk_rs::Error),

    #[error("{0}")]
    InvalidArgument(String),

    #[error("cannot coerce to {0}")]
    CannotCoerce(Type),

    #[error("field {0:?}: {1}")]
    Id(String, String),

    #[error("doc {id:?}{}: {source}", field.as_deref().map(|f| format!(" field {f:?}")).unwrap_or_default())]
    Doc {
        id: String,
        field: Option<String>,
        source: Box<Error>,
    },

    #[error(transparent)]
    Row(Box<Error>),

    #[error(
        "schema mismatch: {0:?}; make the spec match the collection (drop added or changed \
         fields), use a new collection name, or delete the collection and re-run"
    )]
    SchemaMismatch(Vec<String>),

    #[error("{0} document(s) skipped")]
    Skipped(usize),
}

impl Error {
    pub fn skippable(&self) -> bool {
        matches!(self, Error::Doc { .. } | Error::Id(..) | Error::Row(_))
    }
}

impl From<Error> for topk_rs::Error {
    fn from(error: Error) -> topk_rs::Error {
        match error {
            Error::Io(error) => topk_rs::Error::IoError(error),
            Error::Topk(error) => error,
            error => topk_rs::Error::Input(error.into()),
        }
    }
}
