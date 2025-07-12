use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};
use crate::models::types::{AssistantMessage, FunctionExecutionResultMessage, LLMMessage, UserMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct HeadAndTailChatCompletionContextConfig {
    pub head_size: usize,
    pub tail_size: usize,
    #[serde(default)]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

pub struct HeadAndTailChatCompletionContext {
    base: BaseChatCompletionContext,
    head_size: usize,
    tail_size: usize,
}

impl HeadAndTailChatCompletionContext {
    pub fn new(head_size: usize, tail_size: usize, initial_messages: Option<Vec<LLMMessage>>) -> Self {
        assert!(head_size > 0, "head_size must be greater than 0.");
        assert!(tail_size > 0, "tail_size must be greater than 0.");
        Self {
            base: BaseChatCompletionContext::new(initial_messages),
            head_size,
            tail_size,
        }
    }
}

#[async_trait]
impl ChatCompletionContext for HeadAndTailChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) {
        self.base.add_message(message).await;
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>> {
        let messages_len = self.base.messages.len();
        if messages_len <= self.head_size + self.tail_size {
            return Ok(self.base.messages.clone());
        }

        let mut head_messages = self.base.messages[..self.head_size].to_vec();
        if let Some(LLMMessage::Assistant(AssistantMessage { .. })) = head_messages.last() {
            // A bit complex to check for FunctionCall, simplifying for now
            head_messages.pop();
        }

        let mut tail_messages = self.base.messages[messages_len - self.tail_size..].to_vec();
        if let Some(LLMMessage::FunctionExecutionResult(_)) = tail_messages.first() {
            tail_messages.remove(0);
        }

        let num_skipped = messages_len - head_messages.len() - tail_messages.len();
        let placeholder_message = LLMMessage::User(UserMessage {
            content: format!("Skipped {} messages.", num_skipped),
            source: Some("System".to_string()),
        });

        let mut result = head_messages;
        result.push(placeholder_message);
        result.extend(tail_messages);

        Ok(result)
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