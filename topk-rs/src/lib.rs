mod client;

pub use client::Client;
pub use client::ClientConfig;

mod internal_error_code;
pub use internal_error_code::InternalErrorCode;
use topk_protos::v1::control::collection_schema::SchemaValidationError;
use topk_protos::v1::control::doc_validation::ValidationError;
use topk_protos::v1::control::doc_validation::ValidationErrorBag;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lsn timeout")]
    QueryLsnTimeout,

    #[error("collection already exists")]
    CollectionAlreadyExists,

    #[error("collection not found")]
    CollectionNotFound,

    #[error("invalid collection schema")]
    SchemaValidationError(ValidationErrorBag<SchemaValidationError>),

    #[error("invalid argument")]
    DocumentValidationError(ValidationErrorBag<ValidationError>),

    #[error("invalid argument")]
    InvalidArgument(String),

    #[error("tonic error")]
    Unexpected(tonic::Status),

    #[error("permission denied")]
    PermissionDenied,

    #[error("capacity exceeded")]
    CapacityExceeded,

    #[error("tonic transport error")]
    TransportError(#[from] tonic::transport::Error),

    #[error("channel not initialized")]
    TransportChannelNotInitialized,
}
