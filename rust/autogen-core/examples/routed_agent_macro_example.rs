//! Example demonstrating the simple_routed_agent macro
//!
//! This example shows how to use the simple_routed_agent macro to create agents
//! with automatic message routing.

use autogen_core::{
    Agent, AgentId, MessageContext, Result, TypeSafeMessage,
    TextMessage, FunctionCall, CancellationToken, RoutedAgent, simple_routed_agent
};

// Use the simple_routed_agent macro to create an agent structure
simple_routed_agent! {
    struct MacroRoutedAgent {
        id: AgentId,
        counter: i32,
        messages: Vec<String>,
    }
}

// Implement the specific message handlers
#[async_trait::async_trait]
impl RoutedAgent for MacroRoutedAgent {
    async fn handle_text_message(
        &mut self,
        msg: TextMessage,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        println!("MacroAgent received text: {}", msg.content);
        self.messages.push(msg.content.clone());
        self.counter += 1;

        Ok(Some(TypeSafeMessage::text(format!(
            "Processed message #{}: {}",
            self.counter,
            msg.content
        ))))
    }

    async fn handle_function_call_message(
        &mut self,
        call: FunctionCall,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        println!("MacroAgent received function call: {}", call.name);

        match call.name.as_str() {
            "get_stats" => {
                let stats = format!(
                    "Counter: {}, Messages: {}",
                    self.counter,
                    self.messages.len()
                );
                Ok(Some(TypeSafeMessage::text(stats)))
            }
            "get_messages" => {
                let messages_json = serde_json::to_string(&self.messages)
                    .unwrap_or_else(|_| "[]".to_string());
                Ok(Some(TypeSafeMessage::text(messages_json)))
            }
            "reset" => {
                self.counter = 0;
                self.messages.clear();
                Ok(Some(TypeSafeMessage::text("Reset completed")))
            }
            _ => {
                Ok(Some(TypeSafeMessage::text(format!(
                    "Unknown function: {}",
                    call.name
                ))))
            }
        }
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Routed Agent Macro Example ===");
    
    // Create an agent using the macro-generated struct
    let agent_id = AgentId::new("macro", "agent_1")?;
    let mut agent = MacroRoutedAgent::new(agent_id.clone(), 0, Vec::new());
    
    // Create a message context
    let cancellation_token = CancellationToken::new();
    let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
    
    // Test multiple text messages
    println!("\n1. Testing multiple text messages:");
    let messages = vec![
        "First message",
        "Second message", 
        "Third message"
    ];
    
    for msg_text in messages {
        let text_msg = TypeSafeMessage::text(msg_text);
        if let Some(response) = agent.handle_message(text_msg, &context).await? {
            println!("Response: {:?}", response);
        }
    }
    
    // Test get_stats function
    println!("\n2. Testing get_stats function:");
    let stats_call = TypeSafeMessage::function_call(
        "stats_1",
        "get_stats",
        "{}",
    );
    if let Some(response) = agent.handle_message(stats_call, &context).await? {
        println!("Stats: {:?}", response);
    }
    
    // Test get_messages function
    println!("\n3. Testing get_messages function:");
    let messages_call = TypeSafeMessage::function_call(
        "messages_1",
        "get_messages",
        "{}",
    );
    if let Some(response) = agent.handle_message(messages_call, &context).await? {
        println!("Messages: {:?}", response);
    }
    
    // Test reset function
    println!("\n4. Testing reset function:");
    let reset_call = TypeSafeMessage::function_call(
        "reset_1",
        "reset",
        "{}",
    );
    if let Some(response) = agent.handle_message(reset_call, &context).await? {
        println!("Reset response: {:?}", response);
    }
    
    // Test stats after reset
    println!("\n5. Testing stats after reset:");
    let stats_call2 = TypeSafeMessage::function_call(
        "stats_2",
        "get_stats",
        "{}",
    );
    if let Some(response) = agent.handle_message(stats_call2, &context).await? {
        println!("Stats after reset: {:?}", response);
    }
    
    println!("\n=== Macro example completed successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_macro_routed_agent() {
        let agent_id = AgentId::new("test", "macro_agent").unwrap();
        let mut agent = MacroRoutedAgent::new(agent_id.clone(), 0, Vec::new());
        
        let cancellation_token = CancellationToken::new();
        let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
        
        // Test text message
        let text_msg = TypeSafeMessage::text("Test message");
        let response = agent.handle_message(text_msg, &context).await.unwrap();
        
        assert!(response.is_some());
        if let Some(TypeSafeMessage::Text(text_response)) = response {
            assert!(text_response.content.contains("Processed message #1"));
            assert!(text_response.content.contains("Test message"));
        } else {
            panic!("Expected text response");
        }
        
        // Test function call
        let stats_call = TypeSafeMessage::function_call("test", "get_stats", "{}");
        let response = agent.handle_message(stats_call, &context).await.unwrap();
        
        assert!(response.is_some());
        if let Some(TypeSafeMessage::Text(text_response)) = response {
            assert!(text_response.content.contains("Counter: 1"));
            assert!(text_response.content.contains("Messages: 1"));
        } else {
            panic!("Expected text response");
        }
    }

    #[tokio::test]
    async fn test_macro_agent_reset() {
        let agent_id = AgentId::new("test", "reset_agent").unwrap();
        let mut agent = MacroRoutedAgent::new(agent_id.clone(), 0, Vec::new());
        
        let cancellation_token = CancellationToken::new();
        let context = MessageContext::direct_message(Some(agent_id), cancellation_token);
        
        // Add some messages
        for i in 1..=3 {
            let text_msg = TypeSafeMessage::text(format!("Message {}", i));
            agent.handle_message(text_msg, &context).await.unwrap();
        }
        
        // Reset
        let reset_call = TypeSafeMessage::function_call("test", "reset", "{}");
        let response = agent.handle_message(reset_call, &context).await.unwrap();
        assert!(response.is_some());
        
        // Check stats after reset
        let stats_call = TypeSafeMessage::function_call("test", "get_stats", "{}");
        let response = agent.handle_message(stats_call, &context).await.unwrap();
        
        if let Some(TypeSafeMessage::Text(text_response)) = response {
            assert!(text_response.content.contains("Counter: 0"));
            assert!(text_response.content.contains("Messages: 0"));
        } else {
            panic!("Expected text response");
        }
    }
}
