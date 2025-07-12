use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use thiserror::Error;

/// Errors that can occur with cancellation tokens
#[derive(Error, Debug)]
pub enum CancellationError {
    #[error("Operation was cancelled")]
    Cancelled,
    #[error("Timeout occurred")]
    Timeout,
}

type Callback = Box<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<Mutex<Inner>>,
    notify: Arc<Notify>,
}

#[derive(Default)]
struct Inner {
    cancelled: bool,
    callbacks: Vec<Callback>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn cancel(&self) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.cancelled {
            inner.cancelled = true;
            for callback in &inner.callbacks {
                callback();
            }
            inner.callbacks.clear();
            // Notify all waiting tasks
            self.notify.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.lock().unwrap().cancelled
    }

    pub fn add_callback<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
        if inner.cancelled {
            callback();
        } else {
            inner.callbacks.push(Box::new(callback));
        }
    }

    /// Wait for cancellation asynchronously
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }

    /// Check if cancelled and return an error if so
    pub fn check_cancelled(&self) -> Result<(), CancellationError> {
        if self.is_cancelled() {
            Err(CancellationError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Create a child token that will be cancelled when this token is cancelled
    pub fn child(&self) -> CancellationToken {
        let child = CancellationToken::new();
        let child_clone = child.clone();
        self.add_callback(move || {
            child_clone.cancel();
        });
        child
    }

    /// Combine multiple tokens - the result will be cancelled when any of the input tokens is cancelled
    pub fn combine(tokens: Vec<CancellationToken>) -> CancellationToken {
        let combined = CancellationToken::new();
        for token in tokens {
            let combined_clone = combined.clone();
            token.add_callback(move || {
                combined_clone.cancel();
            });
        }
        combined
    }

    /// Create a token that will be cancelled after a timeout
    pub fn with_timeout(duration: std::time::Duration) -> CancellationToken {
        let token = CancellationToken::new();
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            token_clone.cancel();
        });
        token
    }
}

impl fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

impl Serialize for CancellationToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.is_cancelled())
    }
}

impl<'de> Deserialize<'de> for CancellationToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cancelled = bool::deserialize(deserializer)?;
        Ok(CancellationToken {
            inner: Arc::new(Mutex::new(Inner {
                cancelled,
                callbacks: Vec::new(),
            })),
            notify: Arc::new(Notify::new()),
        })
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            notify: Arc::new(Notify::new()),
        }
    }
}