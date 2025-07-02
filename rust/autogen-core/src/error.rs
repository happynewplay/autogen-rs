//! Comprehensive error handling for autogen-core

use std::fmt;

/// Result type alias for autogen operations
pub type Result<T> = std::result::Result<T, AutoGenError>;

/// Main error type for the autogen-core library
#[derive(Debug)]
pub enum AutoGenError {
    /// Agent-related errors
    Agent(AgentError),
    /// Message-related errors
    Message(MessageError),
    /// Runtime-related errors
    Runtime(RuntimeError),
    /// State management errors
    State(StateError),
    /// Cache-related errors
    Cache(CacheError),
    /// Serialization/deserialization errors
    Serialization(SerializationError),
    /// Network/communication errors
    Network(NetworkError),
    /// Configuration errors
    Config(ConfigError),
    /// Validation errors
    Validation(ValidationError),
    /// I/O errors
    Io(std::io::Error),
    /// Generic errors with context
    Other {
        /// Error message
        message: String,
        /// Optional source error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Agent-specific errors
#[derive(Debug)]
pub enum AgentError {
    /// Agent not found
    NotFound {
        /// Agent identifier
        agent_id: String
    },
    /// Agent already exists
    AlreadyExists {
        /// Agent identifier
        agent_id: String
    },
    /// Agent is not running
    NotRunning {
        /// Agent identifier
        agent_id: String
    },
    /// Agent is already running
    AlreadyRunning {
        /// Agent identifier
        agent_id: String
    },
    /// Agent initialization failed
    InitializationFailed {
        /// Agent identifier
        agent_id: String,
        /// Failure reason
        reason: String
    },
    /// Agent shutdown failed
    ShutdownFailed {
        /// Agent identifier
        agent_id: String,
        /// Failure reason
        reason: String
    },
    /// Invalid agent configuration
    InvalidConfig {
        /// Agent identifier
        agent_id: String,
        /// Configuration details
        details: String
    },
    /// Agent factory not found
    FactoryNotFound {
        /// Agent type
        agent_type: String
    },
}

/// Message-related errors
#[derive(Debug)]
pub enum MessageError {
    /// Invalid message format
    InvalidFormat { details: String },
    /// Message too large
    TooLarge { size: usize, max_size: usize },
    /// Message routing failed
    RoutingFailed { target: String, reason: String },
    /// Message handler not found
    HandlerNotFound { message_type: String },
    /// Message processing timeout
    Timeout { timeout_ms: u64 },
    /// Message validation failed
    ValidationFailed { details: String },
}

/// Runtime-related errors
#[derive(Debug)]
pub enum RuntimeError {
    /// Runtime not initialized
    NotInitialized,
    /// Runtime already initialized
    AlreadyInitialized,
    /// Runtime shutdown in progress
    ShuttingDown,
    /// Resource exhaustion
    ResourceExhausted { resource: String, details: String },
    /// Concurrency limit exceeded
    ConcurrencyLimitExceeded { limit: usize },
    /// Subscription failed
    SubscriptionFailed { topic: String, reason: String },
}

/// State management errors
#[derive(Debug)]
pub enum StateError {
    /// State not found
    NotFound { agent_id: String },
    /// State corruption detected
    Corrupted { agent_id: String, details: String },
    /// State version mismatch
    VersionMismatch { expected: u32, found: u32 },
    /// State persistence failed
    PersistenceFailed { reason: String },
    /// State loading failed
    LoadingFailed { reason: String },
}

/// Cache-related errors
#[derive(Debug)]
pub enum CacheError {
    /// Cache miss (not really an error, but useful for flow control)
    Miss { key: String },
    /// Cache write failed
    WriteFailed { key: String, reason: String },
    /// Cache read failed
    ReadFailed { key: String, reason: String },
    /// Cache eviction failed
    EvictionFailed { reason: String },
    /// Cache capacity exceeded
    CapacityExceeded { current: usize, max: usize },
}

/// Serialization/deserialization errors
#[derive(Debug)]
pub enum SerializationError {
    /// JSON serialization failed
    JsonSerialization { details: String },
    /// JSON deserialization failed
    JsonDeserialization { details: String },
    /// Binary serialization failed
    BinarySerialization { details: String },
    /// Binary deserialization failed
    BinaryDeserialization { details: String },
    /// Unsupported format
    UnsupportedFormat { format: String },
}

/// Network/communication errors
#[derive(Debug)]
pub enum NetworkError {
    /// Connection failed
    ConnectionFailed { endpoint: String, reason: String },
    /// Connection timeout
    ConnectionTimeout { endpoint: String, timeout_ms: u64 },
    /// Request failed
    RequestFailed { details: String },
    /// Response parsing failed
    ResponseParsingFailed { details: String },
    /// Network unreachable
    NetworkUnreachable { endpoint: String },
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    /// Missing required configuration
    MissingRequired { key: String },
    /// Invalid configuration value
    InvalidValue { key: String, value: String, expected: String },
    /// Configuration file not found
    FileNotFound { path: String },
    /// Configuration parsing failed
    ParsingFailed { path: String, reason: String },
}

/// Validation errors
#[derive(Debug)]
pub enum ValidationError {
    /// Required field missing
    RequiredFieldMissing { field: String },
    /// Invalid field value
    InvalidFieldValue { field: String, value: String, reason: String },
    /// Field value out of range
    OutOfRange { field: String, value: String, min: String, max: String },
    /// Invalid format
    InvalidFormat { field: String, value: String, expected_format: String },
}

impl fmt::Display for AutoGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutoGenError::Agent(err) => write!(f, "Agent error: {}", err),
            AutoGenError::Message(err) => write!(f, "Message error: {}", err),
            AutoGenError::Runtime(err) => write!(f, "Runtime error: {}", err),
            AutoGenError::State(err) => write!(f, "State error: {}", err),
            AutoGenError::Cache(err) => write!(f, "Cache error: {}", err),
            AutoGenError::Serialization(err) => write!(f, "Serialization error: {}", err),
            AutoGenError::Network(err) => write!(f, "Network error: {}", err),
            AutoGenError::Config(err) => write!(f, "Configuration error: {}", err),
            AutoGenError::Validation(err) => write!(f, "Validation error: {}", err),
            AutoGenError::Io(err) => write!(f, "I/O error: {}", err),
            AutoGenError::Other { message, .. } => write!(f, "{}", message),
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::NotFound { agent_id } => write!(f, "Agent '{}' not found", agent_id),
            AgentError::AlreadyExists { agent_id } => write!(f, "Agent '{}' already exists", agent_id),
            AgentError::NotRunning { agent_id } => write!(f, "Agent '{}' is not running", agent_id),
            AgentError::AlreadyRunning { agent_id } => write!(f, "Agent '{}' is already running", agent_id),
            AgentError::InitializationFailed { agent_id, reason } => {
                write!(f, "Failed to initialize agent '{}': {}", agent_id, reason)
            }
            AgentError::ShutdownFailed { agent_id, reason } => {
                write!(f, "Failed to shutdown agent '{}': {}", agent_id, reason)
            }
            AgentError::InvalidConfig { agent_id, details } => {
                write!(f, "Invalid configuration for agent '{}': {}", agent_id, details)
            }
            AgentError::FactoryNotFound { agent_type } => {
                write!(f, "No factory found for agent type '{}'", agent_type)
            }
        }
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::InvalidFormat { details } => write!(f, "Invalid message format: {}", details),
            MessageError::TooLarge { size, max_size } => {
                write!(f, "Message too large: {} bytes (max: {} bytes)", size, max_size)
            }
            MessageError::RoutingFailed { target, reason } => {
                write!(f, "Failed to route message to '{}': {}", target, reason)
            }
            MessageError::HandlerNotFound { message_type } => {
                write!(f, "No handler found for message type '{}'", message_type)
            }
            MessageError::Timeout { timeout_ms } => {
                write!(f, "Message processing timeout after {} ms", timeout_ms)
            }
            MessageError::ValidationFailed { details } => {
                write!(f, "Message validation failed: {}", details)
            }
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::NotInitialized => write!(f, "Runtime not initialized"),
            RuntimeError::AlreadyInitialized => write!(f, "Runtime already initialized"),
            RuntimeError::ShuttingDown => write!(f, "Runtime is shutting down"),
            RuntimeError::ResourceExhausted { resource, details } => {
                write!(f, "Resource '{}' exhausted: {}", resource, details)
            }
            RuntimeError::ConcurrencyLimitExceeded { limit } => {
                write!(f, "Concurrency limit exceeded (max: {})", limit)
            }
            RuntimeError::SubscriptionFailed { topic, reason } => {
                write!(f, "Failed to subscribe to topic '{}': {}", topic, reason)
            }
        }
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::NotFound { agent_id } => write!(f, "State not found for agent '{}'", agent_id),
            StateError::Corrupted { agent_id, details } => {
                write!(f, "State corrupted for agent '{}': {}", agent_id, details)
            }
            StateError::VersionMismatch { expected, found } => {
                write!(f, "State version mismatch: expected {}, found {}", expected, found)
            }
            StateError::PersistenceFailed { reason } => write!(f, "State persistence failed: {}", reason),
            StateError::LoadingFailed { reason } => write!(f, "State loading failed: {}", reason),
        }
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::Miss { key } => write!(f, "Cache miss for key '{}'", key),
            CacheError::WriteFailed { key, reason } => {
                write!(f, "Failed to write cache key '{}': {}", key, reason)
            }
            CacheError::ReadFailed { key, reason } => {
                write!(f, "Failed to read cache key '{}': {}", key, reason)
            }
            CacheError::EvictionFailed { reason } => write!(f, "Cache eviction failed: {}", reason),
            CacheError::CapacityExceeded { current, max } => {
                write!(f, "Cache capacity exceeded: {} items (max: {})", current, max)
            }
        }
    }
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::JsonSerialization { details } => {
                write!(f, "JSON serialization failed: {}", details)
            }
            SerializationError::JsonDeserialization { details } => {
                write!(f, "JSON deserialization failed: {}", details)
            }
            SerializationError::BinarySerialization { details } => {
                write!(f, "Binary serialization failed: {}", details)
            }
            SerializationError::BinaryDeserialization { details } => {
                write!(f, "Binary deserialization failed: {}", details)
            }
            SerializationError::UnsupportedFormat { format } => {
                write!(f, "Unsupported serialization format: {}", format)
            }
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed { endpoint, reason } => {
                write!(f, "Connection to '{}' failed: {}", endpoint, reason)
            }
            NetworkError::ConnectionTimeout { endpoint, timeout_ms } => {
                write!(f, "Connection to '{}' timed out after {} ms", endpoint, timeout_ms)
            }
            NetworkError::RequestFailed { details } => write!(f, "Request failed: {}", details),
            NetworkError::ResponseParsingFailed { details } => {
                write!(f, "Response parsing failed: {}", details)
            }
            NetworkError::NetworkUnreachable { endpoint } => {
                write!(f, "Network unreachable: {}", endpoint)
            }
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingRequired { key } => write!(f, "Missing required configuration: {}", key),
            ConfigError::InvalidValue { key, value, expected } => {
                write!(f, "Invalid value for '{}': '{}' (expected: {})", key, value, expected)
            }
            ConfigError::FileNotFound { path } => write!(f, "Configuration file not found: {}", path),
            ConfigError::ParsingFailed { path, reason } => {
                write!(f, "Failed to parse configuration file '{}': {}", path, reason)
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::RequiredFieldMissing { field } => {
                write!(f, "Required field missing: {}", field)
            }
            ValidationError::InvalidFieldValue { field, value, reason } => {
                write!(f, "Invalid value for field '{}': '{}' ({})", field, value, reason)
            }
            ValidationError::OutOfRange { field, value, min, max } => {
                write!(f, "Value for field '{}' out of range: '{}' (expected: {} to {})", field, value, min, max)
            }
            ValidationError::InvalidFormat { field, value, expected_format } => {
                write!(f, "Invalid format for field '{}': '{}' (expected: {})", field, value, expected_format)
            }
        }
    }
}

impl std::error::Error for AutoGenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AutoGenError::Io(err) => Some(err),
            AutoGenError::Other { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
            _ => None,
        }
    }
}

impl From<std::io::Error> for AutoGenError {
    fn from(err: std::io::Error) -> Self {
        AutoGenError::Io(err)
    }
}

impl From<serde_json::Error> for AutoGenError {
    fn from(err: serde_json::Error) -> Self {
        AutoGenError::Serialization(SerializationError::JsonSerialization {
            details: err.to_string(),
        })
    }
}

impl AutoGenError {
    /// Create a generic error with a message
    pub fn other<S: Into<String>>(message: S) -> Self {
        AutoGenError::Other {
            message: message.into(),
            source: None,
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            AutoGenError::Network(_) => true,
            AutoGenError::Cache(CacheError::Miss { .. }) => true,
            AutoGenError::Runtime(RuntimeError::ConcurrencyLimitExceeded { .. }) => true,
            AutoGenError::Message(MessageError::Timeout { .. }) => true,
            _ => false,
        }
    }
}