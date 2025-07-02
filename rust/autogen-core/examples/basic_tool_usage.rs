//! Basic example demonstrating tool usage in autogen-core
//!
//! This example shows how to create and use tools with the autogen-core library.

use std::collections::HashMap;
use autogen_core::{
    tools::{FunctionTool, Tool, ParametersSchema, AsyncToolFunction},
    CancellationToken,
};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 AutoGen Core - Basic Tool Usage Example");
    
    // Create a simple calculator tool
    let calculator_tool = create_calculator_tool();
    
    // Test the tool with some arguments
    let args = HashMap::from([
        ("operation".to_string(), json!("add")),
        ("a".to_string(), json!(10)),
        ("b".to_string(), json!(5)),
    ]);
    
    println!("\n📊 Tool Information:");
    println!("Name: {}", calculator_tool.name());
    println!("Description: {}", calculator_tool.description());
    println!("Schema: {:#?}", calculator_tool.schema());
    
    println!("\n🔧 Executing tool with arguments: {:#?}", args);
    
    match calculator_tool.run_json(&args, None, Some("test-call-1".to_string())).await {
        Ok(result) => {
            println!("✅ Tool execution successful!");
            println!("Result: {}", result);
        }
        Err(e) => {
            println!("❌ Tool execution failed: {}", e);
        }
    }
    
    // Test with invalid operation
    let invalid_args = HashMap::from([
        ("operation".to_string(), json!("invalid")),
        ("a".to_string(), json!(10)),
        ("b".to_string(), json!(5)),
    ]);
    
    println!("\n🔧 Testing with invalid operation: {:#?}", invalid_args);
    
    match calculator_tool.run_json(&invalid_args, None, Some("test-call-2".to_string())).await {
        Ok(result) => {
            println!("Result: {}", result);
        }
        Err(e) => {
            println!("❌ Expected error: {}", e);
        }
    }
    
    Ok(())
}

/// Create a simple calculator tool for demonstration
fn create_calculator_tool() -> FunctionTool {
    // Define the parameters schema
    let parameters = Some(ParametersSchema {
        schema_type: "object".to_string(),
        properties: Some(HashMap::from([
            ("operation".to_string(), json!({
                "type": "string",
                "enum": ["add", "subtract", "multiply", "divide"],
                "description": "The mathematical operation to perform"
            })),
            ("a".to_string(), json!({
                "type": "number",
                "description": "The first number"
            })),
            ("b".to_string(), json!({
                "type": "number",
                "description": "The second number"
            })),
        ])),
        required: Some(vec!["operation".to_string(), "a".to_string(), "b".to_string()]),
    });
    
    // Create the function that will be executed
    let calculator_function: AsyncToolFunction = Box::new(|args: HashMap<String, Value>, _cancellation_token: Option<CancellationToken>| {
        Box::pin(async move {
            // Extract arguments
            let operation = args.get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| autogen_core::AutoGenError::other("Missing or invalid 'operation' parameter"))?;
            
            let a = args.get("a")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| autogen_core::AutoGenError::other("Missing or invalid 'a' parameter"))?;
            
            let b = args.get("b")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| autogen_core::AutoGenError::other("Missing or invalid 'b' parameter"))?;
            
            // Perform the calculation
            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(autogen_core::AutoGenError::other("Division by zero"));
                    }
                    a / b
                }
                _ => return Err(autogen_core::AutoGenError::other(format!("Unknown operation: {}", operation))),
            };
            
            Ok(json!({
                "operation": operation,
                "operands": [a, b],
                "result": result
            }))
        })
    });
    
    FunctionTool::new(
        "calculator".to_string(),
        "A simple calculator that can perform basic arithmetic operations".to_string(),
        parameters,
        calculator_function,
    )
}
