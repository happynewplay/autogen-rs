use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::cancellation_token::CancellationToken;
use std::future::Future;
use std::pin::Pin;

// Represents a code block extracted from an agent message.
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeBlock {
    pub code: String,
    pub language: String,
}

// Result of a code execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeResult {
    pub exit_code: i32,
    pub output: String,
}

// Executes code blocks and returns the result.
#[async_trait]
pub trait CodeExecutor {
    // Execute code blocks and return the result.
    async fn execute_code_blocks(
        &mut self,
        code_blocks: &[CodeBlock],
        cancellation_token: CancellationToken,
    ) -> Result<CodeResult, Box<dyn Error>>;

    // Start the code executor.
    async fn start(&mut self) -> Result<(), Box<dyn Error>>;

    // Stop the code executor and release any resources.
    async fn stop(&mut self) -> Result<(), Box<dyn Error>>;

    // Restart the code executor.
    async fn restart(&mut self) -> Result<(), Box<dyn Error>>;
}

/// RAII guard for code executors that automatically handles cleanup
pub struct CodeExecutorGuard<T: CodeExecutor + Send + 'static> {
    executor: Option<T>,
}

impl<T: CodeExecutor + Send + 'static> CodeExecutorGuard<T> {
    /// Create a new guard and start the executor
    pub async fn new(mut executor: T) -> Result<Self, Box<dyn Error>> {
        executor.start().await?;
        Ok(Self {
            executor: Some(executor),
        })
    }

    /// Get a mutable reference to the executor
    pub fn executor(&mut self) -> &mut T {
        self.executor.as_mut().expect("Executor should be available")
    }

    /// Execute code blocks with the wrapped executor
    pub async fn execute_code_blocks(
        &mut self,
        code_blocks: &[CodeBlock],
        cancellation_token: CancellationToken,
    ) -> Result<CodeResult, Box<dyn Error>> {
        self.executor().execute_code_blocks(code_blocks, cancellation_token).await
    }

    /// Manually stop the executor (will also be called on drop)
    pub async fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut executor) = self.executor.take() {
            executor.stop().await?;
        }
        Ok(())
    }
}

impl<T: CodeExecutor + Send + 'static> Drop for CodeExecutorGuard<T> {
    fn drop(&mut self) {
        if let Some(mut executor) = self.executor.take() {
            // In a real implementation, you might want to use a blocking runtime
            // or spawn a task to handle the async cleanup
            tokio::spawn(async move {
                let _ = executor.stop().await;
            });
        }
    }
}