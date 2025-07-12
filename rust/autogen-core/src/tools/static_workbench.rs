use super::base::{Tool, ToolSchema};
use super::workbench::{ResultContent, TextResultContent, ToolResult, Workbench};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct StaticWorkbenchConfig {
    // In Rust, we can't directly serialize a trait object.
    // We would typically use some form of component model or configuration
    // to reconstruct the tools. For now, this is a placeholder.
    #[serde(default)]
    pub tools: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StaticWorkbenchState {
    pub tools: HashMap<String, Value>,
}

pub struct StaticWorkbench {
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
}

impl StaticWorkbench {
    pub fn new(tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self { tools }
    }
}

#[async_trait]
impl Workbench for StaticWorkbench {
    async fn list_tools(&self) -> Result<Vec<ToolSchema>, Box<dyn Error + Send + Sync>> {
        Ok(self.tools.iter().map(|t| t.schema().clone()).collect())
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let tool = self.tools.iter().find(|t| t.name() == name);

        if let Some(tool) = tool {
            match tool.run(arguments).await {
                Ok(result) => {
                    let result_str = tool.return_value_as_string(&result);
                    Ok(ToolResult {
                        name: name.to_string(),
                        result: vec![ResultContent::TextResultContent(TextResultContent {
                            content: result_str,
                        })],
                        is_error: false,
                    })
                }
                Err(e) => Ok(ToolResult {
                    name: name.to_string(),
                    result: vec![ResultContent::TextResultContent(TextResultContent {
                        content: e.to_string(),
                    })],
                    is_error: true,
                }),
            }
        } else {
            Ok(ToolResult {
                name: name.to_string(),
                result: vec![ResultContent::TextResultContent(TextResultContent {
                    content: format!("Tool {} not found.", name),
                })],
                is_error: true,
            })
        }
    }

    async fn start(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error + Send + Sync>> {
        // State saving for tools is complex and depends on the tool's implementation.
        // For now, returning an empty state.
        Ok(HashMap::new())
    }

    async fn load_state(&mut self, _state: &HashMap<String, Value>) -> Result<(), Box<dyn Error + Send + Sync>> {
        // State loading for tools is complex.
        Ok(())
    }
}