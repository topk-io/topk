use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use http::header::{HeaderName, HeaderValue, AUTHORIZATION};
use http::{HeaderMap, Request, Response};
#[cfg(feature = "trace")]
use opentelemetry::{global, propagation::Injector};
use tonic::body::Body;
use tonic::transport::Channel;
use tower::{BoxError, Service, ServiceExt};
#[cfg(feature = "trace")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::client::ClientConfig;
use crate::Error;

#[async_trait]
pub trait AsyncInterceptor: Send + Sync {
    async fn call(&self, request: &mut Request<Body>) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub(super) struct Transport {
    channel: Channel,
    headers: HeaderMap,
    interceptor: Option<Arc<dyn AsyncInterceptor>>,
}

impl Transport {
    pub(super) fn new(channel: Channel, config: &ClientConfig) -> Result<Self, Error> {
        Ok(Self {
            channel,
            headers: config.try_into()?,
            interceptor: config.interceptor().cloned(),
        })
    }

    async fn intercept(&self, mut request: Request<Body>) -> Result<Request<Body>, Error> {
        // Apply tracing headers.
        #[cfg(feature = "trace")]
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(
                &tracing::Span::current().context(),
                &mut HeaderInjector(request.headers_mut()),
            );
        });

        // Apply configured headers.
        request.headers_mut().extend(self.headers.clone());

        // Apply custom interceptor.
        if let Some(interceptor) = &self.interceptor {
            interceptor
                .call(&mut request)
                .await
                .map_err(|e| Error::Interceptor(Arc::new(e)))?;
        }

        if let Some(header) = request.headers_mut().get_mut(AUTHORIZATION) {
            header.set_sensitive(true);
        }

        Ok(request)
    }
}

#[cfg(feature = "trace")]
struct HeaderInjector<'a>(&'a mut HeaderMap);

#[cfg(feature = "trace")]
impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(value) = HeaderValue::from_str(&value) {
                self.0.insert(key, value);
            }
        }
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

impl TryFrom<&ClientConfig> for HeaderMap {
    type Error = Error;

    fn try_from(config: &ClientConfig) -> Result<Self, Self::Error> {
        config
            .headers()
            .iter()
            .map(|(key, value)| {
                let key = key
                    .parse::<HeaderName>()
                    .map_err(|e| Error::Input(anyhow::anyhow!("invalid header name: {e:?}")))?;
                let value = value
                    .parse::<HeaderValue>()
                    .map_err(|e| Error::Input(anyhow::anyhow!("invalid header value: {e:?}")))?;
                Ok((key, value))
            })
            .collect()
    }
}
