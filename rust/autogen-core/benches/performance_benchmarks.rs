//! Performance benchmarks for autogen-core
//!
//! These benchmarks measure the performance of core operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use autogen_core::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Benchmark agent ID creation
fn bench_agent_id_creation(c: &mut Criterion) {
    c.bench_function("agent_id_creation", |b| {
        b.iter(|| {
            let id = AgentId::new(black_box("test_agent"), black_box("instance_1"));
            black_box(id)
        })
    });
}

/// Benchmark topic ID creation
fn bench_topic_id_creation(c: &mut Criterion) {
    c.bench_function("topic_id_creation", |b| {
        b.iter(|| {
            let id = TopicId::new(black_box("user.message"), black_box("session_1"));
            black_box(id)
        })
    });
}

/// Benchmark message context creation
fn bench_message_context_creation(c: &mut Criterion) {
    c.bench_function("message_context_creation", |b| {
        b.iter(|| {
            let token = CancellationToken::new();
            let context = MessageContext::direct_message(None, token);
            black_box(context)
        })
    });
}

/// Benchmark typed message envelope creation and conversion
fn bench_message_envelope_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_envelope");
    
    group.bench_function("typed_creation", |b| {
        b.iter(|| {
            let message = TextMessage {
                content: black_box("Hello, world!".to_string()),
            };
            let token = CancellationToken::new();
            let context = MessageContext::direct_message(None, token);
            let envelope = TypedMessageEnvelope::new(message, context);
            black_box(envelope)
        })
    });
    
    group.bench_function("typed_to_untyped", |b| {
        let message = TextMessage {
            content: "Hello, world!".to_string(),
        };
        let token = CancellationToken::new();
        let context = MessageContext::direct_message(None, token);
        let envelope = TypedMessageEnvelope::new(message, context);
        
        b.iter(|| {
            let envelope_clone = TypedMessageEnvelope::new(
                TextMessage { content: "Hello, world!".to_string() },
                MessageContext::direct_message(None, CancellationToken::new())
            );
            let untyped = envelope_clone.into_untyped();
            black_box(untyped)
        })
    });
    
    group.bench_function("untyped_downcast", |b| {
        let message = TextMessage {
            content: "Hello, world!".to_string(),
        };
        let token = CancellationToken::new();
        let context = MessageContext::direct_message(None, token);
        let envelope = TypedMessageEnvelope::new(message, context);
        let untyped = envelope.into_untyped();
        
        b.iter(|| {
            let untyped_clone = UntypedMessageEnvelope {
                message: Box::new(TextMessage { content: "Hello, world!".to_string() }),
                context: MessageContext::direct_message(None, CancellationToken::new()),
                message_type: "autogen_core::message::TextMessage",
                type_id: std::any::TypeId::of::<TextMessage>(),
            };
            let result = untyped_clone.downcast::<TextMessage>();
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark memory content operations
fn bench_memory_content_operations(c: &mut Criterion) {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    let mut group = c.benchmark_group("memory_content");
    
    group.bench_function("text_creation", |b| {
        b.iter(|| {
            let content = MemoryContent::text(black_box("Sample text content"));
            black_box(content)
        })
    });
    
    group.bench_function("json_creation", |b| {
        let json_data = serde_json::json!({
            "key": "value",
            "number": 42,
            "array": [1, 2, 3, 4, 5]
        });
        
        b.iter(|| {
            let content = MemoryContent::json(black_box(json_data.clone()));
            black_box(content)
        })
    });
    
    group.bench_function("validation", |b| {
        let content = MemoryContent::text("Sample text for validation");
        let limits = ContentLimits::default();
        
        b.iter(|| {
            let result = content.validate(black_box(&limits));
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark error creation and handling
fn bench_error_operations(c: &mut Criterion) {
    use autogen_core::error::ErrorContext;
    
    let mut group = c.benchmark_group("error_operations");
    
    group.bench_function("context_creation", |b| {
        b.iter(|| {
            let context = ErrorContext::new(black_box("test_operation"))
                .with_detail("key1", "value1")
                .with_detail("key2", "value2");
            black_box(context)
        })
    });
    
    group.bench_function("error_creation", |b| {
        b.iter(|| {
            let context = ErrorContext::new("test_operation");
            let error = AutoGenError::validation(context, black_box("Test error message"));
            black_box(error)
        })
    });
    
    group.finish();
}

/// Mock agent for benchmarking
#[derive(Debug)]
struct BenchmarkAgent {
    id: AgentId,
    message_count: Arc<Mutex<usize>>,
}

impl BenchmarkAgent {
    fn new(agent_type: &str, key: &str) -> Self {
        Self {
            id: AgentId::new(agent_type, key).unwrap(),
            message_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for BenchmarkAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        _message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let mut count = self.message_count.lock().await;
        *count += 1;
        Ok(Some(NoResponse))
    }
}

/// Benchmark agent message handling
fn bench_agent_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("agent_operations");
    
    group.bench_function("typed_agent_message_handling", |b| {
        b.to_async(&rt).iter(|| async {
            let mut agent = BenchmarkAgent::new("benchmark", "test");
            let message = TextMessage {
                content: black_box("Benchmark message".to_string()),
            };
            let token = CancellationToken::new();
            let context = MessageContext::direct_message(None, token);
            
            let result = agent.handle_message(message, &context).await;
            black_box(result)
        })
    });
    
    group.bench_function("agent_adapter_message_handling", |b| {
        b.to_async(&rt).iter(|| async {
            let agent = BenchmarkAgent::new("benchmark", "adapter");
            let mut adapter = TypedAgentAdapter::new(agent);
            let message = TextMessage {
                content: black_box("Adapter benchmark message".to_string()),
            };
            let token = CancellationToken::new();
            let context = MessageContext::direct_message(None, token);
            
            let result = adapter.on_message(Box::new(message), &context).await;
            black_box(result)
        })
    });
    
    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    for num_tasks in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_message_creation", num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.to_async(&rt).iter(|| async move {
                    let mut handles = Vec::new();
                    
                    for i in 0..num_tasks {
                        let handle = tokio::spawn(async move {
                            let message = TextMessage {
                                content: format!("Message {}", i),
                            };
                            let token = CancellationToken::new();
                            let context = MessageContext::direct_message(None, token);
                            let envelope = TypedMessageEnvelope::new(message, context);
                            black_box(envelope)
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark serialization operations (if feature is enabled)
#[cfg(feature = "json")]
fn bench_serialization_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    group.bench_function("json_serialize_text_message", |b| {
        let message = TextMessage {
            content: "Hello, world!".to_string(),
        };
        
        b.iter(|| {
            let serialized = serde_json::to_string(black_box(&message));
            black_box(serialized)
        })
    });
    
    group.bench_function("json_deserialize_text_message", |b| {
        let json = r#"{"content":"Hello, world!"}"#;
        
        b.iter(|| {
            let deserialized: Result<TextMessage, _> = serde_json::from_str(black_box(json));
            black_box(deserialized)
        })
    });
    
    group.finish();
}

#[cfg(not(feature = "json"))]
fn bench_serialization_operations(_c: &mut Criterion) {
    // Skip serialization benchmarks if JSON feature is not enabled
}

criterion_group!(
    benches,
    bench_agent_id_creation,
    bench_topic_id_creation,
    bench_message_context_creation,
    bench_message_envelope_operations,
    bench_memory_content_operations,
    bench_error_operations,
    bench_agent_operations,
    bench_concurrent_operations,
    bench_serialization_operations
);

criterion_main!(benches);
