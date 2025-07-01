//! Cancellation token system
//!
//! This module provides cancellation tokens for cooperative cancellation
//! of async operations, following the Python autogen-core design.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;

/// Token for cooperative cancellation of async operations
///
/// CancellationToken allows operations to be cancelled gracefully,
/// similar to the Python asyncio.CancelledError mechanism.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

#[derive(Debug)]
struct CancellationTokenInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    /// Create a new cancellation token
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// 
    /// let token = CancellationToken::new();
    /// assert!(!token.is_cancelled());
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// Create a cancellation token that is already cancelled
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    ///
    /// let token = CancellationToken::already_cancelled();
    /// assert!(token.is_cancelled());
    /// ```
    pub fn already_cancelled() -> Self {
        let token = Self::new();
        token.cancel();
        token
    }

    /// Create a cancellation token that will be cancelled after a timeout
    ///
    /// # Arguments
    /// * `duration` - Duration after which the token will be cancelled
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let token = CancellationToken::with_timeout(Duration::from_secs(5));
    /// assert!(!token.is_cancelled());
    /// # });
    /// ```
    pub fn with_timeout(duration: Duration) -> Self {
        let token = Self::new();
        let token_clone = token.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            token_clone.cancel();
        });
        
        token
    }

    /// Check if the token has been cancelled
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// 
    /// let token = CancellationToken::new();
    /// assert!(!token.is_cancelled());
    /// 
    /// token.cancel();
    /// assert!(token.is_cancelled());
    /// ```
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    /// Cancel the token
    ///
    /// This will notify all tasks waiting on this token.
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// 
    /// let token = CancellationToken::new();
    /// token.cancel();
    /// assert!(token.is_cancelled());
    /// ```
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }

    /// Wait for the token to be cancelled
    ///
    /// This is useful for implementing cooperative cancellation in async functions.
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    ///
    /// async fn cancellable_operation(token: CancellationToken) {
    ///     tokio::select! {
    ///         _ = token.wait_for_cancellation() => {
    ///             println!("Operation was cancelled");
    ///             return;
    ///         }
    ///         _ = do_work() => {
    ///             println!("Work completed");
    ///         }
    ///     }
    /// }
    ///
    /// async fn do_work() {
    ///     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    /// }
    /// ```
    pub async fn wait_for_cancellation(&self) {
        if self.is_cancelled() {
            return;
        }
        self.inner.notify.notified().await;
    }

    /// Throw a cancellation error if the token is cancelled
    ///
    /// # Errors
    /// Returns `AutoGenError::Cancelled` if the token is cancelled
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// 
    /// async fn check_cancellation(token: CancellationToken) -> autogen_core::Result<()> {
    ///     token.throw_if_cancelled()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn throw_if_cancelled(&self) -> crate::Result<()> {
        if self.is_cancelled() {
            Err(crate::AutoGenError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Run a future with this cancellation token
    ///
    /// The future will be cancelled if the token is cancelled.
    ///
    /// # Arguments
    /// * `future` - The future to run
    ///
    /// # Returns
    /// Result containing the future's output or a cancellation error
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    /// use std::time::Duration;
    /// 
    /// async fn example() -> autogen_core::Result<String> {
    ///     let token = CancellationToken::new();
    ///     
    ///     let result = token.run_with_cancellation(async {
    ///         tokio::time::sleep(Duration::from_millis(100)).await;
    ///         "completed".to_string()
    ///     }).await?;
    ///     
    ///     Ok(result)
    /// }
    /// ```
    pub async fn run_with_cancellation<F, T>(&self, future: F) -> crate::Result<T>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::select! {
            result = future => Ok(result),
            _ = self.wait_for_cancellation() => Err(crate::AutoGenError::Cancelled),
        }
    }

    /// Run a future with a timeout and cancellation
    ///
    /// # Arguments
    /// * `duration` - Maximum time to wait
    /// * `future` - The future to run
    ///
    /// # Returns
    /// Result containing the future's output, timeout error, or cancellation error
    pub async fn run_with_timeout<F, T>(&self, duration: Duration, future: F) -> crate::Result<T>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::select! {
            result = timeout(duration, future) => {
                match result {
                    Ok(value) => Ok(value),
                    Err(_) => Err(crate::AutoGenError::timeout(duration.as_millis() as u64)),
                }
            },
            _ = self.wait_for_cancellation() => Err(crate::AutoGenError::Cancelled),
        }
    }

    /// Create a child token that will be cancelled when this token is cancelled
    ///
    /// # Examples
    /// ```
    /// use autogen_core::CancellationToken;
    ///
    /// # tokio_test::block_on(async {
    /// let parent = CancellationToken::new();
    /// let child = parent.child_token();
    ///
    /// assert!(!child.is_cancelled());
    /// parent.cancel();
    ///
    /// // Give the child token a moment to be notified
    /// tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// assert!(child.is_cancelled());
    /// # });
    /// ```
    pub fn child_token(&self) -> Self {
        let child = Self::new();
        let child_clone = child.clone();
        let parent = self.clone();
        
        tokio::spawn(async move {
            parent.wait_for_cancellation().await;
            child_clone.cancel();
        });
        
        child
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_cancellation_token_creation() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancelled_token() {
        let token = CancellationToken::already_cancelled();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_notification() {
        let token = CancellationToken::new();
        let token_clone = token.clone();
        
        let handle = tokio::spawn(async move {
            token_clone.wait_for_cancellation().await;
            "cancelled"
        });
        
        // Give the task a moment to start waiting
        sleep(Duration::from_millis(10)).await;
        token.cancel();
        
        let result = handle.await.unwrap();
        assert_eq!(result, "cancelled");
    }

    #[tokio::test]
    async fn test_run_with_cancellation() {
        let token = CancellationToken::new();
        
        // Test successful completion
        let result = token.run_with_cancellation(async { "success" }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        
        // Test cancellation
        let token = CancellationToken::already_cancelled();
        let result = token.run_with_cancellation(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            "should not complete"
        }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn test_child_token() {
        let parent = CancellationToken::new();
        let child = parent.child_token();
        
        assert!(!child.is_cancelled());
        parent.cancel();
        
        // Give the child token a moment to be notified
        sleep(Duration::from_millis(10)).await;
        assert!(child.is_cancelled());
    }

    #[tokio::test]
    async fn test_timeout_token() {
        let token = CancellationToken::with_timeout(Duration::from_millis(50));
        assert!(!token.is_cancelled());
        
        sleep(Duration::from_millis(100)).await;
        assert!(token.is_cancelled());
    }
}
