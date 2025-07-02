//! Caching system for agents and runtime
//!
//! This module provides caching functionality for storing and retrieving
//! data efficiently, supporting both in-memory and persistent storage.

use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Core trait for cache storage implementations
///
/// This trait defines the interface for cache storage backends,
/// allowing for different storage strategies (in-memory, persistent, etc.).
#[async_trait]
pub trait CacheStore: Send + Sync {
    /// Get a value from the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// The cached value if it exists and is not expired
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;

    /// Set a value in the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache
    /// * `ttl` - Optional time-to-live for the cached value
    async fn set(&self, key: &str, value: serde_json::Value, ttl: Option<Duration>) -> Result<()>;

    /// Remove a value from the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// True if the key was removed, false if it didn't exist
    async fn remove(&self, key: &str) -> Result<bool>;

    /// Clear all values from the cache
    async fn clear(&self) -> Result<()>;

    /// Check if a key exists in the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// True if the key exists and is not expired
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Get the number of items in the cache
    async fn size(&self) -> Result<usize>;

    /// Get all keys in the cache
    async fn keys(&self) -> Result<Vec<String>>;
}

/// Cache entry with expiration support
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached value
    value: serde_json::Value,
    /// When this entry was created
    created_at: Instant,
    /// Optional expiration time
    expires_at: Option<Instant>,
}

impl CacheEntry {
    /// Create a new cache entry
    fn new(value: serde_json::Value, ttl: Option<Duration>) -> Self {
        let created_at = Instant::now();
        let expires_at = ttl.map(|duration| created_at + duration);
        
        Self {
            value,
            created_at,
            expires_at,
        }
    }

    /// Check if this entry has expired
    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }
}

/// In-memory cache store implementation
///
/// This implementation provides a simple in-memory cache with
/// optional TTL support and automatic cleanup of expired entries.
pub struct InMemoryStore {
    /// The cache storage
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Default TTL for cache entries
    default_ttl: Option<Duration>,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: None,
        }
    }

    /// Create a new in-memory store with a default TTL
    ///
    /// # Arguments
    /// * `default_ttl` - Default time-to-live for cache entries
    pub fn with_default_ttl(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Some(default_ttl),
        }
    }

    /// Clean up expired entries
    fn cleanup_expired(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.retain(|_, entry| !entry.is_expired());
        }
    }

    /// Get the effective TTL (provided TTL or default)
    fn effective_ttl(&self, ttl: Option<Duration>) -> Option<Duration> {
        ttl.or(self.default_ttl)
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheStore for InMemoryStore {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        // Clean up expired entries periodically
        self.cleanup_expired();

        let cache = self.cache.read().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read cache: {}", e))
        })?;

        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                Ok(None)
            } else {
                Ok(Some(entry.value.clone()))
            }
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: serde_json::Value, ttl: Option<Duration>) -> Result<()> {
        let effective_ttl = self.effective_ttl(ttl);
        let entry = CacheEntry::new(value, effective_ttl);

        let mut cache = self.cache.write().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to write cache: {}", e))
        })?;

        cache.insert(key.to_string(), entry);
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<bool> {
        let mut cache = self.cache.write().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to write cache: {}", e))
        })?;

        Ok(cache.remove(key).is_some())
    }

    async fn clear(&self) -> Result<()> {
        let mut cache = self.cache.write().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to write cache: {}", e))
        })?;

        cache.clear();
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let cache = self.cache.read().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read cache: {}", e))
        })?;

        if let Some(entry) = cache.get(key) {
            Ok(!entry.is_expired())
        } else {
            Ok(false)
        }
    }

    async fn size(&self) -> Result<usize> {
        // Clean up expired entries first
        self.cleanup_expired();

        let cache = self.cache.read().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read cache: {}", e))
        })?;

        Ok(cache.len())
    }

    async fn keys(&self) -> Result<Vec<String>> {
        // Clean up expired entries first
        self.cleanup_expired();

        let cache = self.cache.read().map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read cache: {}", e))
        })?;

        Ok(cache.keys().cloned().collect())
    }
}

/// Cache wrapper with typed access
///
/// This wrapper provides a type-safe interface for caching
/// specific types of data with automatic serialization/deserialization.
pub struct TypedCache<T> {
    store: Arc<dyn CacheStore>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TypedCache<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    /// Create a new typed cache
    ///
    /// # Arguments
    /// * `store` - The underlying cache store
    pub fn new(store: Arc<dyn CacheStore>) -> Self {
        Self {
            store,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a typed value from the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// The cached value if it exists and can be deserialized
    pub async fn get(&self, key: &str) -> Result<Option<T>> {
        if let Some(value) = self.store.get(key).await? {
            let typed_value: T = serde_json::from_value(value).map_err(|e| {
                crate::AutoGenError::other(format!("Failed to deserialize cached value: {}", e))
            })?;
            Ok(Some(typed_value))
        } else {
            Ok(None)
        }
    }

    /// Set a typed value in the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache
    /// * `ttl` - Optional time-to-live for the cached value
    pub async fn set(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()> {
        let json_value = serde_json::to_value(value).map_err(|e| {
            crate::AutoGenError::other(format!("Failed to serialize value for caching: {}", e))
        })?;
        self.store.set(key, json_value, ttl).await
    }

    /// Remove a value from the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    pub async fn remove(&self, key: &str) -> Result<bool> {
        self.store.remove(key).await
    }

    /// Check if a key exists in the cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    pub async fn exists(&self, key: &str) -> Result<bool> {
        self.store.exists(key).await
    }
}

impl<T> Clone for TypedCache<T> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;
    use tokio::time::sleep;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[tokio::test]
    async fn test_in_memory_store_basic_operations() {
        let store = InMemoryStore::new();

        // Test set and get
        let value = serde_json::json!({"test": "value"});
        store.set("key1", value.clone(), None).await.unwrap();

        let retrieved = store.get("key1").await.unwrap();
        assert_eq!(retrieved, Some(value));

        // Test exists
        assert!(store.exists("key1").await.unwrap());
        assert!(!store.exists("nonexistent").await.unwrap());

        // Test remove
        assert!(store.remove("key1").await.unwrap());
        assert!(!store.remove("key1").await.unwrap());
        assert!(!store.exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_in_memory_store_ttl() {
        let store = InMemoryStore::new();

        // Set value with short TTL
        let value = serde_json::json!("expires_soon");
        store.set("ttl_key", value.clone(), Some(Duration::from_millis(50))).await.unwrap();

        // Should exist immediately
        assert!(store.exists("ttl_key").await.unwrap());
        assert_eq!(store.get("ttl_key").await.unwrap(), Some(value));

        // Wait for expiration
        sleep(Duration::from_millis(100)).await;

        // Should be expired
        assert!(!store.exists("ttl_key").await.unwrap());
        assert_eq!(store.get("ttl_key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_typed_cache() {
        let store = Arc::new(InMemoryStore::new());
        let cache: TypedCache<TestData> = TypedCache::new(store);

        let test_data = TestData {
            id: 42,
            name: "Test".to_string(),
        };

        // Test set and get
        cache.set("test_key", &test_data, None).await.unwrap();
        let retrieved = cache.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data));

        // Test remove
        assert!(cache.remove("test_key").await.unwrap());
        assert_eq!(cache.get("test_key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_cache_size_and_keys() {
        let store = InMemoryStore::new();

        assert_eq!(store.size().await.unwrap(), 0);
        assert!(store.keys().await.unwrap().is_empty());

        // Add some entries
        store.set("key1", serde_json::json!("value1"), None).await.unwrap();
        store.set("key2", serde_json::json!("value2"), None).await.unwrap();

        assert_eq!(store.size().await.unwrap(), 2);
        let keys = store.keys().await.unwrap();
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));

        // Clear cache
        store.clear().await.unwrap();
        assert_eq!(store.size().await.unwrap(), 0);
    }
}
