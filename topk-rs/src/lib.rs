mod client;

pub use client::Client;
pub use client::ClientConfig;

mod internal_error_code;
pub use internal_error_code::InternalErrorCode;
use topk_protos::v1::control::doc_validation::ValidationError;
use topk_protos::v1::control::doc_validation::ValidationErrorBag;
use topk_protos::v1::control::index_schema::SchemaValidationError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lsn timeout")]
    QueryLsnTimeout,

    #[error("index already exists")]
    IndexAlreadyExists,

    #[error("index not found")]
    IndexNotFound,

    #[error("invalid index")]
    SchemaValidationError(ValidationErrorBag<SchemaValidationError>),

    #[error("invalid argument")]
    DocumentValidationError(ValidationErrorBag<ValidationError>),

    #[error("invalid argument")]
    InvalidArgument(String),

    #[error("tonic error")]
    Unexpected(tonic::Status),

    #[error("permission denied")]
    PermissionDenied,

    #[error("tonic transport error")]
    TransportError(#[from] tonic::transport::Error),
}

// impl From<tonic::Status> for Error {
//     fn from(status: tonic::Status) -> Self {
//         match status.code() {
//             // tonic::Code::NotFound => Self::NotFound,
//             // tonic::Code::AlreadyExists => Self::AlreadyExists,
//             // tonic::Code::Internal => Self::Internal,
//             // For `InvalidArgument` we opportunistically try to parse the message as `ValidationErrorBag`
//             tonic::Code::InvalidArgument => match serde_json::from_str(status.message()) {
//                 Ok(v) => Self::ValidationError(v),
//                 Err(_) => Self::InvalidArgument(status.message().to_string()),
//             },
//             tonic::Code::PermissionDenied => Self::PermissionDenied,
//             _ => Self::Unexpected(status),
//         }
//     }
// }
