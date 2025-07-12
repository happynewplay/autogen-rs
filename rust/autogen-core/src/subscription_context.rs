use crate::agent_type::AgentType;

tokio::task_local! {
    static SUBSCRIPTION_CONTEXT: AgentType;
}

/// A struct that provides context for subscription instantiation.
///
/// This static struct can be used to access the current agent type
/// during subscription instantiation.
pub struct SubscriptionInstantiationContext;

impl SubscriptionInstantiationContext {
    /// Populates the subscription context for the duration of the given closure's execution.
    pub async fn with_context<F, R>(agent_type: AgentType, f: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        SUBSCRIPTION_CONTEXT.scope(agent_type, f).await
    }

    /// Returns the current agent type from the subscription context.
    ///
    /// Returns `None` if not called within a subscription instantiation context.
    pub fn agent_type() -> Option<AgentType> {
        SUBSCRIPTION_CONTEXT.try_with(|ctx| ctx.clone()).ok()
    }
}