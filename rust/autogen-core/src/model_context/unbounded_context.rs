//! Unbounded chat completion context implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{error::Result, models::LLMMessage};
use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};

/// Configuration for UnboundedChatCompletionContext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnboundedChatCompletionContextConfig {
    /// Initial messages to include in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

impl Default for UnboundedChatCompletionContextConfig {
    fn default() -> Self {
        Self {
            initial_messages: None,
        }
    }
}

/// An unbounded chat completion context that keeps a view of all the messages.
///
/// This context implementation stores all messages without any limits,
/// making it suitable for scenarios where you want to maintain the complete
/// conversation history.
///
/// # Example
///
/// ```rust
/// use autogen_core::model_context::{UnboundedChatCompletionContext, ChatCompletionContext};
/// use autogen_core::models::{LLMMessage, SystemMessage, UserMessage, UserMessageContent};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut context = UnboundedChatCompletionContext::new();
/// 
/// // Add system message
/// let system_msg = LLMMessage::System(SystemMessage {
///     content: "You are a helpful assistant.".to_string(),
/// });
/// context.add_message(system_msg).await?;
/// 
/// // Add user message
/// let user_msg = LLMMessage::User(UserMessage {
///     content: UserMessageContent::Text("Hello!".to_string()),
///     source: "user".to_string(),
/// });
/// context.add_message(user_msg).await?;
/// 
/// // Get all messages
/// let messages = context.get_messages().await?;
/// assert_eq!(messages.len(), 2);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct UnboundedChatCompletionContext {
    base: BaseChatCompletionContext,
}

impl UnboundedChatCompletionContext {
    /// Create a new unbounded context
    pub fn new() -> Self {
        Self {
            base: BaseChatCompletionContext::new(None),
        }
    }

    /// Create a new unbounded context with initial messages
    pub fn with_initial_messages(initial_messages: Vec<LLMMessage>) -> Self {
        Self {
            base: BaseChatCompletionContext::new(Some(initial_messages)),
        }
    }

    /// Create from configuration
    pub fn from_config(config: UnboundedChatCompletionContextConfig) -> Self {
        Self {
            base: BaseChatCompletionContext::new(config.initial_messages),
        }
    }

    /// Convert to configuration
    pub fn to_config(&self) -> UnboundedChatCompletionContextConfig {
        UnboundedChatCompletionContextConfig {
            initial_messages: self.base.initial_messages().map(|msgs| msgs.to_vec()),
        }
    }
}

impl Default for UnboundedChatCompletionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatCompletionContext for UnboundedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) -> Result<()> {
        self.base.add_message(message).await
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>> {
        // Return all messages without any filtering
        self.base.get_messages().await
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
