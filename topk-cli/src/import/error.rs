use thiserror::Error as ThisError;

use crate::import::source::SourceLocation;

pub const MAX_DOC_BYTES: usize = 2_000_000;
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("{}: couldn't detect an id column — available: {} — pass --id <column>", .from, .columns.join(", "))]
    MissingId {
        from: SourceLocation,
        columns: Vec<String>,
    },
    #[error(transparent)]
    Setup(#[from] anyhow::Error),

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

    #[error("worker task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Topk(topk_rs::Error),

    #[error("{0}")]
    RejectedDocuments(String),

    #[error("{0}")]
    InvalidArgument(String),

    #[error("cannot coerce to {0}")]
    CannotCoerce(&'static str),

    #[error("doc {id:?}: {} exceeds the {} document limit; largest field {field:?}",
        bytesize::ByteSize(*size as u64).to_string_as(false),
        bytesize::ByteSize(MAX_DOC_BYTES as u64).to_string_as(false))]
    Oversized {
        id: String,
        size: usize,
        field: String,
    },

    #[error("{}{}: {source}",
        id.as_deref().map(|i| format!("doc {i:?}")).unwrap_or_else(|| "row".to_string()),
        field.as_deref().map(|f| format!(" field {f:?}")).unwrap_or_default())]
    Doc {
        id: Option<String>,
        field: Option<String>,
        source: Box<Error>,
    },

    #[error(
        "the collection's schema is set when it is created and cannot change; import into a new \
         collection, or change -f, --exclude or --rename:\n  {0}"
    )]
    SchemaMismatch(String),
}

impl From<topk_rs::Error> for Error {
    fn from(error: topk_rs::Error) -> Self {
        match error {
            topk_rs::Error::DocumentValidationError(errors) => Error::RejectedDocuments(
                errors.into_iter().map(|error| match error {
                    topk_rs::error::DocumentValidationError::TextTooLong { doc_id, field, max_length, got_length } => format!(
                        "doc {doc_id:?}, field {field:?}: text length {got_length} exceeds {max_length}; use --continue-on-error or select a different field"
                    ),
                    error => format!("{error:?}"),
                }).collect::<Vec<_>>().join("\n")
            ),
            error => Error::Topk(error),
        }
    }
}
