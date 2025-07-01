//! Subscription system for message routing
//!
//! This module provides the subscription system used for message routing and filtering
//! in the autogen system, following the Python autogen-core design.

use crate::{AgentId, TopicId};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashSet;

/// Trait for message subscriptions
///
/// Subscriptions define which messages an agent is interested in receiving.
/// This follows the Python autogen-core Subscription protocol.
pub trait Subscription: Send + Sync {
    /// Check if this subscription matches a given topic and message type
    ///
    /// # Arguments
    /// * `topic_id` - The topic ID of the message
    /// * `message_type_id` - The type ID of the message
    ///
    /// # Returns
    /// True if the subscription matches, false otherwise
    fn matches(&self, topic_id: &TopicId, message_type_id: TypeId) -> bool;

    /// Get a description of this subscription for debugging
    fn description(&self) -> String;
}

/// Default subscription that matches all messages
///
/// This subscription matches any message regardless of topic or type.
/// Equivalent to the Python DefaultSubscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSubscription;

impl DefaultSubscription {
    /// Create a new default subscription
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultSubscription {
    fn default() -> Self {
        Self::new()
    }
}

impl Subscription for DefaultSubscription {
    fn matches(&self, _topic_id: &TopicId, _message_type_id: TypeId) -> bool {
        true
    }

    fn description(&self) -> String {
        "DefaultSubscription (matches all messages)".to_string()
    }
}

/// Type-based subscription
///
/// This subscription matches messages of a specific type, regardless of topic.
/// Equivalent to the Python TypeSubscription.
#[derive(Debug, Clone)]
pub struct TypeSubscription {
    /// The type ID to match
    type_id: TypeId,
    /// Human-readable type name for debugging
    type_name: String,
}

impl TypeSubscription {
    /// Create a new type subscription
    ///
    /// # Arguments
    /// * `type_id` - The type ID to subscribe to
    /// * `type_name` - Human-readable name of the type
    ///
    /// # Examples
    /// ```
    /// use autogen_core::TypeSubscription;
    /// use std::any::TypeId;
    ///
    /// let subscription = TypeSubscription::new(TypeId::of::<String>(), "String".to_string());
    /// assert_eq!(subscription.type_name(), "String");
    /// ```
    pub fn new(type_id: TypeId, type_name: String) -> Self {
        Self { type_id, type_name }
    }

    /// Create a type subscription for a specific type
    ///
    /// # Examples
    /// ```
    /// use autogen_core::TypeSubscription;
    /// 
    /// let subscription = TypeSubscription::for_type::<String>();
    /// assert_eq!(subscription.type_name(), "alloc::string::String");
    /// ```
    pub fn for_type<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>().to_string(),
        }
    }

    /// Get the type ID this subscription matches
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Get the type name
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl Subscription for TypeSubscription {
    fn matches(&self, _topic_id: &TopicId, message_type_id: TypeId) -> bool {
        self.type_id == message_type_id
    }

    fn description(&self) -> String {
        format!("TypeSubscription({})", self.type_name)
    }
}

/// Topic-based subscription
///
/// This subscription matches messages published to a specific topic,
/// regardless of message type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSubscription {
    /// The topic ID to match
    topic_id: TopicId,
}

impl TopicSubscription {
    /// Create a new topic subscription
    ///
    /// # Arguments
    /// * `topic_id` - The topic ID to subscribe to
    ///
    /// # Examples
    /// ```
    /// use autogen_core::{TopicSubscription, TopicId};
    /// 
    /// let topic_id = TopicId::new("user.message", "session_1").unwrap();
    /// let subscription = TopicSubscription::new(topic_id.clone());
    /// assert_eq!(subscription.topic_id(), &topic_id);
    /// ```
    pub fn new(topic_id: TopicId) -> Self {
        Self { topic_id }
    }

    /// Get the topic ID this subscription matches
    pub fn topic_id(&self) -> &TopicId {
        &self.topic_id
    }
}

impl Subscription for TopicSubscription {
    fn matches(&self, topic_id: &TopicId, _message_type_id: TypeId) -> bool {
        &self.topic_id == topic_id
    }

    fn description(&self) -> String {
        format!("TopicSubscription({})", self.topic_id)
    }
}

/// Type prefix subscription
///
/// This subscription matches messages whose type name starts with a specific prefix.
/// Equivalent to the Python TypePrefixSubscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypePrefixSubscription {
    /// The type name prefix to match
    type_prefix: String,
}

impl TypePrefixSubscription {
    /// Create a new type prefix subscription
    ///
    /// # Arguments
    /// * `type_prefix` - The type name prefix to match
    ///
    /// # Examples
    /// ```
    /// use autogen_core::TypePrefixSubscription;
    /// 
    /// let subscription = TypePrefixSubscription::new("user.".to_string());
    /// assert_eq!(subscription.type_prefix(), "user.");
    /// ```
    pub fn new(type_prefix: String) -> Self {
        Self { type_prefix }
    }

    /// Get the type prefix this subscription matches
    pub fn type_prefix(&self) -> &str {
        &self.type_prefix
    }
}

impl Subscription for TypePrefixSubscription {
    fn matches(&self, _topic_id: &TopicId, message_type_id: TypeId) -> bool {
        // For TypeId, we need to use a registry to map TypeId to type names
        // For now, we'll use a simple approach with type_name
        let type_name = format!("{:?}", message_type_id);
        type_name.starts_with(&self.type_prefix)
    }

    fn description(&self) -> String {
        format!("TypePrefixSubscription({}*)", self.type_prefix)
    }
}

/// Combined subscription that matches multiple conditions
///
/// This subscription allows combining multiple subscription conditions
/// with AND or OR logic.
pub struct CombinedSubscription {
    /// The subscriptions to combine
    subscriptions: Vec<Box<dyn Subscription>>,
    /// Whether to use AND (true) or OR (false) logic
    use_and_logic: bool,
}

impl CombinedSubscription {
    /// Create a new combined subscription with AND logic
    ///
    /// All subscriptions must match for the combined subscription to match.
    pub fn and(subscriptions: Vec<Box<dyn Subscription>>) -> Self {
        Self {
            subscriptions,
            use_and_logic: true,
        }
    }

    /// Create a new combined subscription with OR logic
    ///
    /// Any subscription must match for the combined subscription to match.
    pub fn or(subscriptions: Vec<Box<dyn Subscription>>) -> Self {
        Self {
            subscriptions,
            use_and_logic: false,
        }
    }
}

impl Subscription for CombinedSubscription {
    fn matches(&self, topic_id: &TopicId, message_type_id: TypeId) -> bool {
        if self.use_and_logic {
            self.subscriptions
                .iter()
                .all(|sub| sub.matches(topic_id, message_type_id))
        } else {
            self.subscriptions
                .iter()
                .any(|sub| sub.matches(topic_id, message_type_id))
        }
    }

    fn description(&self) -> String {
        let operator = if self.use_and_logic { " AND " } else { " OR " };
        let descriptions: Vec<String> = self
            .subscriptions
            .iter()
            .map(|sub| sub.description())
            .collect();
        format!("({})", descriptions.join(operator))
    }
}

/// Subscription registry for managing agent subscriptions
///
/// This registry tracks which agents are subscribed to which message types and topics.
#[derive(Default)]
pub struct SubscriptionRegistry {
    /// Map from agent ID to their subscriptions
    agent_subscriptions: std::collections::HashMap<AgentId, Vec<Box<dyn Subscription>>>,
}

impl SubscriptionRegistry {
    /// Create a new subscription registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe an agent to a subscription
    ///
    /// # Arguments
    /// * `agent_id` - The agent to subscribe
    /// * `subscription` - The subscription to add
    pub fn subscribe(&mut self, agent_id: AgentId, subscription: Box<dyn Subscription>) {
        self.agent_subscriptions
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(subscription);
    }

    /// Unsubscribe an agent from all subscriptions
    ///
    /// # Arguments
    /// * `agent_id` - The agent to unsubscribe
    pub fn unsubscribe_all(&mut self, agent_id: &AgentId) {
        self.agent_subscriptions.remove(agent_id);
    }

    /// Find all agents that match a given topic and message type
    ///
    /// # Arguments
    /// * `topic_id` - The topic ID of the message
    /// * `message_type_id` - The type ID of the message
    ///
    /// # Returns
    /// Set of agent IDs that should receive the message
    pub fn find_matching_agents(&self, topic_id: &TopicId, message_type_id: TypeId) -> HashSet<AgentId> {
        let mut matching_agents = HashSet::new();

        for (agent_id, subscriptions) in &self.agent_subscriptions {
            for subscription in subscriptions {
                if subscription.matches(topic_id, message_type_id) {
                    matching_agents.insert(agent_id.clone());
                    break; // Agent matches, no need to check other subscriptions
                }
            }
        }

        matching_agents
    }

    /// Get all subscriptions for an agent
    ///
    /// # Arguments
    /// * `agent_id` - The agent ID
    ///
    /// # Returns
    /// Vector of subscription descriptions
    pub fn get_agent_subscriptions(&self, agent_id: &AgentId) -> Vec<String> {
        self.agent_subscriptions
            .get(agent_id)
            .map(|subs| subs.iter().map(|sub| sub.description()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_subscription() {
        let subscription = DefaultSubscription::new();
        let topic_id = TopicId::new("test", "source").unwrap();
        
        assert!(subscription.matches(&topic_id, TypeId::of::<String>()));
        assert!(subscription.matches(&topic_id, TypeId::of::<i32>()));
        assert_eq!(subscription.description(), "DefaultSubscription (matches all messages)");
    }

    #[test]
    fn test_type_subscription() {
        let subscription = TypeSubscription::for_type::<String>();
        let topic_id = TopicId::new("test", "source").unwrap();
        
        assert!(subscription.matches(&topic_id, TypeId::of::<String>()));
        assert!(!subscription.matches(&topic_id, TypeId::of::<i32>()));
        assert_eq!(subscription.type_name(), "alloc::string::String");
    }

    #[test]
    fn test_topic_subscription() {
        let topic_id = TopicId::new("user.message", "session_1").unwrap();
        let subscription = TopicSubscription::new(topic_id.clone());
        
        assert!(subscription.matches(&topic_id, TypeId::of::<String>()));
        
        let other_topic = TopicId::new("system.message", "session_1").unwrap();
        assert!(!subscription.matches(&other_topic, TypeId::of::<String>()));
    }

    #[test]
    fn test_subscription_registry() {
        let mut registry = SubscriptionRegistry::new();
        let agent_id = AgentId::new("test_agent", "instance_1").unwrap();
        let topic_id = TopicId::new("test", "source").unwrap();
        
        // Subscribe agent to String messages
        registry.subscribe(agent_id.clone(), Box::new(TypeSubscription::for_type::<String>()));
        
        // Check matching
        let matching = registry.find_matching_agents(&topic_id, TypeId::of::<String>());
        assert!(matching.contains(&agent_id));
        
        let no_matching = registry.find_matching_agents(&topic_id, TypeId::of::<i32>());
        assert!(!no_matching.contains(&agent_id));
    }
}
