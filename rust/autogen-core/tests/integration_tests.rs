//! Integration tests for autogen-core
//!
//! These tests verify that different components work together correctly.

use autogen_core::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test basic message creation and handling
#[tokio::test]
async fn test_message_creation() {
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    assert!(context.sender.is_none());
    assert!(context.topic_id.is_none());
    assert!(!context.is_rpc);
    assert!(!context.message_id.is_empty());
}

/// Test typed message system
#[tokio::test]
async fn test_typed_messages() {
    let text_msg = TextMessage {
        content: "Hello, world!".to_string(),
    };
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    let envelope = TypedMessageEnvelope::new(text_msg, context);
    
    assert_eq!(envelope.message.content, "Hello, world!");
    
    // Test conversion to untyped
    let untyped = envelope.into_untyped();
    assert_eq!(untyped.message_type(), "autogen_core::message::TextMessage");
    
    // Test downcast back to typed
    let typed_back = untyped.downcast::<TextMessage>().unwrap();
    assert_eq!(typed_back.message.content, "Hello, world!");
}

/// Test agent ID validation
#[tokio::test]
async fn test_agent_id_validation() {
    // Valid agent IDs
    assert!(AgentId::new("assistant", "main").is_ok());
    assert!(AgentId::new("user-agent", "session_1").is_ok());
    assert!(AgentId::new("tool.executor", "default").is_ok());
    
    // Invalid agent IDs
    assert!(AgentId::new("", "main").is_err());
    assert!(AgentId::new("invalid/type", "main").is_err());
    assert!(AgentId::new("invalid type", "main").is_err());
}

/// Test topic ID validation
#[tokio::test]
async fn test_topic_id_validation() {
    // Valid topic IDs
    assert!(TopicId::new("user.message", "session_1").is_ok());
    assert!(TopicId::new("system.event", "global").is_ok());
    
    // Invalid topic IDs
    assert!(TopicId::new("", "session").is_err());
    assert!(TopicId::new("invalid topic", "session").is_err());
}

/// Test error handling and context
#[tokio::test]
async fn test_error_handling() {
    let context = crate::error::ErrorContext::new("test_operation")
        .with_detail("test_key", "test_value");
    
    let error = AutoGenError::validation(context, "Test validation error");
    
    assert!(error.to_string().contains("Test validation error"));
    assert!(error.to_string().contains("test_operation"));
}

/// Test memory content validation
#[tokio::test]
async fn test_memory_content_validation() {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    let limits = ContentLimits::default();
    
    // Valid content
    let text_content = MemoryContent::text("Hello, world!");
    assert!(text_content.validate(&limits).is_ok());
    
    let json_content = MemoryContent::json(serde_json::json!({"key": "value"}));
    assert!(json_content.validate(&limits).is_ok());
    
    // Test size limits
    let large_text = "x".repeat(limits.max_text_size + 1);
    let large_content = MemoryContent::text(large_text);
    assert!(large_content.validate(&limits).is_err());
}

/// Test tool argument validation
#[tokio::test]
async fn test_tool_args_validation() {
    use autogen_core::tools::{NoArgs, ToolArgs};
    
    let no_args = NoArgs;
    assert!(no_args.validate().is_ok());
    
    let schema = NoArgs::schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"], serde_json::json!({}));
}

/// Mock agent for testing
#[derive(Debug)]
struct MockAgent {
    id: AgentId,
    messages_received: Arc<Mutex<Vec<String>>>,
}

impl MockAgent {
    fn new(agent_type: &str, key: &str) -> Self {
        Self {
            id: AgentId::new(agent_type, key).unwrap(),
            messages_received: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for MockAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let mut messages = self.messages_received.lock().await;
        messages.push(message.content);
        Ok(Some(NoResponse))
    }
}

/// Test typed agent functionality
#[tokio::test]
async fn test_typed_agent() {
    let mut agent = MockAgent::new("test", "mock");
    let message = TextMessage {
        content: "Test message".to_string(),
    };
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    let result = agent.handle_message(message, &context).await;
    assert!(result.is_ok());
    
    let messages = agent.messages_received.lock().await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0], "Test message");
}

/// Test agent adapter functionality
#[tokio::test]
async fn test_agent_adapter() {
    let mock_agent = MockAgent::new("test", "adapter");
    let mut adapter = TypedAgentAdapter::new(mock_agent);
    
    let message = TextMessage {
        content: "Adapter test".to_string(),
    };
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Test with correct message type
    let result = adapter.on_message(Box::new(message), &context).await;
    assert!(result.is_ok());
    
    // Test with incorrect message type
    let wrong_message = RequestMessage {
        request: "wrong type".to_string(),
    };
    let result = adapter.on_message(Box::new(wrong_message), &context).await;
    assert!(result.is_err());
}

/// Test cancellation token functionality
#[tokio::test]
async fn test_cancellation_token() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    
    token.cancel();
    assert!(token.is_cancelled());
    
    // Test child token
    let parent = CancellationToken::new();
    let child = parent.child_token();
    assert!(!child.is_cancelled());

    parent.cancel();

    // Give the child token a moment to be notified
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert!(child.is_cancelled());
}

/// Test concurrent message handling
#[tokio::test]
async fn test_concurrent_message_handling() {
    let agent = Arc::new(Mutex::new(MockAgent::new("test", "concurrent")));
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let agent_clone = agent.clone();
        let handle = tokio::spawn(async move {
            let message = TextMessage {
                content: format!("Message {}", i),
            };
            let token = CancellationToken::new();
            let context = MessageContext::direct_message(None, token);
            
            let mut agent_guard = agent_clone.lock().await;
            agent_guard.handle_message(message, &context).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
    
    // Verify all messages were received
    let agent_guard = agent.lock().await;
    let messages = agent_guard.messages_received.lock().await;
    assert_eq!(messages.len(), 10);
}
