use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::message_context::MessageContext;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

// Forward declaration for type checking only
// In Rust, we'll use a forward declaration of the AgentRuntime trait
// and handle the circular dependency with generics or dynamic dispatch.
use crate::agent_runtime::AgentRuntime;

use std::future::Future;
use std::pin::Pin;
use dyn_clone::DynClone;

#[async_trait]
pub trait Agent: Send + Sync + DynClone {
    fn clone_box(&self) -> Box<dyn Agent>;
    fn metadata(&self) -> AgentMetadata;
    fn id(&self) -> AgentId;

    async fn bind_id_and_runtime(
        &mut self,
        id: AgentId,
        runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>>;

    async fn on_message(
        &mut self,
        message: Value,
        ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>>;

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>>;

    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>>;

    async fn close(&mut self) -> Result<(), Box<dyn Error>>;
}

pub trait AgentFactory: Send + Sync + DynClone {
    fn create(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Agent>, Box<dyn Error>>> + Send>>;

    fn clone_box(&self) -> Box<dyn AgentFactory>;
}

dyn_clone::clone_trait_object!(Agent);
dyn_clone::clone_trait_object!(AgentFactory);