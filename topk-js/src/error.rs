pub struct TopkError(topk_rs::Error);

impl From<topk_rs::Error> for TopkError {
    fn from(error: topk_rs::Error) -> Self {
        TopkError(error)
    }
}

impl From<TopkError> for napi::Error {
    fn from(error: TopkError) -> Self {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("{:?}", error.0.to_string()),
        )
    }
}
