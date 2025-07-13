use crate::agent_id::AgentId;
use crate::message_context::MessageContext;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Marker type for signalling that a message should be dropped by an intervention handler.
#[derive(Debug)]
pub struct DropMessage;

/// Context for intervention handling, providing all necessary information about the message
/// and its routing for intervention handlers to make decisions.
#[derive(Debug, Clone)]
pub struct InterventionContext {
    /// The message being processed
    pub message: Value,
    /// The sender of the message (if any)
    pub sender: Option<AgentId>,
    /// The intended recipient of the message
    pub recipient: AgentId,
    /// Additional metadata associated with the message
    pub metadata: HashMap<String, Value>,
    /// Timestamp when the intervention context was created
    pub timestamp: std::time::SystemTime,
}

impl InterventionContext {
    /// Create a new intervention context
    pub fn new(
        message: Value,
        sender: Option<AgentId>,
        recipient: AgentId,
    ) -> Self {
        Self {
            message,
            sender,
            recipient,
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create an intervention context with metadata
    pub fn with_metadata(
        message: Value,
        sender: Option<AgentId>,
        recipient: AgentId,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            message,
            sender,
            recipient,
            metadata,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Actions that can be taken by an intervention handler in response to a message.
#[derive(Debug, Clone)]
pub enum InterventionAction {
    /// Continue processing the message unchanged
    Continue,
    /// Modify the message content and continue processing
    ModifyMessage(Value),
    /// Drop the message (do not process it)
    DropMessage,
    /// Redirect the message to a different agent
    Redirect(AgentId),
    /// Delay processing of the message by the specified duration
    Delay(std::time::Duration),
}

impl InterventionAction {
    /// Convert an InterventionAction to the Result type used by the current InterventionHandler trait
    pub fn to_result(self) -> Result<Value, DropMessage> {
        match self {
            InterventionAction::Continue => Err(DropMessage), // This case needs special handling
            InterventionAction::ModifyMessage(value) => Ok(value),
            InterventionAction::DropMessage => Err(DropMessage),
            InterventionAction::Redirect(_) => Err(DropMessage), // Redirect not supported in current API
            InterventionAction::Delay(_) => Err(DropMessage), // Delay not supported in current API
        }
    }

    /// Create an InterventionAction from a Result type
    pub fn from_result(result: Result<Value, DropMessage>, original_message: Value) -> Self {
        match result {
            Ok(modified_message) => {
                if modified_message == original_message {
                    InterventionAction::Continue
                } else {
                    InterventionAction::ModifyMessage(modified_message)
                }
            }
            Err(_) => InterventionAction::DropMessage,
        }
    }
}

/// An intervention handler is a trait that can be used to modify, log or drop messages
/// that are being processed by the `AgentRuntime`.
///
/// The handler is called when the message is submitted to the runtime.
///
/// # Example
///
/// ```rust
/// use autogen_core::intervention::{InterventionHandler, InterventionContext, InterventionAction};
/// use autogen_core::message_context::MessageContext;
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use std::error::Error;
///
/// #[derive(Clone)]
/// struct LoggingInterventionHandler;
///
/// #[async_trait]
/// impl InterventionHandler for LoggingInterventionHandler {
///     fn clone_box(&self) -> Box<dyn InterventionHandler> {
///         Box::new(self.clone())
///     }
///
///     fn name(&self) -> &str {
///         "logging_handler"
///     }
///
///     async fn handle_intervention(
///         &self,
///         context: &mut InterventionContext,
///         _message_context: &MessageContext,
///     ) -> Result<InterventionAction, Box<dyn Error + Send + Sync>> {
///         println!("Processing message: {:?}", context.message);
///         Ok(InterventionAction::Continue)
///     }
/// }
/// ```
#[async_trait]
pub trait InterventionHandler: Send + Sync {
    /// Clone the intervention handler
    fn clone_box(&self) -> Box<dyn InterventionHandler>;

    /// Called when a message is submitted to the AgentRuntime using `send_message`.
    async fn on_send(
        &self,
        message: Value,
        _message_context: &MessageContext,
        _recipient: &AgentId,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }

    /// Called when a message is published to the AgentRuntime using `publish_message`.
    async fn on_publish(
        &self,
        message: Value,
        _message_context: &MessageContext,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }

    /// Called when a response is received by the AgentRuntime from an Agent's message handler.
    async fn on_response(
        &self,
        message: Value,
        _sender: &AgentId,
        _recipient: Option<&AgentId>,
    ) -> Result<Value, DropMessage> {
        Ok(message)
    }

    /// Advanced intervention method using the new context and action types.
    /// If implemented, this method will be called instead of the specific on_* methods.
    /// Default implementation delegates to the appropriate on_* method based on context.
    async fn handle_intervention(
        &self,
        context: &mut InterventionContext,
        message_context: &MessageContext,
    ) -> Result<InterventionAction, Box<dyn std::error::Error + Send + Sync>> {
        // Default implementation delegates to existing methods
        let result = if message_context.is_rpc {
            self.on_send(
                context.message.clone(),
                message_context,
                &context.recipient,
            ).await
        } else if message_context.topic_id.is_some() {
            self.on_publish(
                context.message.clone(),
                message_context,
            ).await
        } else {
            // Assume it's a response if not RPC and no topic
            self.on_response(
                context.message.clone(),
                &context.recipient, // In response context, recipient is actually the sender
                context.sender.as_ref(),
            ).await
        };

        Ok(InterventionAction::from_result(result, context.message.clone()))
    }

    /// Get the name of this intervention handler
    fn name(&self) -> &str {
        "unnamed_handler"
    }

    /// Get the priority of this intervention handler (lower numbers = higher priority)
    fn priority(&self) -> i32 {
        0
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

    fn name(&self) -> &str {
        "default_intervention_handler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_id::AgentId;
    use crate::cancellation_token::CancellationToken;

    #[tokio::test]
    async fn test_intervention_context_creation() {
        let message = serde_json::json!({"test": "message"});
        let sender = Some(AgentId::from_str("sender/key").unwrap());
        let recipient = AgentId::from_str("recipient/key").unwrap();

        let context = InterventionContext::new(message.clone(), sender.clone(), recipient.clone());

        assert_eq!(context.message, message);
        assert_eq!(context.sender, sender);
        assert_eq!(context.recipient, recipient);
        assert!(context.metadata.is_empty());
        assert!(context.timestamp <= std::time::SystemTime::now());
    }

    #[tokio::test]
    async fn test_intervention_context_with_metadata() {
        let message = serde_json::json!({"test": "message"});
        let sender = Some(AgentId::from_str("sender/key").unwrap());
        let recipient = AgentId::from_str("recipient/key").unwrap();
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let context = InterventionContext::with_metadata(
            message.clone(),
            sender.clone(),
            recipient.clone(),
            metadata.clone(),
        );

        assert_eq!(context.message, message);
        assert_eq!(context.sender, sender);
        assert_eq!(context.recipient, recipient);
        assert_eq!(context.metadata, metadata);
    }

    #[tokio::test]
    async fn test_intervention_action_conversions() {
        let original_message = serde_json::json!({"original": true});
        let modified_message = serde_json::json!({"modified": true});

        // Test Continue action
        let _action = InterventionAction::Continue;
        let result = InterventionAction::from_result(Ok(original_message.clone()), original_message.clone());
        matches!(result, InterventionAction::Continue);

        // Test ModifyMessage action
        let _action = InterventionAction::ModifyMessage(modified_message.clone());
        let result = InterventionAction::from_result(Ok(modified_message.clone()), original_message.clone());
        matches!(result, InterventionAction::ModifyMessage(_));

        // Test DropMessage action
        let _action = InterventionAction::DropMessage;
        let result = InterventionAction::from_result(Err(DropMessage), original_message.clone());
        matches!(result, InterventionAction::DropMessage);
    }

    #[tokio::test]
    async fn test_default_intervention_handler_new_methods() {
        let handler = DefaultInterventionHandler::new();

        assert_eq!(handler.name(), "default_intervention_handler");
        assert_eq!(handler.priority(), 0);

        // Test the new handle_intervention method
        let message = serde_json::json!({"test": "message"});
        let sender = Some(AgentId::from_str("sender/key").unwrap());
        let recipient = AgentId::from_str("recipient/key").unwrap();
        let mut context = InterventionContext::new(message.clone(), sender, recipient);

        let message_context = MessageContext {
            sender: context.sender.clone(),
            topic_id: None,
            is_rpc: true,
            cancellation_token: CancellationToken::default(),
            message_id: "test_id".to_string(),
        };

        let result = handler.handle_intervention(&mut context, &message_context).await;
        assert!(result.is_ok());
        matches!(result.unwrap(), InterventionAction::Continue);
    }
}