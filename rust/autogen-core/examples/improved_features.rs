//! Example demonstrating the improved features in autogen-core
//! 
//! This example shows how to use the new logging, validation, and macro systems.

use autogen_core::{
    base_agent::BaseAgent,
    tools::base::{Tool, ToolSchema, BaseTool},
    logging::{LLMCallEvent, ToolCallEvent},
    utils::JsonUtils,
    component_config::ComponentModel,
    code_executor::base::{CodeExecutor, CodeExecutorGuard, CodeBlock, CodeResult},
    cancellation_token::CancellationToken,
};
use serde_json::{Value, Map};
use std::collections::HashMap;
use async_trait::async_trait;
use std::error::Error;

/// Example agent that demonstrates the new macro system
#[derive(Clone)]
struct ExampleAgent {
    base: BaseAgent,
    name: String,
}

impl ExampleAgent {
    fn new(name: String) -> Self {
        Self {
            base: BaseAgent::new(format!("Example agent: {}", name)),
            name,
        }
    }
}

// Note: Macro system is available but requires more setup
// For this example, we'll focus on the other improvements

// Example tool with validation
struct ExampleTool {
    base: BaseTool,
}

impl ExampleTool {
    fn new() -> Self {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input text to process"
                }
            },
            "required": ["input"]
        });

        Self {
            base: BaseTool::new(
                "example_tool".to_string(),
                "An example tool that processes text".to_string(),
                parameters,
            ).with_strict(true),
        }
    }
}

#[async_trait]
impl Tool for ExampleTool {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn schema(&self) -> &ToolSchema {
        self.base.schema()
    }

    async fn run(&self, args: &Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // The validation happens automatically in run_json
        let input = args.get("input")
            .and_then(|v| v.as_str())
            .ok_or("Missing input parameter")?;

        let result = format!("Processed: {}", input);
        Ok(Value::String(result))
    }
}

// Example code executor
struct ExampleCodeExecutor;

#[async_trait]
impl CodeExecutor for ExampleCodeExecutor {
    async fn execute_code_blocks(
        &mut self,
        code_blocks: &[CodeBlock],
        _cancellation_token: CancellationToken,
    ) -> Result<CodeResult, Box<dyn Error>> {
        // Simple mock implementation
        let mut output = String::new();
        for block in code_blocks {
            output.push_str(&format!("Executing {} code:\n{}\n", block.language, block.code));
        }
        
        Ok(CodeResult {
            exit_code: 0,
            output,
        })
    }

    async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Code executor started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Code executor stopped");
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), Box<dyn Error>> {
        self.stop().await?;
        self.start().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Demonstrating improved autogen-core features");

    // 1. Demonstrate logging events
    println!("\n📝 1. Logging Events");
    let messages = vec![
        Map::from_iter([
            ("role".to_string(), Value::String("user".to_string())),
            ("content".to_string(), Value::String("Hello!".to_string())),
        ])
    ];
    let response = Map::from_iter([
        ("content".to_string(), Value::String("Hi there!".to_string())),
    ]);

    let llm_event = LLMCallEvent::new(messages, response, 10, 20);
    println!("LLM Event: {}", llm_event);

    let tool_event = ToolCallEvent::new(
        "example_tool".to_string(),
        Map::from_iter([("input".to_string(), Value::String("test".to_string()))]),
        "Processed: test".to_string(),
    );
    println!("Tool Event: {}", tool_event);

    // 2. Demonstrate utils
    println!("\n🔧 2. Utility Functions");
    let mut json_obj = serde_json::json!({
        "name": "test",
        "nested": {
            "value": 42
        }
    });

    let another_obj = serde_json::json!({
        "nested": {
            "new_field": "added"
        },
        "extra": "field"
    });

    JsonUtils::merge_objects(&mut json_obj, &another_obj);
    println!("Merged JSON: {}", json_obj);

    let flattened = JsonUtils::flatten_object(&json_obj);
    println!("Flattened: {:?}", flattened);

    // 3. Demonstrate tool with validation
    println!("\n🛠️ 3. Tool with Validation");
    let tool = ExampleTool::new();
    
    // Valid input
    let valid_args = serde_json::json!({"input": "Hello, world!"});
    match tool.run_json(&valid_args, "test_call_1").await {
        Ok(result) => println!("Tool result: {}", result),
        Err(e) => println!("Tool error: {}", e),
    }

    // Invalid input (missing required field)
    let invalid_args = serde_json::json!({"wrong_field": "value"});
    match tool.run_json(&invalid_args, "test_call_2").await {
        Ok(result) => println!("Tool result: {}", result),
        Err(e) => println!("Tool validation error: {}", e),
    }

    // 4. Demonstrate code executor with RAII
    println!("\n💻 4. Code Executor with RAII");
    let executor = ExampleCodeExecutor;
    let mut guard = CodeExecutorGuard::new(executor).await?;

    let code_blocks = vec![
        CodeBlock {
            code: "print('Hello from Python!')".to_string(),
            language: "python".to_string(),
        },
        CodeBlock {
            code: "console.log('Hello from JavaScript!');".to_string(),
            language: "javascript".to_string(),
        },
    ];

    let result = guard.execute_code_blocks(&code_blocks, CancellationToken::new()).await?;
    println!("Execution result:\n{}", result.output);

    // Guard will automatically clean up when dropped

    // 5. Demonstrate component system
    println!("\n⚙️ 5. Component Configuration");
    let component_model = ComponentModel {
        provider: "example_provider".to_string(),
        component_type: Some("example_component".to_string()),
        version: Some(1),
        component_version: Some(1),
        description: Some("An example component".to_string()),
        label: Some("Example".to_string()),
        config: HashMap::from([
            ("setting1".to_string(), Value::String("value1".to_string())),
            ("setting2".to_string(), Value::Number(42.into())),
        ]),
    };

    println!("Component model: {}", serde_json::to_string_pretty(&component_model)?);

    println!("\n✅ All features demonstrated successfully!");
    Ok(())
}
