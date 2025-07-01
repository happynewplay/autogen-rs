//! Model context management for chat completion interactions.
//!
//! This module provides abstractions for managing conversation context
//! with different strategies for memory management and token limits.

mod chat_completion_context;
mod buffered_context;
mod unbounded_context;
mod token_limited_context;
mod head_and_tail_context;

pub use chat_completion_context::{ChatCompletionContext, ChatCompletionContextState};
pub use buffered_context::{BufferedChatCompletionContext, BufferedChatCompletionContextConfig};
pub use unbounded_context::{UnboundedChatCompletionContext, UnboundedChatCompletionContextConfig};
pub use token_limited_context::{TokenLimitedChatCompletionContext, TokenLimitedChatCompletionContextConfig};
pub use head_and_tail_context::{HeadAndTailChatCompletionContext, HeadAndTailChatCompletionContextConfig};
