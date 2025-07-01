//! Simple chronological list-based memory implementation.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::{CancellationToken, error::Result};
use super::base_memory::{
    Memory, MemoryContent, MemoryQuery, MemoryQueryResult, UpdateContextResult,
};

/// Configuration for ListMemory component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMemoryConfig {
    /// Optional identifier for this memory instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// List of memory contents stored in this memory instance
    #[serde(default)]
    pub memory_contents: Vec<MemoryContent>,
}

impl Default for ListMemoryConfig {
    fn default() -> Self {
        Self {
            name: None,
            memory_contents: Vec::new(),
        }
    }
}

/// Simple chronological list-based memory implementation.
///
/// This memory implementation stores contents in a list and retrieves them in
/// chronological order. It has an `update_context` method that updates model contexts
/// by appending all stored memories.
///
/// The memory content can be directly accessed and modified through the content property,
/// allowing external applications to manage memory contents directly.
///
/// # Example
///
/// ```rust
/// use autogen_core::memory::{ListMemory, MemoryContent};
/// use autogen_core::model_context::UnboundedChatCompletionContext;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Initialize memory
/// let memory = ListMemory::new(Some("chat_history".to_string()));
///
/// // Add memory content
/// let content = MemoryContent::text("User prefers formal language");
/// memory.add(content, None).await?;
///
/// // Create a model context
/// let mut model_context = UnboundedChatCompletionContext::new();
///
/// // Update a model context with memory
/// memory.update_context(&mut model_context).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ListMemory {
    /// Memory instance identifier
    name: String,
    /// Contents stored in memory
    contents: Arc<RwLock<Vec<MemoryContent>>>,
}

impl ListMemory {
    /// Create a new ListMemory instance
    ///
    /// # Arguments
    /// * `name` - Optional identifier for this memory instance
    pub fn new(name: Option<String>) -> Self {
        Self {
            name: name.unwrap_or_else(|| "default_list_memory".to_string()),
            contents: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new ListMemory instance with initial contents
    ///
    /// # Arguments
    /// * `name` - Optional identifier for this memory instance
    /// * `memory_contents` - Initial memory contents
    pub fn with_contents(
        name: Option<String>,
        memory_contents: Vec<MemoryContent>,
    ) -> Self {
        Self {
            name: name.unwrap_or_else(|| "default_list_memory".to_string()),
            contents: Arc::new(RwLock::new(memory_contents)),
        }
    }

    /// Create a ListMemory from configuration
    pub fn from_config(config: ListMemoryConfig) -> Self {
        Self::with_contents(config.name, config.memory_contents)
    }

    /// Convert to configuration
    pub async fn to_config(&self) -> ListMemoryConfig {
        let contents = self.contents.read().await;
        ListMemoryConfig {
            name: Some(self.name.clone()),
            memory_contents: contents.clone(),
        }
    }

    /// Get the memory instance identifier
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a copy of all memory contents
    pub async fn contents(&self) -> Vec<MemoryContent> {
        self.contents.read().await.clone()
    }

    /// Set the memory contents directly
    pub async fn set_contents(&self, contents: Vec<MemoryContent>) {
        let mut guard = self.contents.write().await;
        *guard = contents;
    }
}

#[async_trait]
impl Memory for ListMemory {
    async fn update_context(
        &self,
        model_context: &mut dyn crate::model_context::ChatCompletionContext,
    ) -> Result<UpdateContextResult> {
        let contents = self.contents.read().await;
        
        if contents.is_empty() {
            return Ok(UpdateContextResult::empty());
        }

        // Convert memory contents to system message and add to context
        let memory_strings: Vec<String> = contents
            .iter()
            .map(|content| match &content.content {
                super::base_memory::ContentType::Text(text) => text.clone(),
                super::base_memory::ContentType::Json(json) => json.to_string(),
                super::base_memory::ContentType::Binary(_) => "[Binary content]".to_string(),
                super::base_memory::ContentType::Image(_) => "[Image content]".to_string(),
            })
            .collect();

        let memory_context = format!("Memory contents:\n{}", memory_strings.join("\n"));
        
        // Add system message to context
        let system_message = crate::models::SystemMessage {
            content: memory_context,
        };

        model_context.add_message(crate::models::LLMMessage::System(system_message)).await?;

        Ok(UpdateContextResult::new(MemoryQueryResult::new(contents.clone())))
    }

    async fn query(
        &self,
        _query: impl Into<MemoryQuery> + Send,
        _cancellation_token: Option<CancellationToken>,
    ) -> Result<MemoryQueryResult> {
        // ListMemory returns all memories without any filtering
        let contents = self.contents.read().await;
        Ok(MemoryQueryResult::new(contents.clone()))
    }

    async fn add_validated(
        &self,
        content: MemoryContent,
        _cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        let mut contents = self.contents.write().await;
        contents.push(content);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut contents = self.contents.write().await;
        contents.clear();
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // No resources to clean up for ListMemory
        Ok(())
    }
}
