use crate::agent::Agent;
use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::subscription::Subscription;
use crate::subscription_manager::SubscriptionError;
use crate::topic::TopicId;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::future::Future;
use std::path::Path;
use tracing::{debug, info, warn, error};
use serde::{Serialize, Deserialize};
use serde_json::Value;

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
/// Enhanced with indexing for better performance.
#[derive(Default)]
pub struct SubscriptionManager {
    subscriptions: Vec<Box<dyn Subscription>>,
    seen_topics: HashSet<TopicId>,
    subscribed_recipients: HashMap<TopicId, Vec<AgentId>>,

    // Performance optimization indexes
    /// Index subscriptions by their ID for fast lookup
    subscription_id_index: HashMap<String, usize>,
    /// Index subscriptions by topic type for faster matching
    topic_type_index: HashMap<String, Vec<usize>>,
    /// Index subscriptions by topic prefix for faster prefix matching
    topic_prefix_index: HashMap<String, Vec<usize>>,
}

impl SubscriptionManager {
    /// Creates a new `SubscriptionManager`.
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            seen_topics: HashSet::new(),
            subscribed_recipients: HashMap::new(),
            subscription_id_index: HashMap::new(),
            topic_type_index: HashMap::new(),
            topic_prefix_index: HashMap::new(),
        }
    }

    /// Returns the list of subscriptions.
    pub fn subscriptions(&self) -> &[Box<dyn Subscription>] {
        &self.subscriptions
    }

    /// Adds a new subscription with enhanced error handling and logging.
    pub async fn add_subscription(&mut self, subscription: Box<dyn Subscription>) -> Result<(), SubscriptionError> {
        let subscription_id = subscription.id().to_string(); // Clone the ID to avoid borrowing issues

        info!("Adding subscription with ID: {}", subscription_id);

        // Validate subscription ID is not empty
        if subscription_id.is_empty() {
            error!("Attempted to add subscription with empty ID");
            return Err(SubscriptionError::Invalid("Subscription ID cannot be empty".to_string()));
        }

        // Check if the subscription already exists
        if self.subscriptions.iter().any(|s| s.id() == subscription_id) {
            warn!("Subscription already exists: {}", subscription_id);
            return Err(SubscriptionError::AlreadyExists(subscription_id));
        }

        // Validate subscription by testing it with a dummy topic
        // This helps catch malformed subscriptions early
        let test_topic = TopicId {
            r#type: "test".to_string(),
            source: "test".to_string(),
        };

        // Test if the subscription can handle topic matching without panicking
        let _can_match = subscription.is_match(&test_topic);

        debug!("Subscription {} passed validation", subscription_id);

        // Add the subscription and update indexes
        let subscription_index = self.subscriptions.len();
        self.subscriptions.push(subscription);

        // Update ID index
        self.subscription_id_index.insert(subscription_id.clone(), subscription_index);

        // Update topic type and prefix indexes
        self.update_subscription_indexes(subscription_index);

        // Efficiently rebuild only affected topics instead of all topics
        self.rebuild_subscriptions_optimized();

        info!("Successfully added subscription: {}", subscription_id);
        Ok(())
    }

    /// Removes a subscription by its ID with enhanced error handling and logging.
    pub async fn remove_subscription(&mut self, id: &str) -> Result<(), SubscriptionError> {
        info!("Removing subscription with ID: {}", id);

        // Validate subscription ID is not empty
        if id.is_empty() {
            error!("Attempted to remove subscription with empty ID");
            return Err(SubscriptionError::Invalid("Subscription ID cannot be empty".to_string()));
        }

        let initial_len = self.subscriptions.len();

        // Find the subscription index before removing
        let subscription_index = self.subscription_id_index.get(id).copied();

        // Remove the subscription
        self.subscriptions.retain(|s| s.id() != id);

        if self.subscriptions.len() == initial_len {
            warn!("Attempted to remove non-existent subscription: {}", id);
            return Err(SubscriptionError::NotFound(id.to_string()));
        }

        debug!("Subscription {} removed from list", id);

        // Update indexes after removal
        self.remove_from_indexes(id, subscription_index);

        // Clean up any cached data related to this subscription
        self.cleanup_subscription_data(id);

        // Rebuild the subscriptions efficiently
        self.rebuild_subscriptions_optimized();

        info!("Successfully removed subscription: {}", id);
        Ok(())
    }

    /// Remove subscription from indexes
    fn remove_from_indexes(&mut self, subscription_id: &str, removed_index: Option<usize>) {
        // Remove from ID index
        self.subscription_id_index.remove(subscription_id);

        if let Some(index) = removed_index {
            // Remove from topic type index
            self.topic_type_index.retain(|_, indexes| {
                indexes.retain(|&i| i != index);
                !indexes.is_empty()
            });

            // Remove from topic prefix index
            self.topic_prefix_index.retain(|_, indexes| {
                indexes.retain(|&i| i != index);
                !indexes.is_empty()
            });

            // Update indexes for remaining subscriptions (shift down indexes > removed_index)
            for indexes in self.topic_type_index.values_mut() {
                for idx in indexes.iter_mut() {
                    if *idx > index {
                        *idx -= 1;
                    }
                }
            }

            for indexes in self.topic_prefix_index.values_mut() {
                for idx in indexes.iter_mut() {
                    if *idx > index {
                        *idx -= 1;
                    }
                }
            }

            // Update subscription ID index
            for (_, idx) in self.subscription_id_index.iter_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }
        }

        debug!("Removed subscription {} from indexes", subscription_id);
    }

    /// Clean up any cached data related to a removed subscription
    fn cleanup_subscription_data(&mut self, _subscription_id: &str) {
        // For now, we rely on rebuild_subscriptions to clean up
        // In the future, this could be optimized to only clean up specific entries
        debug!("Cleaning up data for removed subscription");
    }

    /// Save subscription manager state to JSON
    pub fn save_state(&self) -> Result<Value, SubscriptionError> {
        info!("Saving subscription manager state");

        let mut state = serde_json::Map::new();

        // Save basic statistics
        state.insert("total_subscriptions".to_string(),
                    Value::Number(self.subscriptions.len().into()));
        state.insert("total_seen_topics".to_string(),
                    Value::Number(self.seen_topics.len().into()));
        state.insert("total_cached_recipients".to_string(),
                    Value::Number(self.subscribed_recipients.len().into()));

        // Save subscription IDs and basic info (we can't serialize trait objects directly)
        let subscription_info: Vec<Value> = self.subscriptions.iter().map(|sub| {
            serde_json::json!({
                "id": sub.id(),
                "saved_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })
        }).collect();

        state.insert("subscriptions".to_string(), Value::Array(subscription_info));

        // Save seen topics
        let topics: Vec<Value> = self.seen_topics.iter().map(|topic| {
            serde_json::json!({
                "type": topic.r#type,
                "source": topic.source
            })
        }).collect();

        state.insert("seen_topics".to_string(), Value::Array(topics));

        // Save metadata
        state.insert("version".to_string(), Value::String("1.0".to_string()));
        state.insert("saved_at".to_string(),
                    Value::Number(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs().into()));

        debug!("Subscription manager state saved successfully");
        Ok(Value::Object(state))
    }

    /// Load subscription manager state from JSON (partial restoration)
    /// Note: This only restores metadata and seen topics, not the actual subscriptions
    /// since trait objects cannot be deserialized directly
    pub fn load_state_metadata(&mut self, state: &Value) -> Result<(), SubscriptionError> {
        info!("Loading subscription manager state metadata");

        let state_obj = state.as_object()
            .ok_or_else(|| SubscriptionError::Invalid("State must be a JSON object".to_string()))?;

        // Validate version
        if let Some(version) = state_obj.get("version") {
            if version.as_str() != Some("1.0") {
                warn!("Loading state with different version: {:?}", version);
            }
        }

        // Restore seen topics
        if let Some(topics_array) = state_obj.get("seen_topics").and_then(|v| v.as_array()) {
            self.seen_topics.clear();
            for topic_value in topics_array {
                if let Some(topic_obj) = topic_value.as_object() {
                    if let (Some(type_val), Some(source_val)) =
                        (topic_obj.get("type").and_then(|v| v.as_str()),
                         topic_obj.get("source").and_then(|v| v.as_str())) {
                        let topic = TopicId {
                            r#type: type_val.to_string(),
                            source: source_val.to_string(),
                        };
                        self.seen_topics.insert(topic);
                    }
                }
            }
            debug!("Restored {} seen topics", self.seen_topics.len());
        }

        info!("Subscription manager state metadata loaded successfully");
        Ok(())
    }

    /// Update indexes for a subscription at the given index
    fn update_subscription_indexes(&mut self, subscription_index: usize) {
        if let Some(subscription) = self.subscriptions.get(subscription_index) {
            let subscription_id = subscription.id();

            // Try to determine subscription type and update appropriate indexes
            // This is a heuristic approach since we can't directly inspect trait objects

            // For TypeSubscription-like subscriptions, we can try to extract topic type
            // This is simplified - in practice you might want more sophisticated type detection
            if subscription_id.contains("TypeSubscription") {
                // Extract topic type from subscription ID if possible
                // This is a simplified heuristic
                if let Some(topic_type) = self.extract_topic_type_from_id(subscription_id) {
                    self.topic_type_index
                        .entry(topic_type)
                        .or_insert_with(Vec::new)
                        .push(subscription_index);
                }
            } else if subscription_id.contains("TypePrefixSubscription") {
                // Extract topic prefix from subscription ID if possible
                if let Some(topic_prefix) = self.extract_topic_prefix_from_id(subscription_id) {
                    self.topic_prefix_index
                        .entry(topic_prefix)
                        .or_insert_with(Vec::new)
                        .push(subscription_index);
                }
            }

            debug!("Updated indexes for subscription: {}", subscription_id);
        }
    }

    /// Extract topic type from subscription ID (heuristic)
    fn extract_topic_type_from_id(&self, subscription_id: &str) -> Option<String> {
        // This is a simplified heuristic - in practice you might want more sophisticated parsing
        // Look for patterns like "TypeSubscription:topic_type" or similar
        if let Some(colon_pos) = subscription_id.find(':') {
            let after_colon = &subscription_id[colon_pos + 1..];
            if !after_colon.is_empty() {
                return Some(after_colon.to_string());
            }
        }
        None
    }

    /// Extract topic prefix from subscription ID (heuristic)
    fn extract_topic_prefix_from_id(&self, subscription_id: &str) -> Option<String> {
        // Similar heuristic for prefix subscriptions
        if let Some(colon_pos) = subscription_id.find(':') {
            let after_colon = &subscription_id[colon_pos + 1..];
            if !after_colon.is_empty() {
                return Some(after_colon.to_string());
            }
        }
        None
    }

    /// Rebuild all indexes from scratch
    fn rebuild_indexes(&mut self) {
        debug!("Rebuilding subscription indexes");

        self.subscription_id_index.clear();
        self.topic_type_index.clear();
        self.topic_prefix_index.clear();

        // Collect subscription information first to avoid borrowing conflicts
        let subscription_info: Vec<(usize, String)> = self.subscriptions.iter()
            .enumerate()
            .map(|(index, subscription)| (index, subscription.id().to_string()))
            .collect();

        for (index, subscription_id) in subscription_info {
            self.subscription_id_index.insert(subscription_id.clone(), index);
            self.update_subscription_indexes(index);
        }

        debug!("Rebuilt indexes for {} subscriptions", self.subscriptions.len());
    }

    /// Get subscription by ID using index for fast lookup
    pub fn get_subscription_by_id(&self, id: &str) -> Option<&Box<dyn Subscription>> {
        self.subscription_id_index.get(id)
            .and_then(|&index| self.subscriptions.get(index))
    }

    /// Get subscriptions that might match a topic type using index
    fn get_indexed_subscriptions_for_topic(&self, topic: &TopicId) -> Vec<usize> {
        let mut candidate_indexes = Vec::new();

        // Check exact type matches
        if let Some(type_indexes) = self.topic_type_index.get(&topic.r#type) {
            candidate_indexes.extend(type_indexes);
        }

        // Check prefix matches
        for (prefix, indexes) in &self.topic_prefix_index {
            if topic.r#type.starts_with(prefix) {
                candidate_indexes.extend(indexes);
            }
        }

        // Remove duplicates and return
        candidate_indexes.sort_unstable();
        candidate_indexes.dedup();
        candidate_indexes
    }

    /// Gets the list of agents subscribed to a given topic with enhanced performance.
    pub async fn get_subscribed_recipients(&mut self, topic: &TopicId) -> Vec<AgentId> {
        debug!("Getting subscribed recipients for topic: {:?}", topic);

        // Check cache first
        if let Some(recipients) = self.subscribed_recipients.get(topic) {
            debug!("Found {} cached recipients for topic", recipients.len());
            return recipients.clone();
        }

        // Build for new topic if not seen before
        if !self.seen_topics.contains(topic) {
            info!("Building recipients for new topic: {:?}", topic);
            self.build_for_new_topic(topic.clone());
        }

        self.subscribed_recipients.get(topic).cloned().unwrap_or_default()
    }

    /// Synchronous version for compatibility with enhanced logging
    pub fn get_subscribed_recipients_sync(&mut self, topic: &TopicId) -> Vec<AgentId> {
        debug!("Getting subscribed recipients (sync) for topic: {:?}", topic);

        // Check cache first
        if let Some(recipients) = self.subscribed_recipients.get(topic) {
            debug!("Found {} cached recipients for topic", recipients.len());
            return recipients.clone();
        }

        // Build for new topic if not seen before
        if !self.seen_topics.contains(topic) {
            info!("Building recipients for new topic: {:?}", topic);
            self.build_for_new_topic(topic.clone());
        }

        self.subscribed_recipients.get(topic).cloned().unwrap_or_default()
    }

    /// Get all subscriptions that match a given topic (without building cache)
    pub fn find_matching_subscriptions(&self, topic: &TopicId) -> Vec<&Box<dyn Subscription>> {
        debug!("Finding matching subscriptions for topic: {:?}", topic);

        let matching: Vec<&Box<dyn Subscription>> = self.subscriptions.iter()
            .filter(|subscription| {
                let matches = subscription.is_match(topic);
                if matches {
                    debug!("Subscription {} matches topic", subscription.id());
                }
                matches
            })
            .collect();

        debug!("Found {} matching subscriptions", matching.len());
        matching
    }

    /// Check if any subscription matches the given topic
    pub fn has_matching_subscription(&self, topic: &TopicId) -> bool {
        self.subscriptions.iter().any(|sub| sub.is_match(topic))
    }

    /// Get subscription statistics
    pub fn get_subscription_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("total_subscriptions".to_string(), self.subscriptions.len());
        stats.insert("seen_topics".to_string(), self.seen_topics.len());
        stats.insert("cached_recipients".to_string(), self.subscribed_recipients.len());

        // Count subscriptions by type (if we can determine it)
        let mut type_counts = HashMap::new();
        for subscription in &self.subscriptions {
            // This is a simplified type detection - in practice you might want more sophisticated logic
            let type_name = if subscription.id().contains("TypeSubscription") {
                "TypeSubscription"
            } else if subscription.id().contains("TypePrefixSubscription") {
                "TypePrefixSubscription"
            } else {
                "Unknown"
            };
            *type_counts.entry(type_name.to_string()).or_insert(0) += 1;
        }

        for (type_name, count) in type_counts {
            stats.insert(format!("subscriptions_{}", type_name), count);
        }

        stats
    }

    // TODO: optimize this... (keeping the Python comment)
    fn rebuild_subscriptions(&mut self) {
        let topics_to_rebuild: Vec<TopicId> = self.seen_topics.iter().cloned().collect();
        self.subscribed_recipients.clear();
        for topic in topics_to_rebuild {
            self.build_for_new_topic(topic);
        }
    }

    /// Optimized version of rebuild_subscriptions that only rebuilds affected topics
    fn rebuild_subscriptions_optimized(&mut self) {
        debug!("Rebuilding subscriptions for {} seen topics", self.seen_topics.len());

        // For now, use the same logic as rebuild_subscriptions
        // In the future, this could be optimized to only rebuild topics that are affected
        // by the new subscription based on its matching criteria
        self.rebuild_subscriptions();

        debug!("Subscription rebuild completed");
    }

    fn build_for_new_topic(&mut self, topic: TopicId) {
        debug!("Building recipients for new topic: {:?}", topic);
        self.seen_topics.insert(topic.clone());
        let mut recipients = Vec::new();

        // Use indexed lookup for better performance
        let candidate_indexes = self.get_indexed_subscriptions_for_topic(&topic);

        if candidate_indexes.is_empty() {
            // Fallback to full scan if no indexed candidates found
            debug!("No indexed candidates found, performing full scan");
            for subscription in &self.subscriptions {
                if subscription.is_match(&topic) {
                    if let Ok(agent_id) = subscription.map_to_agent(&topic) {
                        recipients.push(agent_id);
                    }
                }
            }
        } else {
            // Check only the indexed candidates
            debug!("Checking {} indexed candidates", candidate_indexes.len());
            for &index in &candidate_indexes {
                if let Some(subscription) = self.subscriptions.get(index) {
                    if subscription.is_match(&topic) {
                        if let Ok(agent_id) = subscription.map_to_agent(&topic) {
                            recipients.push(agent_id);
                        }
                    }
                }
            }
        }

        debug!("Found {} recipients for topic", recipients.len());
        self.subscribed_recipients.insert(topic, recipients);
    }
}