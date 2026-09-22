use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use http::header::AUTHORIZATION;
use http::{Request, Response};
use tonic::body::Body;
use tonic::metadata::{AsciiMetadataValue, MetadataMap};
use tonic::transport::Channel;
use tower::{BoxError, Service, ServiceExt};

#[cfg(feature = "trace")]
use crate::client::trace;
use crate::client::ClientConfig;
use crate::Error;

/// An async interceptor that appends headers to a request.
#[async_trait]
pub trait AsyncInterceptor: Send + Sync {
    async fn call(&self, request: tonic::Request<()>) -> anyhow::Result<tonic::Request<()>>;
}

#[derive(Clone)]
pub(super) struct Transport {
    channel: Channel,
    headers: HashMap<&'static str, AsciiMetadataValue>,
    custom_interceptor: Option<Arc<dyn AsyncInterceptor>>,
}

impl Transport {
    pub(super) fn new(channel: Channel, config: &ClientConfig) -> Result<Self, Error> {
        Ok(Self {
            channel,
            headers: config
                .headers()
                .iter()
                .map(|(key, value)| {
                    let value = AsciiMetadataValue::from_str(value).map_err(|e| {
                        Error::Input(anyhow::anyhow!("invalid header value: {e:?}"))
                    })?;
                    Ok((*key, value))
                })
                .collect::<Result<_, Error>>()?,
            custom_interceptor: config.interceptor().cloned(),
        })
    }

    async fn intercept(&self, request: Request<Body>) -> Result<Request<Body>, Error> {
        // Interceptors receive only metadata and extensions. Keep the body and RPC URI intact.
        let (mut parts, body) = request.into_parts();
        let mut metadata_request = tonic::Request::from_parts(
            MetadataMap::from_headers(parts.headers),
            parts.extensions,
            (),
        );

        // Apply tracing and configured headers first, so the caller can inspect or override them.
        #[cfg(feature = "trace")]
        trace::inject(metadata_request.metadata_mut());
        for (key, value) in &self.headers {
            metadata_request.metadata_mut().insert(*key, value.clone());
        }
        if let Some(interceptor) = &self.custom_interceptor {
            metadata_request = interceptor
                .call(metadata_request)
                .await
                .map_err(|e| Error::Interceptor(Arc::new(e)))?;
        }

        // Restore the intercepted metadata and extensions onto the original HTTP request.
        let (metadata, extensions, ()) = metadata_request.into_parts();
        parts.headers = metadata.into_headers();
        parts.extensions = extensions;
        if let Some(header) = parts.headers.get_mut(AUTHORIZATION) {
            header.set_sensitive(true);
        }
        Ok(Request::from_parts(parts, body))
    }
}

impl Service<Request<Body>> for Transport {
    type Response = Response<Body>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // `call` waits for channel readiness through `oneshot`, after interception.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let transport = self.clone();
        Box::pin(async move {
            let request = transport.intercept(request).await?;
            Ok(transport.channel.oneshot(request).await?)
        })
    }
}

#[cfg(test)]
#[path = "test_transport.rs"]
mod tests;
