//! Basic usage examples for autogen-core
//!
//! This example demonstrates the core concepts and basic usage patterns.

use autogen_core::*;
use std::collections::HashMap;

/// A simple calculator agent that can perform basic arithmetic
#[derive(Debug)]
struct CalculatorAgent {
    id: AgentId,
}

/// Calculator request message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CalculatorRequest {
    operation: String,
    a: f64,
    b: f64,
}

impl Message for CalculatorRequest {
    type Response = CalculatorResponse;
}

/// Calculator response message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CalculatorResponse {
    result: f64,
    success: bool,
    error: Option<String>,
}

impl Message for CalculatorResponse {
    type Response = NoResponse;
}

impl CalculatorAgent {
    fn new(key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new("calculator", key)?,
        })
    }

    fn calculate(&self, operation: &str, a: f64, b: f64) -> Result<f64> {
        match operation {
            "add" => Ok(a + b),
            "subtract" => Ok(a - b),
            "multiply" => Ok(a * b),
            "divide" => {
                if b == 0.0 {
                    Err(AutoGenError::other("Division by zero"))
                } else {
                    Ok(a / b)
                }
            }
            _ => Err(AutoGenError::other(format!("Unknown operation: {}", operation))),
        }
    }
}

#[async_trait::async_trait]
impl TypedAgent<CalculatorRequest> for CalculatorAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: CalculatorRequest,
        _context: &MessageContext,
    ) -> Result<Option<CalculatorResponse>> {
        println!("Calculator: {} {} {}", message.a, message.operation, message.b);

        match self.calculate(&message.operation, message.a, message.b) {
            Ok(result) => {
                println!("Result: {}", result);
                Ok(Some(CalculatorResponse {
                    result,
                    success: true,
                    error: None,
                }))
            }
            Err(e) => {
                println!("Error: {}", e);
                Ok(Some(CalculatorResponse {
                    result: 0.0,
                    success: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
}

/// A logging agent that records all text messages
#[derive(Debug)]
struct LoggingAgent {
    id: AgentId,
    logs: Vec<String>,
}

impl LoggingAgent {
    fn new(key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new("logger", key)?,
            logs: Vec::new(),
        })
    }

    fn get_logs(&self) -> &[String] {
        &self.logs
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for LoggingAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TextMessage,
        context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let log_entry = format!(
            "[{}] {}: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            context.sender.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string()),
            message.content
        );
        
        println!("LOG: {}", log_entry);
        self.logs.push(log_entry);
        
        Ok(Some(NoResponse))
    }
}

/// Example of using memory system
async fn memory_example() -> Result<()> {
    use autogen_core::memory::{ListMemory, Memory, MemoryContent};

    println!("\n=== Memory Example ===");

    // Create a memory instance
    let memory = ListMemory::new();

    // Add some content
    let content1 = MemoryContent::text("The user prefers concise responses");
    let content2 = MemoryContent::json(serde_json::json!({
        "user_id": "user123",
        "preferences": {
            "language": "en",
            "format": "markdown"
        }
    }));

    memory.add(content1, None).await?;
    memory.add(content2, None).await?;

    // Query the memory
    let results = memory.query("user preferences", None).await?;
    println!("Memory query results: {} items", results.results.len());

    for (i, result) in results.results.iter().enumerate() {
        println!("  {}: {:?}", i + 1, result.content);
    }

    Ok(())
}

/// Example of error handling
async fn error_handling_example() -> Result<()> {
    println!("\n=== Error Handling Example ===");

    // Test invalid agent ID
    match AgentId::new("invalid/type", "key") {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }

    // Test calculator with division by zero
    let mut calc = CalculatorAgent::new("error_test")?;
    let request = CalculatorRequest {
        operation: "divide".to_string(),
        a: 10.0,
        b: 0.0,
    };

    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    match calc.handle_message(request, &context).await? {
        Some(response) => {
            if !response.success {
                println!("Calculator error handled: {:?}", response.error);
            }
        }
        None => println!("No response from calculator"),
    }

    Ok(())
}

/// Example of cancellation
async fn cancellation_example() -> Result<()> {
    println!("\n=== Cancellation Example ===");

    let parent_token = CancellationToken::new();
    let child_token = parent_token.child_token();

    println!("Before cancellation:");
    println!("  Parent cancelled: {}", parent_token.is_cancelled());
    println!("  Child cancelled: {}", child_token.is_cancelled());

    parent_token.cancel();

    println!("After parent cancellation:");
    println!("  Parent cancelled: {}", parent_token.is_cancelled());
    println!("  Child cancelled: {}", child_token.is_cancelled());

    Ok(())
}

/// Example of agent adapter usage
async fn agent_adapter_example() -> Result<()> {
    println!("\n=== Agent Adapter Example ===");

    let logger = LoggingAgent::new("adapter_test")?;
    let mut adapter = TypedAgentAdapter::new(logger);

    // Send a text message (correct type)
    let text_msg = TextMessage {
        content: "This is a test message".to_string(),
    };

    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);

    match adapter.on_message(Box::new(text_msg), &context).await {
        Ok(_) => println!("Message handled successfully"),
        Err(e) => println!("Error: {}", e),
    }

    // Try to send wrong message type
    let calc_msg = CalculatorRequest {
        operation: "add".to_string(),
        a: 1.0,
        b: 2.0,
    };

    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);

    match adapter.on_message(Box::new(calc_msg), &context).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error for wrong message type: {}", e),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("AutoGen Core Rust - Basic Usage Examples");
    println!("=========================================");

    // Basic agent usage
    println!("\n=== Basic Agent Usage ===");
    
    let mut calculator = CalculatorAgent::new("main")?;
    let mut logger = LoggingAgent::new("main")?;

    // Create some messages
    let calc_request = CalculatorRequest {
        operation: "add".to_string(),
        a: 15.0,
        b: 25.0,
    };

    let log_message = TextMessage {
        content: "Starting calculation".to_string(),
    };

    // Create contexts
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(Some(calculator.id().clone()), token);

    // Handle messages
    logger.handle_message(log_message, &context).await?;
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    if let Some(response) = calculator.handle_message(calc_request, &context).await? {
        println!("Calculation completed: success={}, result={}", response.success, response.result);
    }

    // Show logs
    println!("\nLogger contents:");
    for log in logger.get_logs() {
        println!("  {}", log);
    }

    // Run other examples
    memory_example().await?;
    error_handling_example().await?;
    cancellation_example().await?;
    agent_adapter_example().await?;

    println!("\nAll examples completed successfully!");
    Ok(())
}
