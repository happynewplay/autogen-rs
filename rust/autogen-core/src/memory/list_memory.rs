use crate::model_context::chat_completion_context::ChatCompletionContext;
use crate::cancellation_token::CancellationToken;
use super::base_memory::{
    Memory, MemoryContent, MemoryQueryResult, UpdateContextResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListMemoryConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub memory_contents: Vec<MemoryContent>,
}

pub struct ListMemory {
    name: String,
    contents: Vec<MemoryContent>,
}

impl ListMemory {
    pub fn new(name: Option<String>, memory_contents: Option<Vec<MemoryContent>>) -> Self {
        Self {
            name: name.unwrap_or_else(|| "default_list_memory".to_string()),
            contents: memory_contents.unwrap_or_default(),
        }
    }

    pub fn from_config(config: ListMemoryConfig) -> Self {
        Self::new(config.name, Some(config.memory_contents))
    }

    pub fn to_config(&self) -> ListMemoryConfig {
        ListMemoryConfig {
            name: Some(self.name.clone()),
            memory_contents: self.contents.clone(),
        }
    }
}

#[async_trait]
impl Memory for ListMemory {
    async fn update_context(
        &self,
        _model_context: &mut dyn ChatCompletionContext,
    ) -> Result<UpdateContextResult, Box<dyn Error>> {
        if self.contents.is_empty() {
            return Ok(UpdateContextResult {
                memories: MemoryQueryResult { results: vec![] },
            });
        }

        // The logic to add a SystemMessage to the context will be implemented
        // once ChatCompletionContext is fully defined.
        // For now, we just return the memories.

        Ok(UpdateContextResult {
            memories: MemoryQueryResult {
                results: self.contents.clone(),
            },
        })
    }

    async fn query(
        &self,
        _query: &str,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<MemoryQueryResult, Box<dyn Error>> {
        // Check for cancellation before processing
        if let Some(token) = &cancellation_token {
            token.check_cancelled()?;
        }

        Ok(MemoryQueryResult {
            results: self.contents.clone(),
        })
    }

    async fn add(
        &mut self,
        content: MemoryContent,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), Box<dyn Error>> {
        // Check for cancellation before processing
        if let Some(token) = &cancellation_token {
            token.check_cancelled()?;
        }

        self.contents.push(content);
        Ok(())
    }

    async fn clear(&mut self) -> Result<(), Box<dyn Error>> {
        self.contents.clear();
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        // No resources to clean up
        Ok(())
    }
}