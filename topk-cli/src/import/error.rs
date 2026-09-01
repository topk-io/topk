use thiserror::Error as ThisError;

use crate::import::spec::Type;

/// The documented per-document limit (docs/limits.mdx: 200KB), decimal on
/// purpose: a little strict yields our error, not the server's.
pub const MAX_DOC_BYTES: usize = 200_000;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("duckdb error: {0}")]
    Duck(#[from] duckdb::Error),

    #[error("elasticsearch error: {0}")]
    Es(#[from] elasticsearch::Error),

    #[error("elasticsearch {status}: {reason}")]
    EsStatus { status: u16, reason: String },

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

    #[error("{} exceeds the {} document limit",
        bytesize::ByteSize(*.0 as u64).to_string_as(false),
        bytesize::ByteSize(MAX_DOC_BYTES as u64).to_string_as(false))]
    Oversized(usize),

    #[error("{}{}: {source}",
        id.as_deref().map(|i| format!("doc {i:?}")).unwrap_or_else(|| "row".to_string()),
        field.as_deref().map(|f| format!(" field {f:?}")).unwrap_or_default())]
    Doc {
        id: Option<String>,
        field: Option<String>,
        source: Box<Error>,
    },

    #[error(
        "schema mismatch; make the spec match the collection (drop added or changed fields), \
         use a new collection name, or delete the collection and re-run:\n  {0}"
    )]
    SchemaMismatch(String),
}
