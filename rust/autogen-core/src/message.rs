//! Message handling and context system
//!
//! This module provides the message handling system used for agent communication
//! in the autogen system, following the Python autogen-core design.

use crate::{AgentId, CancellationToken, TopicId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use uuid::Uuid;

/// Core trait that all messages must implement
///
/// This trait provides type safety and serialization capabilities for messages
/// in the autogen system. All messages must be Send + Sync for thread safety.
pub trait Message: Send + Sync + Debug + 'static {
    /// The type of response this message expects (if any)
    type Response: Send + Sync + Debug + 'static;

    /// Get the type name of this message for routing
    fn message_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Get the TypeId for this message type
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Validate the message content (optional override)
    fn validate(&self) -> crate::Result<()> {
        Ok(())
    }
}

/// A typed message envelope that preserves type information
#[derive(Debug)]
pub struct TypedMessageEnvelope<M: Message> {
    /// The actual message payload
    pub message: M,
    /// Message context
    pub context: MessageContext,
}

impl<M: Message> TypedMessageEnvelope<M> {
    /// Create a new typed message envelope
    pub fn new(message: M, context: MessageContext) -> Self {
        Self { message, context }
    }

    /// Convert to an untyped envelope for storage/routing
    pub fn into_untyped(self) -> UntypedMessageEnvelope {
        UntypedMessageEnvelope {
            message: Box::new(self.message),
            context: self.context,
            message_type: std::any::type_name::<M>(),
            type_id: TypeId::of::<M>(),
        }
    }
}

/// Untyped message envelope for internal routing
#[derive(Debug)]
pub struct UntypedMessageEnvelope {
    /// The message as a trait object
    pub message: Box<dyn Any + Send + Sync>,
    /// Message context
    pub context: MessageContext,
    /// Type name for routing
    pub message_type: &'static str,
    /// TypeId for safe downcasting
    pub type_id: TypeId,
}

/// High-performance message router using type-based dispatch
#[derive(Debug)]
pub struct MessageRouter {
    /// Type-specific handlers for fast dispatch
    handlers: std::collections::HashMap<TypeId, Box<dyn MessageHandlerDispatch>>,
}

/// Trait for type-specific message handling dispatch
trait MessageHandlerDispatch: Send + Sync {
    /// Dispatch a message to the appropriate handler
    fn dispatch(&self, envelope: UntypedMessageEnvelope) -> Result<Option<UntypedMessageEnvelope>>;

    /// Get the message type this handler supports
    fn message_type(&self) -> TypeId;
}

/// Concrete implementation of message handler dispatch
struct TypedMessageHandlerDispatch<M: Message> {
    handler: Box<dyn Fn(TypedMessageEnvelope<M>) -> Result<Option<TypedMessageEnvelope<M::Response>>> + Send + Sync>,
}

impl<M: Message> MessageHandlerDispatch for TypedMessageHandlerDispatch<M> {
    fn dispatch(&self, envelope: UntypedMessageEnvelope) -> Result<Option<UntypedMessageEnvelope>> {
        // Fast path: check type ID first
        if envelope.type_id != TypeId::of::<M>() {
            return Err(crate::AutoGenError::other(format!(
                "Type mismatch: expected {}, got {}",
                std::any::type_name::<M>(),
                envelope.message_type
            )));
        }

        // Safe downcast (we've verified the type)
        let typed_envelope = envelope.downcast::<M>().map_err(|_| {
            crate::AutoGenError::other("Failed to downcast message despite type check")
        })?;

        // Call the handler
        if let Some(response_envelope) = (self.handler)(typed_envelope)? {
            Ok(Some(response_envelope.into_untyped()))
        } else {
            Ok(None)
        }
    }

    fn message_type(&self) -> TypeId {
        TypeId::of::<M>()
    }
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    /// Register a typed message handler
    pub fn register_handler<M: Message, F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(TypedMessageEnvelope<M>) -> Result<Option<TypedMessageEnvelope<M::Response>>> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<M>();
        let dispatch = TypedMessageHandlerDispatch {
            handler: Box::new(handler),
        };

        if self.handlers.insert(type_id, Box::new(dispatch)).is_some() {
            return Err(crate::AutoGenError::other(format!(
                "Handler for type {} already registered",
                std::any::type_name::<M>()
            )));
        }

        Ok(())
    }

    /// Route a message to the appropriate handler
    pub fn route(&self, envelope: UntypedMessageEnvelope) -> Result<Option<UntypedMessageEnvelope>> {
        if let Some(handler) = self.handlers.get(&envelope.type_id) {
            handler.dispatch(envelope)
        } else {
            Err(crate::AutoGenError::other(format!(
                "No handler registered for message type: {}",
                envelope.message_type
            )))
        }
    }

    /// Get the number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Check if a handler is registered for a specific type
    pub fn has_handler<M: Message>(&self) -> bool {
        self.handlers.contains_key(&TypeId::of::<M>())
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl UntypedMessageEnvelope {
    /// Attempt to downcast to a specific message type
    pub fn downcast<M: Message>(self) -> Result<TypedMessageEnvelope<M>, Self> {
        if self.type_id == TypeId::of::<M>() {
            match self.message.downcast::<M>() {
                Ok(message) => Ok(TypedMessageEnvelope::new(*message, self.context)),
                Err(message) => Err(UntypedMessageEnvelope {
                    message,
                    context: self.context,
                    message_type: self.message_type,
                    type_id: self.type_id,
                }),
            }
        } else {
            Err(self)
        }
    }

    /// Get the message type name
    pub fn message_type(&self) -> &'static str {
        self.message_type
    }
}

/// Unit type for messages that don't expect a response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoResponse;

/// Basic text message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub content: String,
}

impl Message for TextMessage {
    type Response = NoResponse;
}

/// Request-response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage<T> {
    pub request: T,
}

impl<T> Message for RequestMessage<T>
where
    T: Send + Sync + Debug + 'static,
{
    type Response = ResponseMessage<T>;
}

/// Response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage<T> {
    pub response: T,
    pub success: bool,
    pub error: Option<String>,
}

impl<T> Message for ResponseMessage<T>
where
    T: Send + Sync + Debug + 'static,
{
    type Response = NoResponse;
}

/// Context information for message handling
///
/// MessageContext provides metadata about a message being processed,
/// including sender information, topic details, and cancellation support.
/// This follows the Python autogen-core MessageContext design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContext {
    /// The agent that sent the message (None if sent externally)
    pub sender: Option<AgentId>,
    
    /// The topic this message was published to (None for direct messages)
    pub topic_id: Option<TopicId>,
    
    /// Whether this is an RPC (request-response) message
    pub is_rpc: bool,
    
    /// Token for cancelling the message processing
    #[serde(skip)]
    pub cancellation_token: CancellationToken,
    
    /// Unique identifier for this message
    pub message_id: String,
}

impl MessageContext {
    /// Create a new MessageContext
    ///
    /// # Arguments
    /// * `sender` - The agent that sent the message
    /// * `topic_id` - The topic this message was published to
    /// * `is_rpc` - Whether this is an RPC message
    /// * `cancellation_token` - Token for cancellation
    /// * `message_id` - Unique message identifier (generates UUID if None)
    pub fn new(
        sender: Option<AgentId>,
        topic_id: Option<TopicId>,
        is_rpc: bool,
        cancellation_token: CancellationToken,
        message_id: Option<String>,
    ) -> Self {
        Self {
            sender,
            topic_id,
            is_rpc,
            cancellation_token,
            message_id: message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        }
    }

    /// Create a new MessageContext for direct messaging
    pub fn direct_message(
        sender: Option<AgentId>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self::new(sender, None, false, cancellation_token, None)
    }

    /// Create a new MessageContext for RPC
    pub fn rpc_message(
        sender: Option<AgentId>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self::new(sender, None, true, cancellation_token, None)
    }

    /// Create a new MessageContext for topic-based messaging
    pub fn topic_message(
        sender: Option<AgentId>,
        topic_id: TopicId,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self::new(sender, Some(topic_id), false, cancellation_token, None)
    }

    /// Check if the operation has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

impl Default for MessageContext {
    fn default() -> Self {
        Self::new(
            None,
            None,
            false,
            CancellationToken::new(),
            None,
        )
    }
}

/// Trait for handling messages
///
/// This trait defines the interface for message handlers, allowing agents
/// to process different types of messages in a type-safe manner.
#[async_trait]
pub trait MessageHandler<T>: Send + Sync {
    /// Handle a message of type T
    ///
    /// # Arguments
    /// * `message` - The message to handle
    /// * `context` - Context information about the message
    ///
    /// # Returns
    /// Result containing the response (if any)
    async fn handle(&mut self, message: T, context: &MessageContext) -> crate::Result<Option<Box<dyn Any + Send>>>;
}

/// Legacy envelope for wrapping messages with metadata
///
/// This is kept for backward compatibility. New code should use
/// TypedMessageEnvelope or UntypedMessageEnvelope instead.
#[derive(Debug)]
#[deprecated(note = "Use TypedMessageEnvelope or UntypedMessageEnvelope instead")]
pub struct MessageEnvelope {
    /// The actual message payload
    pub message: Box<dyn Any + Send>,

    /// Message context
    pub context: MessageContext,

    /// Type name of the message (for routing)
    pub message_type: String,
}

impl MessageEnvelope {
    /// Create a new MessageEnvelope
    pub fn new<T: Any + Send>(
        message: T,
        context: MessageContext,
    ) -> Self {
        Self {
            message: Box::new(message),
            context,
            message_type: std::any::type_name::<T>().to_string(),
        }
    }

    /// Try to downcast the message to a specific type
    pub fn downcast<T: Any + Send>(self) -> Result<(T, MessageContext), Self> {
        match self.message.downcast::<T>() {
            Ok(message) => Ok((*message, self.context)),
            Err(message) => Err(Self {
                message,
                context: self.context,
                message_type: self.message_type,
            }),
        }
    }

    /// Get the message type name
    pub fn message_type(&self) -> &str {
        &self.message_type
    }
}

/// Response envelope for RPC messages
#[derive(Debug)]
pub struct ResponseEnvelope {
    /// The response payload
    pub response: Box<dyn Any + Send>,
    
    /// Original message ID this is responding to
    pub original_message_id: String,
    
    /// The agent that generated this response
    pub sender: AgentId,
}

impl ResponseEnvelope {
    /// Create a new ResponseEnvelope
    pub fn new<T: Any + Send>(
        response: T,
        original_message_id: String,
        sender: AgentId,
    ) -> Self {
        Self {
            response: Box::new(response),
            original_message_id,
            sender,
        }
    }

    /// Try to downcast the response to a specific type
    pub fn downcast<T: Any + Send>(self) -> Result<T, Box<dyn Any + Send>> {
        self.response.downcast::<T>().map(|r| *r)
    }
}

/// Function call representation
///
/// This represents a function call request, typically used with LLM tool calling.
/// Follows the Python autogen-core FunctionCall design.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
    /// Unique identifier for this function call
    pub id: String,
    
    /// JSON-encoded arguments for the function
    pub arguments: String,
    
    /// Name of the function to call
    pub name: String,
}

impl FunctionCall {
    /// Create a new FunctionCall
    pub fn new<S: Into<String>>(id: S, name: S, arguments: S) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }
}

impl fmt::Display for FunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_context_creation() {
        let token = CancellationToken::new();
        let ctx = MessageContext::direct_message(None, token);
        
        assert!(ctx.sender.is_none());
        assert!(ctx.topic_id.is_none());
        assert!(!ctx.is_rpc);
        assert!(!ctx.message_id.is_empty());
    }

    #[test]
    fn test_message_envelope() {
        let message = "test message".to_string();
        let ctx = MessageContext::default();
        let envelope = MessageEnvelope::new(message.clone(), ctx);
        
        assert_eq!(envelope.message_type(), "alloc::string::String");
        
        let (recovered_message, _) = envelope.downcast::<String>().unwrap();
        assert_eq!(recovered_message, message);
    }

    #[test]
    fn test_function_call() {
        let call = FunctionCall::new("call_1", "test_function", r#"{"arg": "value"}"#);
        assert_eq!(call.id, "call_1");
        assert_eq!(call.name, "test_function");
        assert_eq!(call.arguments, r#"{"arg": "value"}"#);
        
        let display = format!("{}", call);
        assert_eq!(display, r#"test_function({"arg": "value"})"#);
    }

    #[test]
    fn test_function_call_serialization() {
        let call = FunctionCall::new("call_1", "test_function", r#"{"arg": "value"}"#);
        let json = serde_json::to_string(&call).unwrap();
        let deserialized: FunctionCall = serde_json::from_str(&json).unwrap();
        assert_eq!(call, deserialized);
    }
}
