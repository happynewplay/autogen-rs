//! Advanced features demonstration for autogen-core
//!
//! This example shows more sophisticated usage patterns including
//! custom tools, memory integration, and error recovery.

use autogen_core::*;
use autogen_core::tools::*;
use autogen_core::memory::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Custom tool arguments for a weather service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WeatherArgs {
    city: String,
    country: Option<String>,
}

impl ToolArgs for WeatherArgs {
    fn validate(&self) -> Result<()> {
        if self.city.is_empty() {
            return Err(AutoGenError::invalid_arguments(
                "weather_tool",
                "City name cannot be empty",
                Some("{ \"city\": \"string\", \"country\": \"string?\" }".to_string())
            ));
        }
        Ok(())
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "Name of the city"
                },
                "country": {
                    "type": "string",
                    "description": "Country code (optional)"
                }
            },
            "required": ["city"]
        })
    }
}

/// Weather tool result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WeatherResult {
    city: String,
    temperature: f64,
    condition: String,
    humidity: u32,
}

impl ToolReturn for WeatherResult {}

/// Mock weather service tool
#[derive(Debug)]
struct WeatherTool {
    name: String,
    description: String,
}

impl WeatherTool {
    fn new() -> Self {
        Self {
            name: "get_weather".to_string(),
            description: "Get current weather information for a city".to_string(),
        }
    }

    // Simulate weather API call
    async fn fetch_weather(&self, city: &str, _country: Option<&str>) -> Result<WeatherResult> {
        // Simulate API delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Mock weather data
        let weather = match city.to_lowercase().as_str() {
            "london" => WeatherResult {
                city: "London".to_string(),
                temperature: 15.5,
                condition: "Cloudy".to_string(),
                humidity: 78,
            },
            "tokyo" => WeatherResult {
                city: "Tokyo".to_string(),
                temperature: 22.3,
                condition: "Sunny".to_string(),
                humidity: 65,
            },
            "new york" => WeatherResult {
                city: "New York".to_string(),
                temperature: 18.7,
                condition: "Rainy".to_string(),
                humidity: 82,
            },
            _ => {
                return Err(AutoGenError::tool(
                    crate::error::ErrorContext::new("weather_fetch")
                        .with_detail("city", city),
                    format!("Weather data not available for {}", city),
                    Some("get_weather".to_string())
                ));
            }
        };

        Ok(weather)
    }
}

#[async_trait::async_trait]
impl TypedTool<WeatherArgs, WeatherResult> for WeatherTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(
        &self,
        args: WeatherArgs,
        _cancellation_token: Option<CancellationToken>,
        _call_id: Option<String>,
    ) -> Result<WeatherResult> {
        args.validate()?;
        self.fetch_weather(&args.city, args.country.as_deref()).await
    }
}

/// Weather agent that uses the weather tool
#[derive(Debug)]
struct WeatherAgent {
    id: AgentId,
    weather_tool: WeatherTool,
    memory: Arc<dyn Memory>,
}

/// Weather query message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WeatherQuery {
    query: String,
}

impl Message for WeatherQuery {
    type Response = TextMessage;
}

impl WeatherAgent {
    fn new(key: &str, memory: Arc<dyn Memory>) -> Result<Self> {
        Ok(Self {
            id: AgentId::new("weather_agent", key)?,
            weather_tool: WeatherTool::new(),
            memory,
        })
    }

    async fn parse_city_from_query(&self, query: &str) -> Option<String> {
        // Simple city extraction (in real implementation, use NLP)
        let query_lower = query.to_lowercase();
        
        if query_lower.contains("london") {
            Some("London".to_string())
        } else if query_lower.contains("tokyo") {
            Some("Tokyo".to_string())
        } else if query_lower.contains("new york") {
            Some("New York".to_string())
        } else {
            None
        }
    }

    async fn remember_query(&self, query: &str, response: &str) -> Result<()> {
        let memory_content = MemoryContent::json(serde_json::json!({
            "type": "weather_query",
            "query": query,
            "response": response,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));

        self.memory.add(memory_content, None).await
    }
}

#[async_trait::async_trait]
impl TypedAgent<WeatherQuery> for WeatherAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: WeatherQuery,
        _context: &MessageContext,
    ) -> Result<Option<TextMessage>> {
        println!("Weather Agent: Processing query '{}'", message.query);

        // Try to extract city from query
        if let Some(city) = self.parse_city_from_query(&message.query).await {
            let args = WeatherArgs {
                city: city.clone(),
                country: None,
            };

            match self.weather_tool.execute(args, None, None).await {
                Ok(weather) => {
                    let response = format!(
                        "Weather in {}: {}°C, {}, {}% humidity",
                        weather.city, weather.temperature, weather.condition, weather.humidity
                    );

                    // Remember this interaction
                    if let Err(e) = self.remember_query(&message.query, &response).await {
                        println!("Warning: Failed to save to memory: {}", e);
                    }

                    Ok(Some(TextMessage { content: response }))
                }
                Err(e) => {
                    let error_response = format!("Sorry, I couldn't get weather data: {}", e);
                    Ok(Some(TextMessage { content: error_response }))
                }
            }
        } else {
            let response = "I couldn't understand which city you're asking about. Please specify a city name.".to_string();
            Ok(Some(TextMessage { content: response }))
        }
    }
}

/// Conversation manager that coordinates multiple agents
#[derive(Debug)]
struct ConversationManager {
    id: AgentId,
    weather_agent: Arc<Mutex<WeatherAgent>>,
    memory: Arc<dyn Memory>,
}

impl ConversationManager {
    fn new(key: &str, weather_agent: Arc<Mutex<WeatherAgent>>, memory: Arc<dyn Memory>) -> Result<Self> {
        Ok(Self {
            id: AgentId::new("conversation_manager", key)?,
            weather_agent,
            memory,
        })
    }

    async fn route_message(&self, content: &str) -> Result<String> {
        let content_lower = content.to_lowercase();
        
        if content_lower.contains("weather") || content_lower.contains("temperature") {
            // Route to weather agent
            let query = WeatherQuery {
                query: content.to_string(),
            };

            let token = CancellationToken::new();
            let context = MessageContext::direct_message(Some(self.id.clone()), token);

            let mut weather_agent = self.weather_agent.lock().await;
            match weather_agent.handle_message(query, &context).await? {
                Some(response) => Ok(response.content),
                None => Ok("Weather agent didn't provide a response".to_string()),
            }
        } else {
            Ok("I can help you with weather information. Try asking about the weather in a city!".to_string())
        }
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for ConversationManager {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        println!("Manager: Received '{}'", message.content);

        match self.route_message(&message.content).await {
            Ok(response) => {
                println!("Manager: Response '{}'", response);
                
                // Save conversation to memory
                let memory_content = MemoryContent::json(serde_json::json!({
                    "type": "conversation",
                    "user_message": message.content,
                    "bot_response": response,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }));

                if let Err(e) = self.memory.add(memory_content, None).await {
                    println!("Warning: Failed to save conversation: {}", e);
                }
            }
            Err(e) => {
                println!("Manager: Error processing message: {}", e);
            }
        }

        Ok(Some(NoResponse))
    }
}

/// Demonstrate error recovery patterns
async fn error_recovery_example() -> Result<()> {
    println!("\n=== Error Recovery Example ===");

    let weather_tool = WeatherTool::new();

    // Test with valid city
    let valid_args = WeatherArgs {
        city: "London".to_string(),
        country: Some("UK".to_string()),
    };

    match weather_tool.execute(valid_args, None, None).await {
        Ok(result) => println!("Success: {:?}", result),
        Err(e) => println!("Unexpected error: {}", e),
    }

    // Test with invalid city
    let invalid_args = WeatherArgs {
        city: "NonexistentCity".to_string(),
        country: None,
    };

    match weather_tool.execute(invalid_args, None, None).await {
        Ok(result) => println!("Unexpected success: {:?}", result),
        Err(e) => println!("Expected error handled: {}", e),
    }

    // Test with empty city (validation error)
    let empty_args = WeatherArgs {
        city: "".to_string(),
        country: None,
    };

    match weather_tool.execute(empty_args, None, None).await {
        Ok(result) => println!("Unexpected success: {:?}", result),
        Err(e) => println!("Validation error handled: {}", e),
    }

    Ok(())
}

/// Demonstrate memory integration
async fn memory_integration_example() -> Result<()> {
    println!("\n=== Memory Integration Example ===");

    let memory: Arc<dyn Memory> = Arc::new(ListMemory::new(Some("weather_memory".to_string())));

    // Create weather agent with memory
    let weather_agent = WeatherAgent::new("memory_test", memory.clone())?;
    let weather_agent = Arc::new(Mutex::new(weather_agent));

    // Create conversation manager
    let mut manager = ConversationManager::new("main", weather_agent, memory.clone())?;

    // Simulate conversation
    let messages = vec![
        "What's the weather like in London?",
        "How about Tokyo?",
        "Tell me about New York weather",
        "What's the temperature in Paris?", // This should fail
    ];

    for msg in messages {
        let text_msg = TextMessage {
            content: msg.to_string(),
        };

        let token = CancellationToken::new();
        let context = MessageContext::direct_message(None, token);

        manager.handle_message(text_msg, &context).await?;
        
        // Small delay between messages
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Query memory for conversation history
    println!("\nQuerying memory for conversation history:");
    let results = memory.query("conversation".into(), None).await?;
    println!("Found {} conversation entries", results.results.len());

    for (i, result) in results.results.iter().enumerate() {
        if let ContentType::Json(data) = &result.content {
            if let Some(user_msg) = data.get("user_message") {
                if let Some(bot_response) = data.get("bot_response") {
                    println!("  {}: User: {} -> Bot: {}", i + 1, user_msg, bot_response);
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("AutoGen Core Rust - Advanced Features");
    println!("=====================================");

    // Run examples
    error_recovery_example().await?;
    memory_integration_example().await?;

    println!("\nAll advanced examples completed successfully!");
    Ok(())
}
