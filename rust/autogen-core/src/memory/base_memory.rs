use crate::model_context::chat_completion_context::ChatCompletionContext;
use crate::cancellation_token::CancellationToken;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

// Assuming Image struct is defined elsewhere, e.g., in crate::image
// For now, let's use a placeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryMimeType {
    #[serde(rename = "text/plain")]
    Text,
    #[serde(rename = "application/json")]
    Json,
    #[serde(rename = "text/markdown")]
    Markdown,
    #[serde(rename = "image/*")]
    Image,
    #[serde(rename = "application/octet-stream")]
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentType {
    Text(String),
    Binary(Vec<u8>),
    Json(Value),
    Image(Image),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    pub content: ContentType,
    pub mime_type: String, // Using String to accommodate custom MIME types
    #[serde(default)]
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub results: Vec<MemoryContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContextResult {
    pub memories: MemoryQueryResult,
}

#[async_trait]
pub trait Memory {
    async fn update_context(
        &self,
        model_context: &mut dyn ChatCompletionContext,
    ) -> Result<UpdateContextResult, Box<dyn Error>>;

    async fn query(
        &self,
        query: &str,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<MemoryQueryResult, Box<dyn Error>>;

    async fn add(
        &mut self,
        content: MemoryContent,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), Box<dyn Error>>;

    async fn clear(&mut self) -> Result<(), Box<dyn Error>>;

    async fn close(&mut self) -> Result<(), Box<dyn Error>>;
}