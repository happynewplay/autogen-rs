use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub id: String,
    pub arguments: String, // JSON args
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMMessage {
    System(SystemMessage),
    User(UserMessage),
    Assistant(AssistantMessage),
    FunctionExecutionResult(FunctionExecutionResultMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub content: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<AssistantMessageContent>,
    #[serde(default)]
    pub thought: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssistantMessageContent {
    Text(String),
    FunctionCall(FunctionCall),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionResultMessage {
    pub function_name: String,
    pub content: String,
    #[serde(default)]
    pub source: Option<String>,
}