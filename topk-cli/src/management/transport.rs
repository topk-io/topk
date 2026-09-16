use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::header::{HeaderValue, AUTHORIZATION};
use http::{Request, Response};
use tonic::body::Body;
use tonic::transport::Channel;
use tower::{BoxError, Service, ServiceExt};

use crate::auth::Auth;

// Authenticated transport
#[derive(Clone)]
pub struct Transport {
    channel: Channel,
    auth: Arc<Auth>,
}

impl Transport {
    pub(super) fn new(channel: Channel, auth: Auth) -> Self {
        Self {
            channel,
            auth: Arc::new(auth),
        }
    }
}

impl Service<Request<Body>> for Transport {
    type Response = Response<Body>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Wait for channel readiness after resolving authentication in `call`.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let auth = self.auth.clone();
        let channel = self.channel.clone();
        Box::pin(async move {
            let token = auth.access_token().await?;
            let mut header = HeaderValue::from_str(&format!("Bearer {token}"))?;
            header.set_sensitive(true);
            request.headers_mut().insert(AUTHORIZATION, header);
            Ok(channel.oneshot(request).await?)
        })
    }
}
