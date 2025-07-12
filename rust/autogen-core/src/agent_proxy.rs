use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::agent_runtime::AgentRuntime;
use crate::cancellation_token::CancellationToken;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// A helper struct that allows you to use an `AgentId` in place of its associated `Agent`.
pub struct AgentProxy {
    id: AgentId,
    runtime: Arc<dyn AgentRuntime>,
}

impl AgentProxy {
    /// Creates a new `AgentProxy`.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The ID of the agent to proxy.
    /// * `runtime` - The agent runtime.
    pub fn new(agent_id: AgentId, runtime: Arc<dyn AgentRuntime>) -> Self {
        Self {
            id: agent_id,
            runtime,
        }
    }

    /// Returns the ID of the target agent for this proxy.
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Returns the metadata of the agent.
    pub async fn metadata(&self) -> Result<AgentMetadata, Box<dyn Error>> {
        self.runtime.agent_metadata(&self.id).await
    }

    /// Sends a message to the agent.
    pub async fn send_message(
        &self,
        message: Value,
        sender: AgentId,
        cancellation_token: Option<CancellationToken>,
        message_id: Option<String>,
    ) -> Result<Value, Box<dyn Error + Send>> {
        self.runtime
            .send_message(
                message,
                self.id.clone(),
                Some(sender),
                cancellation_token,
                message_id,
            )
            .await
    }

    /// Saves the state of the agent. The result must be JSON serializable.
    pub async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        self.runtime.agent_save_state(&self.id).await
    }

    /// Loads the state of the agent obtained from `save_state`.
    ///
    /// # Arguments
    ///
    /// * `state` - State of the agent. Must be JSON serializable.
    pub async fn load_state(&self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        self.runtime.agent_load_state(&self.id, state).await
    }
}