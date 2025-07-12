use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};
use crate::models::types::{LLMMessage, FunctionExecutionResultMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct BufferedChatCompletionContextConfig {
    pub buffer_size: usize,
    #[serde(default)]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

pub struct BufferedChatCompletionContext {
    base: BaseChatCompletionContext,
    buffer_size: usize,
}

impl BufferedChatCompletionContext {
    pub fn new(buffer_size: usize, initial_messages: Option<Vec<LLMMessage>>) -> Self {
        assert!(buffer_size > 0, "buffer_size must be greater than 0.");
        Self {
            base: BaseChatCompletionContext::new(initial_messages),
            buffer_size,
        }
    }
}

#[async_trait]
impl ChatCompletionContext for BufferedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) {
        self.base.add_message(message).await;
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>> {
        let messages_len = self.base.messages.len();
        let start = if messages_len > self.buffer_size {
            messages_len - self.buffer_size
        } else {
            0
        };
        let mut messages = self.base.messages[start..].to_vec();

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