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

pub(super) fn inject(metadata: &mut MetadataMap) {
    let cx = tracing::Span::current().context();
    global::get_text_map_propagator(|p| {
        p.inject_context(&cx, &mut MetadataInjector(metadata));
    });
}
