//! Message types and data structures for language model interactions.

use serde::{Deserialize, Serialize};
use crate::FunctionCall;

/// Image data for multimodal messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// Base64 encoded image data or URL
    pub data: String,
    /// MIME type of the image
    pub mime_type: String,
    /// Optional description of the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// System message contains instructions for the model coming from the developer.
///
/// Note: OpenAI is moving away from using 'system' role in favor of 'developer' role.
/// However, the 'system' role is still allowed in their API and will be automatically
/// converted to 'developer' role on the server side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// The content of the message
    pub content: String,
}

/// User message contains input from end users, or a catch-all for data provided to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// The content of the message
    pub content: UserMessageContent,
    /// The name of the agent that sent this message
    pub source: String,
}

/// Content type for user messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserMessageContent {
    /// Simple text content
    Text(String),
    /// Mixed content with text and images
    Mixed(Vec<UserMessageContentItem>),
}

/// Individual content item in a user message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserMessageContentItem {
    /// Text content item
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String
    },
    /// Image content item
    #[serde(rename = "image")]
    Image {
        /// The image data
        image: Image
    },
}

/// Assistant message are sampled from the language model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The content of the message
    pub content: AssistantMessageContent,
    /// The reasoning text for the completion if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<String>,
    /// The name of the agent that sent this message
    pub source: String,
}

/// Content type for assistant messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssistantMessageContent {
    /// Text content
    Text(String),
    /// Function calls
    FunctionCalls(Vec<FunctionCall>),
}

/// Function execution result contains the output of a function call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionExecutionResult {
    /// The output of the function call
    pub content: String,
    /// The name of the function that was called
    pub name: String,
    /// The ID of the function call
    pub call_id: String,
    /// Whether the function call resulted in an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Function execution result message contains the output of multiple function calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionExecutionResultMessage {
    /// The function execution results
    pub content: Vec<FunctionExecutionResult>,
}

/// Union type for all LLM message types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMMessage {
    /// System message
    #[serde(rename = "SystemMessage")]
    System(SystemMessage),
    /// User message
    #[serde(rename = "UserMessage")]
    User(UserMessage),
    /// Assistant message
    #[serde(rename = "AssistantMessage")]
    Assistant(AssistantMessage),
    /// Function execution result message
    #[serde(rename = "FunctionExecutionResultMessage")]
    FunctionResult(FunctionExecutionResultMessage),
}

/// Request usage statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
}

impl RequestUsage {
    /// Get the total number of tokens
    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Possible finish reasons for model completions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReasons {
    /// Natural stop
    Stop,
    /// Hit length limit
    Length,
    /// Function calls were made
    FunctionCalls,
    /// Content was filtered
    ContentFilter,
    /// Unknown reason
    Unknown,
}

/// Top log probability information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogprob {
    /// The log probability
    pub logprob: f64,
    /// Optional byte representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

/// Token log probability information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    /// The token
    pub token: String,
    /// The log probability
    pub logprob: f64,
    /// Top alternative log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<Vec<TopLogprob>>,
    /// Optional byte representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

/// Create result contains the output of a model completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateResult {
    /// The reason the model finished generating the completion
    pub finish_reason: FinishReasons,
    /// The output of the model completion
    pub content: CreateResultContent,
    /// The usage of tokens in the prompt and completion
    pub usage: RequestUsage,
    /// Whether the completion was generated from a cached response
    pub cached: bool,
    /// The logprobs of the tokens in the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<ChatCompletionTokenLogprob>>,
    /// The reasoning text for the completion if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<String>,
}

/// Content type for create results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateResultContent {
    /// Text content
    Text(String),
    /// Function calls
    FunctionCalls(Vec<FunctionCall>),
}
