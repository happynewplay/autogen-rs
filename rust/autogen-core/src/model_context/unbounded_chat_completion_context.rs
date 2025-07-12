use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};
use crate::models::types::LLMMessage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct UnboundedChatCompletionContextConfig {
    #[serde(default)]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

pub struct UnboundedChatCompletionContext {
    base: BaseChatCompletionContext,
}

impl UnboundedChatCompletionContext {
    pub fn new(initial_messages: Option<Vec<LLMMessage>>) -> Self {
        Self {
            base: BaseChatCompletionContext::new(initial_messages),
        }
    }
}

#[async_trait]
impl ChatCompletionContext for UnboundedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) {
        self.base.add_message(message).await;
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>> {
        Ok(self.base.messages.clone())
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