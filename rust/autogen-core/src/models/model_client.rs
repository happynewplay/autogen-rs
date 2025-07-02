//! Model client traits and capabilities definitions.

use std::collections::HashMap;
use async_trait::async_trait;
#[cfg(feature = "runtime")]
use futures::Stream;
use serde::{Deserialize, Serialize};
use crate::{CancellationToken, error::Result, tools::ToolSchema};
use super::types::{LLMMessage, CreateResult, RequestUsage};

/// Model family constants for different LLM providers and model types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelFamily {
    // OpenAI models
    /// GPT-4.1 model family
    #[serde(rename = "gpt-41")]
    Gpt41,
    /// GPT-4.5 model family
    #[serde(rename = "gpt-45")]
    Gpt45,
    /// GPT-4o model family
    #[serde(rename = "gpt-4o")]
    Gpt4O,
    /// O1 model family
    #[serde(rename = "o1")]
    O1,
    /// O3 model family
    #[serde(rename = "o3")]
    O3,
    /// O4 model family
    #[serde(rename = "o4")]
    O4,
    /// GPT-4 model family
    #[serde(rename = "gpt-4")]
    Gpt4,
    /// GPT-3.5 model family
    #[serde(rename = "gpt-35")]
    Gpt35,
    /// R1 model family
    #[serde(rename = "r1")]
    R1,
    
    // Google models
    /// Gemini 1.5 Flash model family
    #[serde(rename = "gemini-1.5-flash")]
    Gemini15Flash,
    /// Gemini 1.5 Pro model family
    #[serde(rename = "gemini-1.5-pro")]
    Gemini15Pro,
    /// Gemini 2.0 Flash model family
    #[serde(rename = "gemini-2.0-flash")]
    Gemini20Flash,
    /// Gemini 2.5 Pro model family
    #[serde(rename = "gemini-2.5-pro")]
    Gemini25Pro,
    /// Gemini 2.5 Flash model family
    #[serde(rename = "gemini-2.5-flash")]
    Gemini25Flash,
    
    // Anthropic models
    /// Claude 3 Haiku model family
    #[serde(rename = "claude-3-haiku")]
    Claude3Haiku,
    /// Claude 3 Sonnet model family
    #[serde(rename = "claude-3-sonnet")]
    Claude3Sonnet,
    /// Claude 3 Opus model family
    #[serde(rename = "claude-3-opus")]
    Claude3Opus,
    /// Claude 3.5 Haiku model family
    #[serde(rename = "claude-3-5-haiku")]
    Claude35Haiku,
    /// Claude 3.5 Sonnet model family
    #[serde(rename = "claude-3-5-sonnet")]
    Claude35Sonnet,
    /// Claude 3.7 Sonnet model family
    #[serde(rename = "claude-3-7-sonnet")]
    Claude37Sonnet,
    /// Claude 4 Opus model family
    #[serde(rename = "claude-4-opus")]
    Claude4Opus,
    /// Claude 4 Sonnet model family
    #[serde(rename = "claude-4-sonnet")]
    Claude4Sonnet,
    
    // Llama models
    /// Llama 3.3 8B model family
    #[serde(rename = "llama-3.3-8b")]
    Llama338B,
    /// Llama 3.3 70B model family
    #[serde(rename = "llama-3.3-70b")]
    Llama3370B,
    /// Llama 4 Scout model family
    #[serde(rename = "llama-4-scout")]
    Llama4Scout,
    /// Llama 4 Maverick model family
    #[serde(rename = "llama-4-maverick")]
    Llama4Maverick,

    // Mistral models
    /// Codestral model family
    #[serde(rename = "codestral")]
    Codestral,
    /// Open Codestral Mamba model family
    #[serde(rename = "open-codestral-mamba")]
    OpenCodestralMamba,
    /// Mistral model family
    #[serde(rename = "mistral")]
    Mistral,
    /// Ministral model family
    #[serde(rename = "ministral")]
    Ministral,
    /// Pixtral model family
    #[serde(rename = "pixtral")]
    Pixtral,

    /// Unknown model family
    #[serde(rename = "unknown")]
    Unknown,

    /// Custom model family with string identifier
    #[serde(untagged)]
    Custom(String),
}

impl ModelFamily {
    /// Check if this is a Claude model family
    pub fn is_claude(&self) -> bool {
        matches!(self, 
            ModelFamily::Claude3Haiku | ModelFamily::Claude3Sonnet | ModelFamily::Claude3Opus |
            ModelFamily::Claude35Haiku | ModelFamily::Claude35Sonnet | ModelFamily::Claude37Sonnet |
            ModelFamily::Claude4Opus | ModelFamily::Claude4Sonnet
        )
    }

    /// Check if this is a Gemini model family
    pub fn is_gemini(&self) -> bool {
        matches!(self,
            ModelFamily::Gemini15Flash | ModelFamily::Gemini15Pro | ModelFamily::Gemini20Flash |
            ModelFamily::Gemini25Pro | ModelFamily::Gemini25Flash
        )
    }

    /// Check if this is an OpenAI model family
    pub fn is_openai(&self) -> bool {
        matches!(self,
            ModelFamily::Gpt45 | ModelFamily::Gpt41 | ModelFamily::Gpt4O |
            ModelFamily::O1 | ModelFamily::O3 | ModelFamily::O4 |
            ModelFamily::Gpt4 | ModelFamily::Gpt35
        )
    }

    /// Check if this is a Llama model family
    pub fn is_llama(&self) -> bool {
        matches!(self,
            ModelFamily::Llama338B | ModelFamily::Llama3370B |
            ModelFamily::Llama4Scout | ModelFamily::Llama4Maverick
        )
    }

    /// Check if this is a Mistral model family
    pub fn is_mistral(&self) -> bool {
        matches!(self,
            ModelFamily::Codestral | ModelFamily::OpenCodestralMamba |
            ModelFamily::Mistral | ModelFamily::Ministral | ModelFamily::Pixtral
        )
    }
}

/// Model capabilities (deprecated, use ModelInfo instead)
#[deprecated(note = "Use ModelInfo instead")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// True if the model supports vision (image input)
    pub vision: bool,
    /// True if the model supports function calling
    pub function_calling: bool,
    /// True if the model supports JSON output
    pub json_output: bool,
}

/// Model information containing properties about a model's capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// True if the model supports vision (image input)
    pub vision: bool,
    /// True if the model supports function calling
    pub function_calling: bool,
    /// True if the model supports JSON output
    pub json_output: bool,
    /// Model family identifier
    pub family: ModelFamily,
    /// True if the model supports structured output
    pub structured_output: bool,
    /// True if the model supports multiple, non-consecutive system messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_system_messages: Option<bool>,
}

/// Validate that a ModelInfo contains all required fields
pub fn validate_model_info(_model_info: &ModelInfo) -> Result<()> {
    // All fields are required in the struct, so this is mainly for future extensibility
    Ok(())
}

/// Tool choice options for model calls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Automatically choose whether to use tools
    Auto,
    /// Force the model to use tools
    Required,
    /// Disable tool usage
    None,
    /// Force the model to use a specific tool by name
    Specific { name: String },
}

/// JSON output mode options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonOutput {
    /// Enable/disable JSON mode
    Enabled(bool),
    /// Use structured output with a specific schema
    Structured(serde_json::Value),
}

/// Chat completion client trait for interacting with language models.
#[async_trait]
pub trait ChatCompletionClient: Send + Sync {
    /// Create a single response from the model.
    ///
    /// # Arguments
    /// * `messages` - The messages to send to the model
    /// * `tools` - The tools to use with the model
    /// * `tool_choice` - Tool choice strategy
    /// * `json_output` - JSON output configuration
    /// * `extra_create_args` - Extra arguments for the underlying client
    /// * `cancellation_token` - Token for cancellation
    ///
    /// # Returns
    /// The result of the model call
    async fn create(
        &self,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
        tool_choice: Option<ToolChoice>,
        json_output: Option<JsonOutput>,
        extra_create_args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<CreateResult>;

    /// Create a stream of responses from the model.
    ///
    /// # Arguments
    /// * `messages` - The messages to send to the model
    /// * `tools` - The tools to use with the model
    /// * `tool_choice` - Tool choice strategy
    /// * `json_output` - JSON output configuration
    /// * `extra_create_args` - Extra arguments for the underlying client
    /// * `cancellation_token` - Token for cancellation
    ///
    /// # Returns
    /// A stream of string chunks ending with a CreateResult
    #[cfg(feature = "runtime")]
    async fn create_stream(
        &self,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
        tool_choice: Option<ToolChoice>,
        json_output: Option<JsonOutput>,
        extra_create_args: &HashMap<String, serde_json::Value>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<Box<dyn Stream<Item = Result<StreamItem>> + Send + Unpin>>;

    /// Close the client and clean up resources
    async fn close(&self) -> Result<()>;

    /// Get the actual usage statistics for the last request
    fn actual_usage(&self) -> RequestUsage;

    /// Get the total usage statistics across all requests
    fn total_usage(&self) -> RequestUsage;

    /// Count tokens in the given messages and tools
    fn count_tokens(&self, messages: &[LLMMessage], tools: &[ToolSchema]) -> Result<u32>;

    /// Calculate remaining tokens available for the model
    fn remaining_tokens(&self, messages: &[LLMMessage], tools: &[ToolSchema]) -> Result<i32>;

    /// Get model capabilities (deprecated)
    #[deprecated(note = "Use model_info instead")]
    fn capabilities(&self) -> ModelCapabilities;

    /// Get model information
    fn model_info(&self) -> ModelInfo;
}

/// Items that can be yielded from a streaming response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamItem {
    /// Text chunk
    Text(String),
    /// Final result
    Result(CreateResult),
}
