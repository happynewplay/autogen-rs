use crate::agent::Agent;
use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::agent_runtime::AgentRuntime;
use crate::base_agent::BaseAgent;
use crate::message_context::MessageContext;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A context object passed to the closure of a `ClosureAgent`.
/// It provides access to the agent's ID and runtime functionalities.
#[derive(Clone)]
pub struct ClosureContext {
    base_agent: Arc<BaseAgent>,
}

impl ClosureContext {
    /// Returns the ID of the agent.
    pub fn id(&self) -> AgentId {
        self.base_agent.id()
    }

    /// Sends a message to another agent.
    pub async fn send_message(
        &self,
        message: Value,
        recipient: AgentId,
    ) -> Result<Value, Box<dyn Error + Send>> {
        self.base_agent
            .send_message(message, recipient, None, None)
            .await
    }

    /// Publishes a message to a topic.
    pub async fn publish_message(
        &self,
        message: Value,
        topic_id: crate::topic::TopicId,
    ) -> Result<(), Box<dyn Error>> {
        self.base_agent.publish_message(message, topic_id, None).await
    }
}

type Closure<T> = Arc<
    dyn Fn(
            ClosureContext,
            T,
            MessageContext,
        ) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn Error + Send>>> + Send>>
        + Send
        + Sync,
>;

/// An agent that is defined by a closure.
pub struct ClosureAgent<T> {
    base_agent: Arc<BaseAgent>,
    closure: Closure<T>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for ClosureAgent<T> {
    fn clone(&self) -> Self {
        Self {
            base_agent: self.base_agent.clone(),
            closure: self.closure.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: 'static + Send + Sync + serde::de::DeserializeOwned> ClosureAgent<T> {
    /// Creates a new `ClosureAgent`.
    pub fn new<F, Fut>(description: String, closure: F) -> Self
    where
        F: Fn(ClosureContext, T, MessageContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, Box<dyn Error + Send>>> + Send + 'static,
    {
        let base_agent = Arc::new(BaseAgent::new(description));
        let closure_context = ClosureContext {
            base_agent: base_agent.clone(),
        };

        let wrapped_closure: Closure<T> = Arc::new(
            move |ctx: ClosureContext, msg: T, msg_ctx: MessageContext| {
                Box::pin(closure(ctx, msg, msg_ctx))
            },
        );

        Self {
            base_agent,
            closure: wrapped_closure,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T: 'static + Send + Sync + serde::de::DeserializeOwned> Agent for ClosureAgent<T> {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn metadata(&self) -> AgentMetadata {
        self.base_agent.metadata()
    }

    fn id(&self) -> AgentId {
        self.base_agent.id()
    }

    async fn bind_id_and_runtime(
        &mut self,
        _id: AgentId,
        _runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn on_message(
        &mut self,
        message: Value,
        ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        let typed_message: T =
            serde_json::from_value(message).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        let closure_context = ClosureContext {
            base_agent: self.base_agent.clone(),
        };
        (self.closure)(closure_context, typed_message, ctx).await
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        Ok(HashMap::new())
    }

    async fn load_state(&mut self, _state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}