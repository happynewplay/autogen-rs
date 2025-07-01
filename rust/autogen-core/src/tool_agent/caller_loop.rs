//! Caller loop for tool agent interactions.

use std::collections::HashMap;
use crate::{
    agent_id::AgentId,
    agent_runtime::AgentRuntime,
    error::Result,
    models::{
        AssistantMessage, ChatCompletionClient,
        FunctionExecutionResult, FunctionExecutionResultMessage, LLMMessage,
    },
    tools::ToolSchema,
    CancellationToken, FunctionCall,
};
// Tool exceptions are handled inline

/// Start a caller loop for a tool agent.
///
/// This function sends messages to the tool agent and the model client in an alternating
/// fashion until the model client stops generating tool calls.
///
/// # Arguments
/// * `runtime` - The agent runtime to use for sending messages
/// * `tool_agent_id` - The Agent ID of the tool agent
/// * `model_client` - The model client to use for the model API
/// * `input_messages` - The list of input messages
/// * `tool_schema` - The list of tools that the model can use
/// * `cancellation_token` - Optional token to cancel the operation
/// * `caller_source` - Source identifier for the caller (default: "assistant")
///
/// # Returns
/// The list of output messages created in the caller loop
///
/// # Example
///
/// ```rust,no_run
/// use autogen_core::tool_agent::tool_agent_caller_loop;
/// use autogen_core::{AgentId, models::LLMMessage, tools::ToolSchema};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // let runtime = create_runtime();
/// // let tool_agent_id = AgentId::new("tool_agent", "main")?;
/// // let model_client = create_model_client();
/// // let input_messages = vec![/* your messages */];
/// // let tool_schema = vec![/* your tool schemas */];
/// //
/// // let output_messages = tool_agent_caller_loop(
/// //     &runtime,
/// //     tool_agent_id,
/// //     &model_client,
/// //     input_messages,
/// //     tool_schema,
/// //     None,
/// //     "assistant".to_string(),
/// // ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn tool_agent_caller_loop(
    runtime: &mut dyn AgentRuntime,
    tool_agent_id: AgentId,
    model_client: &dyn ChatCompletionClient,
    input_messages: Vec<LLMMessage>,
    tool_schema: Vec<ToolSchema>,
    cancellation_token: Option<CancellationToken>,
    caller_source: Option<String>,
) -> Result<Vec<LLMMessage>> {
    let caller_source = caller_source.unwrap_or_else(|| "assistant".to_string());
    let mut generated_messages: Vec<LLMMessage> = Vec::new();
    let mut current_messages = input_messages;

    // Get a response from the model
    let mut response = model_client.create(
        &current_messages,
        &tool_schema,
        None, // tool_choice
        None, // json_output
        &HashMap::new(), // extra_create_args
        cancellation_token.clone(),
    ).await?;

    // Add the response to the generated messages
    let assistant_content = match &response.content {
        crate::models::CreateResultContent::Text(text) => {
            crate::models::AssistantMessageContent::Text(text.clone())
        }
        crate::models::CreateResultContent::FunctionCalls(calls) => {
            crate::models::AssistantMessageContent::FunctionCalls(calls.clone())
        }
    };

    generated_messages.push(LLMMessage::Assistant(AssistantMessage {
        content: assistant_content,
        thought: response.thought.clone(),
        source: caller_source.clone(),
    }));

    // Keep iterating until the model stops generating tool calls
    loop {
        match &response.content {
            crate::models::CreateResultContent::FunctionCalls(function_calls) => {
                // Execute functions called by the model by sending messages to tool agent
                let mut function_results: Vec<FunctionExecutionResult> = Vec::new();

                for call in function_calls {
                    match runtime.send_message(
                        Box::new(call.clone()),
                        tool_agent_id.clone(),
                        None, // sender
                    ).await {
                        Ok(()) => {
                            // Message sent successfully, but we need to implement a proper response mechanism
                            // For now, create a placeholder result
                            function_results.push(FunctionExecutionResult {
                                content: "Tool executed successfully".to_string(),
                                call_id: call.id.clone(),
                                is_error: Some(false),
                                name: call.name.clone(),
                            });
                        }
                        Err(e) => {
                            // Handle different types of tool exceptions
                            let error_result = match e {
                                crate::error::AutoGenError::ToolNotFound(msg) => {
                                    FunctionExecutionResult {
                                        content: format!("Tool not found: {}", msg),
                                        call_id: call.id.clone(),
                                        is_error: Some(true),
                                        name: call.name.clone(),
                                    }
                                }
                                crate::error::AutoGenError::InvalidArguments(msg) => {
                                    FunctionExecutionResult {
                                        content: format!("Invalid arguments: {}", msg),
                                        call_id: call.id.clone(),
                                        is_error: Some(true),
                                        name: call.name.clone(),
                                    }
                                }
                                _ => {
                                    FunctionExecutionResult {
                                        content: format!("Tool execution error: {}", e),
                                        call_id: call.id.clone(),
                                        is_error: Some(true),
                                        name: call.name.clone(),
                                    }
                                }
                            };
                            function_results.push(error_result);
                        }
                    }
                }

                // Add function results to generated messages
                generated_messages.push(LLMMessage::FunctionResult(FunctionExecutionResultMessage {
                    content: function_results,
                }));

                // Update current messages for next model call
                current_messages.extend(generated_messages.clone());

                // Query the model again with the new response
                response = model_client.create(
                    &current_messages,
                    &tool_schema,
                    None, // tool_choice
                    None, // json_output
                    &HashMap::new(), // extra_create_args
                    cancellation_token.clone(),
                ).await?;

                // Add the new response to generated messages
                let assistant_content = match &response.content {
                    crate::models::CreateResultContent::Text(text) => {
                        crate::models::AssistantMessageContent::Text(text.clone())
                    }
                    crate::models::CreateResultContent::FunctionCalls(calls) => {
                        crate::models::AssistantMessageContent::FunctionCalls(calls.clone())
                    }
                };

                generated_messages.push(LLMMessage::Assistant(AssistantMessage {
                    content: assistant_content,
                    thought: response.thought.clone(),
                    source: caller_source.clone(),
                }));
            }
            crate::models::CreateResultContent::Text(_) => {
                // No more function calls, exit the loop
                break;
            }
        }
    }

    Ok(generated_messages)
}

/// Helper function to convert tool exceptions to function execution results
fn tool_exception_to_result(exception: &dyn std::error::Error, call_id: String, name: String) -> FunctionExecutionResult {
    FunctionExecutionResult {
        content: format!("Error: {}", exception),
        call_id,
        is_error: Some(true),
        name,
    }
}

/// Helper function to handle tool execution results
async fn handle_tool_execution_results(
    runtime: &mut dyn AgentRuntime,
    tool_agent_id: AgentId,
    function_calls: &[FunctionCall],
    _cancellation_token: Option<CancellationToken>,
) -> Vec<FunctionExecutionResult> {
    let mut results = Vec::new();

    for call in function_calls {
        match runtime.send_message(
            Box::new(call.clone()),
            tool_agent_id.clone(),
            None, // sender
        ).await {
            Ok(()) => {
                // Message sent successfully
                results.push(FunctionExecutionResult {
                    content: "Tool executed successfully".to_string(),
                    call_id: call.id.clone(),
                    is_error: Some(false),
                    name: call.name.clone(),
                });
            }
            Err(e) => {
                results.push(FunctionExecutionResult {
                    content: format!("Tool execution error: {}", e),
                    call_id: call.id.clone(),
                    is_error: Some(true),
                    name: call.name.clone(),
                });
            }
        }
    }

    results
}
