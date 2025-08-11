use crate::agent::Agent;
use crate::agent_runtime::AgentRuntime;
use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::message::Message;
use crate::message_context::MessageContext;
use crate::models::client::ModelClient;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConversableAgent {
    id: AgentId,
    metadata: AgentMetadata,
    model_client: Arc<dyn ModelClient>,
    messages: Vec<Message>,
}

use crate::agent_type::AgentType;

impl ConversableAgent {
    /// Creates a new `ConversableAgent`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the agent, used as the `key` for its ID and metadata.
    /// * `model_client` - The client for communicating with the language model.
    pub fn new(name: &str, model_client: Arc<dyn ModelClient>) -> Self {
        let agent_type = AgentType {
            r#type: "autogen.agent.conversable".to_string(),
        };
        let agent_id = AgentId::new(&agent_type, name).expect("Failed to create AgentId");

        Self {
            id: agent_id.clone(),
            metadata: AgentMetadata {
                r#type: agent_type.r#type,
                key: name.to_string(),
                description: "A conversable agent that can chat with other agents and humans."
                    .to_string(),
            },
            model_client,
            messages: Vec::new(),
        }
    }
}

#[async_trait]
impl Agent for ConversableAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }

    fn metadata(&self) -> AgentMetadata {
        self.metadata.clone()
    }

    fn id(&self) -> AgentId {
        self.id.clone()
    }

    async fn bind_id_and_runtime(
        &mut self,
        id: AgentId,
        _runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        self.id = id;
        Ok(())
    }

    async fn on_message(
        &mut self,
        _message: Value,
        _ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        // The main logic for generating replies will be implemented here later.
        // It will involve:
        // 1. Deserializing the incoming message.
        // 2. Adding it to the conversation history (`self.messages`).
        // 3. Calling the LLM via `self.model_client`.
        // 4. Serializing the response and returning it.
        unimplemented!("ConversableAgent::on_message is not yet implemented")
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let messages = serde_json::to_value(&self.messages)?;
        let mut state = HashMap::new();
        state.insert("messages".to_string(), messages);
        Ok(state)
    }

    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        if let Some(messages_value) = state.get("messages") {
            self.messages = serde_json::from_value(messages_value.clone())?;
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        // No-op for now, can be used for cleaning up resources.
        Ok(())
    }
}
