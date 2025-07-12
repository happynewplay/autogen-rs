use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::exceptions::CantHandleException;
use crate::subscription::Subscription;
use crate::topic::TopicId;
use std::error::Error;
use uuid::Uuid;

/// This subscription matches on topics based on a prefix of the type and maps
/// to agents using the source of the topic as the agent key.
pub struct TypePrefixSubscription {
    id: String,
    topic_type_prefix: String,
    agent_type: String,
}

impl TypePrefixSubscription {
    /// Creates a new `TypePrefixSubscription`.
    pub fn new(topic_type_prefix: String, agent_type: AgentType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic_type_prefix,
            agent_type: agent_type.r#type,
        }
    }

    /// Get the topic type prefix
    pub fn topic_type_prefix(&self) -> &str {
        &self.topic_type_prefix
    }

    /// Get the agent type
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }
}

impl Subscription for TypePrefixSubscription {
    fn id(&self) -> &str {
        &self.id
    }

    fn is_match(&self, topic_id: &TopicId) -> bool {
        topic_id.r#type.starts_with(&self.topic_type_prefix)
    }

    fn map_to_agent(&self, topic_id: &TopicId) -> Result<AgentId, Box<dyn Error>> {
        if !self.is_match(topic_id) {
            return Err(Box::new(CantHandleException(
                "TopicId does not match the subscription".to_string(),
            )));
        }
        let agent_type = AgentType {
            r#type: self.agent_type.clone(),
        };
        AgentId::new(&agent_type, &topic_id.source)
            .map_err(|e| Box::new(CantHandleException(e)) as Box<dyn Error>)
    }
}

impl PartialEq for TypePrefixSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            || (self.agent_type == other.agent_type
                && self.topic_type_prefix == other.topic_type_prefix)
    }
}

impl Eq for TypePrefixSubscription {}