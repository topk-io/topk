pub struct TopkError(topk_rs::Error);

impl From<topk_rs::Error> for TopkError {
    fn from(error: topk_rs::Error) -> Self {
        TopkError(error)
    }
}

impl From<TopkError> for napi::Error {
    fn from(error: TopkError) -> Self {
        match error.0 {
            topk_rs::Error::QueryLsnTimeout => {
                napi::Error::new(napi::Status::Cancelled, "lsn timeout")
            }
            topk_rs::Error::CollectionAlreadyExists => {
                napi::Error::new(napi::Status::GenericFailure, "collection already exists")
            }
            topk_rs::Error::CollectionNotFound => {
                napi::Error::new(napi::Status::GenericFailure, "collection not found")
            }
            topk_rs::Error::DocumentValidationError(e) => {
                napi::Error::new(napi::Status::InvalidArg, format!("{:?}", e))
            }
            topk_rs::Error::SchemaValidationError(e) => {
                napi::Error::new(napi::Status::InvalidArg, format!("{:?}", e))
            }
            topk_rs::Error::InvalidArgument(msg) => napi::Error::new(
                napi::Status::InvalidArg,
                format!("invalid argument: {}", msg),
            ),
            topk_rs::Error::InvalidProto => {
                napi::Error::new(napi::Status::GenericFailure, "invalid proto")
            }
            topk_rs::Error::PermissionDenied => {
                napi::Error::new(napi::Status::GenericFailure, "permission denied")
            }
            topk_rs::Error::CapacityExceeded => {
                napi::Error::new(napi::Status::GenericFailure, "capacity exceeded")
            }
            topk_rs::Error::TransportChannelNotInitialized => {
                napi::Error::new(napi::Status::GenericFailure, "channel not initialized")
            }
            topk_rs::Error::MalformedResponse(msg) => napi::Error::new(
                napi::Status::GenericFailure,
                format!("malformed response: {}", msg),
            ),
            topk_rs::Error::Unexpected(status) => napi::Error::new(
                napi::Status::GenericFailure,
                format!("unexpected error: {:?}", status),
            ),
            topk_rs::Error::TransportError(e) => napi::Error::new(
                napi::Status::GenericFailure,
                format!("transport error: {:?}", e),
            ),
        }
    }
}
