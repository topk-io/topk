use std::sync::Arc;

use async_trait::async_trait;
use http::header::AUTHORIZATION;
use http::{Method, Request, Version};
use tonic::body::Body;
use tonic::transport::Channel;

use crate::client::{AsyncInterceptor, ClientConfig};
use crate::Error;

use super::Transport;

#[derive(Clone, Debug, PartialEq)]
struct Marker(u32);

struct InspectInterceptor;

#[async_trait]
impl AsyncInterceptor for InspectInterceptor {
    async fn call(&self, mut request: tonic::Request<()>) -> anyhow::Result<tonic::Request<()>> {
        assert_eq!(request.extensions().get::<Marker>(), Some(&Marker(1)));
        assert_eq!(request.metadata().get("x-configured").unwrap(), "present");
        request.extensions_mut().insert(Marker(2));
        request.metadata_mut().remove("x-remove");
        request
            .metadata_mut()
            .insert("authorization", "Bearer refreshed".parse()?);
        Ok(request)
    }
}

#[tokio::test]
async fn interception_preserves_routing_and_returns_metadata_and_extensions() {
    for interceptor in [
        None,
        Some(Arc::new(InspectInterceptor) as Arc<dyn AsyncInterceptor>),
    ] {
        let custom = interceptor.is_some();
        let mut config =
            ClientConfig::new("api-key", "test").with_headers([("x-configured", "present")]);
        if let Some(interceptor) = interceptor {
            config = config.with_interceptor(interceptor);
        }
        let transport = Transport::new(
            Channel::from_static("http://localhost:1").connect_lazy(),
            &config,
        )
        .unwrap();
        let request = transport
            .intercept(
                Request::builder()
                    .method(Method::POST)
                    .uri("/topk.Service/Method")
                    .version(Version::HTTP_2)
                    .header("x-remove", "original")
                    .extension(Marker(1))
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri(), "/topk.Service/Method");
        assert_eq!(request.version(), Version::HTTP_2);
        assert_eq!(
            request.extensions().get::<Marker>(),
            Some(&Marker(if custom { 2 } else { 1 }))
        );
        assert_eq!(request.headers().contains_key("x-remove"), !custom);
        assert_eq!(
            request.headers()[AUTHORIZATION],
            if custom {
                "Bearer refreshed"
            } else {
                "Bearer api-key"
            }
        );
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
    }
}

#[tokio::test]
async fn invalid_configured_header_is_rejected() {
    let config =
        ClientConfig::new("api-key", "test").with_headers([("x-invalid", "invalid\nheader")]);
    assert!(matches!(
        Transport::new(
            Channel::from_static("http://localhost:1").connect_lazy(),
            &config
        ),
        Err(Error::Input(_))
    ));
}
