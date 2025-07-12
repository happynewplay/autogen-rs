use crate::agent::Agent;
use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::subscription::Subscription;
use crate::topic::TopicId;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;

/// An enum to represent the different ways an agent can be identified.
pub enum AgentIdentifier {
    Id(AgentId),
    Type(AgentType),
    String(String),
}

/// A helper function to resolve an `AgentId` from various identifier types.
pub async fn get_id_from_identifier<F, Fut>(
    id_or_type: AgentIdentifier,
    key: &str,
    lazy: bool,
    instance_getter: F,
) -> Result<AgentId, Box<dyn Error>>
where
    F: Fn(AgentId) -> Fut,
    Fut: Future<Output = Result<Box<dyn Agent>, Box<dyn Error>>>,
{
    let agent_id = match id_or_type {
        AgentIdentifier::Id(id) => id,
        AgentIdentifier::Type(t) => AgentId::new(&t, key)?,
        AgentIdentifier::String(s) => {
            let agent_type = AgentType { r#type: s };
            AgentId::new(&agent_type, key)?
        }
    };

    if !lazy {
        instance_getter(agent_id.clone()).await?;
    }

    Ok(agent_id)
}

/// Manages subscriptions and maps topics to subscribed agents.
///
/// This is a simplified version that matches the Python implementation's design philosophy.
#[derive(Default)]
pub struct SubscriptionManager {
    subscriptions: Vec<Box<dyn Subscription>>,
    seen_topics: HashSet<TopicId>,
    subscribed_recipients: HashMap<TopicId, Vec<AgentId>>,
}

impl SubscriptionManager {
    /// Creates a new `SubscriptionManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the list of subscriptions.
    pub fn subscriptions(&self) -> &[Box<dyn Subscription>] {
        &self.subscriptions
    }

    /// Adds a new subscription (async to match Python version).
    pub async fn add_subscription(&mut self, subscription: Box<dyn Subscription>) -> Result<(), String> {
        // Check if the subscription already exists
        if self.subscriptions.iter().any(|s| s.id() == subscription.id()) {
            return Err("Subscription already exists".to_string());
        }

        self.subscriptions.push(subscription);
        self.rebuild_subscriptions();
        Ok(())
    }

    /// Removes a subscription by its ID (async to match Python version).
    pub async fn remove_subscription(&mut self, id: &str) -> Result<(), String> {
        let initial_len = self.subscriptions.len();
        self.subscriptions.retain(|s| s.id() != id);

        if self.subscriptions.len() == initial_len {
            return Err("Subscription does not exist".to_string());
        }

        // Rebuild the subscriptions
        self.rebuild_subscriptions();
        Ok(())
    }

    /// Gets the list of agents subscribed to a given topic (async to match Python version).
    pub async fn get_subscribed_recipients(&mut self, topic: &TopicId) -> Vec<AgentId> {
        if !self.seen_topics.contains(topic) {
            self.build_for_new_topic(topic.clone());
        }
        self.subscribed_recipients.get(topic).cloned().unwrap_or_default()
    }

    /// Synchronous version for compatibility
    pub fn get_subscribed_recipients_sync(&mut self, topic: &TopicId) -> Vec<AgentId> {
        if !self.seen_topics.contains(topic) {
            self.build_for_new_topic(topic.clone());
        }
        self.subscribed_recipients.get(topic).cloned().unwrap_or_default()
    }

    // TODO: optimize this... (keeping the Python comment)
    fn rebuild_subscriptions(&mut self) {
        let topics_to_rebuild: Vec<TopicId> = self.seen_topics.iter().cloned().collect();
        self.subscribed_recipients.clear();
        for topic in topics_to_rebuild {
            self.build_for_new_topic(topic);
        }
    }

    fn build_for_new_topic(&mut self, topic: TopicId) {
        self.seen_topics.insert(topic.clone());
        let mut recipients = Vec::new();

        for subscription in &self.subscriptions {
            if subscription.is_match(&topic) {
                if let Ok(agent_id) = subscription.map_to_agent(&topic) {
                    recipients.push(agent_id);
                }
            }
        }

        self.subscribed_recipients.insert(topic, recipients);
    }
}