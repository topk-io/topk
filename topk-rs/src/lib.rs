mod client;
mod errors;
mod internal_error_code;

pub mod data;
pub mod query;

pub use client::ClientConfig;
pub use client::{Client, CollectionClient};
pub use errors::SchemaValidationError;
pub use errors::ValidationError;
pub use errors::ValidationErrorBag;
pub use internal_error_code::InternalErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lsn timeout")]
    QueryLsnTimeout,

    #[error("collection already exists")]
    CollectionAlreadyExists,

    #[error("collection not found")]
    CollectionNotFound,

    #[error("document not found")]
    DocumentNotFound,

    #[error("invalid collection schema")]
    SchemaValidationError(ValidationErrorBag<SchemaValidationError>),

    #[error("invalid argument")]
    DocumentValidationError(ValidationErrorBag<ValidationError>),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("tonic error: {0}")]
    Unexpected(tonic::Status),

    #[error("invalid proto")]
    InvalidProto,

    #[error("permission denied")]
    PermissionDenied,

    #[error("capacity exceeded")]
    CapacityExceeded,

    #[error("tonic transport error")]
    TransportError(#[from] tonic::transport::Error),

    #[error("channel not initialized")]
    TransportChannelNotInitialized,

    #[error("malformed response: {0}")]
    MalformedResponse(String),
}
