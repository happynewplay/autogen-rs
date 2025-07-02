//! Example demonstrating the RoutedAgent functionality
//!
//! This example shows how to create and use a routed agent that automatically
//! routes messages to appropriate handlers based on message type.

use autogen_core::{
    Agent, AgentId, MessageContext, Result, RoutedAgent, TypeSafeMessage,
    TextMessage, FunctionCall, CancellationToken
};
use async_trait::async_trait;

/// Example routed agent that handles different message types
pub struct ExampleRoutedAgent {
    id: AgentId,
    state: String,
}

impl ExampleRoutedAgent {
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            state: String::new(),
        }
    }
}

#[async_trait]
impl Agent for ExampleRoutedAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TypeSafeMessage,
        context: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        // Delegate to the routed agent implementation
        self.route_message(message, context).await
    }
}

#[async_trait]
impl RoutedAgent for ExampleRoutedAgent {
    /// Handle text messages
    async fn handle_text_message(
        &mut self,
        message: TextMessage,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        println!("Received text message: {}", message.content);
        self.state = format!("Last text: {}", message.content);
        
        // Echo the message back
        Ok(Some(TypeSafeMessage::text(format!("Echo: {}", message.content))))
    }

    /// Handle function call messages
    async fn handle_function_call_message(
        &mut self,
        message: FunctionCall,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        println!("Received function call: {} with args: {}", message.name, message.arguments);
        
        match message.name.as_str() {
            "get_state" => {
                Ok(Some(TypeSafeMessage::text(format!("Current state: {}", self.state))))
            }
            "set_state" => {
                // Parse arguments to set state
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&message.arguments) {
                    if let Some(new_state) = args.get("state").and_then(|v| v.as_str()) {
                        self.state = new_state.to_string();
                        Ok(Some(TypeSafeMessage::text("State updated successfully")))
                    } else {
                        Ok(Some(TypeSafeMessage::text("Error: 'state' parameter required")))
                    }
                } else {
                    Ok(Some(TypeSafeMessage::text("Error: Invalid JSON arguments")))
                }
            }
            _ => {
                Ok(Some(TypeSafeMessage::text(format!("Unknown function: {}", message.name))))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Routed Agent Example ===");
    
    // Create an agent
    let agent_id = AgentId::new("example", "routed_agent_1")?;
    let mut agent = ExampleRoutedAgent::new(agent_id.clone());
    
    // Create a message context
    let cancellation_token = CancellationToken::new();
    let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
    
    // Test text message handling
    println!("\n1. Testing text message handling:");
    let text_msg = TypeSafeMessage::text("Hello, routed agent!");
    if let Some(response) = agent.handle_message(text_msg, &context).await? {
        println!("Response: {:?}", response);
    }
    
    // Test function call - get_state
    println!("\n2. Testing function call - get_state:");
    let get_state_call = TypeSafeMessage::function_call(
        "call_1",
        "get_state",
        "{}",
    );
    if let Some(response) = agent.handle_message(get_state_call, &context).await? {
        println!("Response: {:?}", response);
    }
    
    // Test function call - set_state
    println!("\n3. Testing function call - set_state:");
    let set_state_call = TypeSafeMessage::function_call(
        "call_2",
        "set_state",
        r#"{"state": "New agent state"}"#,
    );
    if let Some(response) = agent.handle_message(set_state_call, &context).await? {
        println!("Response: {:?}", response);
    }
    
    // Test get_state again to verify state change
    println!("\n4. Testing get_state again:");
    let get_state_call2 = TypeSafeMessage::function_call(
        "call_3",
        "get_state",
        "{}",
    );
    if let Some(response) = agent.handle_message(get_state_call2, &context).await? {
        println!("Response: {:?}", response);
    }
    
    // Test unknown function
    println!("\n5. Testing unknown function:");
    let unknown_call = TypeSafeMessage::function_call(
        "call_4",
        "unknown_function",
        "{}",
    );
    if let Some(response) = agent.handle_message(unknown_call, &context).await? {
        println!("Response: {:?}", response);
    }
    
    println!("\n=== Example completed successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_routed_agent_text_handling() {
        let agent_id = AgentId::new("test", "agent").unwrap();
        let mut agent = ExampleRoutedAgent::new(agent_id.clone());
        
        let cancellation_token = CancellationToken::new();
        let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
        
        let text_msg = TypeSafeMessage::text("Test message");
        let response = agent.handle_message(text_msg, &context).await.unwrap();
        
        assert!(response.is_some());
        if let Some(TypeSafeMessage::Text(text_response)) = response {
            assert_eq!(text_response.content, "Echo: Test message");
        } else {
            panic!("Expected text response");
        }
    }

    #[tokio::test]
    async fn test_routed_agent_function_call() {
        let agent_id = AgentId::new("test", "agent").unwrap();
        let mut agent = ExampleRoutedAgent::new(agent_id.clone());
        
        let cancellation_token = CancellationToken::new();
        let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
        
        // Test set_state
        let set_state_call = TypeSafeMessage::function_call(
            "test_call",
            "set_state",
            r#"{"state": "test_state"}"#,
        );
        let response = agent.handle_message(set_state_call, &context).await.unwrap();
        assert!(response.is_some());
        
        // Test get_state
        let get_state_call = TypeSafeMessage::function_call(
            "test_call_2",
            "get_state",
            "{}",
        );
        let response = agent.handle_message(get_state_call, &context).await.unwrap();
        
        assert!(response.is_some());
        if let Some(TypeSafeMessage::Text(text_response)) = response {
            assert_eq!(text_response.content, "Current state: test_state");
        } else {
            panic!("Expected text response");
        }
    }
}
