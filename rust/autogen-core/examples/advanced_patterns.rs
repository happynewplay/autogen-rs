//! Advanced patterns example for autogen-core
//!
//! This example demonstrates advanced usage patterns including:
//! - Custom message types and routing
//! - Agent composition and delegation
//! - Event-driven architectures
//! - State management patterns

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use autogen_core::{
    AgentId, TextMessage, MessageContext, CancellationToken, Result, AutoGenError,
    TypedAgent, NoResponse, TopicId, Subscription, DefaultSubscription,
};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 AutoGen Core - Advanced Patterns Example");
    
    // Demonstrate custom message types
    custom_message_demo().await?;
    
    // Demonstrate agent composition
    agent_composition_demo().await?;
    
    // Demonstrate event-driven patterns
    event_driven_demo().await?;
    
    // Demonstrate state management
    state_management_demo().await?;
    
    Ok(())
}

/// Demonstrate custom message types and routing
async fn custom_message_demo() -> Result<()> {
    println!("\n=== Custom Message Types Demo ===");
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Create different types of custom messages
    let command_msg = CommandMessage {
        action: "start".to_string(),
        parameters: vec!["param1".to_string(), "param2".to_string()],
        priority: Priority::High,
    };
    
    let query_msg = QueryMessage {
        query: "SELECT * FROM users".to_string(),
        database: "production".to_string(),
        timeout_ms: 5000,
    };
    
    let event_msg = EventMessage {
        event_type: "user_login".to_string(),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "user_id": "12345",
            "ip_address": "192.168.1.1"
        }),
    };
    
    println!("Created command message: {:?}", command_msg);
    println!("Created query message: {:?}", query_msg);
    println!("Created event message: {:?}", event_msg);
    
    // Demonstrate message validation
    match command_msg.validate() {
        Ok(_) => println!("Command message validation passed"),
        Err(e) => println!("Command message validation failed: {}", e),
    }
    
    Ok(())
}

/// Demonstrate agent composition patterns
async fn agent_composition_demo() -> Result<()> {
    println!("\n=== Agent Composition Demo ===");
    
    // Create a composite agent that delegates to specialized agents
    let mut composite_agent = CompositeAgent::new("composite", "main")?;
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Test different message types
    let messages = vec![
        TextMessage { content: "command:start_service".to_string() },
        TextMessage { content: "query:get_users".to_string() },
        TextMessage { content: "event:user_action".to_string() },
        TextMessage { content: "unknown:invalid".to_string() },
    ];
    
    for message in messages {
        println!("\nProcessing message: {}", message.content);
        match composite_agent.handle_message(message, &context).await {
            Ok(response) => println!("Response: {:?}", response),
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}

/// Demonstrate event-driven patterns
async fn event_driven_demo() -> Result<()> {
    println!("\n=== Event-Driven Patterns Demo ===");
    
    // Create event publisher and subscribers
    let mut publisher = EventPublisher::new("publisher", "main")?;
    let mut subscriber1 = EventSubscriber::new("subscriber", "analytics")?;
    let mut subscriber2 = EventSubscriber::new("subscriber", "logging")?;
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Simulate event publishing
    let events = vec![
        "user_login:john_doe",
        "order_placed:order_123",
        "payment_processed:payment_456",
    ];
    
    for event in events {
        println!("\nPublishing event: {}", event);
        
        let message = TextMessage { content: event.to_string() };
        
        // Publisher processes the event
        publisher.handle_message(message.clone(), &context).await?;
        
        // Subscribers process the event
        subscriber1.handle_message(message.clone(), &context).await?;
        subscriber2.handle_message(message, &context).await?;
    }
    
    Ok(())
}

/// Demonstrate state management patterns
async fn state_management_demo() -> Result<()> {
    println!("\n=== State Management Demo ===");
    
    // Create a stateful agent
    let mut stateful_agent = StatefulAgent::new("stateful", "counter")?;
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Test state operations
    let operations = vec![
        "increment",
        "increment",
        "get",
        "decrement",
        "get",
        "reset",
        "get",
    ];
    
    for operation in operations {
        println!("\nOperation: {}", operation);
        
        let message = TextMessage { content: operation.to_string() };
        
        match stateful_agent.handle_message(message, &context).await {
            Ok(response) => println!("Response: {:?}", response),
            Err(e) => println!("Error: {}", e),
        }
        
        println!("Current state: {:?}", stateful_agent.get_state().await);
    }
    
    Ok(())
}

// Custom message types

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandMessage {
    action: String,
    parameters: Vec<String>,
    priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl CommandMessage {
    fn validate(&self) -> Result<()> {
        if self.action.is_empty() {
            return Err(AutoGenError::other("Command action cannot be empty"));
        }
        if self.parameters.len() > 10 {
            return Err(AutoGenError::other("Too many parameters (max 10)"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryMessage {
    query: String,
    database: String,
    timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventMessage {
    event_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    data: serde_json::Value,
}

// Agent implementations

/// Composite agent that delegates to specialized handlers
#[derive(Debug)]
struct CompositeAgent {
    id: AgentId,
    command_handler: CommandHandler,
    query_handler: QueryHandler,
    event_handler: EventHandler,
}

impl CompositeAgent {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
            command_handler: CommandHandler::new(),
            query_handler: QueryHandler::new(),
            event_handler: EventHandler::new(),
        })
    }
}

#[async_trait]
impl TypedAgent<TextMessage> for CompositeAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }
    
    async fn handle_message(
        &mut self,
        message: TextMessage,
        context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let parts: Vec<&str> = message.content.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AutoGenError::other("Invalid message format"));
        }
        
        let (message_type, content) = (parts[0], parts[1]);
        
        match message_type {
            "command" => {
                println!("  Delegating to command handler");
                self.command_handler.handle(content).await
            }
            "query" => {
                println!("  Delegating to query handler");
                self.query_handler.handle(content).await
            }
            "event" => {
                println!("  Delegating to event handler");
                self.event_handler.handle(content).await
            }
            _ => {
                Err(AutoGenError::other(format!("Unknown message type: {}", message_type)))
            }
        }
    }
}

// Specialized handlers

#[derive(Debug)]
struct CommandHandler;

impl CommandHandler {
    fn new() -> Self {
        Self
    }
    
    async fn handle(&mut self, command: &str) -> Result<Option<NoResponse>> {
        println!("    Executing command: {}", command);
        // Simulate command execution
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(Some(NoResponse))
    }
}

#[derive(Debug)]
struct QueryHandler;

impl QueryHandler {
    fn new() -> Self {
        Self
    }
    
    async fn handle(&mut self, query: &str) -> Result<Option<NoResponse>> {
        println!("    Executing query: {}", query);
        // Simulate query execution
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        Ok(Some(NoResponse))
    }
}

#[derive(Debug)]
struct EventHandler;

impl EventHandler {
    fn new() -> Self {
        Self
    }
    
    async fn handle(&mut self, event: &str) -> Result<Option<NoResponse>> {
        println!("    Processing event: {}", event);
        // Simulate event processing
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        Ok(Some(NoResponse))
    }
}

// Event-driven agents

#[derive(Debug)]
struct EventPublisher {
    id: AgentId,
    published_count: u64,
}

impl EventPublisher {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
            published_count: 0,
        })
    }
}

#[async_trait]
impl TypedAgent<TextMessage> for EventPublisher {
    fn id(&self) -> &AgentId {
        &self.id
    }
    
    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        self.published_count += 1;
        println!("  Publisher: Published event '{}' (total: {})", 
                message.content, self.published_count);
        Ok(Some(NoResponse))
    }
}

#[derive(Debug)]
struct EventSubscriber {
    id: AgentId,
    processed_count: u64,
    subscriber_type: String,
}

impl EventSubscriber {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
            processed_count: 0,
            subscriber_type: key.to_string(),
        })
    }
}

#[async_trait]
impl TypedAgent<TextMessage> for EventSubscriber {
    fn id(&self) -> &AgentId {
        &self.id
    }
    
    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        self.processed_count += 1;
        println!("  Subscriber ({}): Processed event '{}' (total: {})", 
                self.subscriber_type, message.content, self.processed_count);
        Ok(Some(NoResponse))
    }
}

// Stateful agent

#[derive(Debug, Clone)]
struct AgentState {
    counter: i32,
    last_operation: String,
    operation_count: u64,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            counter: 0,
            last_operation: "none".to_string(),
            operation_count: 0,
        }
    }
}

#[derive(Debug)]
struct StatefulAgent {
    id: AgentId,
    state: Arc<Mutex<AgentState>>,
}

impl StatefulAgent {
    fn new(agent_type: &str, key: &str) -> Result<Self> {
        Ok(Self {
            id: AgentId::new(agent_type, key)?,
            state: Arc::new(Mutex::new(AgentState::default())),
        })
    }
    
    async fn get_state(&self) -> AgentState {
        self.state.lock().await.clone()
    }
}

#[async_trait]
impl TypedAgent<TextMessage> for StatefulAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }
    
    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let mut state = self.state.lock().await;
        state.operation_count += 1;
        state.last_operation = message.content.clone();
        
        match message.content.as_str() {
            "increment" => {
                state.counter += 1;
                println!("  Counter incremented to: {}", state.counter);
            }
            "decrement" => {
                state.counter -= 1;
                println!("  Counter decremented to: {}", state.counter);
            }
            "get" => {
                println!("  Current counter value: {}", state.counter);
            }
            "reset" => {
                state.counter = 0;
                println!("  Counter reset to: {}", state.counter);
            }
            _ => {
                return Err(AutoGenError::other(format!("Unknown operation: {}", message.content)));
            }
        }
        
        Ok(Some(NoResponse))
    }
}
