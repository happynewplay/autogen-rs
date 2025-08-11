use crate::models::types::LLMMessage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Assuming ToolSchema is defined elsewhere
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema;

#[async_trait]
pub trait ChatCompletionClient {
    async fn count_tokens(
        &self,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
    ) -> Result<usize, Box<dyn Error>>;

    async fn remaining_tokens(
        &self,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
    ) -> Result<isize, Box<dyn Error>>;
}