use crate::agent_id::AgentId;

tokio::task_local! {
    static MESSAGE_HANDLER_CONTEXT: AgentId;
}

/// A struct that provides context for message handling.
///
/// This static struct can be used to access the current agent ID
/// during message handling.
pub struct MessageHandlerContext;

impl MessageHandlerContext {
    /// Populates the message handler context for the duration of the given closure's execution.
    pub async fn with_context<F, R>(agent_id: AgentId, f: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        MESSAGE_HANDLER_CONTEXT.scope(agent_id, f).await
    }

    /// Returns the current agent ID from the message handler context.
    ///
    /// Returns `None` if not called within a message handler context.
    pub fn agent_id() -> Option<AgentId> {
        MESSAGE_HANDLER_CONTEXT.try_with(|ctx| ctx.clone()).ok()
    }

    /// Returns the current agent ID from the message handler context.
    ///
    /// Returns an error if not called within a message handler context.
    pub fn current_agent_id() -> Result<AgentId, &'static str> {
        MESSAGE_HANDLER_CONTEXT.try_with(|ctx| ctx.clone())
            .map_err(|_| "Not in message handler context")
    }
}