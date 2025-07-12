use opentelemetry::trace::{SpanKind, Status, Tracer, TraceContextExt, TracerProvider};
use opentelemetry::{global, KeyValue};
use std::error::Error;

pub const GEN_AI_SYSTEM: &str = "gen_ai.system";
pub const GEN_AI_OPERATION_NAME: &str = "gen_ai.operation.name";
pub const GEN_AI_AGENT_NAME: &str = "gen_ai.agent.name";
pub const GEN_AI_AGENT_ID: &str = "gen_ai.agent.id";
pub const GEN_AI_AGENT_DESCRIPTION: &str = "gen_ai.agent.description";
pub const GEN_AI_TOOL_NAME: &str = "gen_ai.tool.name";
pub const GEN_AI_TOOL_DESCRIPTION: &str = "gen_ai.tool.description";
pub const GEN_AI_TOOL_CALL_ID: &str = "gen_ai.tool.call_id";

pub enum GenAiOperationNameValues {
    ExecuteTool,
    CreateAgent,
    InvokeAgent,
}

impl GenAiOperationNameValues {
    fn as_str(&self) -> &'static str {
        match self {
            GenAiOperationNameValues::ExecuteTool => "execute_tool",
            GenAiOperationNameValues::CreateAgent => "create_agent",
            GenAiOperationNameValues::InvokeAgent => "invoke_agent",
        }
    }
}

fn get_tracer() -> opentelemetry::global::BoxedTracer {
    global::tracer_provider().tracer("autogen-core")
}

fn trace_span<F, R>(
    name: String,
    kind: SpanKind,
    attributes: Vec<KeyValue>,
    f: F,
) -> R
where
    F: FnOnce() -> Result<R, Box<dyn Error>>,
{
    let tracer = get_tracer();
    let span = tracer.span_builder(name).with_kind(kind).with_attributes(attributes).start(&tracer);
    let cx = opentelemetry::Context::current().with_span(span);
    let _guard = cx.attach();

    match f() {
        Ok(result) => {
            let cx = opentelemetry::Context::current();
            let span = cx.span();
            span.set_status(Status::Ok);
            result
        }
        Err(e) => {
            let cx = opentelemetry::Context::current();
            let span = cx.span();
            span.record_error(&*e);
            span.set_status(Status::Error { description: e.to_string().into() });
            // This is tricky in Rust, as we can't just re-raise.
            // The caller will have to handle the error.
            // For now, we panic to indicate a problem.
            panic!("Error in traced block: {}", e);
        }
    }
}

pub fn trace_tool_span<F, R>(
    tool_name: &str,
    tool_description: Option<&str>,
    tool_call_id: Option<&str>,
    f: F,
) -> R
where
    F: FnOnce() -> Result<R, Box<dyn Error>>,
{
    let mut attributes = vec![
        KeyValue::new(GEN_AI_OPERATION_NAME, GenAiOperationNameValues::ExecuteTool.as_str()),
        KeyValue::new(GEN_AI_SYSTEM, "autogen"),
        KeyValue::new(GEN_AI_TOOL_NAME, tool_name.to_string()),
    ];
    if let Some(desc) = tool_description {
        attributes.push(KeyValue::new(GEN_AI_TOOL_DESCRIPTION, desc.to_string()));
    }
    if let Some(id) = tool_call_id {
        attributes.push(KeyValue::new(GEN_AI_TOOL_CALL_ID, id.to_string()));
    }

    trace_span(
        format!("{} {}", GenAiOperationNameValues::ExecuteTool.as_str(), tool_name),
        SpanKind::Internal,
        attributes,
        f,
    )
}

pub fn trace_create_agent_span<F, R>(
    agent_name: &str,
    agent_id: Option<&str>,
    agent_description: Option<&str>,
    f: F,
) -> R
where
    F: FnOnce() -> Result<R, Box<dyn Error>>,
{
    let mut attributes = vec![
        KeyValue::new(GEN_AI_OPERATION_NAME, GenAiOperationNameValues::CreateAgent.as_str()),
        KeyValue::new(GEN_AI_SYSTEM, "autogen"),
        KeyValue::new(GEN_AI_AGENT_NAME, agent_name.to_string()),
    ];
    if let Some(id) = agent_id {
        attributes.push(KeyValue::new(GEN_AI_AGENT_ID, id.to_string()));
    }
    if let Some(desc) = agent_description {
        attributes.push(KeyValue::new(GEN_AI_AGENT_DESCRIPTION, desc.to_string()));
    }

    trace_span(
        format!("{} {}", GenAiOperationNameValues::CreateAgent.as_str(), agent_name),
        SpanKind::Client,
        attributes,
        f,
    )
}

pub fn trace_invoke_agent_span<F, R>(
    agent_name: &str,
    agent_id: Option<&str>,
    agent_description: Option<&str>,
    f: F,
) -> R
where
    F: FnOnce() -> Result<R, Box<dyn Error>>,
{
    let mut attributes = vec![
        KeyValue::new(GEN_AI_OPERATION_NAME, GenAiOperationNameValues::InvokeAgent.as_str()),
        KeyValue::new(GEN_AI_SYSTEM, "autogen"),
        KeyValue::new(GEN_AI_AGENT_NAME, agent_name.to_string()),
    ];
    if let Some(id) = agent_id {
        attributes.push(KeyValue::new(GEN_AI_AGENT_ID, id.to_string()));
    }
    if let Some(desc) = agent_description {
        attributes.push(KeyValue::new(GEN_AI_AGENT_DESCRIPTION, desc.to_string()));
    }

    trace_span(
        format!("{} {}", GenAiOperationNameValues::InvokeAgent.as_str(), agent_name),
        SpanKind::Client,
        attributes,
        f,
    )
}