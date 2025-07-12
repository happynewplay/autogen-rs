use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// This trait defines the basic interface for store/cache operations.
///
/// Sub-implementations should handle the lifecycle of underlying storage.
#[async_trait]
pub trait CacheStore<T: Send> {
    /// Retrieve an item from the store.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the item in the store.
    ///
    /// # Returns
    ///
    /// The value associated with the key if found, else `None`.
    async fn get(&self, key: &str) -> Option<T>;

    /// Set an item in the store.
    ///
    /// # Arguments
    ///
    /// * `key` - The key under which the item is to be stored.
    /// * `value` - The value to be stored in the store.
    async fn set(&self, key: String, value: T);
}

/// An in-memory implementation of `CacheStore`.
#[derive(Debug)]
pub struct InMemoryStore<T> {
    store: Arc<Mutex<HashMap<String, T>>>,
}

impl<T> InMemoryStore<T> {
    /// Creates a new `InMemoryStore`.
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<T> Default for InMemoryStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T: Clone + Send + Sync> CacheStore<T> for InMemoryStore<T> {
    async fn get(&self, key: &str) -> Option<T> {
        let store = self.store.lock().unwrap();
        store.get(key).cloned()
    }

    async fn set(&self, key: String, value: T) {
        let mut store = self.store.lock().unwrap();
        store.insert(key, value);
    }
}

// The component model part from the Python code would require a more elaborate
// setup in Rust, likely involving serde and a macro-based system for component
// registration and configuration. For now, we focus on the core cache store
// functionality.