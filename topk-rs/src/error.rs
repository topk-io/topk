use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lsn timeout")]
    QueryLsnTimeout,

    #[error("retry timeout")]
    RetryTimeout,

    #[error("collection already exists")]
    CollectionAlreadyExists,

    #[error("collection not found")]
    CollectionNotFound,

    #[error("not found")]
    NotFound,

    #[error("invalid collection schema")]
    SchemaValidationError(ValidationErrorBag<SchemaValidationError>),

    #[error("document validation error")]
    DocumentValidationError(ValidationErrorBag<DocumentValidationError>),

    #[error("collection validation error")]
    CollectionValidationError(ValidationErrorBag<CollectionValidationError>),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("invalid proto")]
    InvalidProto,

    #[error("permission denied")]
    PermissionDenied,

    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("request too large: {0}")]
    RequestTooLarge(String),

    #[error("unexpected error: {0}")]
    Unexpected(String),

    #[error("slow down: {0}")]
    SlowDown(String),

    #[error("tonic transport error")]
    TransportError(#[from] tonic::transport::Error),

    #[error("malformed response: {0}")]
    MalformedResponse(String),
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        match self {
            // Retryable
            Error::QueryLsnTimeout => true,
            Error::SlowDown(_) => true,
            // Not retryable
            Error::RetryTimeout => false,
            Error::CollectionAlreadyExists => false,
            Error::CollectionNotFound => false,
            Error::NotFound => false,
            Error::SchemaValidationError(_) => false,
            Error::DocumentValidationError(_) => false,
            Error::CollectionValidationError(_) => false,
            Error::InvalidArgument(_) => false,
            Error::InvalidProto => false,
            Error::PermissionDenied => false,
            Error::QuotaExceeded(_) => false,
            Error::RequestTooLarge(_) => false,
            Error::TransportError(_) => false,
            Error::MalformedResponse(_) => false,
            Error::Unexpected(_) => false,
        }
    }
}

impl From<Status> for Error {
    fn from(status: Status) -> Self {
        match CustomError::try_from(status) {
            // Custom error
            Ok(error) => match error.code() {
                CustomErrorCode::RequiredLsnGreaterThanManifestMaxLsn => Error::QueryLsnTimeout,
                CustomErrorCode::SlowDown => Error::SlowDown(error.message().to_string()),
            },
            Err(e) => match e.code() {
                tonic::Code::NotFound => Error::NotFound,
                tonic::Code::ResourceExhausted => Error::QuotaExceeded(e.message().into()),
                tonic::Code::InvalidArgument => match ValidationErrorBag::try_from(e.clone()) {
                    Ok(errors) => Error::DocumentValidationError(errors),
                    Err(_) => match ValidationErrorBag::try_from(e.clone()) {
                        Ok(errors) => Error::SchemaValidationError(errors),
                        Err(_) => match ValidationErrorBag::try_from(e.clone()) {
                            Ok(errors) => Error::CollectionValidationError(errors),
                            Err(_) => Error::InvalidArgument(e.message().into()),
                        },
                    },
                },
                tonic::Code::OutOfRange => Error::RequestTooLarge(e.message().into()),
                tonic::Code::PermissionDenied => Error::PermissionDenied,
                _ => Error::Unexpected(format!("unexpected error: {:?}", e)),
            },
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchemaValidationError {
    #[error("field `{field}` has no data type")]
    MissingDataType { field: String },

    #[error("field name `{field}` cannot start with an underscore")]
    ReservedFieldName { field: String },

    #[error("Missing index spec for field `{field}`")]
    MissingIndexSpec { field: String },

    #[error("invalid index `{index}` for field `{field}` with type `{data_type}`")]
    InvalidIndex {
        field: String,
        index: String,
        data_type: String,
    },

    #[error("invalid vector index metric `{metric}` for field `{field}` with type `{data_type}`")]
    InvalidVectorIndexMetric {
        field: String,
        metric: String,
        data_type: String,
    },

    #[error("vector field `{field}` cannot be have zero dimension")]
    VectorDimensionCannotBeZero { field: String },

    #[error("Invalid semantic index for field `{field}. Error: {error}`")]
    InvalidSemanticIndex { field: String, error: String },
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DocumentValidationError {
    MissingId {
        doc_offset: usize,
    },

    InvalidId {
        doc_offset: usize,
        got: String,
    },

    MissingField {
        doc_id: String,
        field: String,
    },

    ReservedFieldName {
        doc_id: String,
        field: String,
    },

    InvalidDataType {
        doc_id: String,
        field: String,
        expected_type: String,
        got_value: String,
    },

    InvalidVectorDimension {
        doc_id: String,
        field: String,
        expected_dimension: usize,
        got_dimension: usize,
    },

    NoDocuments,
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum CollectionValidationError {
    InvalidName(String),
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationErrorBag<T: Serialize>(Vec<T>);

impl<T: Serialize + DeserializeOwned> ValidationErrorBag<T> {
    pub fn new(errors: Vec<T>) -> Self {
        Self(errors)
    }

    pub fn from(errors: impl IntoIterator<Item = T>) -> Self {
        Self(errors.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, error: T) {
        self.0.push(error);
    }

    pub fn extend(&mut self, errors: impl IntoIterator<Item = T>) {
        self.0.extend(errors.into_iter());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T: Serialize> From<ValidationErrorBag<T>> for tonic::Status {
    fn from(err: ValidationErrorBag<T>) -> Self {
        match serde_json::to_string(&err) {
            Ok(message) => tonic::Status::invalid_argument(message),
            Err(e) => {
                error!(?e, "failed to serialize response");
                tonic::Status::internal("failed to serialize response")
            }
        }
    }
}

impl<T: Serialize + DeserializeOwned> TryFrom<tonic::Status> for ValidationErrorBag<T> {
    type Error = serde_json::Error;

    fn try_from(status: tonic::Status) -> Result<Self, Self::Error> {
        serde_json::from_str(status.message())
    }
}

use tonic::Status;

#[derive(Clone, PartialEq, Eq)]
pub struct CustomError {
    /// Error message
    message: String,
    /// Custom error code
    code: CustomErrorCode,
}

impl CustomError {
    pub fn new(message: String, code: CustomErrorCode) -> Self {
        Self { message, code }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn code(&self) -> &CustomErrorCode {
        &self.code
    }
}

impl TryFrom<Status> for CustomError {
    type Error = Status;

    fn try_from(status: Status) -> Result<Self, Self::Error> {
        let ddb_error_code = status
            .metadata()
            .get("x-topk-error-code")
            .ok_or(anyhow::anyhow!("x-topk-error-code not found"))
            .map_err(|_| status.clone())?;
        let ddb_error_code = ddb_error_code.to_str().map_err(|_| status.clone())?;
        let ddb_error_code: u32 = ddb_error_code.parse().map_err(|_| status.clone())?;
        let code = CustomErrorCode::try_from(ddb_error_code).map_err(|_| status.clone())?;

        Ok(CustomError {
            message: status.message().to_string(),
            code,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum CustomErrorCode {
    RequiredLsnGreaterThanManifestMaxLsn,
    SlowDown,
}

impl Into<u32> for CustomErrorCode {
    fn into(self) -> u32 {
        match self {
            CustomErrorCode::RequiredLsnGreaterThanManifestMaxLsn => 1000,
            CustomErrorCode::SlowDown => 1429,
        }
    }
}

impl TryFrom<u32> for CustomErrorCode {
    type Error = anyhow::Error;

    fn try_from(code: u32) -> Result<Self, Self::Error> {
        match code {
            1000 => Ok(CustomErrorCode::RequiredLsnGreaterThanManifestMaxLsn),
            1429 => Ok(CustomErrorCode::SlowDown),
            code => Err(anyhow::anyhow!("unknown internal error code: {code}")),
        }
    }
}

impl From<CustomError> for Status {
    fn from(error: CustomError) -> Self {
        let mut status = match error.code {
            CustomErrorCode::RequiredLsnGreaterThanManifestMaxLsn => {
                Status::failed_precondition(error.message)
            }
            CustomErrorCode::SlowDown => Status::resource_exhausted(error.message),
        };

        let error_code: u32 = error.code.into();
        status
            .metadata_mut()
            .insert("x-topk-error-code", error_code.into());

        status
    }
}
