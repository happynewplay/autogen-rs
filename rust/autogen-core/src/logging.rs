use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use crate::agent_id::AgentId;
use crate::message_handler_context::MessageHandlerContext;
use crate::topic::TopicId;

/// Event for logging LLM calls
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMCallEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub messages: Vec<Map<String, Value>>,
    pub response: Map<String, Value>,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub agent_id: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl LLMCallEvent {
    pub fn new(
        messages: Vec<Map<String, Value>>,
        response: Map<String, Value>,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> Self {
        let agent_id = MessageHandlerContext::current_agent_id()
            .map(|id| id.to_string())
            .ok();

        Self {
            event_type: "LLMCall".to_string(),
            messages,
            response,
            prompt_tokens,
            completion_tokens,
            agent_id,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for LLMCallEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "LLMCallEvent serialization error"),
        }
    }
}

/// Event for logging LLM stream start
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMStreamStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub messages: Vec<Map<String, Value>>,
    pub agent_id: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl LLMStreamStartEvent {
    pub fn new(messages: Vec<Map<String, Value>>) -> Self {
        let agent_id = MessageHandlerContext::current_agent_id()
            .map(|id| id.to_string())
            .ok();

        Self {
            event_type: "LLMStreamStart".to_string(),
            messages,
            agent_id,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for LLMStreamStartEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "LLMStreamStartEvent serialization error"),
        }
    }
}

/// Event for logging LLM stream end
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMStreamEndEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub response: Map<String, Value>,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub agent_id: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl LLMStreamEndEvent {
    pub fn new(
        response: Map<String, Value>,
        prompt_tokens: i32,
        completion_tokens: i32,
    ) -> Self {
        let agent_id = MessageHandlerContext::current_agent_id()
            .map(|id| id.to_string())
            .ok();

        Self {
            event_type: "LLMStreamEnd".to_string(),
            response,
            prompt_tokens,
            completion_tokens,
            agent_id,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for LLMStreamEndEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "LLMStreamEndEvent serialization error"),
        }
    }
}

/// Event for logging tool calls
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub tool_name: String,
    pub arguments: Map<String, Value>,
    pub result: String,
    pub agent_id: Option<String>,
}

impl ToolCallEvent {
    pub fn new(
        tool_name: String,
        arguments: Map<String, Value>,
        result: String,
    ) -> Self {
        let agent_id = MessageHandlerContext::current_agent_id()
            .map(|id| id.to_string())
            .ok();

        Self {
            event_type: "ToolCall".to_string(),
            tool_name,
            arguments,
            result,
            agent_id,
        }
    }
}

impl std::fmt::Display for ToolCallEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "ToolCallEvent serialization error"),
        }
    }
}

/// Message kinds for message events
#[derive(Debug, Serialize, Deserialize)]
pub enum MessageKind {
    #[serde(rename = "MessageKind.DIRECT")]
    Direct,
    #[serde(rename = "MessageKind.PUBLISH")]
    Publish,
    #[serde(rename = "MessageKind.RESPOND")]
    Respond,
}

impl std::fmt::Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageKind::Direct => write!(f, "MessageKind.DIRECT"),
            MessageKind::Publish => write!(f, "MessageKind.PUBLISH"),
            MessageKind::Respond => write!(f, "MessageKind.RESPOND"),
        }
    }
}

/// Delivery stages for message events
#[derive(Debug, Serialize, Deserialize)]
pub enum DeliveryStage {
    #[serde(rename = "DeliveryStage.SEND")]
    Send,
    #[serde(rename = "DeliveryStage.DELIVER")]
    Deliver,
}

impl std::fmt::Display for DeliveryStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryStage::Send => write!(f, "DeliveryStage.SEND"),
            DeliveryStage::Deliver => write!(f, "DeliveryStage.DELIVER"),
        }
    }
}

/// Event for logging messages
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: String,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub kind: String,
    pub delivery_stage: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl MessageEvent {
    pub fn new(
        payload: String,
        sender: Option<AgentId>,
        receiver: Option<String>, // Can be AgentId or TopicId
        kind: MessageKind,
        delivery_stage: DeliveryStage,
    ) -> Self {
        Self {
            event_type: "Message".to_string(),
            payload,
            sender: sender.map(|id| id.to_string()),
            receiver,
            kind: kind.to_string(),
            delivery_stage: delivery_stage.to_string(),
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for MessageEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "MessageEvent serialization error"),
        }
    }
}

/// Event for logging dropped messages
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageDroppedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: String,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub kind: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl MessageDroppedEvent {
    pub fn new(
        payload: String,
        sender: Option<AgentId>,
        receiver: Option<String>,
        kind: MessageKind,
    ) -> Self {
        Self {
            event_type: "MessageDropped".to_string(),
            payload,
            sender: sender.map(|id| id.to_string()),
            receiver,
            kind: kind.to_string(),
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for MessageDroppedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "MessageDroppedEvent serialization error"),
        }
    }
}

/// Event for logging message handler exceptions
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageHandlerExceptionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: String,
    pub handling_agent: String,
    pub exception: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl MessageHandlerExceptionEvent {
    pub fn new(
        payload: String,
        handling_agent: AgentId,
        exception: String,
    ) -> Self {
        Self {
            event_type: "MessageHandlerException".to_string(),
            payload,
            handling_agent: handling_agent.to_string(),
            exception,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for MessageHandlerExceptionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "MessageHandlerExceptionEvent serialization error"),
        }
    }
}

/// Event for logging agent construction exceptions
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConstructionExceptionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub agent_id: String,
    pub exception: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl AgentConstructionExceptionEvent {
    pub fn new(agent_id: AgentId, exception: String) -> Self {
        Self {
            event_type: "AgentConstructionException".to_string(),
            agent_id: agent_id.to_string(),
            exception,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl std::fmt::Display for AgentConstructionExceptionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(_) => write!(f, "AgentConstructionExceptionEvent serialization error"),
        }
    }
}