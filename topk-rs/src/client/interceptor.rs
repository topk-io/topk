use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use tonic::{metadata::AsciiMetadataValue, service::Interceptor, Status};

use crate::Error;

/// Intercepts metadata and extensions before each request attempt, including retries.
///
/// Runs after configured headers and tracing. Implementations may resolve credentials
/// asynchronously and own any caching or renewal. Errors stop the request without retrying.
#[async_trait]
pub trait AsyncInterceptor: Send + Sync {
    async fn call(&self, request: tonic::Request<()>) -> anyhow::Result<tonic::Request<()>>;
}

#[derive(Clone)]
pub struct AppendHeadersInterceptor {
    /// Headers
    headers: HashMap<&'static str, AsciiMetadataValue>,
}

impl AppendHeadersInterceptor {
    pub fn new(
        headers: impl IntoIterator<Item = (&'static str, impl AsRef<str>)>,
    ) -> Result<Self, Error> {
        Ok(Self {
            headers: headers
                .into_iter()
                .map(|(key, value)| {
                    let value = AsciiMetadataValue::from_str(value.as_ref()).map_err(|e| {
                        Error::Input(anyhow::anyhow!("invalid header value: {e:?}"))
                    })?;
                    Ok((key, value))
                })
                .collect::<Result<_, Error>>()?,
        })
    }

    pub(super) fn apply(&self, mut request: tonic::Request<()>) -> tonic::Request<()> {
        for (key, value) in self.headers.iter() {
            request.metadata_mut().insert(*key, value.clone());
        }
        request
    }
}

impl Interceptor for AppendHeadersInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        Ok(self.apply(request))
    }
}

#[async_trait]
impl AsyncInterceptor for AppendHeadersInterceptor {
    async fn call(&self, request: tonic::Request<()>) -> anyhow::Result<tonic::Request<()>> {
        Ok(self.apply(request))
    }
}
