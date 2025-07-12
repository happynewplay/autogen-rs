// Re-export the simplified SubscriptionManager from runtime_impl_helpers
pub use crate::runtime_impl_helpers::SubscriptionManager;

use crate::agent_id::AgentId;
use crate::subscription::Subscription;
use crate::type_subscription::TypeSubscription;
use crate::type_prefix_subscription::TypePrefixSubscription;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during subscription management
#[derive(Error, Debug)]
pub enum SubscriptionError {
    #[error("Subscription already exists: {0}")]
    AlreadyExists(String),
    #[error("Subscription not found: {0}")]
    NotFound(String),
    #[error("Invalid subscription: {0}")]
    Invalid(String),
    #[error("Agent not found: {0:?}")]
    AgentNotFound(AgentId),
}

/// Statistics about subscriptions
#[derive(Debug, Clone)]
pub struct SubscriptionStats {
    pub total_subscriptions: usize,
    pub total_seen_topics: usize,
    pub total_cached_recipients: usize,
}

/// Extension methods for the simplified SubscriptionManager
impl SubscriptionManager {
    /// Add a type subscription (convenience method)
    pub async fn add_type_subscription(&mut self, subscription: TypeSubscription) -> Result<(), String> {
        self.add_subscription(Box::new(subscription)).await
    }

    /// Add a type prefix subscription (convenience method)
    pub async fn add_prefix_subscription(&mut self, subscription: TypePrefixSubscription) -> Result<(), String> {
        self.add_subscription(Box::new(subscription)).await
    }

    /// Get subscription statistics
    pub fn get_stats(&self) -> SubscriptionStats {
        SubscriptionStats {
            total_subscriptions: self.subscriptions().len(),
            total_seen_topics: 0, // 未实现: 需要添加seen_topics的getter方法
            total_cached_recipients: 0, // 未实现: 需要添加subscribed_recipients的getter方法
        }
    }
}

/// Global subscription manager instance (simplified)
use once_cell::sync::Lazy;
use std::sync::Mutex;

static GLOBAL_SUBSCRIPTION_MANAGER: Lazy<Arc<Mutex<SubscriptionManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(SubscriptionManager::new())));

/// Get the global subscription manager
pub fn get_global_subscription_manager() -> Arc<Mutex<SubscriptionManager>> {
    GLOBAL_SUBSCRIPTION_MANAGER.clone()
}

/// Convenience functions for global subscription management
pub async fn add_global_subscription(subscription: Box<dyn Subscription>) -> Result<(), String> {
    let mut manager = GLOBAL_SUBSCRIPTION_MANAGER.lock().unwrap();
    manager.add_subscription(subscription).await
}

pub async fn add_global_type_subscription(subscription: TypeSubscription) -> Result<(), String> {
    let mut manager = GLOBAL_SUBSCRIPTION_MANAGER.lock().unwrap();
    manager.add_type_subscription(subscription).await
}

pub async fn add_global_prefix_subscription(subscription: TypePrefixSubscription) -> Result<(), String> {
    let mut manager = GLOBAL_SUBSCRIPTION_MANAGER.lock().unwrap();
    manager.add_prefix_subscription(subscription).await
}
