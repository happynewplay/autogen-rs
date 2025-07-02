//! Tool agent implementation for executing function calls.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{
    agent::{Agent, AgentMetadata},
    agent_id::AgentId,
    error::Result,
    message::MessageContext,
    models::FunctionExecutionResult,
    tools::Tool,
    FunctionCall,
};

/// Base exception for tool-related errors
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("Tool error for call {call_id} on tool {name}: {content}")]
pub struct ToolException {
    /// The call ID that caused the error
    pub call_id: String,
    /// The error message content
    pub content: String,
    /// The name of the tool that caused the error
    pub name: String,
}

/// Exception for when a requested tool is not found
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("Tool not found for call {call_id} on tool {name}: {content}")]
pub struct ToolNotFoundException {
    /// The call ID that caused the error
    pub call_id: String,
    /// The error message content
    pub content: String,
    /// The name of the tool that was not found
    pub name: String,
}

/// Exception for invalid tool arguments
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("Invalid tool arguments for call {call_id} on tool {name}: {content}")]
pub struct InvalidToolArgumentsException {
    /// The call ID that caused the error
    pub call_id: String,
    /// The error message content
    pub content: String,
    /// The name of the tool with invalid arguments
    pub name: String,
}

/// Exception for tool execution errors
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("Tool execution error for call {call_id} on tool {name}: {content}")]
pub struct ToolExecutionException {
    /// The call ID that caused the error
    pub call_id: String,
    /// The error message content
    pub content: String,
    /// The name of the tool that failed to execute
    pub name: String,
}

/// A tool agent accepts direct messages of the type `FunctionCall`,
/// executes the requested tool with the provided arguments, and returns the
/// result as `FunctionExecutionResult` messages.
///
/// # Example
///
/// ```rust
/// use autogen_core::tool_agent::ToolAgent;
/// use autogen_core::tools::{FunctionTool, ParametersSchema, Tool};
/// use autogen_core::FunctionCall;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a simple tool
/// let add_tool = FunctionTool::new(
///     "add".to_string(),
///     "Add two numbers".to_string(),
///     Some(ParametersSchema {
///         schema_type: "object".to_string(),
///         properties: Some({
///             let mut props = HashMap::new();
///             props.insert("a".to_string(), json!({"type": "number"}));
///             props.insert("b".to_string(), json!({"type": "number"}));
///             props
///         }),
///         required: Some(vec!["a".to_string(), "b".to_string()]),
///     }),
///     Box::new(|args, _token| {
///         Box::pin(async move {
///             let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
///             let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
///             Ok(json!(a + b))
///         })
///     }),
/// );
///
/// // Create tool agent
/// let tools: Vec<Box<dyn Tool>> = vec![Box::new(add_tool)];
/// let agent = ToolAgent::new("Math tool agent".to_string(), tools);
///
/// // The agent can now handle FunctionCall messages
/// assert_eq!(agent.tools().len(), 1);
/// # Ok(())
/// # }
/// ```
pub struct ToolAgent {
    /// Description of the agent
    description: String,
    /// Tools available to this agent
    tools: Vec<Box<dyn Tool>>,
    /// Agent metadata
    metadata: AgentMetadata,
    /// Agent ID
    id: AgentId,
}

impl ToolAgent {
    /// Create a new tool agent
    ///
    /// # Arguments
    /// * `description` - Description of the agent
    /// * `tools` - List of tools that the agent can execute
    pub fn new(description: String, tools: Vec<Box<dyn Tool>>) -> Self {
        let metadata = AgentMetadata {
            description: Some(description.clone()),
            ..Default::default()
        };

        // Create a default agent ID - in practice this would be set by the runtime
        let id = AgentId::new("tool_agent", "default").expect("Failed to create agent ID");

        Self {
            description,
            tools,
            metadata,
            id,
        }
    }

    /// Get the tools available to this agent
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Add a tool to the agent
    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Remove a tool by name
    pub fn remove_tool(&mut self, name: &str) -> bool {
        if let Some(pos) = self.tools.iter().position(|tool| tool.name() == name) {
            self.tools.remove(pos);
            true
        } else {
            false
        }
    }

    /// Handle a function call message
    ///
    /// # Arguments
    /// * `message` - The function call message
    /// * `ctx` - The message context
    ///
    /// # Returns
    /// The result of the function execution
    pub async fn handle_function_call(
        &self,
        message: FunctionCall,
        ctx: MessageContext,
    ) -> Result<FunctionExecutionResult> {
        // Find the tool by name
        let tool = self.tools.iter()
            .find(|tool| tool.name() == message.name)
            .ok_or_else(|| {
                crate::error::AutoGenError::ToolNotFound {
                    tool_name: message.name.clone(),
                    available_tools: self.tools.iter().map(|t| t.name().to_string()).collect(),
                }
            })?;

        // Parse arguments
        let arguments: HashMap<String, serde_json::Value> = if message.arguments.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&message.arguments)
                .map_err(|e| crate::error::AutoGenError::InvalidArguments {
                    operation: format!("tool execution: {}", message.name),
                    reason: e.to_string(),
                    expected_schema: None,
                })?
        };

        // Execute the tool
        match tool.run_json(&arguments, Some(ctx.cancellation_token), Some(message.id.clone())).await {
            Ok(result) => {
                let result_as_str = tool.return_value_as_string(&result);
                Ok(FunctionExecutionResult {
                    content: result_as_str,
                    call_id: message.id,
                    is_error: Some(false),
                    name: message.name,
                })
            }
            Err(e) => {
                Ok(FunctionExecutionResult {
                    content: format!("Error: {}", e),
                    call_id: message.id,
                    is_error: Some(true),
                    name: message.name,
                })
            }
        }
    }
}

#[async_trait]
impl Agent for ToolAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn on_message(
        &mut self,
        message: Box<dyn std::any::Any + Send>,
        ctx: &MessageContext,
    ) -> Result<Option<Box<dyn std::any::Any + Send>>> {
        // Try to downcast to FunctionCall
        if let Ok(function_call) = message.downcast::<FunctionCall>() {
            let result = self.handle_function_call(*function_call, ctx.clone()).await?;
            return Ok(Some(Box::new(result)));
        }

        // If not a FunctionCall, return None (unhandled)
        Ok(None)
    }

    async fn on_start(&mut self) -> Result<()> {
        // Initialize any resources if needed
        Ok(())
    }

    async fn on_stop(&mut self) -> Result<()> {
        // Clean up any resources if needed
        Ok(())
    }
}
