use thiserror::Error as ThisError;

use crate::import::source::SourceLocation;

pub const MAX_DOC_BYTES: usize = 2_000_000;
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("{}: couldn't detect an id column — available: {} — pass --id {}", .from, .columns.join(", "), id_hint(.columns))]
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
    Topk(#[from] topk_rs::Error),

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

/// The only column ending in `_id`, else a placeholder.
fn id_hint(columns: &[String]) -> &str {
    let mut ids = columns
        .iter()
        .filter(|column| column.to_ascii_lowercase().ends_with("_id"));
    match (ids.next(), ids.next()) {
        (Some(column), None) => column,
        _ => "<column>",
    }
}
