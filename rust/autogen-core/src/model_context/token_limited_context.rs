//! Token-limited chat completion context implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{
    error::Result,
    models::LLMMessage,
    tools::ToolSchema,
};
use super::chat_completion_context::{BaseChatCompletionContext, ChatCompletionContext};

/// Configuration for TokenLimitedChatCompletionContext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLimitedChatCompletionContextConfig {
    /// The maximum number of tokens to keep in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_limit: Option<u32>,
    /// Tool schema to use in token counting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_schema: Option<Vec<ToolSchema>>,
    /// Initial messages to include in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<LLMMessage>>,
}

/// A token-based chat completion context that maintains a view of the context up to a token limit.
///
/// This context uses a model client to count tokens and removes messages from the middle
/// when the token limit is exceeded, preserving the most recent and oldest messages.
///
/// **Note**: This is an experimental component and may change in the future.
///
/// # Example
///
/// ```rust,no_run
/// use autogen_core::model_context::{TokenLimitedChatCompletionContext, ChatCompletionContext};
/// use autogen_core::models::{LLMMessage, SystemMessage};
/// // Note: You would need to provide an actual ChatCompletionClient implementation
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // let model_client = YourModelClient::new();
/// // let mut context = TokenLimitedChatCompletionContext::new(
/// //     model_client,
/// //     Some(1000), // 1000 token limit
/// //     None,
/// //     None,
/// // );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TokenLimitedChatCompletionContext {
    base: BaseChatCompletionContext,
    token_limit: Option<u32>,
    tool_schema: Vec<ToolSchema>,
    // Note: In a real implementation, you would store the model client here
    // For now, we'll simulate token counting
}

impl TokenLimitedChatCompletionContext {
    /// Create a new token-limited context
    ///
    /// # Arguments
    /// * `token_limit` - Maximum number of tokens to keep (None for model's remaining tokens)
    /// * `tool_schema` - Tool schema to use in token counting
    /// * `initial_messages` - Initial messages to include
    pub fn new(
        token_limit: Option<u32>,
        tool_schema: Option<Vec<ToolSchema>>,
        initial_messages: Option<Vec<LLMMessage>>,
    ) -> Self {
        if let Some(limit) = token_limit {
            if limit == 0 {
                panic!("token_limit must be greater than 0");
            }
        }
        
        Self {
            base: BaseChatCompletionContext::new(initial_messages),
            token_limit,
            tool_schema: tool_schema.unwrap_or_default(),
        }
    }

    /// Create from configuration
    pub fn from_config(config: TokenLimitedChatCompletionContextConfig) -> Self {
        Self::new(config.token_limit, config.tool_schema, config.initial_messages)
    }

    /// Convert to configuration
    pub fn to_config(&self) -> TokenLimitedChatCompletionContextConfig {
        TokenLimitedChatCompletionContextConfig {
            token_limit: self.token_limit,
            tool_schema: if self.tool_schema.is_empty() { None } else { Some(self.tool_schema.clone()) },
            initial_messages: self.base.initial_messages().map(|msgs| msgs.to_vec()),
        }
    }

    /// Simulate token counting (in a real implementation, this would use the model client)
    fn count_tokens(&self, messages: &[LLMMessage]) -> u32 {
        // Simple approximation: count characters and divide by 4
        // In a real implementation, this would use the model client's token counting
        let total_chars: usize = messages.iter()
            .map(|msg| self.message_to_string(msg).len())
            .sum();
        (total_chars / 4) as u32
    }

    /// Convert a message to string for token counting
    fn message_to_string(&self, message: &LLMMessage) -> String {
        match message {
            LLMMessage::System(msg) => msg.content.clone(),
            LLMMessage::User(msg) => match &msg.content {
                crate::models::UserMessageContent::Text(text) => text.clone(),
                crate::models::UserMessageContent::Mixed(items) => {
                    items.iter()
                        .map(|item| match item {
                            crate::models::UserMessageContentItem::Text { text } => text.clone(),
                            crate::models::UserMessageContentItem::Image { .. } => "[Image]".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            },
            LLMMessage::Assistant(msg) => match &msg.content {
                crate::models::AssistantMessageContent::Text(text) => text.clone(),
                crate::models::AssistantMessageContent::FunctionCalls(calls) => {
                    format!("[{} function calls]", calls.len())
                }
            },
            LLMMessage::FunctionResult(msg) => {
                format!("[{} function results]", msg.content.len())
            }
        }
    }
}

#[async_trait]
impl ChatCompletionContext for TokenLimitedChatCompletionContext {
    async fn add_message(&mut self, message: LLMMessage) -> Result<()> {
        self.base.add_message(message).await
    }

    async fn get_messages(&self) -> Result<Vec<LLMMessage>> {
        let mut messages = self.base.messages().to_vec();
        
        if let Some(limit) = self.token_limit {
            // Remove messages from the middle until we're under the token limit
            while self.count_tokens(&messages) > limit && !messages.is_empty() {
                let middle_index = messages.len() / 2;
                messages.remove(middle_index);
            }
        }
        
        // Handle the case where the first message is a function execution result message
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
