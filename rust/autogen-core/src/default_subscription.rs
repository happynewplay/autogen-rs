use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::subscription::Subscription;
use crate::subscription_context::SubscriptionInstantiationContext;
use crate::topic::TopicId;
use crate::type_subscription::TypeSubscription;
use std::error::Error;

/// The default subscription is designed to be a sensible default for applications
/// that only need a global scope for agents.
///
/// This topic by default uses the "default" topic type and attempts to detect
/// the agent type to use based on the instantiation context.
pub struct DefaultSubscription {
    inner: TypeSubscription,
}

impl DefaultSubscription {
    /// Creates a new `DefaultSubscription`.
    ///
    /// # Arguments
    ///
    /// * `topic_type` - The topic type to subscribe to. Defaults to "default".
    /// * `agent_type` - The agent type to use for the subscription. If `None`,
    ///                  it will attempt to detect the agent type from the
    ///                  instantiation context.
    ///
    /// # Panics
    ///
    /// Panics if `agent_type` is `None` and the function is not called within
    /// a subscription instantiation context.
    pub fn new(
        topic_type: Option<String>,
        agent_type: Option<AgentType>,
    ) -> Self {
        let agent_type = agent_type.unwrap_or_else(|| {
            SubscriptionInstantiationContext::agent_type()
                .expect("agent_type must be specified or DefaultSubscription must be created within a subscription callback")
        });

        let inner = TypeSubscription::new(
            topic_type.unwrap_or_else(|| "default".to_string()),
            agent_type,
        );

        Self { inner }
    }

    /// Get the topic type
    pub fn topic_type(&self) -> &str {
        self.inner.topic_type()
    }

    /// Get the agent type
    pub fn agent_type(&self) -> &str {
        self.inner.agent_type()
    }
}

impl Subscription for DefaultSubscription {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn is_match(&self, topic_id: &TopicId) -> bool {
        self.inner.is_match(topic_id)
    }

    fn map_to_agent(&self, topic_id: &TopicId) -> Result<AgentId, Box<dyn Error>> {
        self.inner.map_to_agent(topic_id)
    }
}

impl PartialEq for DefaultSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for DefaultSubscription {}