//! State management system for agents
//!
//! This module provides comprehensive state management functionality
//! including persistence, serialization, and state validation.

use crate::{AgentId, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::fs;

/// Agent state representation
///
/// This structure represents the complete state of an agent,
/// including both runtime state and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Agent identifier
    pub agent_id: AgentId,
    /// Runtime state data
    pub state: HashMap<String, serde_json::Value>,
    /// State metadata
    pub metadata: StateMetadata,
    /// State version for compatibility checking
    pub version: u32,
}

impl AgentState {
    /// Create a new agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    /// * `state` - The initial state data
    pub fn new(agent_id: AgentId, state: HashMap<String, serde_json::Value>) -> Self {
        Self {
            agent_id,
            state,
            metadata: StateMetadata::new(),
            version: 1,
        }
    }

    /// Get a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    pub fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Some(value) = self.state.get(key) {
            let typed_value: T = serde_json::from_value(value.clone()).map_err(|e| {
                crate::AutoGenError::other(format!("Failed to deserialize state value: {}", e))
            })?;
            Ok(Some(typed_value))
        } else {
            Ok(None)
        }
    }

    /// Set a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    /// * `value` - The state value
    pub fn set<T>(&mut self, key: &str, value: T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value).map_err(|e| {
            crate::AutoGenError::other(format!("Failed to serialize state value: {}", e))
        })?;
        self.state.insert(key.to_string(), json_value);
        self.metadata.update_modified();
        Ok(())
    }

    /// Remove a state value
    ///
    /// # Arguments
    /// * `key` - The state key
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.state.remove(key);
        if result.is_some() {
            self.metadata.update_modified();
        }
        result
    }

    /// Check if a key exists
    ///
    /// # Arguments
    /// * `key` - The state key
    pub fn contains_key(&self, key: &str) -> bool {
        self.state.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<&String> {
        self.state.keys().collect()
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.state.clear();
        self.metadata.update_modified();
    }

    /// Validate the state
    pub fn validate(&self) -> Result<()> {
        // Basic validation - can be extended
        if self.agent_id.agent_type().is_empty() || self.agent_id.key().is_empty() {
            return Err(crate::AutoGenError::other("Invalid agent ID in state"));
        }
        Ok(())
    }
}

/// State metadata
///
/// This structure contains metadata about the agent state,
/// such as checksums and custom fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    /// State checksum for integrity verification
    pub checksum: Option<String>,
    /// Custom metadata fields
    pub custom: HashMap<String, String>,
    /// Modification counter
    pub modification_count: u64,
}

impl StateMetadata {
    /// Create new state metadata
    pub fn new() -> Self {
        Self {
            checksum: None,
            custom: HashMap::new(),
            modification_count: 0,
        }
    }

    /// Update the modification counter
    pub fn update_modified(&mut self) {
        self.modification_count += 1;
    }

    /// Set a custom metadata field
    ///
    /// # Arguments
    /// * `key` - The metadata key
    /// * `value` - The metadata value
    pub fn set_custom<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.custom.insert(key.into(), value.into());
        self.update_modified();
    }
}

impl Default for StateMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for state persistence backends
///
/// This trait defines the interface for different state storage
/// backends (file system, database, cloud storage, etc.).
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save agent state
    ///
    /// # Arguments
    /// * `state` - The agent state to save
    async fn save_state(&self, state: &AgentState) -> Result<()>;

    /// Load agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    ///
    /// # Returns
    /// The loaded agent state if it exists
    async fn load_state(&self, agent_id: &AgentId) -> Result<Option<AgentState>>;

    /// Delete agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    ///
    /// # Returns
    /// True if the state was deleted, false if it didn't exist
    async fn delete_state(&self, agent_id: &AgentId) -> Result<bool>;

    /// List all stored agent IDs
    ///
    /// # Returns
    /// A list of all agent IDs that have stored state
    async fn list_agents(&self) -> Result<Vec<AgentId>>;

    /// Check if state exists for an agent
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    async fn exists(&self, agent_id: &AgentId) -> Result<bool>;
}

/// File system-based state store
///
/// This implementation stores agent state as JSON files
/// in a specified directory structure.
pub struct FileSystemStateStore {
    /// Base directory for state storage
    base_dir: std::path::PathBuf,
}

impl FileSystemStateStore {
    /// Create a new file system state store
    ///
    /// # Arguments
    /// * `base_dir` - The base directory for state storage
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the file path for an agent's state
    fn get_state_path(&self, agent_id: &AgentId) -> std::path::PathBuf {
        let filename = format!("{}_{}.json", agent_id.agent_type(), agent_id.key());
        self.base_dir.join(filename)
    }

    /// Ensure the base directory exists
    async fn ensure_base_dir(&self) -> Result<()> {
        if !self.base_dir.exists() {
            fs::create_dir_all(&self.base_dir).await.map_err(|e| {
                crate::AutoGenError::other(format!("Failed to create state directory: {}", e))
            })?;
        }
        Ok(())
    }
}

#[async_trait]
impl StateStore for FileSystemStateStore {
    async fn save_state(&self, state: &AgentState) -> Result<()> {
        self.ensure_base_dir().await?;

        let path = self.get_state_path(&state.agent_id);
        let json = serde_json::to_string_pretty(state).map_err(|e| {
            crate::AutoGenError::other(format!("Failed to serialize state: {}", e))
        })?;

        fs::write(&path, json).await.map_err(|e| {
            crate::AutoGenError::other(format!("Failed to write state file: {}", e))
        })?;

        Ok(())
    }

    async fn load_state(&self, agent_id: &AgentId) -> Result<Option<AgentState>> {
        let path = self.get_state_path(agent_id);

        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path).await.map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read state file: {}", e))
        })?;

        let state: AgentState = serde_json::from_str(&json).map_err(|e| {
            crate::AutoGenError::other(format!("Failed to deserialize state: {}", e))
        })?;

        // Validate the loaded state
        state.validate()?;

        Ok(Some(state))
    }

    async fn delete_state(&self, agent_id: &AgentId) -> Result<bool> {
        let path = self.get_state_path(agent_id);

        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                crate::AutoGenError::other(format!("Failed to delete state file: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_agents(&self) -> Result<Vec<AgentId>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut agents = Vec::new();
        let mut entries = fs::read_dir(&self.base_dir).await.map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read state directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            crate::AutoGenError::other(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    // Parse filename: "agent_type_key.json"
                    // Use the first underscore as separator since key might contain underscores
                    if let Some(underscore_pos) = filename.find('_') {
                        let agent_type = &filename[..underscore_pos];
                        let key = &filename[underscore_pos + 1..];

                        if let Ok(agent_id) = AgentId::new(agent_type, key) {
                            agents.push(agent_id);
                        }
                    }
                }
            }
        }

        Ok(agents)
    }

    async fn exists(&self, agent_id: &AgentId) -> Result<bool> {
        let path = self.get_state_path(agent_id);
        Ok(path.exists())
    }
}

/// State manager for coordinating state operations
///
/// This manager provides a high-level interface for state management,
/// including automatic persistence and state validation.
pub struct StateManager {
    /// The state store backend
    store: Arc<dyn StateStore>,
}

impl StateManager {
    /// Create a new state manager
    ///
    /// # Arguments
    /// * `store` - The state store backend
    pub fn new(store: Arc<dyn StateStore>) -> Self {
        Self { store }
    }

    /// Save agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    /// * `state` - The state data to save
    pub async fn save_agent_state(
        &self,
        agent_id: &AgentId,
        state: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let agent_state = AgentState::new(agent_id.clone(), state);
        agent_state.validate()?;
        self.store.save_state(&agent_state).await
    }

    /// Load agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    ///
    /// # Returns
    /// The loaded state data if it exists
    pub async fn load_agent_state(
        &self,
        agent_id: &AgentId,
    ) -> Result<Option<HashMap<String, serde_json::Value>>> {
        if let Some(agent_state) = self.store.load_state(agent_id).await? {
            Ok(Some(agent_state.state))
        } else {
            Ok(None)
        }
    }

    /// Delete agent state
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    pub async fn delete_agent_state(&self, agent_id: &AgentId) -> Result<bool> {
        self.store.delete_state(agent_id).await
    }

    /// Check if state exists for an agent
    ///
    /// # Arguments
    /// * `agent_id` - The agent identifier
    pub async fn has_state(&self, agent_id: &AgentId) -> Result<bool> {
        self.store.exists(agent_id).await
    }

    /// List all agents with stored state
    pub async fn list_agents(&self) -> Result<Vec<AgentId>> {
        self.store.list_agents().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_state_operations() {
        let agent_id = AgentId::new("test", "agent").unwrap();
        let mut state = AgentState::new(agent_id, HashMap::new());

        // Test setting and getting values
        state.set("counter", 42u32).unwrap();
        state.set("name", "test_agent".to_string()).unwrap();

        let counter: Option<u32> = state.get("counter").unwrap();
        assert_eq!(counter, Some(42));

        let name: Option<String> = state.get("name").unwrap();
        assert_eq!(name, Some("test_agent".to_string()));

        // Test removing values
        let removed = state.remove("counter");
        assert!(removed.is_some());
        assert!(!state.contains_key("counter"));
    }

    #[tokio::test]
    async fn test_filesystem_state_store() {
        let temp_dir = std::env::temp_dir().join("autogen_test_state");
        let store = FileSystemStateStore::new(&temp_dir);

        let agent_id = AgentId::new("test", "fs_agent").unwrap();
        let mut state_data = HashMap::new();
        state_data.insert("test_key".to_string(), serde_json::json!("test_value"));

        let state = AgentState::new(agent_id.clone(), state_data);

        // Test save and load
        store.save_state(&state).await.unwrap();
        assert!(store.exists(&agent_id).await.unwrap());

        let loaded_state = store.load_state(&agent_id).await.unwrap();
        assert!(loaded_state.is_some());

        let loaded_state = loaded_state.unwrap();
        assert_eq!(loaded_state.agent_id, agent_id);
        assert_eq!(
            loaded_state.state.get("test_key"),
            Some(&serde_json::json!("test_value"))
        );

        // Test list agents
        let agents = store.list_agents().await.unwrap();
        assert!(agents.contains(&agent_id));

        // Test delete
        assert!(store.delete_state(&agent_id).await.unwrap());
        assert!(!store.exists(&agent_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_state_manager() {
        let temp_dir = std::env::temp_dir().join("autogen_test_manager");
        let store = Arc::new(FileSystemStateStore::new(&temp_dir));
        let manager = StateManager::new(store);

        let agent_id = AgentId::new("test", "manager_agent").unwrap();
        let mut state_data = HashMap::new();
        state_data.insert("value".to_string(), serde_json::json!(123));

        // Test save and load through manager
        manager.save_agent_state(&agent_id, state_data.clone()).await.unwrap();
        assert!(manager.has_state(&agent_id).await.unwrap());

        let loaded_state = manager.load_agent_state(&agent_id).await.unwrap();
        assert_eq!(loaded_state, Some(state_data));

        // Test delete
        assert!(manager.delete_agent_state(&agent_id).await.unwrap());
        assert!(!manager.has_state(&agent_id).await.unwrap());
    }
}
