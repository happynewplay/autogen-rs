use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::exceptions::CantHandleException;
use crate::subscription::Subscription;
use crate::topic::TopicId;
use std::error::Error;
use uuid::Uuid;

/// This subscription matches on topics based on the type and maps to agents
/// using the source of the topic as the agent key.
pub struct TypeSubscription {
    id: String,
    topic_type: String,
    agent_type: String,
}

impl TypeSubscription {
    /// Creates a new `TypeSubscription`.
    pub fn new(topic_type: String, agent_type: AgentType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            topic_type,
            agent_type: agent_type.r#type,
        }
    }

    /// Get the topic type
    pub fn topic_type(&self) -> &str {
        &self.topic_type
    }

    /// Get the agent type
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }
}

impl Subscription for TypeSubscription {
    fn id(&self) -> &str {
        &self.id
    }

    fn is_match(&self, topic_id: &TopicId) -> bool {
        topic_id.r#type == self.topic_type
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

impl PartialEq for TypeSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            || (self.agent_type == other.agent_type && self.topic_type == other.topic_type)
    }
}

impl Eq for TypeSubscription {}