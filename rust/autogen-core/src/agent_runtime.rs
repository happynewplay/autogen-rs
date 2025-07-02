//! Agent runtime system
//!
//! This module provides the core agent runtime interface and types
//! for managing agents in the autogen system, following the Python autogen-core design.
//!
//! This module is only available when the "runtime" feature is enabled.

#[cfg(feature = "runtime")]
use crate::{Agent, AgentId, AgentMetadata, Result, TopicId, Subscription, message::MessageRouter, message::UntypedMessageEnvelope};
#[cfg(feature = "runtime")]
use async_trait::async_trait;
#[cfg(feature = "runtime")]
use std::any::Any;
#[cfg(feature = "runtime")]
use std::collections::HashMap;
#[cfg(feature = "runtime")]
use std::sync::Arc;
#[cfg(feature = "runtime")]
use tokio::sync::RwLock;

#[cfg(feature = "runtime")]
/// Core trait for agent runtimes
///
/// The `AgentRuntime` trait defines the interface for managing agents,
/// message routing, and lifecycle management. This follows the Python
/// autogen-core AgentRuntime protocol.
#[async_trait]
pub trait AgentRuntime: Send + Sync {
    /// Register an agent with the runtime
    ///
    /// # Arguments
    /// * `agent_type` - The type identifier for the agent
    /// * `agent` - The agent instance to register
    /// * `subscriptions` - Optional subscriptions for the agent
    ///
    /// # Returns
    /// The assigned agent ID
    async fn register_agent(
        &mut self,
        agent_type: &str,
        agent: Box<dyn Agent>,
        subscriptions: Option<Vec<Box<dyn Subscription>>>,
    ) -> Result<AgentId>;

    /// Register an agent with a specific ID
    ///
    /// # Arguments
    /// * `agent_id` - The specific ID to assign to the agent
    /// * `agent` - The agent instance to register
    /// * `subscriptions` - Optional subscriptions for the agent
    async fn register_agent_with_id(
        &mut self,
        agent_id: AgentId,
        agent: Box<dyn Agent>,
        subscriptions: Option<Vec<Box<dyn Subscription>>>,
    ) -> Result<()>;

    /// Unregister an agent from the runtime
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to unregister
    async fn unregister_agent(&mut self, agent_id: &AgentId) -> Result<()>;

    /// Send a message to a specific agent
    ///
    /// # Arguments
    /// * `message` - The message to send
    /// * `recipient` - The ID of the recipient agent
    /// * `sender` - Optional ID of the sending agent
    async fn send_message(
        &mut self,
        message: Box<dyn Any + Send>,
        recipient: AgentId,
        sender: Option<AgentId>,
    ) -> Result<()>;

    /// Publish a message to a topic
    ///
    /// # Arguments
    /// * `message` - The message to publish
    /// * `topic_id` - The topic to publish to
    /// * `sender` - Optional ID of the sending agent
    async fn publish_message(
        &mut self,
        message: Box<dyn Any + Send>,
        topic_id: TopicId,
        sender: Option<AgentId>,
    ) -> Result<()>;

    /// Send a request and wait for a response
    ///
    /// # Arguments
    /// * `message` - The request message
    /// * `recipient` - The ID of the recipient agent
    /// * `sender` - Optional ID of the sending agent
    ///
    /// # Returns
    /// The response from the recipient agent
    async fn send_request(
        &mut self,
        message: Box<dyn Any + Send>,
        recipient: AgentId,
        sender: Option<AgentId>,
    ) -> Result<Box<dyn Any + Send>>;

    /// Start the runtime
    ///
    /// This method starts the runtime and begins processing messages.
    async fn start(&mut self) -> Result<()>;

    /// Stop the runtime
    ///
    /// This method stops the runtime and shuts down all agents.
    async fn stop(&mut self) -> Result<()>;

    /// Check if the runtime is running
    fn is_running(&self) -> bool;

    /// Get the number of registered agents
    fn agent_count(&self) -> usize;

    /// Get a list of all registered agent IDs
    fn list_agents(&self) -> Vec<AgentId>;

    /// Save the state of all agents
    ///
    /// # Returns
    /// A map from agent ID to agent state
    async fn save_state(&self) -> Result<HashMap<AgentId, HashMap<String, serde_json::Value>>>;

    /// Load state into agents
    ///
    /// # Arguments
    /// * `state` - The state to load (from save_state)
    async fn load_state(&mut self, state: HashMap<AgentId, HashMap<String, serde_json::Value>>) -> Result<()>;
}

/// High-performance agent registry with optimized lookups
#[cfg(feature = "runtime")]
pub struct AgentRegistry {
    /// Agents indexed by ID for O(1) lookup
    agents: Arc<RwLock<HashMap<AgentId, Arc<RwLock<Box<dyn Agent>>>>>>,
    /// Type-based routing for fast message dispatch
    message_router: Arc<RwLock<MessageRouter>>,
    /// Agent metadata cache
    metadata_cache: Arc<RwLock<HashMap<AgentId, AgentMetadata>>>,
    /// Performance metrics
    metrics: Arc<RwLock<RegistryMetrics>>,
}

#[cfg(feature = "runtime")]
#[derive(Debug, Default)]
/// Performance metrics for the agent registry
pub struct RegistryMetrics {
    /// Total number of registered agents
    pub agent_count: usize,
    /// Total messages routed
    pub messages_routed: u64,
    /// Average message routing time (microseconds)
    pub avg_routing_time_us: f64,
    /// Cache hit rate for agent lookups
    pub cache_hit_rate: f64,
}

#[cfg(feature = "runtime")]
impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            message_router: Arc::new(RwLock::new(MessageRouter::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(RegistryMetrics::default())),
        }
    }

    /// Register an agent with optimized storage
    pub async fn register_agent(&self, agent_id: AgentId, agent: Box<dyn Agent>) -> Result<()> {
        let agent_arc = Arc::new(RwLock::new(agent));

        // Store agent
        {
            let mut agents = self.agents.write().await;
            if agents.insert(agent_id.clone(), agent_arc.clone()).is_some() {
                return Err(crate::AutoGenError::other(format!(
                    "Agent with ID {} already registered", agent_id
                )));
            }
        }

        // Cache metadata
        {
            let agent_guard = agent_arc.read().await;
            let metadata = agent_guard.metadata();
            let mut cache = self.metadata_cache.write().await;
            cache.insert(agent_id.clone(), metadata);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.agent_count += 1;
        }

        Ok(())
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &AgentId) -> Result<()> {
        let removed = {
            let mut agents = self.agents.write().await;
            agents.remove(agent_id)
        };

        if removed.is_some() {
            // Remove from metadata cache
            {
                let mut cache = self.metadata_cache.write().await;
                cache.remove(agent_id);
            }

            // Update metrics
            {
                let mut metrics = self.metrics.write().await;
                metrics.agent_count = metrics.agent_count.saturating_sub(1);
            }

            Ok(())
        } else {
            Err(crate::AutoGenError::agent_not_found(agent_id.clone(), vec![]))
        }
    }

    /// Get an agent by ID with optimized lookup
    pub async fn get_agent(&self, agent_id: &AgentId) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// Get agent metadata from cache
    pub async fn get_agent_metadata(&self, agent_id: &AgentId) -> Option<AgentMetadata> {
        let cache = self.metadata_cache.read().await;
        cache.get(agent_id).cloned()
    }

    /// List all registered agent IDs
    pub async fn list_agents(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// Get the number of registered agents
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Get registry metrics
    pub async fn get_metrics(&self) -> RegistryMetrics {
        let metrics = self.metrics.read().await;
        RegistryMetrics {
            agent_count: metrics.agent_count,
            messages_routed: metrics.messages_routed,
            avg_routing_time_us: metrics.avg_routing_time_us,
            cache_hit_rate: metrics.cache_hit_rate,
        }
    }

    /// Route a message with performance tracking
    pub async fn route_message(&self, envelope: UntypedMessageEnvelope) -> Result<Option<UntypedMessageEnvelope>> {
        let start_time = std::time::Instant::now();

        let result = {
            let router = self.message_router.read().await;
            router.route(envelope)
        };

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.messages_routed += 1;

            let routing_time = start_time.elapsed().as_micros() as f64;
            metrics.avg_routing_time_us =
                (metrics.avg_routing_time_us * (metrics.messages_routed - 1) as f64 + routing_time)
                / metrics.messages_routed as f64;
        }

        result
    }

    /// Batch register multiple agents for better performance
    pub async fn batch_register_agents(&self, agents: Vec<(AgentId, Box<dyn Agent>)>) -> Result<Vec<Result<()>>> {
        let mut results = Vec::with_capacity(agents.len());

        for (agent_id, agent) in agents {
            let result = self.register_agent(agent_id, agent).await;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(feature = "runtime")]
impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "runtime")]
/// Runtime configuration
///
/// Configuration options for agent runtimes.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of concurrent message handlers
    pub max_concurrent_handlers: usize,
    
    /// Message queue size limit
    pub message_queue_size: usize,
    
    /// Whether to enable telemetry
    pub enable_telemetry: bool,
    
    /// Custom properties
    pub properties: HashMap<String, String>,
}

#[cfg(feature = "runtime")]
impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_handlers: 100,
            message_queue_size: 1000,
            enable_telemetry: false,
            properties: HashMap::new(),
        }
    }
}

#[cfg(feature = "runtime")]
/// Runtime statistics
///
/// Statistics about runtime performance and message processing.
#[derive(Debug, Clone, Default)]
pub struct RuntimeStats {
    /// Total number of messages processed
    pub messages_processed: u64,
    
    /// Total number of messages published to topics
    pub messages_published: u64,
    
    /// Total number of RPC requests processed
    pub rpc_requests_processed: u64,
    
    /// Number of currently active agents
    pub active_agents: usize,
    
    /// Number of messages currently in queue
    pub queued_messages: usize,
    
    /// Average message processing time in milliseconds
    pub avg_processing_time_ms: f64,
}

/// Runtime event types
///
/// Events that can be emitted by the runtime for monitoring and debugging.
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    /// An agent was registered
    AgentRegistered {
        /// The agent ID
        agent_id: AgentId,
        /// The agent type
        agent_type: String,
    },
    
    /// An agent was unregistered
    AgentUnregistered {
        /// The agent ID
        agent_id: AgentId,
    },
    
    /// A message was sent
    MessageSent {
        /// The sender agent ID (if any)
        sender: Option<AgentId>,
        /// The recipient agent ID
        recipient: AgentId,
        /// The message type name
        message_type: String,
    },
    
    /// A message was published to a topic
    MessagePublished {
        /// The sender agent ID (if any)
        sender: Option<AgentId>,
        /// The topic ID
        topic_id: TopicId,
        /// The message type name
        message_type: String,
    },
    
    /// An RPC request was made
    RpcRequest {
        /// The sender agent ID (if any)
        sender: Option<AgentId>,
        /// The recipient agent ID
        recipient: AgentId,
        /// The request type name
        request_type: String,
    },
    
    /// The runtime was started
    RuntimeStarted,
    
    /// The runtime was stopped
    RuntimeStopped,
    
    /// An error occurred
    Error {
        /// The error message
        message: String,
        /// The agent ID associated with the error (if any)
        agent_id: Option<AgentId>,
    },
}

/// Trait for runtime event handlers
///
/// Implement this trait to receive runtime events for monitoring and debugging.
#[async_trait]
pub trait RuntimeEventHandler: Send + Sync {
    /// Handle a runtime event
    ///
    /// # Arguments
    /// * `event` - The runtime event
    async fn handle_event(&mut self, event: RuntimeEvent);
}

/// Simple logging event handler
///
/// A basic event handler that logs events to the console.
#[derive(Debug, Default)]
pub struct LoggingEventHandler;

impl LoggingEventHandler {
    /// Create a new logging event handler
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RuntimeEventHandler for LoggingEventHandler {
    async fn handle_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::AgentRegistered { agent_id, agent_type } => {
                println!("Agent registered: {} (type: {})", agent_id, agent_type);
            }
            RuntimeEvent::AgentUnregistered { agent_id } => {
                println!("Agent unregistered: {}", agent_id);
            }
            RuntimeEvent::MessageSent { sender, recipient, message_type } => {
                println!("Message sent: {:?} -> {} ({})", sender, recipient, message_type);
            }
            RuntimeEvent::MessagePublished { sender, topic_id, message_type } => {
                println!("Message published: {:?} -> {} ({})", sender, topic_id, message_type);
            }
            RuntimeEvent::RpcRequest { sender, recipient, request_type } => {
                println!("RPC request: {:?} -> {} ({})", sender, recipient, request_type);
            }
            RuntimeEvent::RuntimeStarted => {
                println!("Runtime started");
            }
            RuntimeEvent::RuntimeStopped => {
                println!("Runtime stopped");
            }
            RuntimeEvent::Error { message, agent_id } => {
                println!("Runtime error: {} (agent: {:?})", message, agent_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_concurrent_handlers, 100);
        assert_eq!(config.message_queue_size, 1000);
        assert!(!config.enable_telemetry);
        assert!(config.properties.is_empty());
    }

    #[test]
    fn test_runtime_stats_default() {
        let stats = RuntimeStats::default();
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.messages_published, 0);
        assert_eq!(stats.rpc_requests_processed, 0);
        assert_eq!(stats.active_agents, 0);
        assert_eq!(stats.queued_messages, 0);
        assert_eq!(stats.avg_processing_time_ms, 0.0);
    }

    #[tokio::test]
    async fn test_logging_event_handler() {
        let mut handler = LoggingEventHandler::new();
        let agent_id = AgentId::new("test", "agent").unwrap();
        
        let event = RuntimeEvent::AgentRegistered {
            agent_id,
            agent_type: "TestAgent".to_string(),
        };
        
        // This should not panic
        handler.handle_event(event).await;
    }
}
