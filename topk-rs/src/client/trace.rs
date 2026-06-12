use tonic::{service::Interceptor, Status};

use crate::client::interceptor::AppendHeadersInterceptor;

#[derive(Clone)]
pub struct TracingInterceptor {
    headers: AppendHeadersInterceptor,
}

impl TracingInterceptor {
    pub fn new(headers: AppendHeadersInterceptor) -> Self {
        Self { headers }
    }
}

impl Interceptor for TracingInterceptor {
    #[cfg(feature = "trace")]
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        inner::inject(request.metadata_mut());

        self.headers.call(request)
    }

    #[cfg(not(feature = "trace"))]
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        self.headers.call(request)
    }
}

#[cfg(feature = "trace")]
mod inner {
    use opentelemetry::global;
    use opentelemetry::propagation::Injector;
    use std::str::FromStr;
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
