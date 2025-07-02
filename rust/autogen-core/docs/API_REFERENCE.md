# AutoGen Core Rust - API Reference

This document provides a comprehensive reference for the AutoGen Core Rust API.

## Core Traits and Types

### Agent Traits

#### `Agent`

The unified trait for implementing type-safe agents using `TypeSafeMessage`.

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> &AgentId;

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>>;

    fn metadata(&self) -> AgentMetadata {
        AgentMetadata::default()
    }

    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        Ok(HashMap::new())
    }

    async fn load_state(&mut self, _state: HashMap<String, serde_json::Value>) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
```

**Required Methods:**
- `id()`: Returns the agent's unique identifier
- `handle_message()`: Processes incoming messages using `TypeSafeMessage`

**Optional Methods:**
- `metadata()`: Returns agent metadata
- `on_start()`: Called when agent starts
- `on_stop()`: Called when agent stops
- `save_state()`: Save agent state for persistence
- `load_state()`: Load agent state from persistence
- `close()`: Cleanup when agent shuts down
```

### Message Traits

#### `Message`

Core trait for all messages.

```rust
pub trait Message: Send + Sync + Debug + 'static {
    type Response: Send + Sync + Debug + 'static;
    
    fn message_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
```

#### `EfficientMessage`

High-performance message trait with serialization.

```rust
pub trait EfficientMessage: Send + Sync + Debug + 'static {
    type Response: EfficientMessage;
    
    fn serialize(&self) -> Result<Vec<u8>>;
    fn deserialize(data: &[u8]) -> Result<Self> where Self: Sized;
    fn validate(&self) -> Result<()> { Ok(()) }
}
```

## Core Types

### `AgentId`

Unique identifier for agents.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId {
    // Private fields
}

impl AgentId {
    pub fn new(agent_type: &str, key: &str) -> Result<Self>;
    pub fn agent_type(&self) -> &str;
    pub fn key(&self) -> &str;
}
```

**Methods:**
- `new(agent_type, key)`: Creates a new agent ID
- `agent_type()`: Returns the agent type
- `key()`: Returns the agent key

**Validation Rules:**
- Agent type and key cannot be empty
- No whitespace allowed
- Maximum length: 100 characters each

### `TopicId`

Identifier for message topics.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicId {
    // Private fields
}

impl TopicId {
    pub fn new(topic_type: &str, source: &str) -> Result<Self>;
    pub fn topic_type(&self) -> &str;
    pub fn source(&self) -> &str;
}
```

### `MessageContext`

Context information for messages.

```rust
#[derive(Debug, Clone)]
pub struct MessageContext {
    pub sender: Option<AgentId>,
    pub topic_id: Option<TopicId>,
    pub cancellation_token: CancellationToken,
    pub metadata: HashMap<String, String>,
}

impl MessageContext {
    pub fn direct_message(sender: Option<AgentId>, token: CancellationToken) -> Self;
    pub fn topic_message(topic_id: TopicId, sender: Option<AgentId>, token: CancellationToken) -> Self;
}
```

### `CancellationToken`

Token for cancelling operations.

```rust
#[derive(Debug, Clone)]
pub struct CancellationToken {
    // Private fields
}

impl CancellationToken {
    pub fn new() -> Self;
    pub fn cancel(&self);
    pub fn is_cancelled(&self) -> bool;
}
```

## Message Types

### `TextMessage`

Simple text message.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub content: String,
}

impl Message for TextMessage {
    type Response = NoResponse;
}
```

### `NoResponse`

Unit type for messages that don't expect a response.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoResponse;

impl Message for NoResponse {
    type Response = NoResponse;
}
```

### `RequestMessage<T>`

Generic request message.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage<T> {
    pub request: T,
    pub request_id: String,
}

impl<T> Message for RequestMessage<T>
where T: Send + Sync + Debug + 'static
{
    type Response = ResponseMessage<T>;
}
```

### `ResponseMessage<T>`

Generic response message.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage<T> {
    pub response: T,
    pub success: bool,
    pub error_message: Option<String>,
    pub request_id: String,
}
```

## Runtime System

### `AgentRuntime`

Main runtime interface.

```rust
#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn register_agent(&mut self, agent: Box<dyn Agent>) -> Result<()>;
    async fn send_message(&mut self, message: Box<dyn Any + Send>, recipient: AgentId, sender: Option<AgentId>) -> Result<()>;
    async fn publish_message(&mut self, message: Box<dyn Any + Send>, topic_id: TopicId, sender: Option<AgentId>) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    fn stats(&self) -> RuntimeStats;
}
```

### `SingleThreadedAgentRuntime`

Simple single-threaded runtime implementation.

```rust
pub struct SingleThreadedAgentRuntime {
    // Private fields
}

impl SingleThreadedAgentRuntime {
    pub fn new(config: RuntimeConfig) -> Self;
    pub async fn register_agent(&mut self, agent: Box<dyn Agent>) -> Result<()>;
    pub async fn register_typed_agent<M, A>(&mut self, agent: A) -> Result<()>
    where
        M: Message,
        A: TypedAgent<M> + 'static;
}
```

### `RuntimeConfig`

Configuration for agent runtimes.

```rust
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_concurrent_handlers: usize,
    pub message_queue_size: usize,
    pub enable_telemetry: bool,
    pub properties: HashMap<String, String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_handlers: 100,
            message_queue_size: 1000,
            enable_telemetry: false,
            properties: HashMap::new(),
        }
    }
}
```

### `RuntimeStats`

Runtime performance statistics.

```rust
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub agents_registered: usize,
    pub messages_processed: u64,
    pub messages_routed: u64,
    pub avg_processing_time_us: f64,
    pub avg_routing_time_us: f64,
    pub errors_encountered: u64,
}
```

## Subscription System

### `Subscription`

Base trait for message subscriptions.

```rust
pub trait Subscription: Send + Sync + Debug {
    fn matches(&self, message_type: &str, topic_id: Option<&TopicId>) -> bool;
    fn description(&self) -> String;
}
```

### `DefaultSubscription`

Matches all messages.

```rust
#[derive(Debug, Clone)]
pub struct DefaultSubscription;

impl Subscription for DefaultSubscription {
    fn matches(&self, _message_type: &str, _topic_id: Option<&TopicId>) -> bool {
        true
    }
}
```

### `TypeSubscription`

Matches specific message types.

```rust
#[derive(Debug, Clone)]
pub struct TypeSubscription {
    pub message_type: String,
}

impl TypeSubscription {
    pub fn new<M: Message>() -> Self {
        Self {
            message_type: std::any::type_name::<M>().to_string(),
        }
    }
}
```

### `TopicSubscription`

Matches specific topics.

```rust
#[derive(Debug, Clone)]
pub struct TopicSubscription {
    pub topic_id: TopicId,
}

impl TopicSubscription {
    pub fn new(topic_id: TopicId) -> Self {
        Self { topic_id }
    }
}
```

## Error Handling

### `AutoGenError`

Main error type for the library.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AutoGenError {
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

impl AutoGenError {
    pub fn validation(msg: impl Into<String>) -> Self;
    pub fn runtime(msg: impl Into<String>) -> Self;
    pub fn serialization(msg: impl Into<String>) -> Self;
    pub fn network(msg: impl Into<String>) -> Self;
    pub fn other(msg: impl Into<String>) -> Self;
}
```

### `Result<T>`

Type alias for results.

```rust
pub type Result<T> = std::result::Result<T, AutoGenError>;
```

## Performance Optimization

### `EfficientMessageEnvelope`

High-performance message envelope.

```rust
pub enum EfficientMessageEnvelope {
    Inline { /* small messages */ },
    Heap { /* large messages */ },
    Legacy { /* compatibility */ },
}

impl EfficientMessageEnvelope {
    pub fn new<M: EfficientMessage>(message: M, context: MessageContext) -> Result<Self>;
    pub fn deserialize<M: EfficientMessage>(&self) -> Result<M>;
    pub fn is_type<M: EfficientMessage>(&self) -> bool;
}
```

### `MessagePerformanceMetrics`

Performance tracking for messages.

```rust
#[derive(Debug, Default)]
pub struct MessagePerformanceMetrics {
    pub inline_messages: u64,
    pub heap_messages: u64,
    pub legacy_messages: u64,
    pub bytes_saved: u64,
    pub avg_message_size: f64,
}

impl MessagePerformanceMetrics {
    pub fn record_message(&mut self, envelope: &EfficientMessageEnvelope);
    pub fn allocation_avoidance_rate(&self) -> f64;
}
```

## Feature Gates

### Available Features

- **`runtime`**: Agent runtime system (requires tokio)
- **`json`**: JSON serialization support
- **`protobuf`**: Protocol Buffer support
- **`http`**: HTTP client functionality
- **`telemetry`**: Tracing and observability
- **`images`**: Image processing support
- **`validation`**: Input validation and schema checking
- **`config-support`**: Configuration file support
- **`testing`**: Testing utilities and helpers

### Feature Combinations

- **`minimal`**: JSON only
- **`standard`**: Runtime + JSON + validation + HTTP
- **`full`**: All features enabled

## Constants

```rust
/// Current version of autogen-core
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Root logger name for the autogen-core library
pub const ROOT_LOGGER_NAME: &str = "autogen_core";

/// Event logger name for agent events
pub const EVENT_LOGGER_NAME: &str = "autogen_core.events";

/// Trace logger name for distributed tracing
pub const TRACE_LOGGER_NAME: &str = "autogen_core.trace";
```

For more detailed examples and usage patterns, see the [User Guide](USER_GUIDE.md) and [examples directory](../examples/).
