use crate::agent::Agent;
use crate::base_agent::BaseAgent;
use crate::message_context::MessageContext;
use async_trait::async_trait;
use serde_json::Value;
use std::any::TypeId;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type MessageHandler = Arc<
    dyn Fn(
            Arc<BaseAgent>,
            Value,
            MessageContext,
        ) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn Error + Send>>> + Send>>
        + Send
        + Sync,
>;

/// A base class for agents that route messages to handlers based on the type of the message.
pub struct RoutedAgent {
    base_agent: Arc<BaseAgent>,
    handlers: HashMap<TypeId, Vec<MessageHandler>>,
}

impl Clone for RoutedAgent {
    fn clone(&self) -> Self {
        Self {
            base_agent: self.base_agent.clone(),
            handlers: self.handlers.clone(),
        }
    }
}

impl RoutedAgent {
    /// Creates a new `RoutedAgent`.
    pub fn new(description: String) -> Self {
        Self {
            base_agent: Arc::new(BaseAgent::new(description)),
            handlers: HashMap::new(),
        }
    }

    /// Registers a message handler for a specific message type.
    pub fn register_handler<T: 'static, F, Fut>(&mut self, handler: F)
    where
        F: Fn(Arc<BaseAgent>, T, MessageContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, Box<dyn Error + Send>>> + Send + 'static,
        T: serde::de::DeserializeOwned,
    {
        let type_id = TypeId::of::<T>();
        let wrapped_handler: MessageHandler = Arc::new(move |agent, msg_val, ctx| {
            let msg: T = serde_json::from_value(msg_val).unwrap(); // Handle error properly
            Box::pin(handler(agent, msg, ctx))
        });

        self.handlers
            .entry(type_id)
            .or_default()
            .push(wrapped_handler);
    }

    /// Called when a message is received that does not have a matching message handler.
    pub async fn on_unhandled_message(
        &self,
        message: Value,
        _ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        println!("Unhandled message: {:?}", message);
        Ok(Value::Null)
    }
}

#[async_trait]
impl Agent for RoutedAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn metadata(&self) -> crate::agent_metadata::AgentMetadata {
        self.base_agent.metadata()
    }

    fn id(&self) -> crate::agent_id::AgentId {
        self.base_agent.id()
    }

    async fn bind_id_and_runtime(
        &mut self,
        _id: crate::agent_id::AgentId,
        _runtime: &dyn crate::agent_runtime::AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn on_message(
        &mut self,
        message: Value,
        ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        self.on_unhandled_message(message, ctx).await
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let state = self.base_agent.save_state()?;
        // Convert single Value to HashMap format expected by Agent trait
        let mut state_map = HashMap::new();
        state_map.insert("base_state".to_string(), state);
        Ok(state_map)
    }

    async fn load_state(&mut self, _state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        // Note: Cannot modify Arc<BaseAgent> directly
        // In a full implementation, you would need to redesign this to use Arc<Mutex<BaseAgent>>
        // or implement a different state management approach
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}