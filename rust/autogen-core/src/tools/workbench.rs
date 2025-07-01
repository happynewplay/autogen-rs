//! Workbench trait and types for managing collections of tools.

use std::collections::HashMap;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use crate::{CancellationToken, error::Result};
use super::base_tool::ToolSchema;

/// Text result content for workbench operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextResultContent {
    /// The text content
    pub content: String,
}

/// Image result content for workbench operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageResultContent {
    /// Base64 encoded image data
    pub data: String,
    /// MIME type of the image
    pub mime_type: String,
    /// Optional description of the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Result content types for workbench operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkbenchResultContent {
    /// Text content
    #[serde(rename = "text")]
    Text(TextResultContent),
    /// Image content
    #[serde(rename = "image")]
    Image(ImageResultContent),
}

/// Result of a workbench tool execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkbenchResult {
    /// The name of the tool that was executed
    pub name: String,
    /// The result content
    pub result: Vec<WorkbenchResultContent>,
    /// Whether the execution resulted in an error
    #[serde(default)]
    pub is_error: bool,
}

impl WorkbenchResult {
    /// Create a new successful result with text content
    pub fn text(name: String, content: String) -> Self {
        Self {
            name,
            result: vec![WorkbenchResultContent::Text(TextResultContent { content })],
            is_error: false,
        }
    }

    /// Create a new error result
    pub fn error(name: String, error_message: String) -> Self {
        Self {
            name,
            result: vec![WorkbenchResultContent::Text(TextResultContent { 
                content: error_message 
            })],
            is_error: true,
        }
    }

    /// Create a new result with image content
    pub fn image(name: String, data: String, mime_type: String, description: Option<String>) -> Self {
        Self {
            name,
            result: vec![WorkbenchResultContent::Image(ImageResultContent {
                data,
                mime_type,
                description,
            })],
            is_error: false,
        }
    }

    /// Add text content to the result
    pub fn add_text(&mut self, content: String) {
        self.result.push(WorkbenchResultContent::Text(TextResultContent { content }));
    }

    /// Add image content to the result
    pub fn add_image(&mut self, data: String, mime_type: String, description: Option<String>) {
        self.result.push(WorkbenchResultContent::Image(ImageResultContent {
            data,
            mime_type,
            description,
        }));
    }
}

/// A workbench is a component that provides a set of tools that may share
/// resources and state.
///
/// A workbench is responsible for managing the lifecycle of the tools and
/// providing a single interface to call them. The tools provided by the workbench
/// may be dynamic and their availabilities may change after each tool execution.
///
/// A workbench can be started by calling the `start` method and stopped by calling
/// the `stop` method. It can also be used as an asynchronous context manager.
#[async_trait]
pub trait Workbench: Send + Sync {
    /// List all available tools in the workbench.
    ///
    /// # Returns
    /// A vector of tool schemas describing the available tools
    async fn list_tools(&self) -> Result<Vec<ToolSchema>>;

    /// Call a tool by name with the given arguments.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to call
    /// * `arguments` - The arguments to pass to the tool
    /// * `cancellation_token` - Optional token to cancel the operation
    /// * `call_id` - Optional identifier for this tool call
    ///
    /// # Returns
    /// The result of the tool execution
    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<&HashMap<String, serde_json::Value>>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<WorkbenchResult>;

    /// Start the workbench and initialize any resources.
    ///
    /// This method should be called before using the workbench.
    async fn start(&mut self) -> Result<()>;

    /// Stop the workbench and release any resources.
    ///
    /// This method should be called when the workbench is no longer needed.
    async fn stop(&mut self) -> Result<()>;

    /// Reset the workbench to its initialized, started state.
    async fn reset(&mut self) -> Result<()>;

    /// Save the state of the workbench.
    ///
    /// # Returns
    /// A map containing the serialized state
    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>>;

    /// Load state into the workbench.
    ///
    /// # Arguments
    /// * `state` - The state to load
    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<()>;
}

/// A workbench that supports streaming tool execution.
#[async_trait]
pub trait StreamWorkbench: Workbench {
    /// Call a tool with streaming output.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to call
    /// * `arguments` - The arguments to pass to the tool
    /// * `cancellation_token` - Optional token to cancel the operation
    /// * `call_id` - Optional identifier for this tool call
    ///
    /// # Returns
    /// A stream of intermediate results ending with the final result
    async fn call_tool_stream(
        &self,
        name: &str,
        arguments: Option<&HashMap<String, serde_json::Value>>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<Box<dyn Stream<Item = Result<WorkbenchStreamItem>> + Send + Unpin>>;
}

/// Items that can be yielded from a streaming workbench
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkbenchStreamItem {
    /// Intermediate result
    Intermediate(serde_json::Value),
    /// Final result
    Final(WorkbenchResult),
}

/// Helper trait for workbench context management
#[async_trait]
pub trait WorkbenchContext {
    /// Enter the workbench context (start it)
    async fn enter(&mut self) -> Result<()>;

    /// Exit the workbench context (stop it)
    async fn exit(&mut self) -> Result<()>;
}

#[async_trait]
impl<T: Workbench> WorkbenchContext for T {
    async fn enter(&mut self) -> Result<()> {
        self.start().await
    }

    async fn exit(&mut self) -> Result<()> {
        self.stop().await
    }
}
