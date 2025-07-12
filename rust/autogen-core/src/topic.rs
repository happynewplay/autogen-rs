use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

fn is_valid_topic_type(value: &str) -> bool {
    let re = Regex::new(r"^[\w\-\.\:\=]+\Z").unwrap();
    re.is_match(value)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct TopicId {
    pub r#type: String,
    pub source: String,
}

impl TopicId {
    pub fn new(r#type: &str, source: &str) -> Result<Self, String> {
        if !is_valid_topic_type(r#type) {
            return Err(format!(
                "Invalid topic type: {}. Must match the pattern: `^[\\w\\-\\.\\:\\=]+\\Z`",
                r#type
            ));
        }
        Ok(Self {
            r#type: r#type.to_string(),
            source: source.to_string(),
        })
    }

    pub fn from_str(topic_id: &str) -> Result<Self, String> {
        let parts: Vec<&str> = topic_id.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid topic id: {}", topic_id));
        }
        let r#type = parts[0];
        let source = parts[1];
        Self::new(r#type, source)
    }
}

impl fmt::Display for TopicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.r#type, self.source)
    }
}