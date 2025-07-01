//! Base tool traits and types.

use std::collections::HashMap;
use std::fmt::Debug;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use crate::{CancellationToken, error::Result};

/// Trait for tool arguments that can be validated and serialized
pub trait ToolArgs: Debug + Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Validate the arguments
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Get the JSON schema for these arguments
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

/// Trait for tool return values that can be serialized
pub trait ToolReturn: Debug + Serialize + Send + Sync + 'static {
    /// Convert the result to a string representation
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self))
    }
}

/// Empty arguments for tools that don't need parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoArgs;

impl ToolArgs for NoArgs {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
}

/// Simple text result for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResult {
    pub text: String,
}

impl ToolReturn for TextResult {}

/// Simple JSON result for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResult {
    pub data: serde_json::Value,
}

impl ToolReturn for JsonResult {}

/// Error result for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResult {
    pub error: String,
    pub code: Option<String>,
}

impl ToolReturn for ErrorResult {}

/// Tool schema for describing tool parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametersSchema {
    /// The type of the parameters (usually "object")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Properties of the parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    /// Required parameter names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Schema definition for a tool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSchema {
    /// The name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// Parameters schema for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ParametersSchema>,
    /// Whether the tool requires strict parameter validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Result of a tool execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// The name of the tool that was executed
    pub name: String,
    /// The result content
    pub result: Vec<ToolResultContent>,
    /// Whether the execution resulted in an error
    #[serde(default)]
    pub is_error: bool,
}

/// Content types for tool results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultContent {
    /// Text content
    #[serde(rename = "text")]
    Text { content: String },
    /// Image content
    #[serde(rename = "image")]
    Image { 
        /// Base64 encoded image data
        data: String,
        /// MIME type of the image
        mime_type: String,
    },
}

/// Event logged when a tool is called
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    /// The name of the tool that was called
    pub tool_name: String,
    /// The arguments passed to the tool
    pub arguments: HashMap<String, serde_json::Value>,
    /// The result of the tool call as a string
    pub result: String,
}

/// Type-safe tool trait for tools with specific argument and return types
#[async_trait]
pub trait TypedTool<Args: ToolArgs, Return: ToolReturn>: Send + Sync {
    /// Get the name of the tool
    fn name(&self) -> &str;

    /// Get the description of the tool
    fn description(&self) -> &str;

    /// Get the schema for this tool
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: Some(ParametersSchema {
                schema_type: "object".to_string(),
                properties: Some(HashMap::from([
                    ("args".to_string(), Args::schema())
                ])),
                required: Some(vec!["args".to_string()]),
            }),
            strict: Some(true),
        }
    }

    /// Execute the tool with typed arguments
    ///
    /// # Arguments
    /// * `args` - The typed arguments
    /// * `cancellation_token` - Token for cancelling the operation
    /// * `call_id` - Optional identifier for this tool call
    ///
    /// # Returns
    /// The typed result of the tool execution
    async fn execute(
        &self,
        args: Args,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<Return>;
}

/// Base trait for all tools (legacy, for backward compatibility)
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the name of the tool
    fn name(&self) -> &str;

    /// Get the description of the tool
    fn description(&self) -> &str;

    /// Get the schema for this tool
    fn schema(&self) -> ToolSchema;

    /// Get the return type information
    fn return_type(&self) -> String;

    /// Get the state type information (if any)
    fn state_type(&self) -> Option<String> {
        None
    }

    /// Convert a return value to its string representation
    fn return_value_as_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            _ => value.to_string(),
        }
    }

    /// Run the tool with JSON arguments
    ///
    /// # Arguments
    /// * `args` - The arguments as a JSON object
    /// * `cancellation_token` - Token for cancelling the operation
    /// * `call_id` - Optional identifier for this tool call
    ///
    /// # Returns
    /// The result of the tool execution
    async fn run_json(
        &self,
        args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<serde_json::Value>;

    /// Save the current state of the tool
    async fn save_state_json(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(HashMap::new())
    }

    /// Load state into the tool
    async fn load_state_json(&self, _state: &HashMap<String, serde_json::Value>) -> Result<()> {
        Ok(())
    }
}

/// Trait for tools that support streaming results
#[async_trait]
pub trait StreamTool: Tool {
    /// Run the tool with streaming output
    ///
    /// # Arguments
    /// * `args` - The arguments as a JSON object
    /// * `cancellation_token` - Token for cancelling the operation
    /// * `call_id` - Optional identifier for this tool call
    ///
    /// # Returns
    /// A stream of intermediate results ending with the final result
    async fn run_stream(
        &self,
        args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<Box<dyn Stream<Item = Result<serde_json::Value>> + Send + Unpin>>;
}

/// Base implementation for tools
#[derive(Debug)]
pub struct BaseTool {
    /// The name of the tool
    name: String,
    /// The description of the tool
    description: String,
    /// The schema for the tool
    schema: ToolSchema,
    /// Whether to use strict parameter validation
    strict: bool,
}

impl BaseTool {
    /// Create a new base tool
    pub fn new(
        name: String,
        description: String,
        parameters: Option<ParametersSchema>,
        strict: bool,
    ) -> Self {
        let schema = ToolSchema {
            name: name.clone(),
            description: description.clone(),
            parameters,
            strict: Some(strict),
        };

        Self {
            name,
            description,
            schema,
            strict,
        }
    }

    /// Get whether strict validation is enabled
    pub fn is_strict(&self) -> bool {
        self.strict
    }
}

#[async_trait]
impl Tool for BaseTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        self.schema.clone()
    }

    fn return_type(&self) -> String {
        "serde_json::Value".to_string()
    }

    async fn run_json(
        &self,
        _args: &HashMap<String, serde_json::Value>,
        _cancellation_token: Option<CancellationToken>,
        _call_id: Option<String>,
    ) -> Result<serde_json::Value> {
        // Base implementation - should be overridden
        Ok(serde_json::Value::Null)
    }
}

/// Base tool with state management
#[async_trait]
pub trait BaseToolWithState: Tool {
    /// The type of state this tool maintains
    type State: Serialize + for<'de> Deserialize<'de>;

    /// Save the current state
    fn save_state(&self) -> Self::State;

    /// Load state
    fn load_state(&mut self, state: Self::State) -> Result<()>;
}

/// Base streaming tool implementation
#[derive(Debug)]
pub struct BaseStreamTool {
    base: BaseTool,
}

impl BaseStreamTool {
    /// Create a new base stream tool
    pub fn new(
        name: String,
        description: String,
        parameters: Option<ParametersSchema>,
        strict: bool,
    ) -> Self {
        Self {
            base: BaseTool::new(name, description, parameters, strict),
        }
    }
}

#[async_trait]
impl Tool for BaseStreamTool {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn schema(&self) -> ToolSchema {
        self.base.schema()
    }

    fn return_type(&self) -> String {
        self.base.return_type()
    }

    async fn run_json(
        &self,
        args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<serde_json::Value> {
        self.base.run_json(args, cancellation_token, call_id).await
    }
}

#[async_trait]
impl StreamTool for BaseStreamTool {
    async fn run_stream(
        &self,
        _args: &HashMap<String, serde_json::Value>,
        _cancellation_token: Option<CancellationToken>,
        _call_id: Option<String>,
    ) -> Result<Box<dyn Stream<Item = Result<serde_json::Value>> + Send + Unpin>> {
        // Base implementation - should be overridden
        use futures::stream;
        Ok(Box::new(stream::empty()))
    }
}

/// Adapter to convert TypedTool to Tool for backward compatibility
pub struct TypedToolAdapter<Args: ToolArgs, Return: ToolReturn, T: TypedTool<Args, Return>> {
    inner: T,
    _phantom: std::marker::PhantomData<(Args, Return)>,
}

impl<Args: ToolArgs, Return: ToolReturn, T: TypedTool<Args, Return>> TypedToolAdapter<Args, Return, T> {
    /// Create a new adapter
    pub fn new(tool: T) -> Self {
        Self {
            inner: tool,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a reference to the inner typed tool
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner typed tool
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[async_trait]
impl<Args: ToolArgs, Return: ToolReturn, T: TypedTool<Args, Return>> Tool for TypedToolAdapter<Args, Return, T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn schema(&self) -> ToolSchema {
        self.inner.schema()
    }

    fn return_type(&self) -> String {
        std::any::type_name::<Return>().to_string()
    }

    async fn run_json(
        &self,
        args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
        call_id: Option<String>,
    ) -> Result<serde_json::Value> {
        // Try to deserialize the arguments
        let typed_args: Args = if let Some(args_value) = args.get("args") {
            serde_json::from_value(args_value.clone())
                .map_err(|e| crate::AutoGenError::invalid_arguments(
                    format!("tool_{}", self.name()),
                    format!("Failed to deserialize arguments: {}", e),
                    Some(serde_json::to_string(&Args::schema()).unwrap_or_default())
                ))?
        } else {
            return Err(crate::AutoGenError::invalid_arguments(
                format!("tool_{}", self.name()),
                "Missing 'args' field in arguments",
                Some(serde_json::to_string(&Args::schema()).unwrap_or_default())
            ));
        };

        // Validate arguments
        typed_args.validate()?;

        // Execute the tool
        let result = self.inner.execute(typed_args, cancellation_token, call_id).await?;

        // Serialize the result
        serde_json::to_value(result)
            .map_err(|e| crate::AutoGenError::tool(
                crate::error::ErrorContext::new("tool_execution")
                    .with_detail("tool_name", self.name())
                    .with_detail("operation", "serialize_result"),
                format!("Failed to serialize tool result: {}", e),
                Some(self.name().to_string())
            ))
    }
}
