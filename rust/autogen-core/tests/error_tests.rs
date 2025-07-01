//! Error path tests for autogen-core
//!
//! These tests verify that error conditions are handled correctly.

use autogen_core::*;
use autogen_core::error::ErrorContext;

/// Test invalid agent ID creation
#[test]
fn test_invalid_agent_id_errors() {
    // Empty type
    let result = AgentId::new("", "key");
    assert!(result.is_err());
    
    // Invalid characters in type
    let result = AgentId::new("invalid/type", "key");
    assert!(result.is_err());
    
    let result = AgentId::new("invalid type", "key");
    assert!(result.is_err());
    
    let result = AgentId::new("invalid@type", "key");
    assert!(result.is_err());
}

/// Test invalid topic ID creation
#[test]
fn test_invalid_topic_id_errors() {
    // Empty topic type
    let result = TopicId::new("", "source");
    assert!(result.is_err());
    
    // Invalid characters
    let result = TopicId::new("invalid topic", "source");
    assert!(result.is_err());
    
    let result = TopicId::new("invalid/topic", "source");
    assert!(result.is_err());
}

/// Test memory content size limit errors
#[tokio::test]
async fn test_memory_size_limit_errors() {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    let mut limits = ContentLimits::default();
    limits.max_text_size = 10; // Very small limit for testing
    
    // Text content exceeding limit
    let large_text = "x".repeat(11);
    let content = MemoryContent::text(large_text);
    let result = content.validate(&limits);
    assert!(result.is_err());
    
    // Binary content exceeding limit
    limits.max_binary_size = 5;
    let large_binary = vec![0u8; 6];
    let content = MemoryContent::binary(large_binary);
    let result = content.validate(&limits);
    assert!(result.is_err());
}

/// Test MIME type compatibility errors
#[tokio::test]
async fn test_mime_type_compatibility_errors() {
    use autogen_core::memory::{MemoryContent, MemoryMimeType, ContentType, ContentLimits};
    
    // Create content with incompatible MIME type
    let content = MemoryContent {
        content: ContentType::Text("hello".to_string()),
        mime_type: MemoryMimeType::Image, // Wrong MIME type for text content
        metadata: None,
    };
    
    let limits = ContentLimits::default();
    let result = content.validate(&limits);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not compatible"));
}

/// Test image format validation errors
#[tokio::test]
async fn test_image_format_validation_errors() {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    // Invalid image format
    let content = MemoryContent::image("base64data", "invalid_format", None);
    let limits = ContentLimits::default();
    let result = content.validate(&limits);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported image format"));
}

/// Test JSON serialization errors
#[tokio::test]
async fn test_json_serialization_errors() {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    // Create JSON content that will fail size validation
    let mut limits = ContentLimits::default();
    limits.max_json_size = 10; // Very small limit
    
    let large_json = serde_json::json!({
        "very_long_key_name_that_exceeds_limit": "very_long_value_that_also_exceeds_the_limit"
    });
    
    let content = MemoryContent::json(large_json);
    let result = content.validate(&limits);
    assert!(result.is_err());
}

/// Test message downcast errors
#[tokio::test]
async fn test_message_downcast_errors() {
    let text_message = TextMessage {
        content: "Hello".to_string(),
    };
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    let envelope = TypedMessageEnvelope::new(text_message, context);
    let untyped = envelope.into_untyped();
    
    // Try to downcast to wrong type
    let result = untyped.downcast::<RequestMessage<String>>();
    assert!(result.is_err());
}

/// Test agent adapter with wrong message type
#[tokio::test]
async fn test_agent_adapter_wrong_message_type() {
    #[derive(Debug)]
    struct TestAgent {
        id: AgentId,
    }
    
    #[async_trait::async_trait]
    impl TypedAgent<TextMessage> for TestAgent {
        fn id(&self) -> &AgentId {
            &self.id
        }
        
        async fn handle_message(
            &mut self,
            _message: TextMessage,
            _context: &MessageContext,
        ) -> Result<Option<NoResponse>> {
            Ok(Some(NoResponse))
        }
    }
    
    let agent = TestAgent {
        id: AgentId::new("test", "agent").unwrap(),
    };
    let mut adapter = TypedAgentAdapter::new(agent);
    
    // Send wrong message type
    let wrong_message = RequestMessage {
        request: 42i32,
    };
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    let result = adapter.on_message(Box::new(wrong_message), &context).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot handle message"));
}

/// Test error context creation and details
#[test]
fn test_error_context_details() {
    let context = ErrorContext::new("test_operation")
        .with_detail("key1", "value1")
        .with_detail("key2", "value2");
    
    assert_eq!(context.operation, "test_operation");
    assert_eq!(context.details.get("key1"), Some(&"value1".to_string()));
    assert_eq!(context.details.get("key2"), Some(&"value2".to_string()));
    assert!(context.details.get("nonexistent").is_none());
}

/// Test specific error types
#[test]
fn test_specific_error_types() {
    let agent_id = AgentId::new("test", "agent").unwrap();
    
    // Test agent not found error
    let error = AutoGenError::agent_not_found(agent_id.clone(), vec![]);
    assert!(error.to_string().contains("Agent not found"));
    
    // Test tool not found error
    let error = AutoGenError::tool_not_found("nonexistent_tool", vec!["tool1".to_string(), "tool2".to_string()]);
    assert!(error.to_string().contains("Tool 'nonexistent_tool' not found"));
    
    // Test invalid arguments error
    let error = AutoGenError::invalid_arguments("test_operation", "invalid format", Some("expected schema".to_string()));
    assert!(error.to_string().contains("Invalid arguments for test_operation"));
}

/// Test error chain and source tracking
#[test]
fn test_error_chain() {
    use std::error::Error;
    
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let context = ErrorContext::new("file_operation");
    let autogen_error = AutoGenError::runtime(context, io_error);
    
    // Test that the source error is preserved
    assert!(autogen_error.source().is_some());
    assert!(autogen_error.to_string().contains("File not found"));
}

/// Test cancellation scenarios
#[tokio::test]
async fn test_cancellation_scenarios() {
    let token = CancellationToken::new();
    
    // Test immediate cancellation
    token.cancel();
    assert!(token.is_cancelled());
    
    // Test child token cancellation
    let parent = CancellationToken::new();
    let child = parent.child_token();
    
    parent.cancel();
    assert!(child.is_cancelled());
    
    // Test multiple child tokens
    let parent = CancellationToken::new();
    let child1 = parent.child_token();
    let child2 = parent.child_token();
    
    parent.cancel();
    assert!(child1.is_cancelled());
    assert!(child2.is_cancelled());
}

/// Test timeout scenarios
#[tokio::test]
async fn test_timeout_scenarios() {
    let error = AutoGenError::timeout(5000);
    assert!(error.to_string().contains("timed out after 5000ms"));
    assert!(error.is_timeout());
    assert!(!error.is_cancelled());
}

/// Test validation error scenarios
#[test]
fn test_validation_error_scenarios() {
    let context = ErrorContext::new("validation_test");
    let error = AutoGenError::validation(context, "Invalid input format");
    
    assert!(error.to_string().contains("Invalid input format"));
    assert!(error.to_string().contains("validation_test"));
}

/// Test concurrent error handling
#[tokio::test]
async fn test_concurrent_error_handling() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let error_count = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let error_count_clone = error_count.clone();
        let handle = tokio::spawn(async move {
            // Simulate an operation that might fail
            if i % 2 == 0 {
                let context = ErrorContext::new("concurrent_test");
                let _error = AutoGenError::validation(context, format!("Error {}", i));
                let mut count = error_count_clone.lock().await;
                *count += 1;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    let final_count = *error_count.lock().await;
    assert_eq!(final_count, 5); // Half of the operations should have "failed"
}
