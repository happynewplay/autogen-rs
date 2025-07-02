//! Closure-based agent implementation
//!
//! This module provides an agent implementation that uses closures
//! for message handling, similar to Python's lambda-based agents.

use crate::{Agent, AgentId, AgentMetadata, MessageContext, Result, TypeSafeMessage};
use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Type alias for async message handler closures
pub type AsyncMessageHandler = Box<
    dyn Fn(TypeSafeMessage, MessageContext) -> Pin<Box<dyn Future<Output = Result<Option<TypeSafeMessage>>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for async lifecycle handler closures
pub type AsyncLifecycleHandler = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

/// Context provided to closure agents
///
/// This context provides access to agent state and metadata
/// that can be used within closure handlers.
#[derive(Debug, Clone)]
pub struct ClosureContext {
    /// Agent state
    pub state: HashMap<String, serde_json::Value>,
    /// Agent metadata
    pub metadata: AgentMetadata,
}

impl ClosureContext {
    /// Create a new closure context
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            metadata: AgentMetadata::default(),
        }
    }

    /// Get a state value
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Set a state value
    pub fn set_state<K: Into<String>>(&mut self, key: K, value: serde_json::Value) {
        self.state.insert(key.into(), value);
    }
}

impl Default for ClosureContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent implementation using closures for message handling
///
/// This agent allows defining message handling logic using closures,
/// providing a flexible and lightweight way to create agents without
/// implementing the full Agent trait.
pub struct ClosureAgent {
    /// Agent identifier
    id: AgentId,
    
    /// Agent context (state and metadata)
    context: ClosureContext,
    
    /// Message handler closure
    message_handler: Option<AsyncMessageHandler>,
    
    /// Startup handler closure
    start_handler: Option<AsyncLifecycleHandler>,
    
    /// Shutdown handler closure
    stop_handler: Option<AsyncLifecycleHandler>,
}

impl ClosureAgent {
    /// Create a new closure agent
    ///
    /// # Arguments
    /// * `id` - The agent identifier
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            context: ClosureContext::new(),
            message_handler: None,
            start_handler: None,
            stop_handler: None,
        }
    }

    /// Set the message handler
    ///
    /// # Arguments
    /// * `handler` - The message handler closure
    pub fn with_message_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(TypeSafeMessage, MessageContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<TypeSafeMessage>>> + Send + 'static,
    {
        self.message_handler = Some(Box::new(move |msg, ctx| Box::pin(handler(msg, ctx))));
        self
    }

    /// Set the startup handler
    ///
    /// # Arguments
    /// * `handler` - The startup handler closure
    pub fn with_start_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.start_handler = Some(Box::new(move || Box::pin(handler())));
        self
    }

    /// Set the shutdown handler
    ///
    /// # Arguments
    /// * `handler` - The shutdown handler closure
    pub fn with_stop_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.stop_handler = Some(Box::new(move || Box::pin(handler())));
        self
    }

    /// Set initial state
    ///
    /// # Arguments
    /// * `state` - The initial state
    pub fn with_state(mut self, state: HashMap<String, serde_json::Value>) -> Self {
        self.context.state = state;
        self
    }

    /// Set metadata
    ///
    /// # Arguments
    /// * `metadata` - The agent metadata
    pub fn with_metadata(mut self, metadata: AgentMetadata) -> Self {
        self.context.metadata = metadata;
        self
    }

    /// Get the agent context
    pub fn context(&self) -> &ClosureContext {
        &self.context
    }

    /// Get a mutable reference to the agent context
    pub fn context_mut(&mut self) -> &mut ClosureContext {
        &mut self.context
    }
}

#[async_trait]
impl Agent for ClosureAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        if let Some(handler) = &self.message_handler {
            handler(message, context.clone()).await
        } else {
            // Default behavior: return None (no response)
            Ok(None)
        }
    }

    fn metadata(&self) -> AgentMetadata {
        self.context.metadata.clone()
    }

    async fn on_start(&mut self) -> Result<()> {
        if let Some(handler) = &self.start_handler {
            handler().await
        } else {
            Ok(())
        }
    }

    async fn on_stop(&mut self) -> Result<()> {
        if let Some(handler) = &self.stop_handler {
            handler().await
        } else {
            Ok(())
        }
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(self.context.state.clone())
    }

    async fn load_state(&mut self, state: HashMap<String, serde_json::Value>) -> Result<()> {
        self.context.state = state;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.on_stop().await
    }
}

/// Builder for creating ClosureAgent instances with a fluent interface
pub struct ClosureAgentBuilder {
    id: Option<AgentId>,
    context: ClosureContext,
    message_handler: Option<AsyncMessageHandler>,
    start_handler: Option<AsyncLifecycleHandler>,
    stop_handler: Option<AsyncLifecycleHandler>,
}

impl ClosureAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            id: None,
            context: ClosureContext::new(),
            message_handler: None,
            start_handler: None,
            stop_handler: None,
        }
    }

    /// Set the agent ID
    pub fn with_id(mut self, id: AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the message handler
    pub fn with_message_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(TypeSafeMessage, MessageContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<TypeSafeMessage>>> + Send + 'static,
    {
        self.message_handler = Some(Box::new(move |msg, ctx| Box::pin(handler(msg, ctx))));
        self
    }

    /// Set the startup handler
    pub fn with_start_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.start_handler = Some(Box::new(move || Box::pin(handler())));
        self
    }

    /// Set the shutdown handler
    pub fn with_stop_handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.stop_handler = Some(Box::new(move || Box::pin(handler())));
        self
    }

    /// Set initial state
    pub fn with_state(mut self, state: HashMap<String, serde_json::Value>) -> Self {
        self.context.state = state;
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: AgentMetadata) -> Self {
        self.context.metadata = metadata;
        self
    }

    /// Build the ClosureAgent
    pub fn build(self) -> Result<ClosureAgent> {
        let id = self.id.ok_or_else(|| {
            crate::AutoGenError::other("Agent ID is required")
        })?;

        Ok(ClosureAgent {
            id,
            context: self.context,
            message_handler: self.message_handler,
            start_handler: self.start_handler,
            stop_handler: self.stop_handler,
        })
    }
}

impl Default for ClosureAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a simple closure agent
///
/// # Arguments
/// * `id` - The agent identifier
/// * `handler` - The message handler closure
///
/// # Returns
/// A configured ClosureAgent
pub fn closure_agent<F, Fut>(id: AgentId, handler: F) -> ClosureAgent
where
    F: Fn(TypeSafeMessage, MessageContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Option<TypeSafeMessage>>> + Send + 'static,
{
    ClosureAgent::new(id).with_message_handler(handler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentId, MessageContext, TypeSafeMessage, TextMessage};

    #[tokio::test]
    async fn test_closure_agent_creation() {
        let id = AgentId::new("test", "closure").unwrap();
        let agent = ClosureAgent::new(id.clone());

        assert_eq!(agent.id(), &id);
        assert!(agent.context().state.is_empty());
    }

    #[tokio::test]
    async fn test_closure_agent_with_handler() {
        let id = AgentId::new("test", "handler").unwrap();
        let mut agent = ClosureAgent::new(id.clone())
            .with_message_handler(|msg, _ctx| async move {
                match msg {
                    TypeSafeMessage::Text(text) => {
                        Ok(Some(TypeSafeMessage::text(format!("Echo: {}", text.content))))
                    }
                    _ => Ok(None),
                }
            });

        let message = TypeSafeMessage::Text(TextMessage {
            content: "Hello".to_string(),
        });
        let context = MessageContext::default();

        let response = agent.handle_message(message, &context).await.unwrap();
        assert!(response.is_some());

        if let Some(TypeSafeMessage::Text(text)) = response {
            assert_eq!(text.content, "Echo: Hello");
        } else {
            panic!("Expected text response");
        }
    }

    #[tokio::test]
    async fn test_closure_agent_builder() {
        let id = AgentId::new("test", "builder").unwrap();
        
        let mut state = HashMap::new();
        state.insert("counter".to_string(), serde_json::json!(0));

        let agent = ClosureAgentBuilder::new()
            .with_id(id.clone())
            .with_state(state)
            .with_message_handler(|_msg, _ctx| async move { Ok(None) })
            .build()
            .unwrap();

        assert_eq!(agent.id(), &id);
        assert_eq!(agent.context().get_state("counter"), Some(&serde_json::json!(0)));
    }

    #[tokio::test]
    async fn test_convenience_function() {
        let id = AgentId::new("test", "convenience").unwrap();
        
        let mut agent = closure_agent(id.clone(), |msg, _ctx| async move {
            match msg {
                TypeSafeMessage::Text(_) => Ok(Some(TypeSafeMessage::text("Handled"))),
                _ => Ok(None),
            }
        });

        let message = TypeSafeMessage::Text(TextMessage {
            content: "Test".to_string(),
        });
        let context = MessageContext::default();

        let response = agent.handle_message(message, &context).await.unwrap();
        assert!(response.is_some());
    }
}
