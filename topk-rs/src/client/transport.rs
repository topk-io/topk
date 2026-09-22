use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::header::AUTHORIZATION;
use http::{Request, Response};
use tonic::body::Body;
use tonic::metadata::MetadataMap;
use tonic::transport::Channel;
use tower::{BoxError, Service, ServiceExt};

use crate::client::{AsyncInterceptor, TracingInterceptor};
use crate::Error;

#[derive(Clone)]
pub(super) struct Transport {
    channel: Channel,
    default_interceptor: TracingInterceptor,
    custom_interceptor: Option<Arc<dyn AsyncInterceptor>>,
}

impl Transport {
    pub(super) fn new(
        channel: Channel,
        default_interceptor: TracingInterceptor,
        custom_interceptor: Option<Arc<dyn AsyncInterceptor>>,
    ) -> Self {
        Self {
            channel,
            default_interceptor,
            custom_interceptor,
        }
    }

    async fn intercept(&self, request: Request<Body>) -> Result<Request<Body>, Error> {
        // Interceptors receive only metadata and extensions. Keep the body and RPC URI intact.
        let (mut parts, body) = request.into_parts();
        let mut metadata_request = tonic::Request::from_parts(
            MetadataMap::from_headers(parts.headers),
            parts.extensions,
            (),
        );

        // Apply configured headers and tracing first, so the caller can inspect or override them.
        metadata_request = self
            .default_interceptor
            .call(metadata_request)
            .await
            .map_err(|e| Error::Interceptor(Arc::new(e)))?;
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
