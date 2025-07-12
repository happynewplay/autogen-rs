use crate::agent_id::AgentId;
use crate::message_context::MessageContext;
use async_trait::async_trait;
use serde_json::Value;

/// Marker type for signalling that a message should be dropped by an intervention handler.
#[derive(Debug)]
pub struct DropMessage;

/// An intervention handler is a trait that can be used to modify, log or drop messages
/// that are being processed by the `AgentRuntime`.
///
/// The handler is called when the message is submitted to the runtime.
#[async_trait]
pub trait InterventionHandler: Send + Sync {
    /// Clone the intervention handler
    fn clone_box(&self) -> Box<dyn InterventionHandler>;
    /// Called when a message is submitted to the AgentRuntime using `send_message`.
    async fn on_send(
        &self,
        message: Value,
        message_context: &MessageContext,
        recipient: &AgentId,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }

    /// Called when a message is published to the AgentRuntime using `publish_message`.
    async fn on_publish(
        &self,
        message: Value,
        message_context: &MessageContext,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }

    /// Called when a response is received by the AgentRuntime from an Agent's message handler.
    async fn on_response(
        &self,
        message: Value,
        sender: &AgentId,
        recipient: Option<&AgentId>,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }
}

/// Simple struct that provides a default implementation for all intervention
/// handler methods, that simply returns the message unchanged. Allows for easy
/// subclassing to override only the desired methods.
#[derive(Default, Clone)]
pub struct DefaultInterventionHandler;

impl DefaultInterventionHandler {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl InterventionHandler for DefaultInterventionHandler {
    fn clone_box(&self) -> Box<dyn InterventionHandler> {
        Box::new(self.clone())
    }
}