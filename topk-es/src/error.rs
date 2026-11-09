use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;
use thiserror::Error as ThisError;
use topk_rs::Error as TopkError;

#[derive(Debug, Clone, ThisError)]
pub enum Error {
    #[error("index_not_found_exception: {0}")]
    IndexNotFound(String),

    #[error("no_handler_found_exception: {0}")]
    NoHandler(String),

    #[error("api_not_available_exception: {0}")]
    ApiNotAvailable(String),

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

    #[error("unavailable_shards_exception: {0}")]
    Unavailable(String),

    #[error("es_rejected_execution_exception: {0}")]
    TooManyRequests(String),

    #[error("security_exception: {0}")]
    Unauthorized(String),

    #[error("security_exception: {0}")]
    Forbidden(String),

    #[error("cluster_block_exception: {0}")]
    Blocked(String),

    #[error("content_too_long_exception: {0}")]
    ContentTooLong(String),

    #[error("timeout_exception: {0}")]
    Timeout(String),

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
    // Domain errors raised inside #[serde(try_from = ...)] arrive stringified as
    // "<error_type>: <reason>"; recover the variant from our own Display prefix.
    fn from(e: serde_json::Error) -> Self {
        let msg = e.to_string();
        match msg.split_once(": ") {
            Some(("illegal_argument_exception", rest)) => Error::Unsupported(rest.into()),
            Some(("action_request_validation_exception", rest)) => Error::BadRequest(rest.into()),
            _ => Error::SerdeJson(msg),
        }
    }
}

impl From<TopkError> for Error {
    fn from(e: TopkError) -> Self {
        match e {
            TopkError::CollectionNotFound => Error::IndexNotFound(e.to_string()),
            TopkError::CollectionAlreadyExists => Error::IndexAlreadyExists(e.to_string()),
            TopkError::NotFound | TopkError::DocumentNotFound(_) => {
                Error::DocumentNotFound(e.to_string())
            }
            TopkError::PartitionNotFound => Error::SourceNotFound(e.to_string()),
            TopkError::InvalidArgument(msg) => Error::BadRequest(msg),
            TopkError::DocumentValidationError(_)
            | TopkError::SchemaValidationError(_)
            | TopkError::CollectionValidationError(_) => Error::BadRequest(e.to_string()),
            TopkError::RequestTooLarge(msg) => Error::ContentTooLong(msg),
            TopkError::SlowDown(msg) => Error::TooManyRequests(msg),
            TopkError::QuotaExceeded(msg) => Error::Blocked(msg),
            TopkError::QueryLsnTimeout => {
                Error::Unavailable("timed out waiting for writes to become searchable".into())
            }
            TopkError::Unavailable(_) | TopkError::TransportError(_) => {
                Error::Unavailable(e.to_string())
            }
            TopkError::DeadlineExceeded(_) | TopkError::RetryTimeout => {
                Error::Timeout(e.to_string())
            }
            TopkError::PermissionDenied => Error::Forbidden(e.to_string()),
            TopkError::Unauthenticated(_) => Error::Unauthorized(e.to_string()),
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
            Error::NoHandler(msg) => (400, "no_handler_found_exception", msg.clone()),
            Error::ApiNotAvailable(msg) => (410, "api_not_available_exception", msg.clone()),
            Error::InvalidIndexName(msg) => (400, "invalid_index_name_exception", msg.clone()),
            Error::InvalidDocId(msg) => (400, "invalid_document_id_exception", msg.clone()),
            Error::DocumentNotFound(msg) => (404, "not_found", msg.clone()),
            Error::SourceNotFound(msg) => (404, "resource_not_found_exception", msg.clone()),
            Error::IndexAlreadyExists(msg) => {
                (400, "resource_already_exists_exception", msg.clone())
            }
            Error::SerdeJson(msg) => (400, "json_parse_exception", msg.clone()),
            Error::Unavailable(msg) => (503, "unavailable_shards_exception", msg.clone()),
            Error::TooManyRequests(msg) => (429, "es_rejected_execution_exception", msg.clone()),
            Error::Unauthorized(msg) => (401, "security_exception", msg.clone()),
            Error::Forbidden(msg) => (403, "security_exception", msg.clone()),
            Error::Blocked(msg) => (403, "cluster_block_exception", msg.clone()),
            Error::ContentTooLong(msg) => (413, "content_too_long_exception", msg.clone()),
            Error::Timeout(msg) => (504, "timeout_exception", msg.clone()),
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

        // ES emits this one as a bare string, before the structured envelope exists.
        if let Error::NoHandler(msg) = &self {
            return (code, Json(json!({ "error": msg }))).into_response();
        }

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

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tonic::Status;

    use super::*;

    #[rstest]
    #[case::slow_down(TopkError::SlowDown("x".into()), 429, "es_rejected_execution_exception")]
    #[case::quota(TopkError::QuotaExceeded("x".into()), 403, "cluster_block_exception")]
    #[case::unauthenticated(
        TopkError::Unauthenticated(Status::unauthenticated("x")),
        401,
        "security_exception"
    )]
    #[case::permission_denied(TopkError::PermissionDenied, 403, "security_exception")]
    #[case::too_large(TopkError::RequestTooLarge("x".into()), 413, "content_too_long_exception")]
    #[case::unavailable(
        TopkError::Unavailable(Status::unavailable("x")),
        503,
        "unavailable_shards_exception"
    )]
    #[case::deadline(
        TopkError::DeadlineExceeded(Status::deadline_exceeded("x")),
        504,
        "timeout_exception"
    )]
    #[case::retry_timeout(TopkError::RetryTimeout, 504, "timeout_exception")]
    fn topk_errors_map_to_es_status(
        #[case] error: TopkError,
        #[case] status: u16,
        #[case] error_type: &str,
    ) {
        let (code, body) = Error::from(error).parts();
        assert_eq!((code, body.error_type), (status, error_type));
    }
}
