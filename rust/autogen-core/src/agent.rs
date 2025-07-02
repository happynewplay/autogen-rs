//! Core agent traits and types
//!
//! This module defines the unified `Agent` trait that provides type safety
//! while maintaining flexibility for the AutoGen agent system.

use crate::{AgentId, MessageContext, Result, TypeSafeMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified Agent trait that provides type safety with flexibility
///
/// This trait replaces the previous dual TypedAgent/Agent design with a single
/// unified interface that uses TypeSafeMessage for compile-time type safety
/// while maintaining runtime flexibility.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the unique identifier for this agent
    fn id(&self) -> &AgentId;

    /// Handle an incoming message
    ///
    /// This is the primary method agents use to process incoming messages.
    /// Uses TypeSafeMessage enum for type safety without Box<dyn Any>.
    ///
    /// # Arguments
    /// * `message` - The incoming message (type-safe enum)
    /// * `context` - Context information about the message
    ///
    /// # Returns
    /// An optional response message
    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>>;

    /// Get metadata about this agent
    ///
    /// Returns information about the agent's capabilities, subscriptions,
    /// and other metadata that the runtime can use for optimization.
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata::default()
    }

    /// Called when the agent is started
    ///
    /// Override this method to perform initialization when the agent
    /// is registered with the runtime.
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called when the agent is stopped
    ///
    /// Override this method to perform cleanup when the agent
    /// is being shut down.
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// Save the current state of the agent
    ///
    /// This method should return a serializable representation of the agent's state
    /// that can be used to restore the agent later.
    /// This corresponds to the Python `save_state` method.
    ///
    /// # Returns
    /// A HashMap containing the agent's state (must be JSON serializable)
    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(HashMap::new())
    }

    /// Load state into the agent
    ///
    /// This method restores the agent's state from a previously saved state.
    /// This corresponds to the Python `load_state` method.
    ///
    /// # Arguments
    /// * `state` - The state to load (from save_state)
    async fn load_state(&mut self, _state: HashMap<String, serde_json::Value>) -> Result<()> {
        Ok(())
    }

    /// Called when the agent is being shut down
    ///
    /// This method allows the agent to perform cleanup operations before
    /// the runtime shuts down. This corresponds to the Python `close` method.
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Metadata about an agent's capabilities and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Human-readable description of the agent
    pub description: Option<String>,
    /// Tags for categorizing the agent
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Whether this agent can handle concurrent messages
    pub concurrent: bool,
    /// Maximum number of concurrent messages (if concurrent is true)
    pub max_concurrent: Option<usize>,
}

impl Default for AgentMetadata {
    fn default() -> Self {
        Self {
            description: None,
            tags: Vec::new(),
            properties: HashMap::new(),
            concurrent: false,
            max_concurrent: None,
        }
    }
}

// TypedAgent and TypedAgentAdapter removed in favor of unified Agent trait

/// Type identifier for agents
///
/// Used to categorize agents and enable type-based routing and filtering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentType(pub String);

impl AgentType {
    /// Create a new agent type
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self(name.into())
    }

    /// Get the type name
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentType {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AgentType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Proxy for communicating with remote agents
///
/// An `AgentProxy` represents a remote agent and provides methods
/// for sending messages to it through the runtime.
#[derive(Debug, Clone)]
pub struct AgentProxy {
    /// The ID of the target agent
    pub agent_id: AgentId,
    /// Runtime handle for message sending
    runtime_handle: RuntimeHandle,
}

impl AgentProxy {
    /// Create a new agent proxy
    pub(crate) fn new(agent_id: AgentId, runtime_handle: RuntimeHandle) -> Self {
        Self {
            agent_id,
            runtime_handle,
        }
    }

    /// Send a message to the target agent
    pub async fn send_message(
        &self,
        message: crate::TypeSafeMessage,
    ) -> Result<()> {
        self.runtime_handle
            .send_message(self.agent_id.clone(), message)
            .await
    }

    /// Send a message and wait for a response
    pub async fn send_request(
        &self,
        message: crate::TypeSafeMessage,
    ) -> Result<crate::TypeSafeMessage> {
        self.runtime_handle
            .send_request(self.agent_id.clone(), message)
            .await
    }
}

/// Handle for communicating with the agent runtime
///
/// This handle allows agents to send messages, make requests, publish to topics,
/// and interact with the runtime system. This follows the Python autogen-core
/// runtime handle design.
#[derive(Debug, Clone)]
pub struct RuntimeHandle {
    // This would contain the actual runtime communication mechanism
    // For now, we'll use a placeholder
    _placeholder: (),
}

impl RuntimeHandle {
    pub(crate) fn new() -> Self {
        Self { _placeholder: () }
    }

    /// Send a message to another agent
    ///
    /// # Arguments
    /// * `target` - The ID of the target agent
    /// * `message` - The message to send
    pub async fn send_message(
        &self,
        _target: AgentId,
        _message: TypeSafeMessage,
    ) -> Result<()> {
        // TODO: Implement actual message sending
        Ok(())
    }

    /// Send a request and wait for a response
    ///
    /// # Arguments
    /// * `target` - The ID of the target agent
    /// * `message` - The request message
    ///
    /// # Returns
    /// The response from the target agent
    pub async fn send_request(
        &self,
        _target: AgentId,
        _message: TypeSafeMessage,
    ) -> Result<TypeSafeMessage> {
        // For now, return an error indicating this feature is not yet implemented
        // In a full implementation, this would:
        // 1. Send the message to the target agent via the runtime
        // 2. Wait for a response with a timeout
        // 3. Return the typed response
        Err(crate::AutoGenError::other(
            "Request/response messaging is not yet implemented. Use publish_message for one-way communication."
        ))
    }

    /// Publish a message to a topic
    ///
    /// # Arguments
    /// * `topic_id` - The topic to publish to
    /// * `message` - The message to publish
    pub async fn publish_message(
        &self,
        _topic_id: crate::TopicId,
        _message: TypeSafeMessage,
    ) -> Result<()> {
        // TODO: Implement actual topic publishing
        Ok(())
    }

    /// Stop the runtime
    pub async fn stop(&self) -> Result<()> {
        // TODO: Implement runtime shutdown
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_creation() {
        let agent_type = AgentType::new("test_agent");
        assert_eq!(agent_type.name(), "test_agent");
    }

    #[test]
    fn test_agent_type_from_str() {
        let agent_type = AgentType::from("test_agent");
        assert_eq!(agent_type.name(), "test_agent");
    }

    #[test]
    fn test_agent_metadata_default() {
        let metadata = AgentMetadata::default();
        assert!(metadata.description.is_none());
        assert!(metadata.tags.is_empty());
        assert!(metadata.properties.is_empty());
        assert!(!metadata.concurrent);
        assert!(metadata.max_concurrent.is_none());
    }
}
