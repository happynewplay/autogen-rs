use crate::models::types::{FunctionCall, FunctionExecutionResultMessage};
use crate::tools::base::Tool;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum ToolError {
    ToolNotFound(String),
    InvalidToolArguments(String),
    ToolExecutionError(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::ToolNotFound(s) => write!(f, "Tool not found: {}", s),
            ToolError::InvalidToolArguments(s) => write!(f, "Invalid tool arguments: {}", s),
            ToolError::ToolExecutionError(s) => write!(f, "Tool execution error: {}", s),
        }
    }
}

impl Error for ToolError {}

pub struct ToolAgent {
    description: String,
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
}

impl ToolAgent {
    pub fn new(description: String, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self { description, tools }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn tools(&self) -> &[Arc<dyn Tool + Send + Sync>] {
        &self.tools
    }

    pub async fn handle_function_call(
        &self,
        message: &FunctionCall,
    ) -> Result<FunctionExecutionResultMessage, ToolError> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == message.name)
            .ok_or_else(|| ToolError::ToolNotFound(message.name.clone()))?;

        let arguments: serde_json::Value = serde_json::from_str(&message.arguments)
            .map_err(|e| ToolError::InvalidToolArguments(e.to_string()))?;

        let result = tool
            .run_json(&arguments, &message.id)
            .await
            .map_err(|e| ToolError::ToolExecutionError(e.to_string()))?;

        let result_as_str = tool.return_value_as_string(&result);

        Ok(FunctionExecutionResultMessage {
            content: result_as_str,
            function_name: message.name.clone(),
            source: None,
        })
    }
}