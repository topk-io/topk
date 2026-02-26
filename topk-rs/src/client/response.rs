#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestId(String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for RequestId {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Response<T> {
    pub inner: T,
    pub request_id: Option<RequestId>,
}

impl<T> Response<T> {
    pub fn new(inner: T, request_id: Option<RequestId>) -> Self {
        Self { inner, request_id }
    }

    pub fn request_id(&self) -> Option<&RequestId> {
        self.request_id.as_ref()
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> std::ops::Deref for Response<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: IntoIterator> IntoIterator for Response<T> {
    type Item = T::Item;
    type IntoIter = T::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<T> From<tonic::Response<T>> for Response<T> {
    fn from(response: tonic::Response<T>) -> Self {
        let request_id = extract_request_id(&response);
        Self::new(response.into_inner(), request_id)
    }
}

pub fn extract_request_id(response: &tonic::Response<impl Sized>) -> Option<RequestId> {
    response
        .metadata()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(RequestId::from)
}
