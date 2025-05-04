use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

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

    #[error("document validation error")]
    DocumentValidationError(ValidationErrorBag<DocumentValidationError>),

    #[error("collection validation error")]
    CollectionValidationError(ValidationErrorBag<CollectionValidationError>),

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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InternalErrorCode {
    RequiredLsnGreaterThanManifestMaxLsn = 1000,
}

impl InternalErrorCode {
    /// Get the numeric code associated with the enum variant.
    pub fn code(&self) -> u32 {
        *self as u32
    }

    pub fn parse_status(e: &Status) -> anyhow::Result<InternalErrorCode> {
        let ddb_error_code = e
            .metadata()
            .get("x-topk-error-code")
            .ok_or(anyhow::anyhow!("x-topk-error-code not found"))?;
        let ddb_error_code = ddb_error_code.to_str()?;
        let ddb_error_code: u32 = ddb_error_code.parse()?;
        let code = InternalErrorCode::try_from(ddb_error_code)?;

        Ok(code)
    }
}

impl From<InternalErrorCode> for Status {
    fn from(error: InternalErrorCode) -> Self {
        let mut status = match error {
            InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn => {
                Status::failed_precondition("Lsn is greater than manifest max lsn")
            }
        };

        status
            .metadata_mut()
            .insert("x-topk-error-code", error.code().into());

        status
    }
}

impl TryFrom<u32> for InternalErrorCode {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1000 => Ok(InternalErrorCode::RequiredLsnGreaterThanManifestMaxLsn),
            _ => Err(anyhow::anyhow!("unknown internal error code: {}", value)),
        }
    }
}
