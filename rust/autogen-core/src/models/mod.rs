//! Language model integration and message types.
//!
//! This module provides abstractions for working with various language models,
//! including message types, model capabilities, and client interfaces.

mod model_client;
mod types;

pub use model_client::{
    ChatCompletionClient, ModelFamily, ModelInfo, ModelCapabilities,
    validate_model_info,
};
pub use types::{
    SystemMessage, UserMessage, UserMessageContent, UserMessageContentItem,
    AssistantMessage, AssistantMessageContent, FunctionExecutionResult,
    FunctionExecutionResultMessage, LLMMessage, RequestUsage, CreateResult,
    CreateResultContent, FinishReasons, ChatCompletionTokenLogprob, TopLogprob,
};
