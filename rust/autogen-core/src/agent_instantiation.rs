use crate::agent_id::AgentId;
use crate::agent_runtime::AgentRuntime;
use std::sync::Arc;

tokio::task_local! {
    static AGENT_INSTANTIATION_CONTEXT: (Arc<dyn AgentRuntime>, AgentId);
}

/// A struct that provides context for agent instantiation.
///
/// This struct can be used to access the current runtime and agent ID
/// during agent instantiation -- inside the factory function or the agent's
/// class constructor.
pub struct AgentInstantiationContext;

impl AgentInstantiationContext {
    /// Populates the instantiation context for the duration of the given closure's execution.
    ///
    /// # Arguments
    ///
    /// * `runtime` - The agent runtime to set in the context.
    /// * `agent_id` - The agent ID to set in the context.
    /// * `f` - The closure to execute within the context.
    ///
    /// # Returns
    ///
    /// The result of the closure execution.
    pub async fn with_context<F, R>(
        runtime: Arc<dyn AgentRuntime>,
        agent_id: AgentId,
        f: F,
    ) -> R
    where
        F: std::future::Future<Output = R>,
    {
        AGENT_INSTANTIATION_CONTEXT.scope((runtime, agent_id), f).await
    }

    /// Returns the current agent runtime from the instantiation context.
    ///
    /// # Panics
    ///
    /// Panics if called outside of an instantiation context.
    pub fn current_runtime() -> Arc<dyn AgentRuntime> {
        AGENT_INSTANTIATION_CONTEXT.with(|ctx| ctx.0.clone())
    }

    /// Returns the current agent ID from the instantiation context.
    ///
    /// # Panics
    ///
    /// Panics if called outside of an instantiation context.
    pub fn current_agent_id() -> AgentId {
        AGENT_INSTANTIATION_CONTEXT.with(|ctx| ctx.1.clone())
    }

    /// Checks if the code is currently running within an agent factory call.
    ///
    /// # Returns
    ///
    /// `true` if in an instantiation context, `false` otherwise.
    pub fn is_in_factory_call() -> bool {
        AGENT_INSTANTIATION_CONTEXT.try_with(|_| {}).is_ok()
    }
}