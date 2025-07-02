//! Function tool implementation for wrapping Rust functions as tools.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{CancellationToken, error::Result};
use super::base_tool::{Tool, ToolSchema, ParametersSchema};

/// Configuration for a function tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionToolConfig {
    /// The source code of the function
    pub source_code: String,
    /// The name of the function
    pub name: String,
    /// The description of the function
    pub description: String,
    /// Whether the function supports cancellation
    pub has_cancellation_support: bool,
}

/// Type alias for async function that can be called by the tool
pub type AsyncToolFunction = Box<
    dyn Fn(
        HashMap<String, serde_json::Value>,
        Option<CancellationToken>,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>>
    + Send
    + Sync,
>;

/// Create custom tools by wrapping Rust functions.
///
/// `FunctionTool` offers an interface for executing Rust functions asynchronously.
/// Functions can be provided as closures or function pointers, and the tool will
/// handle argument validation and execution.
///
/// # Example
///
/// ```rust
/// use autogen_core::tools::{FunctionTool, ParametersSchema, ToolSchema, Tool};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a simple function tool
/// let add_tool = FunctionTool::new(
///     "add".to_string(),
///     "Add two numbers together".to_string(),
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
/// // Use the tool
/// let mut args = HashMap::new();
/// args.insert("a".to_string(), json!(5));
/// args.insert("b".to_string(), json!(3));
/// 
/// let result = add_tool.run_json(&args, None, None).await?;
/// assert_eq!(result, json!(8));
/// # Ok(())
/// # }
/// ```
pub struct FunctionTool {
    /// The name of the tool
    name: String,
    /// The description of the tool
    description: String,
    /// The schema for the tool parameters
    schema: ToolSchema,
    /// The function to execute
    function: AsyncToolFunction,
    /// Whether the function supports cancellation
    has_cancellation_support: bool,
}

impl FunctionTool {
    /// Create a new function tool
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `description` - Description of what the tool does
    /// * `parameters` - Schema for the tool parameters
    /// * `function` - The async function to execute
    pub fn new(
        name: String,
        description: String,
        parameters: Option<ParametersSchema>,
        function: AsyncToolFunction,
    ) -> Self {
        let schema = ToolSchema {
            name: name.clone(),
            description: description.clone(),
            parameters,
            strict: Some(false),
        };

        Self {
            name,
            description,
            schema,
            function,
            has_cancellation_support: false,
        }
    }

    /// Create a new function tool with cancellation support
    ///
    /// # Arguments
    /// * `name` - The name of the tool
    /// * `description` - Description of what the tool does
    /// * `parameters` - Schema for the tool parameters
    /// * `function` - The async function to execute
    pub fn new_with_cancellation(
        name: String,
        description: String,
        parameters: Option<ParametersSchema>,
        function: AsyncToolFunction,
    ) -> Self {
        let schema = ToolSchema {
            name: name.clone(),
            description: description.clone(),
            parameters,
            strict: Some(false),
        };

        Self {
            name,
            description,
            schema,
            function,
            has_cancellation_support: true,
        }
    }

    /// Create from configuration
    pub fn from_config(config: FunctionToolConfig) -> Result<Self> {
        // In a real implementation, this would compile and load the function from source code
        // For now, we'll create a placeholder function
        let function: AsyncToolFunction = Box::new(|_args, _token| {
            Box::pin(async move {
                Ok(serde_json::Value::String("Function not implemented".to_string()))
            })
        });

        let name = config.name.clone();
        let description = config.description.clone();

        Ok(Self {
            name: name.clone(),
            description: description.clone(),
            schema: ToolSchema {
                name,
                description,
                parameters: None,
                strict: Some(false),
            },
            function,
            has_cancellation_support: config.has_cancellation_support,
        })
    }

    /// Convert to configuration
    pub fn to_config(&self) -> FunctionToolConfig {
        FunctionToolConfig {
            source_code: "// Source code not available".to_string(),
            name: self.name.clone(),
            description: self.description.clone(),
            has_cancellation_support: self.has_cancellation_support,
        }
    }

    /// Check if the function supports cancellation
    pub fn has_cancellation_support(&self) -> bool {
        self.has_cancellation_support
    }
}

#[async_trait]
impl Tool for FunctionTool {
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
        args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
        _call_id: Option<String>,
    ) -> Result<serde_json::Value> {
        // Execute the function
        let result = (self.function)(args.clone(), cancellation_token).await?;
        
        // Log the tool call event
        let event = super::base_tool::ToolCallEvent {
            tool_name: self.name.clone(),
            arguments: args.clone(),
            result: self.return_value_as_string(&result),
        };
        
        // In a real implementation, this would use the logging system
        #[cfg(feature = "tracing")]
        tracing::info!("Tool call: {:?}", event);
        #[cfg(not(feature = "tracing"))]
        let _ = event; // Suppress unused variable warning
        
        Ok(result)
    }
}

/// Helper macro for creating function tools from closures
#[macro_export]
macro_rules! function_tool {
    ($name:expr, $description:expr, $params:expr, $func:expr) => {
        FunctionTool::new(
            $name.to_string(),
            $description.to_string(),
            $params,
            Box::new($func),
        )
    };
}

/// Helper function to create a simple function tool from a closure
pub fn create_function_tool<F, Fut>(
    name: &str,
    description: &str,
    parameters: Option<ParametersSchema>,
    func: F,
) -> FunctionTool
where
    F: Fn(HashMap<String, serde_json::Value>, Option<CancellationToken>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
{
    FunctionTool::new(
        name.to_string(),
        description.to_string(),
        parameters,
        Box::new(move |args, token| Box::pin(func(args, token))),
    )
}
