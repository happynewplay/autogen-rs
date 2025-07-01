//! Static workbench implementation with a fixed set of tools.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::{CancellationToken, error::Result};
use super::{
    base_tool::{Tool, ToolSchema},
    workbench::{Workbench, StreamWorkbench, WorkbenchResult, WorkbenchStreamItem},
};

/// Configuration for StaticWorkbench
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticWorkbenchConfig {
    /// List of tools in the workbench
    #[serde(default)]
    pub tools: Vec<String>, // In a real implementation, this would be ComponentModel
}

impl Default for StaticWorkbenchConfig {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
        }
    }
}

/// State for StaticWorkbench
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticWorkbenchState {
    /// The type identifier
    #[serde(rename = "type")]
    pub state_type: String,
    /// Tool states
    #[serde(default)]
    pub tools: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl Default for StaticWorkbenchState {
    fn default() -> Self {
        Self {
            state_type: "StaticWorkbenchState".to_string(),
            tools: HashMap::new(),
        }
    }
}

/// A workbench that provides a static set of tools that do not change after
/// each tool execution.
///
/// This workbench is suitable for scenarios where you have a fixed set of tools
/// that don't need to be dynamically added or removed during execution.
///
/// # Example
///
/// ```rust
/// use autogen_core::tools::{StaticWorkbench, FunctionTool, Workbench, ParametersSchema};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create some tools
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
/// // Create workbench with tools
/// let tools: Vec<Box<dyn Tool>> = vec![Box::new(add_tool)];
/// let mut workbench = StaticWorkbench::new(tools);
///
/// // Start the workbench
/// workbench.start().await?;
///
/// // List available tools
/// let tool_schemas = workbench.list_tools().await?;
/// assert_eq!(tool_schemas.len(), 1);
/// assert_eq!(tool_schemas[0].name, "add");
/// # Ok(())
/// # }
/// ```
pub struct StaticWorkbench {
    /// The tools in this workbench
    tools: Arc<RwLock<Vec<Box<dyn Tool>>>>,
    /// Whether the workbench is started
    started: Arc<RwLock<bool>>,
}

impl StaticWorkbench {
    /// Create a new static workbench with the given tools
    ///
    /// # Arguments
    /// * `tools` - A vector of tools to include in the workbench
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        Self {
            tools: Arc::new(RwLock::new(tools)),
            started: Arc::new(RwLock::new(false)),
        }
    }

    /// Create an empty static workbench
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Add a tool to the workbench
    ///
    /// # Arguments
    /// * `tool` - The tool to add
    pub async fn add_tool(&self, tool: Box<dyn Tool>) {
        let mut tools = self.tools.write().await;
        tools.push(tool);
    }

    /// Remove a tool from the workbench by name
    ///
    /// # Arguments
    /// * `name` - The name of the tool to remove
    ///
    /// # Returns
    /// True if the tool was found and removed, false otherwise
    pub async fn remove_tool(&self, name: &str) -> bool {
        let mut tools = self.tools.write().await;
        if let Some(pos) = tools.iter().position(|tool| tool.name() == name) {
            tools.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get the number of tools in the workbench
    pub async fn tool_count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Check if the workbench is started
    pub async fn is_started(&self) -> bool {
        *self.started.read().await
    }

    /// Create from configuration
    pub fn from_config(_config: StaticWorkbenchConfig) -> Self {
        // In a real implementation, this would load tools from the configuration
        Self::empty()
    }

    /// Convert to configuration
    pub async fn to_config(&self) -> StaticWorkbenchConfig {
        let tools = self.tools.read().await;
        StaticWorkbenchConfig {
            tools: tools.iter().map(|tool| tool.name().to_string()).collect(),
        }
    }
}

#[async_trait]
impl Workbench for StaticWorkbench {
    async fn list_tools(&self) -> Result<Vec<ToolSchema>> {
        let tools = self.tools.read().await;
        Ok(tools.iter().map(|tool| tool.schema()).collect())
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<&HashMap<String, serde_json::Value>>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<WorkbenchResult> {
        let tools = self.tools.read().await;
        
        // Find the tool by name
        let tool = tools.iter()
            .find(|tool| tool.name() == name)
            .ok_or_else(|| crate::error::AutoGenError::ToolNotFound(name.to_string()))?;

        // Execute the tool
        let args = arguments.cloned().unwrap_or_default();
        match tool.run_json(&args, cancellation_token, call_id).await {
            Ok(result) => {
                let content = tool.return_value_as_string(&result);
                Ok(WorkbenchResult::text(name.to_string(), content))
            }
            Err(e) => {
                Ok(WorkbenchResult::error(name.to_string(), e.to_string()))
            }
        }
    }

    async fn start(&mut self) -> Result<()> {
        let mut started = self.started.write().await;
        *started = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut started = self.started.write().await;
        *started = false;
        Ok(())
    }

    async fn reset(&mut self) -> Result<()> {
        // For static workbench, reset just means ensuring it's started
        self.start().await
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        let tools = self.tools.read().await;
        let mut tool_states = HashMap::new();
        
        for tool in tools.iter() {
            let state = tool.save_state_json().await?;
            if !state.is_empty() {
                tool_states.insert(tool.name().to_string(), state);
            }
        }

        let state = StaticWorkbenchState {
            state_type: "StaticWorkbenchState".to_string(),
            tools: tool_states,
        };

        let mut result = HashMap::new();
        result.insert("workbench_state".to_string(), serde_json::to_value(state)?);
        Ok(result)
    }

    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(state_value) = state.get("workbench_state") {
            let workbench_state: StaticWorkbenchState = serde_json::from_value(state_value.clone())?;
            
            let tools = self.tools.read().await;
            for tool in tools.iter() {
                if let Some(tool_state) = workbench_state.tools.get(tool.name()) {
                    tool.load_state_json(tool_state).await?;
                }
            }
        }
        Ok(())
    }
}

/// A static workbench that supports streaming tool execution.
pub struct StaticStreamWorkbench {
    base: StaticWorkbench,
}

impl StaticStreamWorkbench {
    /// Create a new static stream workbench
    pub fn new(tools: Vec<Box<dyn Tool>>) -> Self {
        Self {
            base: StaticWorkbench::new(tools),
        }
    }

    /// Create an empty static stream workbench
    pub fn empty() -> Self {
        Self {
            base: StaticWorkbench::empty(),
        }
    }
}

#[async_trait]
impl Workbench for StaticStreamWorkbench {
    async fn list_tools(&self) -> Result<Vec<ToolSchema>> {
        self.base.list_tools().await
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<&HashMap<String, serde_json::Value>>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<WorkbenchResult> {
        self.base.call_tool(name, arguments, cancellation_token, call_id).await
    }

    async fn start(&mut self) -> Result<()> {
        self.base.start().await
    }

    async fn stop(&mut self) -> Result<()> {
        self.base.stop().await
    }

    async fn reset(&mut self) -> Result<()> {
        self.base.reset().await
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        self.base.save_state().await
    }

    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<()> {
        self.base.load_state(state).await
    }
}

#[async_trait]
impl StreamWorkbench for StaticStreamWorkbench {
    async fn call_tool_stream(
        &self,
        name: &str,
        arguments: Option<&HashMap<String, serde_json::Value>>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<Box<dyn Stream<Item = Result<WorkbenchStreamItem>> + Send + Unpin>> {
        // For now, just call the regular tool and return the result as a stream
        // In a real implementation, this would check if the tool supports streaming
        let result = self.call_tool(name, arguments, cancellation_token, call_id).await?;
        
        use futures::stream;
        let stream = stream::once(async move {
            Ok(WorkbenchStreamItem::Final(result))
        });
        Ok(Box::new(Box::pin(stream)))
    }
}
