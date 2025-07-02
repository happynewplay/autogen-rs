//! Single-threaded agent runtime implementation
//!
//! This module provides a single-threaded implementation of the agent runtime
//! for simple use cases, following the Python autogen-core SingleThreadedAgentRuntime design.

use crate::{
    Agent, AgentId, AgentRuntime, CancellationToken, MessageContext, Result, RuntimeConfig,
    RuntimeEvent, RuntimeEventHandler, RuntimeStats, Subscription, SubscriptionRegistry, TopicId,
    AutoGenError, agent_runtime::AgentRegistry,
};
use async_trait::async_trait;

use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Single-threaded agent runtime with performance optimizations
///
/// A simple, single-threaded implementation of the agent runtime that processes
/// messages sequentially. This version uses optimized data structures for better
/// performance while maintaining simplicity.
pub struct SingleThreadedAgentRuntime {
    /// High-performance agent registry
    agent_registry: AgentRegistry,

    /// Subscription registry
    subscription_registry: SubscriptionRegistry,

    /// Runtime configuration
    config: RuntimeConfig,

    /// Runtime statistics
    stats: RuntimeStats,

    /// Whether the runtime is currently running
    is_running: bool,

    /// Event handlers
    event_handlers: Vec<Box<dyn RuntimeEventHandler>>,

    /// Message queue sender
    message_sender: Option<mpsc::UnboundedSender<RuntimeMessage>>,

    /// Message queue receiver
    message_receiver: Option<mpsc::UnboundedReceiver<RuntimeMessage>>,

    /// Cancellation token for stopping the runtime
    cancellation_token: CancellationToken,

    /// Performance metrics tracking
    performance_metrics: PerformanceMetrics,
}

/// Performance metrics for the runtime
#[derive(Debug, Default, Clone)]
pub struct PerformanceMetrics {
    /// Total messages processed
    pub messages_processed: u64,
    /// Average message processing time (microseconds)
    pub avg_processing_time_us: f64,
    /// Peak memory usage (bytes)
    pub peak_memory_usage: usize,
    /// Agent lookup cache hits
    pub cache_hits: u64,
    /// Agent lookup cache misses
    pub cache_misses: u64,
}

/// Internal message type for the runtime
#[derive(Debug)]
enum RuntimeMessage {
    /// Direct message to a specific agent
    DirectMessage {
        message: crate::TypeSafeMessage,
        recipient: AgentId,
        sender: Option<AgentId>,
        response_sender: Option<tokio::sync::oneshot::Sender<Result<crate::TypeSafeMessage>>>,
    },

    /// Topic-based message
    TopicMessage {
        message: crate::TypeSafeMessage,
        topic_id: TopicId,
        sender: Option<AgentId>,
    },
    
    /// Shutdown signal
    Shutdown,
}

impl SingleThreadedAgentRuntime {
    /// Create a new single-threaded agent runtime
    ///
    /// # Examples
    /// ```
    /// use autogen_core::{SingleThreadedAgentRuntime, AgentRuntime};
    ///
    /// let runtime = SingleThreadedAgentRuntime::new();
    /// assert!(!runtime.is_running());
    /// ```
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            agent_registry: AgentRegistry::new(),
            subscription_registry: SubscriptionRegistry::new(),
            config: RuntimeConfig::default(),
            stats: RuntimeStats::default(),
            is_running: false,
            event_handlers: Vec::new(),
            message_sender: Some(sender),
            message_receiver: Some(receiver),
            cancellation_token: CancellationToken::new(),
            performance_metrics: PerformanceMetrics::default(),
        }
    }

    /// Create a new single-threaded agent runtime with configuration
    ///
    /// # Arguments
    /// * `config` - Runtime configuration
    ///
    /// # Examples
    /// ```
    /// use autogen_core::{SingleThreadedAgentRuntime, RuntimeConfig, AgentRuntime};
    ///
    /// let config = RuntimeConfig::default();
    /// let runtime = SingleThreadedAgentRuntime::with_config(config);
    /// assert!(!runtime.is_running());
    /// ```
    pub fn with_config(config: RuntimeConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            agent_registry: AgentRegistry::new(),
            subscription_registry: SubscriptionRegistry::new(),
            config,
            stats: RuntimeStats::default(),
            is_running: false,
            event_handlers: Vec::new(),
            performance_metrics: PerformanceMetrics::default(),
            message_sender: Some(sender),
            message_receiver: Some(receiver),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Add an event handler to the runtime
    ///
    /// # Arguments
    /// * `handler` - The event handler to add
    pub fn add_event_handler(&mut self, handler: Box<dyn RuntimeEventHandler>) {
        self.event_handlers.push(handler);
    }

    /// Get runtime statistics
    pub fn stats(&self) -> &RuntimeStats {
        &self.stats
    }

    /// Emit a runtime event to all handlers
    async fn emit_event(&mut self, event: RuntimeEvent) {
        for handler in &mut self.event_handlers {
            handler.handle_event(event.clone()).await;
        }
    }

    /// Process a single runtime message
    async fn process_message(&mut self, message: RuntimeMessage) -> Result<()> {
        match message {
            RuntimeMessage::DirectMessage {
                message,
                recipient,
                sender,
                response_sender,
            } => {
                self.stats.messages_processed += 1;
                
                if let Some(agent_arc) = self.agent_registry.get_agent(&recipient).await {
                    let context = MessageContext::direct_message(sender.clone(), self.cancellation_token.clone());

                    let mut agent = agent_arc.write().await;
                    let result = agent.handle_message(message, &context).await;

                    if let Some(sender_channel) = response_sender {
                        match result {
                            Ok(Some(response)) => {
                                let _ = sender_channel.send(Ok(response));
                            }
                            Ok(None) => {
                                let _ = sender_channel.send(Err(AutoGenError::other(
                                    "No response from agent"
                                )));
                            }
                            Err(e) => {
                                let _ = sender_channel.send(Err(e));
                            }
                        }
                    } else if let Err(e) = result {
                        self.emit_event(RuntimeEvent::Error {
                            message: format!("Error processing message: {}", e),
                            agent_id: Some(recipient),
                        }).await;
                    }
                } else {
                    let error = AutoGenError::Agent(crate::error::AgentError::NotFound {
                        agent_id: recipient.to_string(),
                    });
                    if let Some(sender_channel) = response_sender {
                        let _ = sender_channel.send(Err(error));
                    } else {
                        self.emit_event(RuntimeEvent::Error {
                            message: format!("Agent not found: {}", recipient),
                            agent_id: Some(recipient),
                        }).await;
                    }
                }
            }
            
            RuntimeMessage::TopicMessage {
                message,
                topic_id,
                sender,
            } => {
                self.stats.messages_published += 1;

                let message_type_id = std::any::TypeId::of::<crate::TypeSafeMessage>();
                let matching_agents = self.subscription_registry.find_matching_agents(&topic_id, message_type_id);

                // For topic messages, we need to handle the fact that we can't clone arbitrary messages
                // In a real implementation, we would use Arc<dyn Any + Send + Sync> or implement
                // a proper message cloning mechanism. For now, we'll process agents sequentially
                // and only the first matching agent will receive the message.
                if let Some(first_agent_id) = matching_agents.iter().next() {
                    if let Some(agent_arc) = self.agent_registry.get_agent(first_agent_id).await {
                        let context = MessageContext::topic_message(
                            sender.clone(),
                            topic_id.clone(),
                            self.cancellation_token.clone(),
                        );

                        let mut agent = agent_arc.write().await;
                        if let Err(e) = agent.handle_message(message, &context).await {
                            self.emit_event(RuntimeEvent::Error {
                                message: format!("Error processing topic message: {}", e),
                                agent_id: Some(first_agent_id.clone()),
                            }).await;
                        }
                    }
                }

                // TODO: Implement proper message broadcasting with Arc or cloning mechanism
                if matching_agents.len() > 1 {
                    self.emit_event(RuntimeEvent::Error {
                        message: format!("Topic message broadcasting to multiple agents not yet implemented. Only first agent received the message."),
                        agent_id: None,
                    }).await;
                }
            }
            
            RuntimeMessage::Shutdown => {
                self.is_running = false;
                return Ok(());
            }
        }
        
        Ok(())
    }

    /// Main message processing loop
    async fn message_loop(&mut self) -> Result<()> {
        let mut receiver = self.message_receiver.take().unwrap();
        
        while self.is_running {
            tokio::select! {
                message = receiver.recv() => {
                    match message {
                        Some(msg) => {
                            if let Err(e) = self.process_message(msg).await {
                                self.emit_event(RuntimeEvent::Error {
                                    message: format!("Error in message loop: {}", e),
                                    agent_id: None,
                                }).await;
                            }
                        }
                        None => {
                            // Channel closed, stop the runtime
                            self.is_running = false;
                            break;
                        }
                    }
                }
                _ = self.cancellation_token.wait_for_cancellation() => {
                    self.is_running = false;
                    break;
                }
            }
        }
        
        self.message_receiver = Some(receiver);
        Ok(())
    }
}

impl Default for SingleThreadedAgentRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRuntime for SingleThreadedAgentRuntime {
    async fn register_agent(
        &mut self,
        agent_type: &str,
        agent: Box<dyn Agent>,
        subscriptions: Option<Vec<Box<dyn Subscription>>>,
    ) -> Result<AgentId> {
        let agent_id = AgentId::new(agent_type, Uuid::new_v4().to_string())?;
        
        // TODO: Implement proper runtime handle
        // agent.bind_id_and_runtime(agent_id.clone(), runtime_handle).await?;
        
        self.agent_registry.register_agent(agent_id.clone(), agent).await?;

        if let Some(subs) = subscriptions {
            for subscription in subs {
                self.subscription_registry.subscribe(agent_id.clone(), subscription);
            }
        }

        self.stats.active_agents = self.agent_registry.agent_count().await;
        
        self.emit_event(RuntimeEvent::AgentRegistered {
            agent_id: agent_id.clone(),
            agent_type: agent_type.to_string(),
        }).await;
        
        Ok(agent_id)
    }

    async fn register_agent_with_id(
        &mut self,
        agent_id: AgentId,
        agent: Box<dyn Agent>,
        subscriptions: Option<Vec<Box<dyn Subscription>>>,
    ) -> Result<()> {
        // Check if agent already exists
        let agent_list = self.agent_registry.list_agents().await;
        if agent_list.contains(&agent_id) {
            return Err(AutoGenError::other(format!("Agent with ID {} already exists", agent_id)));
        }

        // TODO: Implement proper runtime handle
        // agent.bind_id_and_runtime(agent_id.clone(), runtime_handle).await?;

        self.agent_registry.register_agent(agent_id.clone(), agent).await?;

        if let Some(subs) = subscriptions {
            for subscription in subs {
                self.subscription_registry.subscribe(agent_id.clone(), subscription);
            }
        }

        self.stats.active_agents = self.agent_registry.agent_count().await;
        
        self.emit_event(RuntimeEvent::AgentRegistered {
            agent_id: agent_id.clone(),
            agent_type: agent_id.agent_type().to_string(),
        }).await;
        
        Ok(())
    }

    async fn unregister_agent(&mut self, agent_id: &AgentId) -> Result<()> {
        match self.agent_registry.unregister_agent(agent_id).await {
            Ok(_) => {
                self.subscription_registry.unsubscribe_all(agent_id);
                self.stats.active_agents = self.agent_registry.agent_count().await;

                self.emit_event(RuntimeEvent::AgentUnregistered {
                    agent_id: agent_id.clone(),
                }).await;

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn send_message(
        &mut self,
        message: crate::TypeSafeMessage,
        recipient: AgentId,
        sender: Option<AgentId>,
    ) -> Result<()> {
        if let Some(sender_ref) = &self.message_sender {
            let runtime_message = RuntimeMessage::DirectMessage {
                message,
                recipient: recipient.clone(),
                sender: sender.clone(),
                response_sender: None,
            };
            
            sender_ref.send(runtime_message).map_err(|_| {
                AutoGenError::other("Failed to send message to runtime queue")
            })?;
            
            self.emit_event(RuntimeEvent::MessageSent {
                sender,
                recipient,
                message_type: "Unknown".to_string(), // TODO: Get actual type name
            }).await;
            
            Ok(())
        } else {
            Err(AutoGenError::other("Runtime message sender not available"))
        }
    }

    async fn publish_message(
        &mut self,
        message: crate::TypeSafeMessage,
        topic_id: TopicId,
        sender: Option<AgentId>,
    ) -> Result<()> {
        if let Some(sender_ref) = &self.message_sender {
            let runtime_message = RuntimeMessage::TopicMessage {
                message,
                topic_id: topic_id.clone(),
                sender: sender.clone(),
            };
            
            sender_ref.send(runtime_message).map_err(|_| {
                AutoGenError::other("Failed to send message to runtime queue")
            })?;
            
            self.emit_event(RuntimeEvent::MessagePublished {
                sender,
                topic_id,
                message_type: "Unknown".to_string(), // TODO: Get actual type name
            }).await;
            
            Ok(())
        } else {
            Err(AutoGenError::other("Runtime message sender not available"))
        }
    }

    async fn send_request(
        &mut self,
        message: crate::TypeSafeMessage,
        recipient: AgentId,
        sender: Option<AgentId>,
    ) -> Result<crate::TypeSafeMessage> {
        self.stats.rpc_requests_processed += 1;
        
        if let Some(sender_ref) = &self.message_sender {
            let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
            
            let runtime_message = RuntimeMessage::DirectMessage {
                message,
                recipient: recipient.clone(),
                sender: sender.clone(),
                response_sender: Some(response_sender),
            };
            
            sender_ref.send(runtime_message).map_err(|_| {
                AutoGenError::other("Failed to send request to runtime queue")
            })?;
            
            self.emit_event(RuntimeEvent::RpcRequest {
                sender,
                recipient,
                request_type: "Unknown".to_string(), // TODO: Get actual type name
            }).await;
            
            response_receiver.await.map_err(|_| {
                AutoGenError::other("Failed to receive response")
            })?
        } else {
            Err(AutoGenError::other("Runtime message sender not available"))
        }
    }

    async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(AutoGenError::other("Runtime is already running"));
        }
        
        self.is_running = true;
        self.emit_event(RuntimeEvent::RuntimeStarted).await;
        
        // Start all agents
        let agent_ids = self.agent_registry.list_agents().await;
        for agent_id in agent_ids {
            if let Some(agent_arc) = self.agent_registry.get_agent(&agent_id).await {
                let mut agent = agent_arc.write().await;
                agent.on_start().await?;
            }
        }
        
        // Start the message processing loop
        self.message_loop().await?;
        
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }
        
        // Send shutdown signal
        if let Some(sender) = &self.message_sender {
            let _ = sender.send(RuntimeMessage::Shutdown);
        }
        
        // Cancel the runtime
        self.cancellation_token.cancel();
        
        // Stop all agents
        let agent_ids = self.agent_registry.list_agents().await;
        for agent_id in agent_ids {
            if let Some(agent_arc) = self.agent_registry.get_agent(&agent_id).await {
                let mut agent = agent_arc.write().await;
                agent.on_stop().await?;
            }
        }
        
        self.is_running = false;
        self.emit_event(RuntimeEvent::RuntimeStopped).await;
        
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn agent_count(&self) -> usize {
        // For now, return 0. In a real implementation, we would need to make this async
        // or cache the count in the runtime
        0
    }

    fn list_agents(&self) -> Vec<AgentId> {
        // For now, return empty. In a real implementation, we would need to make this async
        // or cache the list in the runtime
        Vec::new()
    }

    async fn save_state(&self) -> Result<HashMap<AgentId, HashMap<String, serde_json::Value>>> {
        let mut state = HashMap::new();

        let agent_ids = self.agent_registry.list_agents().await;
        for agent_id in agent_ids {
            if let Some(agent_arc) = self.agent_registry.get_agent(&agent_id).await {
                let agent = agent_arc.read().await;
                let agent_state = agent.save_state().await?;
                state.insert(agent_id, agent_state);
            }
        }

        Ok(state)
    }

    async fn load_state(&mut self, state: HashMap<AgentId, HashMap<String, serde_json::Value>>) -> Result<()> {
        for (agent_id, agent_state) in state {
            if let Some(agent_arc) = self.agent_registry.get_agent(&agent_id).await {
                let mut agent = agent_arc.write().await;
                agent.load_state(agent_state).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_single_threaded_runtime_creation() {
        let runtime = SingleThreadedAgentRuntime::new();
        assert!(!runtime.is_running());
        assert_eq!(runtime.agent_count(), 0);
        assert!(runtime.list_agents().is_empty());
    }

    #[test]
    fn test_single_threaded_runtime_with_config() {
        let config = RuntimeConfig {
            max_concurrent_handlers: 50,
            message_queue_size: 500,
            enable_telemetry: true,
            properties: HashMap::new(),
        };
        
        let runtime = SingleThreadedAgentRuntime::with_config(config.clone());
        assert!(!runtime.is_running());
        assert_eq!(runtime.config.max_concurrent_handlers, 50);
        assert_eq!(runtime.config.message_queue_size, 500);
        assert!(runtime.config.enable_telemetry);
    }

    #[test]
    fn test_runtime_stats() {
        let runtime = SingleThreadedAgentRuntime::new();
        let stats = runtime.stats();
        
        assert_eq!(stats.messages_processed, 0);
        assert_eq!(stats.messages_published, 0);
        assert_eq!(stats.rpc_requests_processed, 0);
        assert_eq!(stats.active_agents, 0);
        assert_eq!(stats.queued_messages, 0);
        assert_eq!(stats.avg_processing_time_ms, 0.0);
    }
}
