use crate::models::types::LLMMessage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionContextState {
    #[serde(default)]
    pub messages: Vec<LLMMessage>,
}

#[async_trait]
pub trait ChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage);
    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>>;
    async fn clear(&mut self);
    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>, Box<dyn Error>>;
    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<(), Box<dyn Error>>;
}

pub struct BaseChatCompletionContext {
    pub messages: Vec<LLMMessage>,
    pub initial_messages: Option<Vec<LLMMessage>>,
}

impl BaseChatCompletionContext {
    pub fn new(initial_messages: Option<Vec<LLMMessage>>) -> Self {
        let messages = initial_messages.clone().unwrap_or_default();
        Self {
            messages,
            initial_messages,
        }
    }
}

#[async_trait]
impl ChatCompletionContext for BaseChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) {
        self.messages.push(message);
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>, Box<dyn Error>> {
        // Base implementation returns all messages
        // Concrete implementations can override this for specific behavior
        Ok(self.messages.clone())
    }

    async fn clear(&mut self) {
        self.messages.clear();
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>, Box<dyn Error>> {
        let state = ChatCompletionContextState {
            messages: self.messages.clone(),
        };
        let json_value = serde_json::to_value(state)?;
        // a bit of a hack to convert to HashMap<String, Value>
        let map = json_value.as_object().unwrap().clone().into_iter().collect();
        Ok(map)
    }

    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<(), Box<dyn Error>> {
        let json_value = serde_json::to_value(state)?;
        let state: ChatCompletionContextState = serde_json::from_value(json_value)?;
        self.messages = state.messages;
        Ok(())
    }
}