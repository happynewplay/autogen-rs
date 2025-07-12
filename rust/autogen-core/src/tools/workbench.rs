use super::base::ToolSchema;
use crate::memory::base_memory::Image;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use futures::stream::BoxStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResultContent {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResultContent {
    pub content: Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResultContent {
    TextResultContent(TextResultContent),
    ImageResultContent(ImageResultContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub result: Vec<ResultContent>,
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    pub fn to_text(&self, replace_image: Option<&str>) -> String {
        self.result
            .iter()
            .map(|content| match content {
                ResultContent::TextResultContent(text) => text.content.clone(),
                ResultContent::ImageResultContent(_) => {
                    replace_image.unwrap_or("[Image]").to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
pub trait Workbench {
    async fn list_tools(&self) -> Result<Vec<ToolSchema>, Box<dyn Error + Send + Sync>>;
    async fn call_tool(
        &self,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>>;
    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn stop(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn reset(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error + Send + Sync>>;
    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[async_trait]
pub trait StreamWorkbench: Workbench {
    async fn call_tool_stream<'a>(
        &'a self,
        name: &'a str,
        arguments: &'a Value,
    ) -> Result<BoxStream<'a, Result<ToolResult, Box<dyn Error + Send + Sync>>>, Box<dyn Error + Send + Sync>>;
}