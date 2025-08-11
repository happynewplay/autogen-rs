use crate::message::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelClientError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type ModelClientResult<T> = Result<T, ModelClientError>;

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub messages: Vec<Message>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // Other parameters like top_p, stream etc. can be added here
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    pub message: Message,
    // Other info like usage, finish_reason etc.
}

/// A trait for clients that can send chat completion requests to a model.
#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn send_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> ModelClientResult<ChatCompletionResponse>;
}
