use opentelemetry::trace::{Link, SpanContext, TraceContextExt};
use opentelemetry::{Context, propagation::TextMapPropagator};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvelopeMetadata {
    pub traceparent: Option<String>,
    pub tracestate: Option<String>,
    #[serde(skip)]
    pub links: Option<Vec<Link>>,
}

fn get_carrier_for_envelope_metadata(envelope_metadata: &EnvelopeMetadata) -> HashMap<String, String> {
    let mut carrier = HashMap::new();
    if let Some(traceparent) = &envelope_metadata.traceparent {
        carrier.insert("traceparent".to_string(), traceparent.clone());
    }
    if let Some(tracestate) = &envelope_metadata.tracestate {
        carrier.insert("tracestate".to_string(), tracestate.clone());
    }
    carrier
}

pub fn get_telemetry_envelope_metadata() -> EnvelopeMetadata {
    let propagator = TraceContextPropagator::new();
    let mut carrier = HashMap::new();
    propagator.inject_context(&Context::current(), &mut carrier);
    EnvelopeMetadata {
        traceparent: carrier.get("traceparent").cloned(),
        tracestate: carrier.get("tracestate").cloned(),
        links: None,
    }
}

pub fn get_telemetry_context(metadata: Option<&EnvelopeMetadata>) -> Context {
    if let Some(md) = metadata {
        let carrier = get_carrier_for_envelope_metadata(md);
        let propagator = TraceContextPropagator::new();
        propagator.extract(&carrier)
    } else {
        Context::new()
    }
}

pub fn get_telemetry_links(metadata: Option<&EnvelopeMetadata>) -> Option<Vec<Link>> {
    if let Some(md) = metadata {
        let context = get_telemetry_context(Some(md));
        let span = context.span();
        let span_context = span.span_context().clone();
        if span_context.is_valid() {
            Some(vec![Link::new(span_context, vec![])])
        } else {
            None
        }
    } else {
        None
    }
}