use opentelemetry::trace::SpanKind;
use opentelemetry::KeyValue;
use std::collections::HashMap;
use std::fmt::Display;

pub trait TracingConfig<Operation, Destination, ExtraAttributes> {
    fn name(&self) -> &str;
    fn build_attributes(
        &self,
        operation: &Operation,
        destination: &Destination,
        extra_attributes: Option<&ExtraAttributes>,
    ) -> HashMap<String, KeyValue>;
    fn get_span_name(&self, operation: &Operation, destination: &Destination) -> String;
    fn get_span_kind(&self, operation: &Operation) -> SpanKind;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessagingOperation {
    Create,
    Send,
    Publish,
    Receive,
    Intercept,
    Process,
    Ack,
}

impl Display for MessagingOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// In Rust, we'd use a more idiomatic way to represent this, perhaps an enum or structs.
// For now, a String can represent the destination.
pub type MessagingDestination = String;

#[derive(Default)]
pub struct ExtraMessageRuntimeAttributes {
    pub message_size: Option<i64>,
    pub message_type: Option<String>,
}

pub struct MessageRuntimeTracingConfig {
    runtime_name: String,
}

impl MessageRuntimeTracingConfig {
    pub fn new(runtime_name: impl Into<String>) -> Self {
        Self {
            runtime_name: runtime_name.into(),
        }
    }

    fn get_operation_type(&self, operation: &MessagingOperation) -> &'static str {
        match operation {
            MessagingOperation::Send | MessagingOperation::Publish => "publish",
            MessagingOperation::Create => "create",
            MessagingOperation::Receive | MessagingOperation::Intercept | MessagingOperation::Ack => "receive",
            MessagingOperation::Process => "process",
        }
    }
}

impl TracingConfig<MessagingOperation, MessagingDestination, ExtraMessageRuntimeAttributes>
    for MessageRuntimeTracingConfig
{
    fn name(&self) -> &str {
        &self.runtime_name
    }

    fn build_attributes(
        &self,
        operation: &MessagingOperation,
        destination: &MessagingDestination,
        extra_attributes: Option<&ExtraMessageRuntimeAttributes>,
    ) -> HashMap<String, KeyValue> {
        let mut attributes = HashMap::new();
        attributes.insert(
            "messaging.operation".to_string(),
            KeyValue::new("messaging.operation", self.get_operation_type(operation)),
        );
        attributes.insert(
            "messaging.destination".to_string(),
            KeyValue::new("messaging.destination", destination.clone()),
        );

        if let Some(extra) = extra_attributes {
            if let Some(size) = extra.message_size {
                attributes.insert(
                    "messaging.message.envelope.size".to_string(),
                    KeyValue::new("messaging.message.envelope.size", size),
                );
            }
            if let Some(msg_type) = &extra.message_type {
                attributes.insert(
                    "messaging.message.type".to_string(),
                    KeyValue::new("messaging.message.type", msg_type.clone()),
                );
            }
        }

        attributes
    }

    fn get_span_name(&self, operation: &MessagingOperation, destination: &MessagingDestination) -> String {
        format!("{} {} {}", super::constants::NAMESPACE, operation.to_string().to_lowercase(), destination)
    }

    fn get_span_kind(&self, operation: &MessagingOperation) -> SpanKind {
        match operation {
            MessagingOperation::Create | MessagingOperation::Send | MessagingOperation::Publish => SpanKind::Producer,
            MessagingOperation::Receive
            | MessagingOperation::Intercept
            | MessagingOperation::Process
            | MessagingOperation::Ack => SpanKind::Consumer,
        }
    }
}