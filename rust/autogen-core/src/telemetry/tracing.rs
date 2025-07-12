use super::propagation::{get_telemetry_links, EnvelopeMetadata};
use super::tracing_config::TracingConfig;
use opentelemetry::trace::{SpanKind, Tracer, TracerProvider, TraceContextExt};
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::Tracer as SdkTracer;
use std::fmt::Display;
use std::sync::Arc;

pub struct TraceHelper<Operation, Destination, ExtraAttributes> {
    tracer: Arc<SdkTracer>,
    instrumentation_builder_config: Box<dyn TracingConfig<Operation, Destination, ExtraAttributes> + Send + Sync>,
}

impl<
        Operation: Display + Send + Sync + 'static,
        Destination: Display + Send + Sync + 'static,
        ExtraAttributes: Send + Sync + 'static,
    > TraceHelper<Operation, Destination, ExtraAttributes>
{
    pub fn new(
        tracer_provider: Option<Arc<opentelemetry_sdk::trace::TracerProvider>>,
        instrumentation_builder_config: Box<dyn TracingConfig<Operation, Destination, ExtraAttributes> + Send + Sync>,
    ) -> Self {
        // Get the name before moving the config
        let tracer_name = instrumentation_builder_config.name().to_string();

        // Simplified tracer creation to avoid complex type conversions
        let tracer: Arc<SdkTracer> = if let Some(provider) = tracer_provider {
            Arc::new(provider.tracer(tracer_name.clone()))
        } else {
            // Create a default SDK tracer
            let provider = opentelemetry_sdk::trace::TracerProvider::default();
            Arc::new(provider.tracer(tracer_name))
        };

        Self {
            tracer,
            instrumentation_builder_config,
        }
    }

    // This is not a context manager in Rust, but a function that takes a closure.
    pub fn trace_block<F, R>(
        &self,
        operation: Operation,
        destination: Destination,
        parent: Option<&EnvelopeMetadata>,
        extra_attributes: Option<&ExtraAttributes>,
        kind: Option<SpanKind>,
        f: F,
    ) -> R
    where
        F: FnOnce() -> R,
    {
        let span_name = self
            .instrumentation_builder_config
            .get_span_name(&operation, &destination);
        let span_kind = kind.unwrap_or_else(|| self.instrumentation_builder_config.get_span_kind(&operation));
        let links = get_telemetry_links(parent);
        let attributes = self
            .instrumentation_builder_config
            .build_attributes(&operation, &destination, extra_attributes);

        let mut span_builder = self.tracer.span_builder(span_name).with_kind(span_kind);
        if let Some(links) = links {
            span_builder = span_builder.with_links(links);
        }
        let mut kv_attributes: Vec<KeyValue> = Vec::new();
        for (_, v) in attributes {
            kv_attributes.push(v);
        }

        span_builder = span_builder.with_attributes(kv_attributes);

        let span = span_builder.start(self.tracer.as_ref());
        let cx = opentelemetry::Context::current().with_span(span);
        let _guard = cx.attach();

        f()
    }
}