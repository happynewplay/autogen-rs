use crate::agent::{Agent, AgentFactory};
use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::agent_type::AgentType;
use crate::cancellation_token::CancellationToken;
use crate::subscription::Subscription;
use crate::topic::TopicId;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn register_factory(
        &self,
        agent_type: AgentType,
        factory: Box<dyn AgentFactory>,
    ) -> Result<(), Box<dyn Error>>;

    async fn register_agent_instance(
        &self,
        agent: Arc<Mutex<dyn Agent>>,
        agent_id: AgentId,
    ) -> Result<AgentId, Box<dyn Error>>;

    async fn send_message(
        &self,
        message: Value,
        recipient: AgentId,
        sender: Option<AgentId>,
        cancellation_token: Option<CancellationToken>,
        message_id: Option<String>,
    ) -> Result<Value, Box<dyn Error + Send>>;

    async fn publish_message(
        &self,
        message: Value,
        topic_id: TopicId,
        sender: Option<AgentId>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), Box<dyn Error>>;

    async fn add_subscription(
        &self,
        subscription: Box<dyn Subscription>,
    ) -> Result<(), Box<dyn Error>>;

    async fn agent_metadata(&self, agent: &AgentId) -> Result<AgentMetadata, Box<dyn Error>>;

    async fn agent_save_state(
        &self,
        agent: &AgentId,
    ) -> Result<HashMap<String, Value>, Box<dyn Error>>;

    async fn agent_load_state(
        &self,
        agent: &AgentId,
        state: &HashMap<String, Value>,
    ) -> Result<(), Box<dyn Error>>;

    /// Remove a subscription from the runtime
    async fn remove_subscription(&self, id: &str) -> Result<(), Box<dyn Error>>;

    /// Save the state of the entire runtime
    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>>;

    /// Load the state of the entire runtime
    async fn load_state(&self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>>;

    /// Try to get the underlying agent instance
    async fn try_get_underlying_agent_instance(
        &self,
        id: &AgentId,
    ) -> Result<Box<dyn Agent>, Box<dyn Error>>;

    /// Get an agent by ID or type with optional lazy loading
    async fn get_agent(
        &self,
        id_or_type: &str,
        key: Option<&str>,
        lazy: bool,
    ) -> Result<AgentId, Box<dyn Error>>;

    /// Add a message serializer to the runtime
    fn add_message_serializer(&self, serializer: Box<dyn crate::serialization::MessageSerializer<Value>>);

    /// Get the number of unprocessed messages
    fn unprocessed_messages_count(&self) -> usize;
}