/// Represents an error that occurred in the TopK SDK.
///
/// This struct wraps errors from the underlying topk-rs library and provides
/// appropriate error handling for the JavaScript/TypeScript interface.
#[derive(Debug)]
pub struct TopkError(topk_rs::Error);

impl From<topk_rs::Error> for TopkError {
    fn from(error: topk_rs::Error) -> Self {
        TopkError(error)
    }
}

impl From<TopkError> for napi::Error {
    fn from(error: TopkError) -> Self {
        match error.0 {
            // Custom errors
            topk_rs::Error::QueryLsnTimeout => {
                napi::Error::new(napi::Status::Cancelled, format!("{:?}", error))
            }
            // Not found errors
            topk_rs::Error::CollectionAlreadyExists => {
                napi::Error::new(napi::Status::GenericFailure, "collection already exists")
            }
            topk_rs::Error::CollectionNotFound => {
                napi::Error::new(napi::Status::GenericFailure, "collection not found")
            }
            // Validation errors
            topk_rs::Error::DocumentValidationError(_) => {
                napi::Error::new(napi::Status::InvalidArg, format!("{:?}", error))
            }
            topk_rs::Error::SchemaValidationError(_) => {
                napi::Error::new(napi::Status::InvalidArg, format!("{:?}", error))
            }
            topk_rs::Error::CollectionValidationError(_) => {
                napi::Error::new(napi::Status::InvalidArg, format!("{:?}", error))
            }
            topk_rs::Error::InvalidArgument(_) => napi::Error::new(
                napi::Status::InvalidArg,
                format!("invalid argument: {}", error.0),
            ),
            topk_rs::Error::RequestTooLarge(_) => napi::Error::new(
                napi::Status::InvalidArg,
                format!("request too large: {}", error.0),
            ),
            topk_rs::Error::PermissionDenied => {
                napi::Error::new(napi::Status::GenericFailure, "permission denied")
            }
            // Other errors
            _ => napi::Error::new(napi::Status::GenericFailure, format!("{:?}", error)),
        }
    }
}
