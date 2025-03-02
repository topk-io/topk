use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

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
pub enum ValidationError {
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

    NoDocuments,
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
