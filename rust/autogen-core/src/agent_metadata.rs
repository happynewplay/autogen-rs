use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentMetadata {
    pub r#type: String,
    pub key: String,
    pub description: String,
}