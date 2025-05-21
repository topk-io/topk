use std::{collections::HashMap, str::FromStr};
use tonic::{metadata::AsciiMetadataValue, service::Interceptor, Status};

#[derive(Clone)]
pub struct AppendHeadersInterceptor {
    headers: HashMap<&'static str, String>,
}

impl AppendHeadersInterceptor {
    pub fn new(headers: impl Into<HashMap<&'static str, String>>) -> Self {
        Self {
            headers: headers.into(),
        }
    }
}

impl Interceptor for AppendHeadersInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        for (key, value) in self.headers.iter() {
            request.metadata_mut().insert(
                *key,
                AsciiMetadataValue::from_str(value.as_str()).expect("invalid header value"),
            );
        }

        Ok(request)
    }
}
