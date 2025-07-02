# AutoGen Core Rust - User Guide

Welcome to AutoGen Core Rust! This guide will help you get started with building AI agent systems using the Rust implementation of AutoGen Core.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Core Concepts](#core-concepts)
3. [Feature Gates](#feature-gates)
4. [Message System](#message-system)
5. [Agent Development](#agent-development)
6. [Runtime Configuration](#runtime-configuration)
7. [Performance Optimization](#performance-optimization)
8. [Error Handling](#error-handling)
9. [Best Practices](#best-practices)
10. [Migration from Python](#migration-from-python)

## Quick Start

### Installation

Add AutoGen Core to your `Cargo.toml`:

```toml
[dependencies]
autogen-core = "0.6.2"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Example

```rust
use autogen_core::{
    Agent, AgentId, TypeSafeMessage, TextMessage, MessageContext,
    Result, CancellationToken
};
use async_trait::async_trait;

#[derive(Debug)]
struct EchoAgent {
    id: AgentId,
}

impl EchoAgent {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
        })
    }
}

#[async_trait]
impl Agent for EchoAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        _context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        match message {
            TypeSafeMessage::Text(text_msg) => {
                println!("Echo: {}", text_msg.content);
                Ok(Some(TypeSafeMessage::Text(TextMessage {
                    content: format!("Echo: {}", text_msg.content),
                })))
            }
            _ => Ok(None), // Don't handle other message types
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut agent = EchoAgent::new("echo", "main")?;
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    let message = TextMessage {
        content: "Hello, AutoGen!".to_string(),
    };
    
    agent.handle_message(message, &context).await?;
    Ok(())
}
```

## Core Concepts

### Agents

Agents are autonomous entities that process messages and perform actions. AutoGen Core provides a unified agent trait:

- **`Agent`**: Unified type-safe agent trait that uses `TypeSafeMessage` for compile-time safety

### Messages

Messages are the primary communication mechanism between agents. The system supports:

- **Type-safe messages**: Compile-time type checking
- **Efficient serialization**: Optimized for performance
- **Validation**: Built-in message validation

### Runtime

The runtime manages agent lifecycle and message routing:

- **`AgentRuntime`**: Main runtime interface
- **`SingleThreadedAgentRuntime`**: Simple single-threaded implementation

## Feature Gates

AutoGen Core uses feature gates to allow minimal builds for different environments:

### Core Features

```toml
[dependencies]
autogen-core = { version = "0.6.2", features = ["minimal"] }
```

- **`minimal`**: JSON serialization only
- **`standard`**: Runtime + JSON + validation + HTTP
- **`full`**: All features enabled

### Available Features

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `runtime` | Agent runtime system | tokio, futures |
| `json` | JSON serialization | serde_json |
| `protobuf` | Protocol Buffer support | prost |
| `http` | HTTP client functionality | reqwest |
| `telemetry` | Tracing and observability | opentelemetry |
| `images` | Image processing | image |
| `validation` | Input validation | jsonschema, regex |

## Message System

### Basic Messages

```rust
use autogen_core::{TextMessage, NoResponse};

// Simple text message
let message = TextMessage {
    content: "Hello, world!".to_string(),
};
```

### Custom Messages

```rust
use autogen_core::{Message, NoResponse};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomMessage {
    id: u64,
    data: String,
}

impl Message for CustomMessage {
    type Response = NoResponse;
    
    fn validate(&self) -> autogen_core::Result<()> {
        if self.data.is_empty() {
            return Err(autogen_core::AutoGenError::other("Data cannot be empty"));
        }
        Ok(())
    }
}
```

### Efficient Messages

For high-performance scenarios, use the `EfficientMessage` trait:

```rust
use autogen_core::message_v2::EfficientMessage;

impl EfficientMessage for CustomMessage {
    type Response = NoResponse;
    
    fn serialize(&self) -> autogen_core::Result<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| autogen_core::AutoGenError::other(format!("Serialization failed: {}", e)))
    }
    
    fn deserialize(data: &[u8]) -> autogen_core::Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| autogen_core::AutoGenError::other(format!("Deserialization failed: {}", e)))
    }
}
```

## Agent Development

### Type-Safe Agents

```rust
use autogen_core::{Agent, AgentId, TypeSafeMessage, MessageContext, Result};
use async_trait::async_trait;

#[derive(Debug)]
struct MyAgent {
    id: AgentId,
    state: MyState,
}

#[async_trait]
impl Agent for MyAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        match message {
            TypeSafeMessage::Custom { message_type, payload } if message_type == "MyMessage" => {
                // Deserialize custom message
                let my_message: MyMessage = serde_json::from_value(payload)?;

                // Process the message
                self.state.update(&my_message);

                // Return response
                let response = MyResponse {
                    success: true,
                    data: "Processed".to_string(),
                };
                Ok(Some(TypeSafeMessage::Custom {
                    message_type: "MyResponse".to_string(),
                    payload: serde_json::to_value(response)?,
                }))
            }
            _ => Ok(None), // Don't handle other message types
        }
    }
}
```

### Agent Composition

```rust
struct CompositeAgent {
    id: AgentId,
    sub_agents: Vec<Box<dyn Agent>>,
}

impl CompositeAgent {
    async fn delegate_message(&mut self, message: TextMessage) -> Result<()> {
        for agent in &mut self.sub_agents {
            // Delegate to appropriate sub-agent
            agent.on_message(Box::new(message.clone()), &context).await?;
        }
        Ok(())
    }
}
```

## Runtime Configuration

### Basic Runtime Setup

```rust
use autogen_core::{SingleThreadedAgentRuntime, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = RuntimeConfig {
        max_concurrent_handlers: 100,
        message_queue_size: 1000,
        enable_telemetry: true,
        ..Default::default()
    };
    
    let mut runtime = SingleThreadedAgentRuntime::new(config);
    
    // Register agents
    runtime.register_agent(Box::new(my_agent)).await?;
    
    // Start runtime
    runtime.start().await?;
    
    Ok(())
}
```

### Advanced Configuration

```rust
use autogen_core::{RuntimeConfig, RuntimeEvent, RuntimeEventHandler};

struct CustomEventHandler;

impl RuntimeEventHandler for CustomEventHandler {
    async fn handle_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::AgentRegistered { agent_id } => {
                println!("Agent registered: {}", agent_id);
            }
            RuntimeEvent::MessageSent { sender, recipient, .. } => {
                println!("Message sent from {:?} to {}", sender, recipient);
            }
            _ => {}
        }
    }
}

let config = RuntimeConfig {
    max_concurrent_handlers: 200,
    message_queue_size: 2000,
    enable_telemetry: true,
    properties: [
        ("environment".to_string(), "production".to_string()),
        ("version".to_string(), "1.0.0".to_string()),
    ].into_iter().collect(),
};
```

## Performance Optimization

### Message Optimization

1. **Use Efficient Messages**: For high-throughput scenarios
2. **Batch Processing**: Process multiple messages together
3. **Avoid Allocations**: Use inline storage for small messages

```rust
use autogen_core::message_v2::{EfficientMessageEnvelope, MessagePerformanceMetrics};

let mut metrics = MessagePerformanceMetrics::default();

// Create efficient envelope
let envelope = EfficientMessageEnvelope::new(message, context)?;
metrics.record_message(&envelope);

println!("Allocation avoidance rate: {:.1}%", metrics.allocation_avoidance_rate());
```

### Runtime Optimization

1. **Configure Thread Pools**: Adjust concurrent handlers
2. **Tune Queue Sizes**: Balance memory vs throughput
3. **Enable Telemetry**: Monitor performance metrics

```rust
let config = RuntimeConfig {
    max_concurrent_handlers: num_cpus::get() * 2,
    message_queue_size: 10_000,
    enable_telemetry: true,
    ..Default::default()
};
```

## Error Handling

### Error Types

AutoGen Core provides comprehensive error handling:

```rust
use autogen_core::{AutoGenError, Result};

// Validation errors
let agent_id = AgentId::new("", "key")?; // Returns validation error

// Runtime errors
let result = agent.handle_message(message, &context).await;
match result {
    Ok(response) => println!("Success: {:?}", response),
    Err(AutoGenError::Validation(msg)) => println!("Validation error: {}", msg),
    Err(AutoGenError::Runtime(msg)) => println!("Runtime error: {}", msg),
    Err(e) => println!("Other error: {}", e),
}
```

### Error Recovery

```rust
use std::time::Duration;

async fn retry_with_backoff<F, T>(mut operation: F, max_attempts: u32) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    for attempt in 1..=max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_attempts => {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## Best Practices

### 1. Use Type-Safe Agents

Prefer `TypedAgent<M>` over the legacy `Agent` trait for better type safety and performance.

### 2. Validate Messages

Always implement validation for custom messages:

```rust
impl Message for MyMessage {
    type Response = MyResponse;
    
    fn validate(&self) -> Result<()> {
        if self.field.is_empty() {
            return Err(AutoGenError::validation("Field cannot be empty"));
        }
        Ok(())
    }
}
```

### 3. Handle Errors Gracefully

Implement proper error handling and recovery strategies:

```rust
async fn handle_message_safely(&mut self, message: M) -> Result<Option<R>> {
    match self.process_message(message).await {
        Ok(response) => Ok(response),
        Err(e) => {
            // Log error
            eprintln!("Error processing message: {}", e);
            
            // Return graceful failure
            Ok(None)
        }
    }
}
```

### 4. Use Feature Gates

Only enable features you need to minimize dependencies:

```toml
# For embedded systems
autogen-core = { version = "0.6.2", features = ["minimal"] }

# For web services
autogen-core = { version = "0.6.2", features = ["standard"] }

# For full-featured applications
autogen-core = { version = "0.6.2", features = ["full"] }
```

### 5. Monitor Performance

Use telemetry and metrics to monitor your agent system:

```rust
use autogen_core::message_v2::MessagePerformanceMetrics;

let mut metrics = MessagePerformanceMetrics::default();
// ... record messages ...
println!("Performance: {:.1}% allocation avoidance", metrics.allocation_avoidance_rate());
```

## Migration from Python

### Key Differences

| Python | Rust | Notes |
|--------|------|-------|
| `Agent` | `Agent` | Unified trait in Rust |
| `on_message` | `handle_message` | Uses `TypeSafeMessage` |
| Dynamic typing | Static typing | Compile-time type checking |
| GIL limitations | True parallelism | Better performance |

### Migration Steps

1. **Define Message Types**: Convert Python message classes to Rust structs
2. **Implement Agents**: Use unified `Agent` trait with `TypeSafeMessage`
3. **Update Runtime**: Use `SingleThreadedAgentRuntime` or custom runtime
4. **Handle Errors**: Use Rust's `Result` type
5. **Test Thoroughly**: Leverage Rust's type system for correctness

### Example Migration

Python:
```python
class MyAgent(Agent):
    async def on_message(self, message: Any, ctx: MessageContext) -> Any:
        if isinstance(message, TextMessage):
            return f"Echo: {message.content}"
        return None
```

Rust:
```rust
#[async_trait]
impl Agent for MyAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        match message {
            TypeSafeMessage::Text(text_msg) => {
                Ok(Some(TypeSafeMessage::Text(TextMessage {
                    content: format!("Echo: {}", text_msg.content),
                })))
            }
            _ => Ok(None), // Don't handle other message types
        }
    }
}
```

For more examples and advanced usage, see the [examples directory](../examples/).
