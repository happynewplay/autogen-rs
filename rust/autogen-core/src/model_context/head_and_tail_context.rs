//! Head and tail chat completion context implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{
    error::Result, 
    models::{LLMMessage, UserMessage, UserMessageContent, AssistantMessageContent},
};
use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};

/// Configuration for HeadAndTailChatCompletionContext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadAndTailChatCompletionContextConfig {
    /// The size of the head (first N messages)
    pub head_size: usize,
    /// The size of the tail (last M messages)
    pub tail_size: usize,
    /// Initial messages to include in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

/// A chat completion context that keeps a view of the first n and last m messages,
/// where n is the head size and m is the tail size.
///
/// When the total number of messages exceeds head_size + tail_size, this context
/// will keep the first `head_size` messages, insert a placeholder message indicating
/// how many messages were skipped, and then include the last `tail_size` messages.
///
/// # Example
///
/// ```rust
/// use autogen_core::model_context::{HeadAndTailChatCompletionContext, ChatCompletionContext};
/// use autogen_core::models::{LLMMessage, UserMessage, UserMessageContent};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut context = HeadAndTailChatCompletionContext::new(2, 2); // Keep first 2 and last 2
/// 
/// // Add 6 messages
/// for i in 0..6 {
///     let msg = LLMMessage::User(UserMessage {
///         content: UserMessageContent::Text(format!("Message {}", i)),
///         source: "user".to_string(),
///     });
///     context.add_message(msg).await?;
/// }
/// 
/// let messages = context.get_messages().await?;
/// // Should have: first 2 + placeholder + last 2 = 5 messages
/// assert_eq!(messages.len(), 5);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct HeadAndTailChatCompletionContext {
    base: BaseChatCompletionContext,
    head_size: usize,
    tail_size: usize,
}

impl HeadAndTailChatCompletionContext {
    /// Create a new head and tail context
    ///
    /// # Arguments
    /// * `head_size` - The number of messages to keep from the beginning
    /// * `tail_size` - The number of messages to keep from the end
    ///
    /// # Panics
    /// Panics if head_size or tail_size is 0
    pub fn new(head_size: usize, tail_size: usize) -> Self {
        if head_size == 0 {
            panic!("head_size must be greater than 0");
        }
        if tail_size == 0 {
            panic!("tail_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(None),
            head_size,
            tail_size,
        }
    }

    /// Create a new head and tail context with initial messages
    ///
    /// # Arguments
    /// * `head_size` - The number of messages to keep from the beginning
    /// * `tail_size` - The number of messages to keep from the end
    /// * `initial_messages` - Initial messages to include
    ///
    /// # Panics
    /// Panics if head_size or tail_size is 0
    pub fn with_initial_messages(
        head_size: usize, 
        tail_size: usize, 
        initial_messages: Vec<LLMMessage>
    ) -> Self {
        if head_size == 0 {
            panic!("head_size must be greater than 0");
        }
        if tail_size == 0 {
            panic!("tail_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(Some(initial_messages)),
            head_size,
            tail_size,
        }
    }

    /// Create from configuration
    ///
    /// # Panics
    /// Panics if head_size or tail_size is 0
    pub fn from_config(config: HeadAndTailChatCompletionContextConfig) -> Self {
        if config.head_size == 0 {
            panic!("head_size must be greater than 0");
        }
        if config.tail_size == 0 {
            panic!("tail_size must be greater than 0");
        }
        Self {
            base: BaseChatCompletionContext::new(config.initial_messages),
            head_size: config.head_size,
            tail_size: config.tail_size,
        }
    }

    /// Convert to configuration
    pub fn to_config(&self) -> HeadAndTailChatCompletionContextConfig {
        HeadAndTailChatCompletionContextConfig {
            head_size: self.head_size,
            tail_size: self.tail_size,
            initial_messages: self.base.initial_messages().map(|msgs| msgs.to_vec()),
        }
    }

    /// Get the head size
    pub fn head_size(&self) -> usize {
        self.head_size
    }

    /// Get the tail size
    pub fn tail_size(&self) -> usize {
        self.tail_size
    }
}

#[async_trait]
impl ChatCompletionContext for HeadAndTailChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) -> Result<()> {
        self.base.add_message(message).await
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>> {
        let all_messages = self.base.messages();
        
        // Get head messages
        let mut head_messages: Vec<LLMMessage> = all_messages
            .iter()
            .take(self.head_size)
            .cloned()
            .collect();
        
        // Handle the case where the last message in head is a function call message
        if let Some(last_msg) = head_messages.last() {
            if let LLMMessage::Assistant(assistant_msg) = last_msg {
                if matches!(assistant_msg.content, AssistantMessageContent::FunctionCalls(_)) {
                    // Remove the last message from the head to avoid incomplete function call context
                    head_messages.pop();
                }
            }
        }
        
        // Get tail messages
        let tail_start = if all_messages.len() > self.tail_size {
            all_messages.len() - self.tail_size
        } else {
            0
        };
        
        let mut tail_messages: Vec<LLMMessage> = all_messages[tail_start..].to_vec();
        
        // Handle the case where the first message in tail is a function execution result message
        if let Some(first_msg) = tail_messages.first() {
            if matches!(first_msg, LLMMessage::FunctionResult(_)) {
                // Remove the first message from the tail
                tail_messages.remove(0);
            }
        }
        
        let num_skipped = all_messages.len().saturating_sub(self.head_size + self.tail_size);
        
        if num_skipped <= 0 {
            // If there are not enough messages to fill the head and tail,
            // return all messages
            return Ok(all_messages.to_vec());
        }
        
        // Create placeholder message for skipped content
        let placeholder_message = LLMMessage::User(UserMessage {
            content: UserMessageContent::Text(format!("Skipped {} messages.", num_skipped)),
            source: "System".to_string(),
        });
        
        // Combine head + placeholder + tail
        let mut result = head_messages;
        result.push(placeholder_message);
        result.extend(tail_messages);
        
        Ok(result)
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
