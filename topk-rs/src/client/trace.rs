use async_trait::async_trait;
use tonic::{service::Interceptor, Status};

use crate::client::interceptor::{AppendHeadersInterceptor, AsyncInterceptor};

#[derive(Clone)]
pub struct TracingInterceptor {
    headers: AppendHeadersInterceptor,
}

impl TracingInterceptor {
    pub fn new(headers: AppendHeadersInterceptor) -> Self {
        Self { headers }
    }

    fn apply(&self, request: tonic::Request<()>) -> tonic::Request<()> {
        #[cfg(feature = "trace")]
        let request = {
            let mut request = request;
            inner::inject(request.metadata_mut());
            request
        };
        self.headers.apply(request)
    }
}

impl Interceptor for TracingInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        Ok(self.apply(request))
    }
}

#[async_trait]
impl AsyncInterceptor for TracingInterceptor {
    async fn call(&self, request: tonic::Request<()>) -> anyhow::Result<tonic::Request<()>> {
        Ok(self.apply(request))
    }
}

#[cfg(feature = "trace")]
mod inner {
    use std::str::FromStr;

    use opentelemetry::global;
    use opentelemetry::propagation::Injector;
    use tonic::metadata::{MetadataKey, MetadataMap};
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    struct MetadataInjector<'a>(&'a mut MetadataMap);

    impl<'a> Injector for MetadataInjector<'a> {
        fn set(&mut self, key: &str, value: String) {
            if let Ok(key) = MetadataKey::from_str(key) {
                if let Ok(val) = value.parse() {
                    self.0.insert(key, val);
                }
            }
        }
    }

    pub fn inject(metadata: &mut MetadataMap) {
        let cx = tracing::Span::current().context();
        global::get_text_map_propagator(|p| {
            p.inject_context(&cx, &mut MetadataInjector(metadata));
        });
    }
}
