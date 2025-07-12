use crate::agent_type::AgentType;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

fn is_valid_agent_type(value: &str) -> bool {
    let re = Regex::new(r"^[\w\-\.]+\Z").unwrap();
    re.is_match(value)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, PartialOrd, Ord)]
pub struct AgentId {
    pub r#type: String,
    pub key: String,
}

impl AgentId {
    pub fn new(agent_type: &AgentType, key: &str) -> Result<Self, String> {
        if !is_valid_agent_type(&agent_type.r#type) {
            return Err(format!(
                "Invalid agent type: {}. Allowed values MUST match the regex: `^[\\w\\-\\.]+\\Z`",
                agent_type.r#type
            ));
        }
        Ok(Self {
            r#type: agent_type.r#type.clone(),
            key: key.to_string(),
        })
    }

    pub fn from_str(agent_id: &str) -> Result<Self, String> {
        let parts: Vec<&str> = agent_id.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid agent id: {}", agent_id));
        }
        let agent_type = parts[0];
        let key = parts[1];
        if !is_valid_agent_type(agent_type) {
            return Err(format!(
                "Invalid agent type: {}. Allowed values MUST match the regex: `^[\\w\\-\\.]+\\Z`",
                agent_type
            ));
        }
        Ok(Self {
            r#type: agent_type.to_string(),
            key: key.to_string(),
        })
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.r#type, self.key)
    }
}