# AutoGen Core Rust - Best Practices Guide

This guide outlines best practices for developing robust, performant, and maintainable agent systems with AutoGen Core Rust.

## Table of Contents

1. [Feature Gate Management](#feature-gate-management)
2. [Message System Optimization](#message-system-optimization)
3. [Performance Best Practices](#performance-best-practices)
4. [Error Handling Strategies](#error-handling-strategies)
5. [Agent Design Patterns](#agent-design-patterns)
6. [Testing and Debugging](#testing-and-debugging)
7. [Production Deployment](#production-deployment)

## Feature Gate Management

### ✅ Choose the Right Feature Set

```toml
# For embedded/constrained environments
autogen-core = { version = "0.6.2", features = ["minimal"] }

# For typical applications
autogen-core = { version = "0.6.2", features = ["standard"] }

# For full-featured services
autogen-core = { version = "0.6.2", features = ["full"] }
```

### ✅ Understand Feature Dependencies

- `runtime` → requires `tokio`, `futures`
- `networking` → requires `runtime`, `http`
- `telemetry` → requires `runtime`, `tracing`
- `testing` → requires `runtime`, `anyhow`

### ❌ Avoid Over-Including Features

Don't include features you don't need:

```toml
# Bad: Includes unnecessary features
autogen-core = { version = "0.6.2", features = ["full"] }

# Good: Only includes what you need
autogen-core = { version = "0.6.2", features = ["runtime", "json", "validation"] }
```

## Message System Optimization

### ✅ Use Type-Safe Messages

Use the unified `Agent` trait with `TypeSafeMessage`:

```rust
// Good: Type-safe agent with unified trait
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
        // Pattern match for type safety
        match message {
            TypeSafeMessage::Text(text_msg) => {
                // Handle text message
                Ok(Some(TypeSafeMessage::Text(TextMessage {
                    content: format!("Processed: {}", text_msg.content),
                })))
            }
            TypeSafeMessage::Custom { message_type, payload } if message_type == "MyMessage" => {
                // Handle custom message type
                let my_msg: MyMessage = serde_json::from_value(payload)?;
                let response = MyResponse { data: my_msg.data };
                Ok(Some(TypeSafeMessage::Custom {
                    message_type: "MyResponse".to_string(),
                    payload: serde_json::to_value(response)?,
                }))
            }
            _ => Ok(None), // Message not handled by this agent
        }
    }
}
```

### ✅ Implement Efficient Messages for High Throughput

For performance-critical scenarios, use `EfficientMessage`:

```rust
use autogen_core::message_v2::EfficientMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HighThroughputMessage {
    id: u64,
    data: String,
}

impl EfficientMessage for HighThroughputMessage {
    type Response = NoResponse;
    
    fn serialize(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| AutoGenError::serialization(e.to_string()))
    }
    
    fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(|e| AutoGenError::serialization(e.to_string()))
    }
    
    fn validate(&self) -> Result<()> {
        if self.data.len() > 1024 {
            return Err(AutoGenError::validation("Data too large"));
        }
        Ok(())
    }
}
```

### ✅ Always Validate Messages

```rust
impl Message for MyMessage {
    type Response = MyResponse;
    
    fn validate(&self) -> Result<()> {
        if self.field.is_empty() {
            return Err(AutoGenError::validation("Field cannot be empty"));
        }
        if self.field.len() > 1000 {
            return Err(AutoGenError::validation("Field too long"));
        }
        Ok(())
    }
}
```

## Performance Best Practices

### ✅ Use Efficient Message Envelopes

```rust
use autogen_core::message_v2::{EfficientMessageEnvelope, MessagePerformanceMetrics};

let mut metrics = MessagePerformanceMetrics::default();

// Small messages use inline storage (no allocation)
let small_msg = TextMessage { content: "Hi".to_string() };
let envelope = EfficientMessageEnvelope::new(small_msg, context)?;
metrics.record_message(&envelope);

println!("Allocation avoidance: {:.1}%", metrics.allocation_avoidance_rate());
```

### ✅ Batch Process Messages

```rust
async fn process_batch(&mut self, messages: Vec<MyMessage>) -> Result<Vec<MyResponse>> {
    // Process in batch for better performance
    let mut responses = Vec::with_capacity(messages.len());
    
    for message in messages {
        let response = self.process_single(message).await?;
        responses.push(response);
    }
    
    Ok(responses)
}
```

### ✅ Configure Runtime for Your Workload

```rust
let config = RuntimeConfig {
    // Adjust based on your CPU cores
    max_concurrent_handlers: num_cpus::get() * 2,
    
    // Balance memory vs throughput
    message_queue_size: 10_000,
    
    // Enable for production monitoring
    enable_telemetry: true,
    
    properties: [
        ("environment".to_string(), "production".to_string()),
        ("service_name".to_string(), "my-agent-service".to_string()),
    ].into_iter().collect(),
};
```

### ❌ Avoid Excessive Boxing

```rust
// Bad: Unnecessary boxing
let message = Box::new(TextMessage { content: "Hello".to_string() });

// Good: Use typed messages directly
let message = TextMessage { content: "Hello".to_string() };
```

## Error Handling Strategies

### ✅ Use Structured Error Handling

```rust
async fn handle_message_with_recovery(&mut self, message: MyMessage) -> Result<Option<MyResponse>> {
    match self.process_message(message).await {
        Ok(response) => Ok(response),
        Err(AutoGenError::Validation(msg)) => {
            // Log validation errors but continue
            eprintln!("Validation error: {}", msg);
            Ok(None)
        }
        Err(AutoGenError::Runtime(msg)) => {
            // Runtime errors might be recoverable
            eprintln!("Runtime error: {}", msg);
            self.attempt_recovery().await?;
            Ok(None)
        }
        Err(e) => {
            // Other errors are fatal
            Err(e)
        }
    }
}
```

### ✅ Implement Retry Logic with Backoff

```rust
use std::time::Duration;

async fn retry_with_exponential_backoff<F, T>(
    mut operation: F,
    max_attempts: u32,
) -> Result<T>
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

### ✅ Use Graceful Degradation

```rust
async fn get_data_with_fallback(&self) -> Result<String> {
    // Try primary service
    match self.primary_service.get_data().await {
        Ok(data) => Ok(data),
        Err(_) => {
            // Fall back to cache
            match self.cache.get_data().await {
                Ok(data) => {
                    eprintln!("Using cached data due to primary service failure");
                    Ok(data)
                }
                Err(_) => {
                    // Final fallback
                    Ok("default_data".to_string())
                }
            }
        }
    }
}
```

## Agent Design Patterns

### ✅ Use Composition for Complex Agents

```rust
struct CompositeAgent {
    id: AgentId,
    command_handler: CommandHandler,
    query_handler: QueryHandler,
    event_handler: EventHandler,
}

impl CompositeAgent {
    async fn route_message(&mut self, message: TextMessage) -> Result<Option<NoResponse>> {
        let parts: Vec<&str> = message.content.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AutoGenError::validation("Invalid message format"));
        }
        
        match parts[0] {
            "command" => self.command_handler.handle(parts[1]).await,
            "query" => self.query_handler.handle(parts[1]).await,
            "event" => self.event_handler.handle(parts[1]).await,
            _ => Err(AutoGenError::validation("Unknown message type")),
        }
    }
}
```

### ✅ Implement State Management Safely

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct AgentState {
    counter: i32,
    last_update: chrono::DateTime<chrono::Utc>,
}

struct StatefulAgent {
    id: AgentId,
    state: Arc<Mutex<AgentState>>,
}

impl StatefulAgent {
    async fn update_state<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AgentState) -> Result<()>,
    {
        let mut state = self.state.lock().await;
        updater(&mut *state)?;
        state.last_update = chrono::Utc::now();
        Ok(())
    }
}
```

### ✅ Use Event-Driven Patterns

```rust
#[derive(Debug, Clone)]
enum AgentEvent {
    MessageReceived { sender: AgentId, content: String },
    StateChanged { old_value: i32, new_value: i32 },
    ErrorOccurred { error: String },
}

struct EventDrivenAgent {
    id: AgentId,
    event_handlers: Vec<Box<dyn EventHandler>>,
}

impl EventDrivenAgent {
    async fn emit_event(&mut self, event: AgentEvent) -> Result<()> {
        for handler in &mut self.event_handlers {
            handler.handle_event(&event).await?;
        }
        Ok(())
    }
}
```

## Testing and Debugging

### ✅ Write Comprehensive Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_agent_message_handling() {
        let mut agent = MyAgent::new("test", "agent").unwrap();
        let token = CancellationToken::new();
        let context = MessageContext::direct_message(None, token);
        
        let message = MyMessage {
            data: "test_data".to_string(),
        };
        
        let result = agent.handle_message(message, &context).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.is_some());
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let mut agent = MyAgent::new("test", "agent").unwrap();
        let token = CancellationToken::new();
        let context = MessageContext::direct_message(None, token);
        
        let invalid_message = MyMessage {
            data: String::new(), // Invalid empty data
        };
        
        let result = agent.handle_message(invalid_message, &context).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            AutoGenError::Validation(_) => {}, // Expected
            e => panic!("Unexpected error type: {:?}", e),
        }
    }
}
```

### ✅ Use Performance Benchmarks

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn benchmark_message_processing(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("message_processing", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let mut agent = MyAgent::new("bench", "agent").unwrap();
                    let token = CancellationToken::new();
                    let context = MessageContext::direct_message(None, token);
                    
                    let message = MyMessage {
                        data: black_box("benchmark_data".to_string()),
                    };
                    
                    agent.handle_message(message, &context).await.unwrap()
                })
            })
        });
    }
    
    criterion_group!(benches, benchmark_message_processing);
    criterion_main!(benches);
}
```

## Production Deployment

### ✅ Enable Telemetry and Monitoring

```rust
use autogen_core::message_v2::MessagePerformanceMetrics;

struct ProductionAgent {
    id: AgentId,
    metrics: MessagePerformanceMetrics,
    start_time: std::time::Instant,
}

impl ProductionAgent {
    async fn handle_with_metrics(&mut self, message: MyMessage) -> Result<Option<MyResponse>> {
        let start = std::time::Instant::now();
        
        let result = self.handle_message_impl(message).await;
        
        let duration = start.elapsed();
        self.log_metrics(duration, &result);
        
        result
    }
    
    fn log_metrics(&mut self, duration: std::time::Duration, result: &Result<Option<MyResponse>>) {
        println!("Message processed in {:?}", duration);
        
        if result.is_err() {
            println!("Error occurred during message processing");
        }
        
        // Log performance metrics periodically
        if self.start_time.elapsed().as_secs() % 60 == 0 {
            println!("Performance metrics: {:.1}% allocation avoidance", 
                    self.metrics.allocation_avoidance_rate());
        }
    }
}
```

### ✅ Implement Health Checks

```rust
#[derive(Debug)]
struct HealthStatus {
    healthy: bool,
    last_message_time: Option<std::time::Instant>,
    error_count: u64,
}

impl MyAgent {
    fn health_check(&self) -> HealthStatus {
        let healthy = self.error_count < 10 && 
                     self.last_message_time
                         .map(|t| t.elapsed().as_secs() < 300)
                         .unwrap_or(true);
        
        HealthStatus {
            healthy,
            last_message_time: self.last_message_time,
            error_count: self.error_count,
        }
    }
}
```

### ✅ Use Structured Logging

```rust
use tracing::{info, warn, error, instrument};

impl MyAgent {
    #[instrument(skip(self, message))]
    async fn handle_message_with_logging(
        &mut self,
        message: MyMessage,
        context: &MessageContext,
    ) -> Result<Option<MyResponse>> {
        info!(
            agent_id = %self.id,
            message_size = message.data.len(),
            sender = ?context.sender,
            "Processing message"
        );
        
        match self.handle_message(message, context).await {
            Ok(response) => {
                info!("Message processed successfully");
                Ok(response)
            }
            Err(e) => {
                error!(error = %e, "Failed to process message");
                Err(e)
            }
        }
    }
}
```

## Summary

Following these best practices will help you build robust, performant, and maintainable agent systems:

1. **Choose appropriate feature gates** for your deployment environment
2. **Use type-safe messages** and efficient serialization for performance
3. **Implement comprehensive error handling** with retry and fallback strategies
4. **Design agents using composition** and event-driven patterns
5. **Write thorough tests** including performance benchmarks
6. **Enable monitoring and telemetry** for production deployments

For more examples and detailed implementations, see the [examples directory](../examples/) and [API reference](API_REFERENCE.md).
