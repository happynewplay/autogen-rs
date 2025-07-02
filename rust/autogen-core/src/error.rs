//! Error types for autogen-core
//!
//! This module defines the error types used throughout the autogen-core library.
//! All errors implement the standard Error trait and provide detailed context.



/// Result type alias for autogen-core operations
pub type Result<T> = std::result::Result<T, AutoGenError>;

/// Error context for providing additional information about errors
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation that was being performed when the error occurred
    pub operation: String,
    /// Additional context information
    pub details: std::collections::HashMap<String, String>,
    /// Timestamp when the error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            details: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add a detail to the context
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Main error type for autogen-core
#[derive(Debug, thiserror::Error)]
pub enum AutoGenError {
    /// Agent-related errors with detailed context
    #[error("Agent error in {context:?}: {source}")]
    Agent {
        /// Error context
        context: ErrorContext,
        /// Underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Runtime errors with detailed context
    #[error("Runtime error in {context:?}: {source}")]
    Runtime {
        /// Error context
        context: ErrorContext,
        /// Underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Message handling errors with detailed context
    #[error("Message error in {context:?}: {source}")]
    Message {
        /// Error context
        context: ErrorContext,
        /// Underlying error
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Serialization/deserialization errors
    #[error("Serialization error in {context:?}: {source}")]
    Serialization {
        /// Error context
        context: ErrorContext,
        /// Underlying serialization error
        #[source]
        source: serde_json::Error,
    },

    /// Network/HTTP errors
    #[cfg(feature = "http")]
    #[error("Network error in {context:?}: {source}")]
    Network {
        /// Error context
        context: ErrorContext,
        /// Underlying network error
        #[source]
        source: reqwest::Error,
    },

    /// I/O errors
    #[error("I/O error in {context:?}: {source}")]
    Io {
        /// Error context
        context: ErrorContext,
        /// Underlying I/O error
        #[source]
        source: std::io::Error,
    },

    /// Configuration errors with detailed context
    #[error("Configuration error in {context:?}: {message}")]
    Config {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
    },

    /// Tool execution errors with detailed context
    #[error("Tool error in {context:?}: {message}")]
    Tool {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
        /// Tool name that caused the error
        tool_name: Option<String>,
    },

    /// Model client errors with detailed context
    #[error("Model error in {context:?}: {message}")]
    Model {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
        /// Model name that caused the error
        model_name: Option<String>,
    },

    /// Cancellation errors
    #[error("Operation was cancelled")]
    Cancelled,

    /// Timeout errors
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds
        duration_ms: u64
    },

    /// Validation errors
    #[error("Validation error in {context:?}: {message}")]
    Validation {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String
    },

    /// Invalid agent type errors with validation details
    #[error("Invalid agent type '{agent_type}': {reason}")]
    InvalidAgentType {
        /// The invalid agent type
        agent_type: String,
        /// Reason why it's invalid
        reason: String,
        /// Expected format or pattern
        expected_format: Option<String>,
    },

    /// Invalid agent ID errors with validation details
    #[error("Invalid agent ID '{agent_id}': {reason}")]
    InvalidAgentId {
        /// The invalid agent ID
        agent_id: String,
        /// Reason why it's invalid
        reason: String,
    },

    /// Agent not found errors with search context
    #[error("Agent not found: {agent_id}")]
    AgentNotFound {
        /// The agent ID that was not found
        agent_id: crate::AgentId,
        /// Available agents (for debugging)
        available_agents: Vec<crate::AgentId>,
    },

    /// Message handling errors with type information
    #[error("Cannot handle message of type '{message_type}': {reason}")]
    CantHandle {
        /// The message type that couldn't be handled
        message_type: String,
        /// Reason why it can't be handled
        reason: String,
        /// Supported message types
        supported_types: Vec<String>,
    },

    /// Message dropped errors with routing information
    #[error("Message dropped during routing: {reason}")]
    MessageDropped {
        /// Reason why the message was dropped
        reason: String,
        /// Message type
        message_type: String,
        /// Intended recipient
        recipient: Option<crate::AgentId>,
    },

    /// Tool not found error with available tools
    #[error("Tool '{tool_name}' not found")]
    ToolNotFound {
        /// The tool name that was not found
        tool_name: String,
        /// Available tools
        available_tools: Vec<String>,
    },

    /// Invalid arguments error with validation details
    #[error("Invalid arguments for {operation}: {reason}")]
    InvalidArguments {
        /// The operation that received invalid arguments
        operation: String,
        /// Reason why arguments are invalid
        reason: String,
        /// Expected argument schema
        expected_schema: Option<String>,
    },

    /// Generic errors with context
    #[error("Error in {context:?}: {message}")]
    Other {
        /// Error context
        context: ErrorContext,
        /// Error message
        message: String,
    },
}

/// Simple error type for basic error messages (backward compatibility)
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct SimpleError {
    /// Error message
    pub message: String,
}

impl SimpleError {
    /// Create a new simple error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl AutoGenError {
    /// Create a new agent error with context
    pub fn agent(context: ErrorContext, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Agent {
            context,
            source: Box::new(source),
        }
    }

    /// Create a new runtime error with context
    pub fn runtime(context: ErrorContext, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Runtime {
            context,
            source: Box::new(source),
        }
    }

    /// Create a new message error with context
    pub fn message(context: ErrorContext, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message {
            context,
            source: Box::new(source),
        }
    }

    /// Create a new configuration error
    pub fn config(context: ErrorContext, message: impl Into<String>) -> Self {
        Self::Config {
            context,
            message: message.into(),
        }
    }

    /// Create a new tool error
    pub fn tool(context: ErrorContext, message: impl Into<String>, tool_name: Option<String>) -> Self {
        Self::Tool {
            context,
            message: message.into(),
            tool_name,
        }
    }

    /// Create a new model error
    pub fn model(context: ErrorContext, message: impl Into<String>, model_name: Option<String>) -> Self {
        Self::Model {
            context,
            message: message.into(),
            model_name,
        }
    }

    /// Create a new validation error
    pub fn validation(context: ErrorContext, message: impl Into<String>) -> Self {
        Self::Validation {
            context,
            message: message.into(),
        }
    }

    /// Create an agent not found error
    pub fn agent_not_found(agent_id: crate::AgentId, available_agents: Vec<crate::AgentId>) -> Self {
        Self::AgentNotFound {
            agent_id,
            available_agents,
        }
    }

    /// Create a tool not found error
    pub fn tool_not_found(tool_name: impl Into<String>, available_tools: Vec<String>) -> Self {
        Self::ToolNotFound {
            tool_name: tool_name.into(),
            available_tools,
        }
    }

    /// Create an invalid arguments error
    pub fn invalid_arguments(
        operation: impl Into<String>,
        reason: impl Into<String>,
        expected_schema: Option<String>
    ) -> Self {
        Self::InvalidArguments {
            operation: operation.into(),
            reason: reason.into(),
            expected_schema,
        }
    }

    /// Create a generic error with default context
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            context: ErrorContext::new("unknown"),
            message: message.into(),
        }
    }

    /// Create a simple agent not found error (backward compatibility)
    pub fn simple_agent_not_found(agent_id: impl Into<String>) -> Self {
        // For backward compatibility, create a dummy AgentId
        let agent_id_str = agent_id.into();
        match crate::AgentId::new("unknown", &agent_id_str) {
            Ok(id) => Self::agent_not_found(id, vec![]),
            Err(_) => Self::other(format!("Agent not found: {}", agent_id_str)),
        }
    }

    /// Create a simple tool not found error (backward compatibility)
    pub fn simple_tool_not_found(tool_name: impl Into<String>) -> Self {
        Self::tool_not_found(tool_name, vec![])
    }

    /// Create a simple invalid arguments error (backward compatibility)
    pub fn simple_invalid_arguments(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::invalid_arguments("unknown", &msg, None)
    }

    /// Create a new timeout error
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Check if this error is a cancellation
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Check if this error is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for AutoGenError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            context: ErrorContext::new("json_serialization"),
            source: err,
        }
    }
}
