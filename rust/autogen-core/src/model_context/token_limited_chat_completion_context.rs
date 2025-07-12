use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};
use crate::models::model_client::{ChatCompletionClient, ToolSchema};
use crate::models::types::{LLMMessage, FunctionExecutionResultMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct TokenLimitedChatCompletionContextConfig {
    // In Rust, we can't directly serialize a trait object.
    // We would typically use some form of component model or configuration
    // to reconstruct the model client. For now, this is a placeholder.
    // model_client: ComponentModel,
    pub token_limit: Option<usize>,
    #[serde(default)]
    pub tool_schema: Option<Vec<ToolSchema>>,
    #[serde(default)]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

pub struct TokenLimitedChatCompletionContext {
    base: BaseChatCompletionContext,
    model_client: Arc<dyn ChatCompletionClient + Send + Sync>,
    token_limit: Option<usize>,
    tool_schema: Vec<ToolSchema>,
}

impl TokenLimitedChatCompletionContext {
    pub fn new(
        model_client: Arc<dyn ChatCompletionClient + Send + Sync>,
        token_limit: Option<usize>,
        tool_schema: Option<Vec<ToolSchema>>,
        initial_messages: Option<Vec<LLMMessage>>,
    ) -> Self {
        if let Some(limit) = token_limit {
            assert!(limit > 0, "token_limit must be greater than 0.");
        }
        Self {
            base: BaseChatCompletionContext::new(initial_messages),
            model_client,
            token_limit,
            tool_schema: tool_schema.unwrap_or_default(),
        }
    }
}

#[async_trait]
impl ChatCompletionContext for TokenLimitedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) {
        self.base.add_message(message).await;
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>> {
        let mut messages = self.base.messages.clone();
        if let Some(token_limit) = self.token_limit {
            let mut token_count = self
                .model_client
                .count_tokens(&messages, &self.tool_schema)
                .await?;
            while token_count > token_limit && !messages.is_empty() {
                let middle_index = messages.len() / 2;
                messages.remove(middle_index);
                token_count = self
                    .model_client
                    .count_tokens(&messages, &self.tool_schema)
                    .await?;
            }
        } else {
            let mut remaining_tokens = self
                .model_client
                .remaining_tokens(&messages, &self.tool_schema)
                .await?;
            while remaining_tokens < 0 && !messages.is_empty() {
                let middle_index = messages.len() / 2;
                messages.remove(middle_index);
                remaining_tokens = self
                    .model_client
                    .remaining_tokens(&messages, &self.tool_schema)
                    .await?;
            }
        }

        if let Some(LLMMessage::FunctionExecutionResult(_)) = messages.first() {
            messages.remove(0);
        }

        Ok(messages)
    }

    async fn clear(&mut self) {
        self.base.clear().await;
    }

    async fn save_state(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, Box<dyn Error>> {
        self.base.save_state().await
    }

    async fn load_state(
        &mut self,
        state: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        self.base.load_state(state).await
    }
}