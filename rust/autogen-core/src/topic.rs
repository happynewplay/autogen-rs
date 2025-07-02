//! Topic system for message routing
//!
//! This module provides the topic system used for message routing and subscription
//! in the autogen system, following the Python autogen-core design.

use crate::error::{AutoGenError, Result};
#[cfg(feature = "validation")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Validates if a string is a valid topic type according to cloud event spec
/// Must match the pattern: ^[\w\-\.\:=]+\Z
fn is_valid_topic_type(value: &str) -> bool {
    #[cfg(feature = "validation")]
    {
        let re = Regex::new(r"^[\w\-\.\:=]+$").unwrap();
        re.is_match(value)
    }
    #[cfg(not(feature = "validation"))]
    {
        // Basic validation without regex
        !value.is_empty() && value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == ':' || c == '=' || c == '_')
    }
}

/// TopicId defines the scope of a broadcast message
///
/// In essence, agent runtime implements a publish-subscribe model through its 
/// broadcast API: when publishing a message, the topic must be specified.
/// This follows the CloudEvents specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopicId {
    /// Type of the event that this topic_id contains
    /// Adheres to the cloud event spec
    #[serde(rename = "type")]
    pub topic_type: String,
    
    /// Identifies the context in which an event happened
    /// Adheres to the cloud event spec
    pub source: String,
}

impl TopicId {
    /// Create a new TopicId with validation
    ///
    /// # Arguments
    /// * `topic_type` - Type of the event (must match cloud event spec pattern)
    /// * `source` - Context identifier where the event happened
    ///
    /// # Examples
    /// ```
    /// use autogen_core::TopicId;
    /// 
    /// let topic = TopicId::new("user.message", "chat_session_1").unwrap();
    /// assert_eq!(topic.topic_type(), "user.message");
    /// assert_eq!(topic.source(), "chat_session_1");
    /// ```
    pub fn new<T: Into<String>, S: Into<String>>(topic_type: T, source: S) -> Result<Self> {
        let topic_type = topic_type.into();
        if !is_valid_topic_type(&topic_type) {
            return Err(AutoGenError::Validation(crate::error::ValidationError::InvalidFormat {
                field: "topic_type".to_string(),
                value: topic_type,
                expected_format: "^[\\w\\-\\.\\:=]+\\Z".to_string(),
            }));
        }

        Ok(Self {
            topic_type,
            source: source.into(),
        })
    }

    /// Get the topic type
    pub fn topic_type(&self) -> &str {
        &self.topic_type
    }

    /// Get the source
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl fmt::Display for TopicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.topic_type, self.source)
    }
}

/// DefaultTopicId provides a sensible default for the topic_id and source fields
///
/// This is equivalent to the Python DefaultTopicId class, providing convenient
/// defaults for topic creation.
#[derive(Debug, Clone)]
pub struct DefaultTopicId {
    topic_id: TopicId,
}

impl DefaultTopicId {
    /// Create a new DefaultTopicId
    ///
    /// # Arguments
    /// * `topic_type` - Topic type to publish message to (defaults to "default")
    /// * `source` - Topic source (defaults to "default" if None)
    ///
    /// # Examples
    /// ```
    /// use autogen_core::DefaultTopicId;
    /// 
    /// let default_topic = DefaultTopicId::new(None, None).unwrap();
    /// assert_eq!(default_topic.topic_type(), "default");
    /// assert_eq!(default_topic.source(), "default");
    /// 
    /// let custom_topic = DefaultTopicId::new(Some("user.input"), Some("session_1")).unwrap();
    /// assert_eq!(custom_topic.topic_type(), "user.input");
    /// assert_eq!(custom_topic.source(), "session_1");
    /// ```
    pub fn new(topic_type: Option<&str>, source: Option<&str>) -> Result<Self> {
        let topic_type = topic_type.unwrap_or("default");
        let source = source.unwrap_or("default");
        
        let topic_id = TopicId::new(topic_type, source)?;
        Ok(Self { topic_id })
    }

    /// Get the underlying TopicId
    pub fn topic_id(&self) -> &TopicId {
        &self.topic_id
    }

    /// Get the topic type
    pub fn topic_type(&self) -> &str {
        self.topic_id.topic_type()
    }

    /// Get the source
    pub fn source(&self) -> &str {
        self.topic_id.source()
    }
}

impl From<DefaultTopicId> for TopicId {
    fn from(default_topic: DefaultTopicId) -> Self {
        default_topic.topic_id
    }
}

impl fmt::Display for DefaultTopicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.topic_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_id_creation() {
        let topic = TopicId::new("user.message", "session_1").unwrap();
        assert_eq!(topic.topic_type(), "user.message");
        assert_eq!(topic.source(), "session_1");
    }

    #[test]
    fn test_topic_id_validation() {
        // Valid topic types
        assert!(TopicId::new("user.message", "source").is_ok());
        assert!(TopicId::new("user-message", "source").is_ok());
        assert!(TopicId::new("user_message", "source").is_ok());
        assert!(TopicId::new("user:message", "source").is_ok());
        assert!(TopicId::new("user=message", "source").is_ok());

        // Invalid topic types (should fail validation)
        assert!(TopicId::new("user message", "source").is_err()); // space not allowed
        assert!(TopicId::new("user@message", "source").is_err()); // @ not allowed
    }

    #[test]
    fn test_topic_id_display() {
        let topic = TopicId::new("user.message", "session_1").unwrap();
        assert_eq!(format!("{}", topic), "user.message@session_1");
    }

    #[test]
    fn test_default_topic_id() {
        let default_topic = DefaultTopicId::new(None, None).unwrap();
        assert_eq!(default_topic.topic_type(), "default");
        assert_eq!(default_topic.source(), "default");

        let custom_topic = DefaultTopicId::new(Some("user.input"), Some("session_1")).unwrap();
        assert_eq!(custom_topic.topic_type(), "user.input");
        assert_eq!(custom_topic.source(), "session_1");
    }

    #[test]
    fn test_topic_id_serialization() {
        let topic = TopicId::new("user.message", "session_1").unwrap();
        let json = serde_json::to_string(&topic).unwrap();
        let deserialized: TopicId = serde_json::from_str(&json).unwrap();
        assert_eq!(topic, deserialized);
    }
}
