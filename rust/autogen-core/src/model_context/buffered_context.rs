//! Buffered chat completion context implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{error::Result, models::LLMMessage};
use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};

/// Configuration for BufferedChatCompletionContext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedChatCompletionContextConfig {
    /// The size of the buffer
    pub buffer_size: usize,
    /// Initial messages to include in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

/// A buffered chat completion context that keeps a view of the last n messages,
/// where n is the buffer size. The buffer size is set at initialization.
///
/// This context is useful when you want to limit memory usage and only keep
/// the most recent conversation history.
///
/// # Example
///
/// ```rust
/// use autogen_core::model_context::{BufferedChatCompletionContext, ChatCompletionContext};
/// use autogen_core::models::{LLMMessage, SystemMessage, UserMessage, UserMessageContent};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut context = BufferedChatCompletionContext::new(2); // Keep only last 2 messages
/// 
/// // Add multiple messages
/// for i in 0..5 {
///     let msg = LLMMessage::User(UserMessage {
///         content: UserMessageContent::Text(format!("Message {}", i)),
///         source: "user".to_string(),
///     });
///     context.add_message(msg).await?;
/// }
/// 
/// // Only the last 2 messages should be kept
/// let messages = context.get_messages().await?;
/// assert_eq!(messages.len(), 2);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct BufferedChatCompletionContext {
    base: BaseChatCompletionContext,
    buffer_size: usize,
}

impl BufferedChatCompletionContext {
    /// Create a new buffered context
    ///
    /// # Arguments
    /// * `buffer_size` - The maximum number of messages to keep
    ///
    /// # Panics
    /// Panics if buffer_size is 0
    pub fn new(buffer_size: usize) -> Self {
        if buffer_size == 0 {
            panic!("buffer_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(None),
            buffer_size,
        }
    }

    /// Create a new buffered context with initial messages
    ///
    /// # Arguments
    /// * `buffer_size` - The maximum number of messages to keep
    /// * `initial_messages` - Initial messages to include
    ///
    /// # Panics
    /// Panics if buffer_size is 0
    pub fn with_initial_messages(buffer_size: usize, initial_messages: Vec<LLMMessage>) -> Self {
        if buffer_size == 0 {
            panic!("buffer_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(Some(initial_messages)),
            buffer_size,
        }
    }

    /// Create from configuration
    ///
    /// # Panics
    /// Panics if buffer_size is 0
    pub fn from_config(config: BufferedChatCompletionContextConfig) -> Self {
        if config.buffer_size == 0 {
            panic!("buffer_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(config.initial_messages),
            buffer_size: config.buffer_size,
        }
    }

    /// Convert to configuration
    pub fn to_config(&self) -> BufferedChatCompletionContextConfig {
        BufferedChatCompletionContextConfig {
            buffer_size: self.buffer_size,
            initial_messages: self.base.initial_messages().map(|msgs| msgs.to_vec()),
        }
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

#[async_trait]
impl ChatCompletionContext for BufferedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) -> Result<()> {
        self.base.add_message(message).await
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>> {
        let all_messages = self.base.messages();
        
        // Get the last buffer_size messages
        let start_index = if all_messages.len() > self.buffer_size {
            all_messages.len() - self.buffer_size
        } else {
            0
        };
        
        let mut messages: Vec<LLMMessage> = all_messages[start_index..].to_vec();
        
        // Handle the case where the first message is a function execution result message
        // Remove it if it exists to avoid incomplete function call context
        if let Some(first_msg) = messages.first() {
            if matches!(first_msg, LLMMessage::FunctionResult(_)) {
                messages.remove(0);
            }
        }
        
        Ok(messages)
    }

    async fn clear(&mut self) -> Result<()> {
        self.base.clear().await
    }

    async fn save_state(&self) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        self.base.save_state().await
    }

    async fn load_state(&mut self, state: &std::collections::HashMap<String, serde_json::Value>) -> Result<()> {
        self.base.load_state(state).await
    }
}
