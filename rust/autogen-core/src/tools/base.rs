use async_trait::async_trait;
use serde_json::Value;
use serde::{Serialize, de::DeserializeOwned};
use std::error::Error;
use std::any::TypeId;
use futures::Stream;
use std::pin::Pin;
use thiserror::Error;
use crate::cancellation_token::CancellationToken;
use jsonschema::{JSONSchema, ValidationError};
use std::collections::HashMap;
use tracing::{info, warn, error};
use crate::logging::ToolCallEvent;

/// Errors that can occur during tool operations
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Cancellation error: {0}")]
    CancellationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Schema error: {0}")]
    SchemaError(String),
}

/// Enhanced tool schema with more metadata
#[derive(Debug)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub strict: Option<bool>,
    pub returns: Option<Value>,
    /// Compiled JSON schema for validation
    compiled_schema: Option<JSONSchema>,
}

impl Clone for ToolSchema {
    fn clone(&self) -> Self {
        // JSONSchema doesn't implement Clone, so we recompile it
        let compiled_schema = JSONSchema::compile(&self.parameters).ok();
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
            strict: self.strict,
            returns: self.returns.clone(),
            compiled_schema,
        }
    }
}

impl ToolSchema {
    pub fn new(name: String, description: String, parameters: Value) -> Self {
        let compiled_schema = JSONSchema::compile(&parameters).ok();
        Self {
            name,
            description,
            parameters,
            strict: None,
            returns: None,
            compiled_schema,
        }
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }

    pub fn with_returns(mut self, returns: Value) -> Self {
        self.returns = Some(returns);
        self
    }

    /// Validate arguments against the schema
    pub fn validate_args(&self, args: &Value) -> Result<(), ToolError> {
        if let Some(ref schema) = self.compiled_schema {
            let result = schema.validate(args);
            if let Err(errors) = result {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                return Err(ToolError::ValidationError(error_messages.join("; ")));
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> &ToolSchema;

    /// Get the argument types for this tool
    fn args_type(&self) -> TypeId {
        TypeId::of::<Value>() // Default to JSON Value
    }

    /// Get the return type for this tool
    fn return_type(&self) -> TypeId {
        TypeId::of::<Value>() // Default to JSON Value
    }

    /// Get the state type for this tool (if it has state)
    fn state_type(&self) -> Option<TypeId> {
        None // Default: no state
    }

    async fn run(&self, args: &Value) -> Result<Value, Box<dyn Error + Send + Sync>>;

    /// Run with cancellation token support
    async fn run_with_cancellation(
        &self,
        args: &Value,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<Value, ToolError> {
        // Check cancellation before starting
        if let Some(ref token) = cancellation_token {
            token.check_cancelled().map_err(|_| ToolError::CancellationError("Operation was cancelled".to_string()))?;
        }

        // Run the tool
        let result = self.run(args).await.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Check cancellation after completion
        if let Some(ref token) = cancellation_token {
            token.check_cancelled().map_err(|_| ToolError::CancellationError("Operation was cancelled".to_string()))?;
        }

        Ok(result)
    }

    fn return_value_as_string(&self, value: &Value) -> String {
        value.to_string()
    }

    async fn run_json(
        &self,
        args: &Value,
        call_id: &str,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Validate arguments against schema
        self.schema().validate_args(args)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        info!("Running tool '{}' with call_id: {}", self.name(), call_id);

        let result = self.run(args).await?;

        // Log the tool call event
        let mut args_map = HashMap::new();
        if let Value::Object(obj) = args {
            for (k, v) in obj {
                args_map.insert(k.clone(), v.clone());
            }
        }

        let event = ToolCallEvent::new(
            self.name().to_string(),
            serde_json::Map::from_iter(args_map),
            self.return_value_as_string(&result),
        );

        info!("{}", event);

        Ok(result)
    }
}

/// Trait for tools that have state
#[async_trait]
pub trait ToolWithState<S>: Tool
where
    S: Serialize + DeserializeOwned + Send + 'static,
{
    /// Save the current state of the tool
    async fn save_state(&self) -> Result<S, ToolError>;

    /// Load state into the tool
    async fn load_state(&mut self, state: S) -> Result<(), ToolError>;

    /// Save state as JSON
    async fn save_state_json(&self) -> Result<Value, ToolError> {
        let state = self.save_state().await?;
        serde_json::to_value(state).map_err(|e| ToolError::StateError(e.to_string()))
    }

    /// Load state from JSON
    async fn load_state_json(&mut self, state: Value) -> Result<(), ToolError> {
        let typed_state: S = serde_json::from_value(state)
            .map_err(|e| ToolError::StateError(e.to_string()))?;
        self.load_state(typed_state).await
    }
}

/// Trait for streaming tools (equivalent to Python's StreamTool)
#[async_trait]
pub trait StreamTool: Tool {
    type StreamItem: Send + 'static;

    /// Run the tool and return a stream of results
    fn run_stream(
        &self,
        args: &Value,
    ) -> Pin<Box<dyn Stream<Item = Result<Self::StreamItem, ToolError>> + Send>>;

    /// Run the tool with cancellation support and return a stream
    fn run_stream_with_cancellation(
        &self,
        args: &Value,
        cancellation_token: Option<CancellationToken>,
    ) -> Pin<Box<dyn Stream<Item = Result<Self::StreamItem, ToolError>> + Send>>;
}

/// Base implementation for tools (equivalent to Python's BaseTool)
pub struct BaseTool {
    name: String,
    description: String,
    schema: ToolSchema,
}

impl BaseTool {
    pub fn new(name: String, description: String, parameters: Value) -> Self {
        let schema = ToolSchema::new(name.clone(), description.clone(), parameters);

        Self {
            name,
            description,
            schema,
        }
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.schema = self.schema.with_strict(strict);
        self
    }

    pub fn with_returns(mut self, returns: Value) -> Self {
        self.schema = self.schema.with_returns(returns);
        self
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

    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn run(&self, _args: &Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        error!("BaseTool.run() called but not implemented for tool '{}'", self.name);
        Err(Box::new(ToolError::ExecutionFailed(
            "BaseTool.run() must be implemented by subclasses".to_string()
        )))
    }
}