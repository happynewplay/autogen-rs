//! Agent identification system
//!
//! This module provides the `AgentId` type for uniquely identifying agents
//! within the autogen system, following the Python autogen-core design.

use crate::error::{AutoGenError, Result};
#[cfg(feature = "validation")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Validates if a string is a valid agent type according to autogen-core rules
/// Must match the regex: ^[\w\-\.]+\Z
fn is_valid_agent_type(value: &str) -> bool {
    #[cfg(feature = "validation")]
    {
        let re = Regex::new(r"^[\w\-\.]+$").unwrap();
        re.is_match(value)
    }
    #[cfg(not(feature = "validation"))]
    {
        // Basic validation without regex
        !value.is_empty() && value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_')
    }
}

/// Agent type wrapper that ensures validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentType {
    type_name: String,
}

impl AgentType {
    /// Create a new AgentType with validation
    pub fn new<T: Into<String>>(type_name: T) -> Result<Self> {
        let type_name = type_name.into();
        if !is_valid_agent_type(&type_name) {
            return Err(AutoGenError::InvalidAgentType {
                agent_type: type_name,
                reason: "Agent type must match the regex pattern".to_string(),
                expected_format: Some("^[\\w\\-\\.]+$".to_string()),
            });
        }
        Ok(Self { type_name })
    }

    /// Get the type name
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name)
    }
}

impl FromStr for AgentType {
    type Err = AutoGenError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// Unique identifier for an agent
///
/// Agent ID uniquely identifies an agent instance within an agent runtime -
/// including distributed runtime. It is the 'address' of the agent instance
/// for receiving messages.
///
/// This follows the Python autogen-core design where AgentId consists of
/// a validated type and a key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId {
    /// The validated type/category of the agent
    #[serde(rename = "type")]
    agent_type: String,
    /// Unique key within the agent type
    key: String,
}

impl AgentId {
    /// Create a new AgentId with the given type and key
    ///
    /// # Arguments
    /// * `agent_type` - The type identifier for the agent (can be String or AgentType)
    /// * `key` - The unique key for this agent instance
    ///
    /// # Examples
    /// ```
    /// use autogen_core::AgentId;
    ///
    /// let id = AgentId::new("assistant", "main").unwrap();
    /// assert_eq!(id.agent_type(), "assistant");
    /// assert_eq!(id.key(), "main");
    /// ```
    pub fn new<T, K>(agent_type: T, key: K) -> Result<Self>
    where
        T: TryInto<String>,
        T::Error: std::fmt::Display,
        K: Into<String>,
    {
        let type_str = agent_type.try_into().map_err(|e| {
            AutoGenError::InvalidAgentType {
                agent_type: "unknown".to_string(),
                reason: format!("Failed to convert agent type: {}", e),
                expected_format: None,
            }
        })?;

        if !is_valid_agent_type(&type_str) {
            return Err(AutoGenError::InvalidAgentType {
                agent_type: type_str,
                reason: "Agent type must match the regex pattern".to_string(),
                expected_format: Some("^[\\w\\-\\.]+$".to_string()),
            });
        }

        Ok(Self {
            agent_type: type_str,
            key: key.into(),
        })
    }

    /// Create a new AgentId from an AgentType and key
    pub fn from_type_and_key(agent_type: AgentType, key: String) -> Self {
        Self {
            agent_type: agent_type.type_name,
            key,
        }
    }

    /// Parse an AgentId from a string in the format "type/key"
    /// Following Python autogen-core convention
    pub fn from_str(agent_id: &str) -> Result<Self> {
        let parts: Vec<&str> = agent_id.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(AutoGenError::InvalidAgentId {
                agent_id: agent_id.to_string(),
                reason: "Expected format: 'type/key'".to_string(),
            });
        }
        Self::new(parts[0], parts[1])
    }

    /// Get the agent type
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }

    /// Get the agent key
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.agent_type, self.key)
    }
}

impl FromStr for AgentId {
    type Err = AutoGenError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_agent_id_creation() {
        let id = AgentId::new("test_agent", "instance_1").unwrap();
        assert_eq!(id.agent_type(), "test_agent");
        assert_eq!(id.key(), "instance_1");
    }

    #[test]
    fn test_agent_id_with_uuid() {
        let id = AgentId::new("test_agent", Uuid::new_v4().to_string()).unwrap();
        assert_eq!(id.agent_type(), "test_agent");
        assert!(Uuid::parse_str(id.key()).is_ok());
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new("test_agent", "instance_1").unwrap();
        assert_eq!(format!("{}", id), "test_agent/instance_1");
    }

    #[test]
    fn test_agent_id_from_string() {
        let id = AgentId::from_str("test_agent/instance_1").unwrap();
        assert_eq!(id.agent_type(), "test_agent");
        assert_eq!(id.key(), "instance_1");
    }

    #[test]
    fn test_agent_id_from_invalid_string() {
        let result = AgentId::from_str("invalid_format");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_id_serialization() {
        let id = AgentId::new("test_agent", "instance_1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_agent_type_validation() {
        // Valid agent types
        assert!(AgentId::new("valid_type", "key").is_ok());
        assert!(AgentId::new("valid-type", "key").is_ok());
        assert!(AgentId::new("valid.type", "key").is_ok());

        // Invalid agent types
        assert!(AgentId::new("invalid type", "key").is_err()); // space not allowed
        assert!(AgentId::new("invalid@type", "key").is_err()); // @ not allowed
    }
}
