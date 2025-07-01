//! Base chat completion context trait and types.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{error::Result, models::LLMMessage};

/// State for chat completion context serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionContextState {
    /// The messages in the context
    #[serde(default)]
    pub messages: Vec<LLMMessage>,
}

impl Default for ChatCompletionContextState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

/// An abstract base trait for defining the interface of a chat completion context.
/// 
/// A chat completion context lets agents store and retrieve LLM messages.
/// It can be implemented with different recall strategies.
///
/// # Example
///
/// ```rust
/// use autogen_core::model_context::{UnboundedChatCompletionContext, ChatCompletionContext};
/// use autogen_core::models::{LLMMessage, SystemMessage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut context = UnboundedChatCompletionContext::new();
/// 
/// let message = LLMMessage::System(SystemMessage {
///     content: "You are a helpful assistant.".to_string(),
/// });
/// 
/// context.add_message(message).await?;
/// let messages = context.get_messages().await?;
/// assert_eq!(messages.len(), 1);
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait ChatCompletionContext: Send + Sync {
    /// Add a message to the context.
    ///
    /// # Arguments
    /// * `message` - The message to add
    async fn add_message(&mut self, message: LLMMessage) -> Result<()>;

    /// Get all messages from the context.
    ///
    /// The specific behavior depends on the implementation:
    /// - `UnboundedChatCompletionContext` returns all messages
    /// - `BufferedChatCompletionContext` returns the last N messages
    /// - `TokenLimitedChatCompletionContext` returns messages within token limit
    /// - `HeadAndTailChatCompletionContext` returns first N and last M messages
    ///
    /// # Returns
    /// A vector of LLM messages
    async fn get_messages(&self) -> Result<Vec<LLMMessage>>;

    /// Clear all messages from the context.
    async fn clear(&mut self) -> Result<()>;

    /// Save the current state of the context.
    ///
    /// # Returns
    /// A map containing the serialized state
    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>>;

    /// Load state into the context.
    ///
    /// # Arguments
    /// * `state` - The state to load
    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<()>;
}

/// Base implementation for chat completion contexts
#[derive(Debug)]
pub struct BaseChatCompletionContext {
    /// All messages in the context
    messages: Vec<LLMMessage>,
    /// Initial messages that were provided at creation
    initial_messages: Option<Vec<LLMMessage>>,
}

impl BaseChatCompletionContext {
    /// Create a new base context
    pub fn new(initial_messages: Option<Vec<LLMMessage>>) -> Self {
        let messages = initial_messages.clone().unwrap_or_default();
        Self {
            messages,
            initial_messages,
        }
    }

    /// Get a reference to all messages
    pub fn messages(&self) -> &[LLMMessage] {
        &self.messages
    }

    /// Get a mutable reference to all messages
    pub fn messages_mut(&mut self) -> &mut Vec<LLMMessage> {
        &mut self.messages
    }

    /// Get the initial messages
    pub fn initial_messages(&self) -> Option<&[LLMMessage]> {
        self.initial_messages.as_deref()
    }
}

#[async_trait]
impl ChatCompletionContext for BaseChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) -> Result<()> {
        self.messages.push(message);
        Ok(())
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>> {
        Ok(self.messages.clone())
    }

    async fn clear(&mut self) -> Result<()> {
        self.messages.clear();
        Ok(())
    }

    async fn save_state(&self) -> Result<HashMap<String, serde_json::Value>> {
        let state = ChatCompletionContextState {
            messages: self.messages.clone(),
        };
        let mut map = HashMap::new();
        map.insert("state".to_string(), serde_json::to_value(state)?);
        Ok(map)
    }

    async fn load_state(&mut self, state: &HashMap<String, serde_json::Value>) -> Result<()> {
        if let Some(state_value) = state.get("state") {
            let state: ChatCompletionContextState = serde_json::from_value(state_value.clone())?;
            self.messages = state.messages;
        }
        Ok(())
    }
}
