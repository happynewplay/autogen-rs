//! Base agent implementation
//!
//! This module provides a base agent implementation that handles common
//! functionality and provides a foundation for building custom agents.

use crate::{Agent, AgentId, AgentMetadata, MessageContext, Result, TypeSafeMessage};
use async_trait::async_trait;
use std::collections::HashMap;

/// Base agent implementation that provides common functionality
///
/// This agent provides a foundation for building custom agents with
/// default implementations for common operations like state management,
/// metadata handling, and lifecycle management.
pub struct BaseAgent {
    /// Unique identifier for this agent
    id: AgentId,
    
    /// Agent metadata
    metadata: AgentMetadata,
    
    /// Agent state storage
    state: HashMap<String, serde_json::Value>,
    
    /// Whether the agent is currently running
    is_running: bool,
}

impl BaseAgent {
    /// Create a new base agent
    ///
    /// # Arguments
    /// * `id` - The unique identifier for this agent
    /// * `metadata` - Optional metadata for the agent
    pub fn new(id: AgentId, metadata: Option<AgentMetadata>) -> Self {
        Self {
            id,
            metadata: metadata.unwrap_or_default(),
            state: HashMap::new(),
            is_running: false,
        }
    }

    /// Create a new base agent with a description
    ///
    /// # Arguments
    /// * `id` - The unique identifier for this agent
    /// * `description` - Description of the agent
    pub fn with_description<S: Into<String>>(id: AgentId, description: S) -> Self {
        let mut metadata = AgentMetadata::default();
        metadata.description = Some(description.into());
        Self::new(id, Some(metadata))
    }

    /// Get the agent's state
    pub fn state(&self) -> &HashMap<String, serde_json::Value> {
        &self.state
    }

    /// Get a mutable reference to the agent's state
    pub fn state_mut(&mut self) -> &mut HashMap<String, serde_json::Value> {
        &mut self.state
    }

    /// Set a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    /// * `value` - The state value
    pub fn set_state<K: Into<String>>(&mut self, key: K, value: serde_json::Value) {
        self.state.insert(key.into(), value);
    }

    /// Get a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    ///
    /// # Returns
    /// The state value if it exists
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Remove a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    ///
    /// # Returns
    /// The removed value if it existed
    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.state.remove(key)
    }

    /// Clear all state
    pub fn clear_state(&mut self) {
        self.state.clear();
    }

    /// Check if the agent is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Update the agent's metadata
    ///
    /// # Arguments
    /// * `metadata` - The new metadata
    pub fn set_metadata(&mut self, metadata: AgentMetadata) {
        self.metadata = metadata;
    }

    /// Add a tag to the agent's metadata
    ///
    /// # Arguments
    /// * `tag` - The tag to add
    pub fn add_tag<S: Into<String>>(&mut self, tag: S) {
        self.metadata.tags.push(tag.into());
    }

    /// Set a property in the agent's metadata
    ///
    /// # Arguments
    /// * `key` - The property key
    /// * `value` - The property value
    pub fn set_property<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.metadata.properties.insert(key.into(), value.into());
    }

    /// Handle message processing (override in subclasses)
    ///
    /// This method should be overridden by subclasses to provide
    /// custom message handling logic. The default implementation
    /// returns None (no response).
    ///
    /// # Arguments
    /// * `message` - The message to handle
    /// * `context` - The message context
    ///
    /// # Returns
    /// An optional response message
    pub async fn handle_message_impl(
        &mut self,
        _message: TypeSafeMessage,
        _context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        // Default implementation does nothing
        Ok(None)
    }

    /// Handle agent startup (override in subclasses)
    ///
    /// This method is called when the agent is starting up.
    /// Override this method to perform custom initialization.
    pub async fn on_start_impl(&mut self) -> Result<()> {
        self.is_running = true;
        Ok(())
    }

    /// Handle agent shutdown (override in subclasses)
    ///
    /// This method is called when the agent is shutting down.
    /// Override this method to perform custom cleanup.
    pub async fn on_stop_impl(&mut self) -> Result<()> {
        self.is_running = false;
        Ok(())
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        self.handle_message_impl(message, context).await
    }

    fn metadata(&self) -> AgentMetadata {
        self.metadata.clone()
    }

    async fn on_start(&mut self) -> Result<()> {
        self.on_start_impl().await
    }

    async fn on_stop(&mut self) -> Result<()> {
        self.on_stop_impl().await
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(self.state.clone())
    }

    async fn load_state(&mut self, state: HashMap<String, serde_json::Value>) -> Result<()> {
        self.state = state;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.on_stop_impl().await
    }
}

/// Builder for creating BaseAgent instances
///
/// This builder provides a fluent interface for configuring
/// and creating BaseAgent instances.
pub struct BaseAgentBuilder {
    id: Option<AgentId>,
    metadata: AgentMetadata,
    initial_state: HashMap<String, serde_json::Value>,
}

impl BaseAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            id: None,
            metadata: AgentMetadata::default(),
            initial_state: HashMap::new(),
        }
    }

    /// Set the agent ID
    ///
    /// # Arguments
    /// * `id` - The agent ID
    pub fn with_id(mut self, id: AgentId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the agent description
    ///
    /// # Arguments
    /// * `description` - The agent description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.metadata.description = Some(description.into());
        self
    }

    /// Add a tag
    ///
    /// # Arguments
    /// * `tag` - The tag to add
    pub fn with_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Set a property
    ///
    /// # Arguments
    /// * `key` - The property key
    /// * `value` - The property value
    pub fn with_property<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.properties.insert(key.into(), value.into());
        self
    }

    /// Set initial state
    ///
    /// # Arguments
    /// * `key` - The state key
    /// * `value` - The state value
    pub fn with_state<K: Into<String>>(mut self, key: K, value: serde_json::Value) -> Self {
        self.initial_state.insert(key.into(), value);
        self
    }

    /// Enable concurrent message handling
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent messages (None for unlimited)
    pub fn with_concurrency(mut self, max_concurrent: Option<usize>) -> Self {
        self.metadata.concurrent = true;
        self.metadata.max_concurrent = max_concurrent;
        self
    }

    /// Build the BaseAgent
    ///
    /// # Returns
    /// A configured BaseAgent instance
    ///
    /// # Errors
    /// Returns an error if no agent ID was provided
    pub fn build(self) -> Result<BaseAgent> {
        let id = self.id.ok_or_else(|| {
            crate::AutoGenError::other("Agent ID is required")
        })?;

        let mut agent = BaseAgent::new(id, Some(self.metadata));
        agent.state = self.initial_state;
        Ok(agent)
    }
}

impl Default for BaseAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentId;

    #[tokio::test]
    async fn test_base_agent_creation() {
        let id = AgentId::new("test", "agent").unwrap();
        let agent = BaseAgent::new(id.clone(), None);

        assert_eq!(agent.id(), &id);
        assert!(!agent.is_running());
        assert!(agent.state().is_empty());
    }

    #[tokio::test]
    async fn test_base_agent_state_management() {
        let id = AgentId::new("test", "agent").unwrap();
        let mut agent = BaseAgent::new(id, None);

        // Test setting and getting state
        agent.set_state("key1", serde_json::json!("value1"));
        agent.set_state("key2", serde_json::json!(42));

        assert_eq!(agent.get_state("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(agent.get_state("key2"), Some(&serde_json::json!(42)));
        assert_eq!(agent.get_state("nonexistent"), None);

        // Test removing state
        let removed = agent.remove_state("key1");
        assert_eq!(removed, Some(serde_json::json!("value1")));
        assert_eq!(agent.get_state("key1"), None);

        // Test clearing state
        agent.clear_state();
        assert!(agent.state().is_empty());
    }

    #[tokio::test]
    async fn test_base_agent_lifecycle() {
        let id = AgentId::new("test", "agent").unwrap();
        let mut agent = BaseAgent::new(id, None);

        assert!(!agent.is_running());

        // Test startup
        agent.on_start().await.unwrap();
        assert!(agent.is_running());

        // Test shutdown
        agent.on_stop().await.unwrap();
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_base_agent_builder() {
        let id = AgentId::new("test", "builder").unwrap();
        
        let agent = BaseAgentBuilder::new()
            .with_id(id.clone())
            .with_description("Test agent")
            .with_tag("test")
            .with_property("env", "test")
            .with_state("counter", serde_json::json!(0))
            .with_concurrency(Some(5))
            .build()
            .unwrap();

        assert_eq!(agent.id(), &id);
        assert_eq!(agent.metadata().description, Some("Test agent".to_string()));
        assert!(agent.metadata().tags.contains(&"test".to_string()));
        assert_eq!(agent.metadata().properties.get("env"), Some(&"test".to_string()));
        assert_eq!(agent.get_state("counter"), Some(&serde_json::json!(0)));
        assert!(agent.metadata().concurrent);
        assert_eq!(agent.metadata().max_concurrent, Some(5));
    }
}
