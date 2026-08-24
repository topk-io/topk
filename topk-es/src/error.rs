use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error as ThisError;
use topk_rs::Error as TopkError;

#[derive(Debug, Clone, ThisError)]
pub enum Error {
    #[error("index_not_found_exception: {0}")]
    IndexNotFound(String),

    #[error("no_handler_found_exception: {0}")]
    NoHandler(String),

    #[error("invalid_index_name_exception: {0}")]
    InvalidIndexName(String),

    #[error("invalid_document_id_exception: {0}")]
    InvalidDocId(String),

    #[error("not_found: {0}")]
    DocumentNotFound(String),

    #[error("resource_not_found_exception: {0}")]
    SourceNotFound(String),

    #[error("resource_already_exists_exception: {0}")]
    IndexAlreadyExists(String),

    #[error("internal_server_error: {0}")]
    Internal(String),

    #[error("parsing_exception: {0}")]
    InvalidQuery(String),

    #[error("illegal_argument_exception: {0}")]
    Unsupported(String),

    #[error("action_request_validation_exception: {0}")]
    BadRequest(String),

    #[error("media_type_header_exception: {0}")]
    NotAcceptable(String),

    #[error("media_type_header_exception: {0}")]
    UnsupportedMediaType(String),

    #[error("json_parse_exception: {0}")]
    SerdeJson(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerdeJson(e.to_string())
    }
}

impl From<TopkError> for Error {
    fn from(e: TopkError) -> Self {
        match e {
            TopkError::CollectionNotFound => Error::IndexNotFound(e.to_string()),
            TopkError::CollectionAlreadyExists => Error::IndexAlreadyExists(e.to_string()),
            TopkError::NotFound => Error::DocumentNotFound(e.to_string()),
            TopkError::InvalidArgument(msg) => Error::BadRequest(msg),
            TopkError::DocumentValidationError(_) | TopkError::SchemaValidationError(_) => {
                Error::BadRequest(e.to_string())
            }
            _ => Error::Internal(e.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    #[serde(rename = "type")]
    pub error_type: &'static str,
    pub reason: String,
}

impl Error {
    pub fn parts(&self) -> (u16, ErrorBody) {
        let (status, error_type, reason) = match self {
            Error::IndexNotFound(msg) => (404, "index_not_found_exception", msg.clone()),
            Error::NoHandler(msg) => (404, "no_handler_found_exception", msg.clone()),
            Error::InvalidIndexName(msg) => (400, "invalid_index_name_exception", msg.clone()),
            Error::InvalidDocId(msg) => (400, "invalid_document_id_exception", msg.clone()),
            Error::DocumentNotFound(msg) => (404, "not_found", msg.clone()),
            Error::SourceNotFound(msg) => (404, "resource_not_found_exception", msg.clone()),
            Error::IndexAlreadyExists(msg) => {
                (400, "resource_already_exists_exception", msg.clone())
            }
            Error::SerdeJson(msg) => (400, "json_parse_exception", msg.clone()),
            Error::Internal(_) => (500, "internal_server_error", "Internal error".into()),
            Error::InvalidQuery(msg) => (400, "parsing_exception", msg.clone()),
            Error::Unsupported(msg) => (400, "illegal_argument_exception", msg.clone()),
            Error::BadRequest(msg) => (400, "action_request_validation_exception", msg.clone()),
            Error::NotAcceptable(msg) => (406, "media_type_header_exception", msg.clone()),
            Error::UnsupportedMediaType(msg) => (406, "media_type_header_exception", msg.clone()),
        };
        (status, ErrorBody { error_type, reason })
    }
}

#[derive(Serialize)]
struct ErrorDetails {
    root_cause: Vec<ErrorBody>,
    #[serde(flatten)]
    body: ErrorBody,
}

#[derive(Serialize)]
struct ErrorResponseBody {
    error: ErrorDetails,
    status: u16,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, body) = self.parts();
        let code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = ErrorResponseBody {
            error: ErrorDetails {
                root_cause: vec![body.clone()],
                body,
            },
            status,
        };

        (code, Json(body)).into_response()
    }
}
