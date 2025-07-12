use crate::agent_id::AgentId;
use crate::topic::TopicId;
use std::error::Error;

/// Subscriptions define the topics that an agent is interested in.
pub trait Subscription: Send + Sync {
    /// Get the ID of the subscription.
    ///
    /// Implementations should return a unique ID for the subscription.
    fn id(&self) -> &str;

    /// Check if a given topic_id matches the subscription.
    fn is_match(&self, topic_id: &TopicId) -> bool;

    /// Map a topic_id to an agent. Should only be called if `is_match` returns true.
    fn map_to_agent(&self, topic_id: &TopicId) -> Result<AgentId, Box<dyn Error>>;
}

impl PartialEq for dyn Subscription {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for dyn Subscription {}