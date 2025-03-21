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
      format!("failed to create collection: {:?}", error.0),
    )
  }
}
