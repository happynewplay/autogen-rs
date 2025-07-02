//! Error handling example for autogen-core
//!
//! This example demonstrates comprehensive error handling patterns,
//! recovery strategies, and best practices for robust agent systems.

use std::time::Duration;
use autogen_core::{
    AgentId, TextMessage, MessageContext, CancellationToken, Result, AutoGenError,
    TypedAgent, NoResponse,
};
use async_trait::async_trait;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 AutoGen Core - Error Handling Example");
    
    // Demonstrate different error types
    error_types_demo().await?;
    
    // Demonstrate error recovery
    error_recovery_demo().await?;
    
    // Demonstrate timeout handling
    timeout_handling_demo().await?;
    
    // Demonstrate validation errors
    validation_errors_demo().await?;
    
    // Demonstrate agent error handling
    agent_error_handling_demo().await?;
    
    Ok(())
}

/// Demonstrate different types of errors that can occur
async fn error_types_demo() -> Result<()> {
    println!("\n=== Error Types Demo ===");
    
    // 1. Validation errors
    println!("1. Validation Error:");
    let invalid_agent_id = AgentId::new("", "key");
    match invalid_agent_id {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   Error: {}", e),
    }
    
    // 2. Serialization errors
    println!("\n2. Serialization Error:");
    #[cfg(feature = "json")]
    {
        use autogen_core::message_v2::EfficientMessage;
        
        // Create a message with invalid UTF-8 to trigger serialization error
        let problematic_message = TextMessage {
            content: "Valid content".to_string(),
        };
        
        match problematic_message.serialize() {
            Ok(_) => println!("   Serialization succeeded"),
            Err(e) => println!("   Serialization error: {}", e),
        }
    }
    
    // 3. Agent not found errors
    println!("\n3. Agent Not Found Error:");
    let non_existent_agent = AgentId::new("non_existent", "agent")?;
    println!("   Would fail when trying to send message to: {}", non_existent_agent);
    
    // 4. Message type mismatch
    println!("\n4. Message Type Mismatch:");
    println!("   This would occur when an agent receives an unexpected message type");
    
    Ok(())
}

/// Demonstrate error recovery strategies
async fn error_recovery_demo() -> Result<()> {
    println!("\n=== Error Recovery Demo ===");
    
    // 1. Retry with exponential backoff
    println!("1. Retry with Exponential Backoff:");
    let mut attempt = 0;
    let max_attempts = 3;
    
    loop {
        attempt += 1;
        println!("   Attempt {}/{}", attempt, max_attempts);
        
        // Simulate an operation that might fail
        let result = simulate_flaky_operation(attempt).await;
        
        match result {
            Ok(value) => {
                println!("   Success: {}", value);
                break;
            }
            Err(e) if attempt < max_attempts => {
                println!("   Failed: {} - Retrying...", e);
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                println!("   Final failure after {} attempts: {}", max_attempts, e);
                break;
            }
        }
    }
    
    // 2. Graceful degradation
    println!("\n2. Graceful Degradation:");
    let primary_result = simulate_primary_service().await;
    let final_result = match primary_result {
        Ok(value) => {
            println!("   Primary service succeeded: {}", value);
            value
        }
        Err(e) => {
            println!("   Primary service failed: {} - Using fallback", e);
            simulate_fallback_service().await?
        }
    };
    println!("   Final result: {}", final_result);
    
    Ok(())
}

/// Demonstrate timeout handling
async fn timeout_handling_demo() -> Result<()> {
    println!("\n=== Timeout Handling Demo ===");
    
    // 1. Operation with timeout
    println!("1. Operation with Timeout:");
    let timeout_duration = Duration::from_millis(500);
    
    let result = tokio::time::timeout(
        timeout_duration,
        simulate_slow_operation()
    ).await;
    
    match result {
        Ok(Ok(value)) => println!("   Operation completed: {}", value),
        Ok(Err(e)) => println!("   Operation failed: {}", e),
        Err(_) => println!("   Operation timed out after {:?}", timeout_duration),
    }
    
    // 2. Cancellation token usage
    println!("\n2. Cancellation Token:");
    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    // Simulate cancellation after a delay
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        token_clone.cancel();
        println!("   Cancellation requested");
    });
    
    let result = simulate_cancellable_operation(token).await;
    match result {
        Ok(value) => println!("   Operation completed: {}", value),
        Err(e) => println!("   Operation cancelled or failed: {}", e),
    }
    
    Ok(())
}

/// Demonstrate validation errors
async fn validation_errors_demo() -> Result<()> {
    println!("\n=== Validation Errors Demo ===");
    
    // 1. Message validation
    println!("1. Message Validation:");
    
    #[cfg(feature = "json")]
    {
        use autogen_core::message_v2::EfficientMessage;
        
        // Test empty content validation
        let empty_message = TextMessage {
            content: String::new(),
        };
        
        match empty_message.validate() {
            Ok(_) => println!("   Empty message validation passed"),
            Err(e) => println!("   Empty message validation failed: {}", e),
        }
        
        // Test oversized content validation
        let oversized_message = TextMessage {
            content: "A".repeat(20_000),
        };
        
        match oversized_message.validate() {
            Ok(_) => println!("   Oversized message validation passed"),
            Err(e) => println!("   Oversized message validation failed: {}", e),
        }
    }
    
    // 2. Agent ID validation
    println!("\n2. Agent ID Validation:");
    
    let invalid_ids = vec![
        ("", "valid_key"),
        ("valid_type", ""),
        ("invalid type", "valid_key"), // spaces not allowed
        ("valid_type", "invalid key"), // spaces not allowed
    ];
    
    for (agent_type, key) in invalid_ids {
        match AgentId::new(agent_type, key) {
            Ok(id) => println!("   Unexpected success for '{}'/'{}'", agent_type, key),
            Err(e) => println!("   Validation failed for '{}'/'{}'': {}", agent_type, key, e),
        }
    }
    
    Ok(())
}

/// Demonstrate agent-level error handling
async fn agent_error_handling_demo() -> Result<()> {
    println!("\n=== Agent Error Handling Demo ===");
    
    let mut error_prone_agent = ErrorProneAgent::new("error_agent", "demo")?;
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Test different error scenarios
    let test_cases = vec![
        ("normal", "This should work fine"),
        ("error", "This will trigger an error"),
        ("panic", "This will cause a panic"),
        ("timeout", "This will simulate a timeout"),
    ];
    
    for (test_type, content) in test_cases {
        println!("\nTesting '{}' scenario:", test_type);
        
        let message = TextMessage {
            content: format!("{}:{}", test_type, content),
        };
        
        let result = error_prone_agent.handle_message(message, &context).await;
        
        match result {
            Ok(response) => {
                println!("   Success: {:?}", response);
            }
            Err(e) => {
                println!("   Error handled gracefully: {}", e);
            }
        }
    }
    
    Ok(())
}

// Helper functions for demonstrations

async fn simulate_flaky_operation(attempt: u32) -> Result<String> {
    if attempt < 3 {
        Err(AutoGenError::other(format!("Simulated failure on attempt {}", attempt)))
    } else {
        Ok("Operation succeeded!".to_string())
    }
}

async fn simulate_primary_service() -> Result<String> {
    Err(AutoGenError::other("Primary service is down"))
}

async fn simulate_fallback_service() -> Result<String> {
    Ok("Fallback service result".to_string())
}

async fn simulate_slow_operation() -> Result<String> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok("Slow operation completed".to_string())
}

async fn simulate_cancellable_operation(token: CancellationToken) -> Result<String> {
    for i in 0..10 {
        if token.is_cancelled() {
            return Err(AutoGenError::other("Operation was cancelled"));
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("   Working... step {}/10", i + 1);
    }
    
    Ok("Cancellable operation completed".to_string())
}

/// An agent that demonstrates various error scenarios
#[derive(Debug)]
struct ErrorProneAgent {
    id: AgentId,
}

impl ErrorProneAgent {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
        })
    }
}

#[async_trait]
impl TypedAgent<TextMessage> for ErrorProneAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }
    
    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let parts: Vec<&str> = message.content.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AutoGenError::other("Invalid message format"));
        }
        
        let (test_type, _content) = (parts[0], parts[1]);
        
        match test_type {
            "normal" => {
                println!("   Agent processing normal message");
                Ok(Some(NoResponse))
            }
            "error" => {
                Err(AutoGenError::other("Simulated agent error"))
            }
            "panic" => {
                // In a real scenario, this would be caught by the runtime
                Err(AutoGenError::other("Simulated panic scenario"))
            }
            "timeout" => {
                // Simulate a long-running operation
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(Some(NoResponse))
            }
            _ => {
                Err(AutoGenError::other(format!("Unknown test type: {}", test_type)))
            }
        }
    }
}
