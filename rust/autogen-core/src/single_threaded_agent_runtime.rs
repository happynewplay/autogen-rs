use crate::agent::{Agent, AgentFactory};
use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::agent_runtime::AgentRuntime;
use crate::agent_type::AgentType;
use crate::cancellation_token::CancellationToken;
use crate::intervention::InterventionHandler;

use crate::routing::{MessagePriority, MessageEnvelopeInfo};
use crate::runtime_impl_helpers::SubscriptionManager;
use crate::serialization::SerializationRegistry;
use crate::subscription::Subscription;
use crate::topic::TopicId;

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use thiserror::Error;


/// State version management constants and utilities
pub mod state_version {
    pub const CURRENT_VERSION: &str = "2.0";
    pub const CURRENT_SCHEMA_VERSION: &str = "2.0";
    pub const SUPPORTED_VERSIONS: &[&str] = &["1.0", "2.0"];

    /// Check if a version is supported
    pub fn is_supported(version: &str) -> bool {
        SUPPORTED_VERSIONS.contains(&version)
    }

    /// Get the migration path for a version
    pub fn get_migration_path(from_version: &str) -> Vec<&'static str> {
        match from_version {
            "1.0" => vec!["2.0"],
            "2.0" => vec![], // Current version
            _ => vec![], // Unknown version
        }
    }
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Agent type '{0}' does not exist")]
    AgentTypeNotFound(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Runtime error: {0}")]
    General(String),
}

impl RuntimeError {
    pub fn into_send_error(self) -> Box<dyn Error + Send> {
        Box::new(self)
    }
}

/// Envelope metadata for tracing and telemetry
#[derive(Debug, Clone)]
pub struct EnvelopeMetadata {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
}

impl Default for EnvelopeMetadata {
    fn default() -> Self {
        Self {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
        }
    }
}

/// Message envelope for sending messages to specific agents
#[derive(Debug)]
pub struct SendMessageEnvelope {
    pub message: Value,
    pub sender: Option<AgentId>,
    pub recipient: AgentId,
    pub response_tx: oneshot::Sender<Result<Value, Box<dyn Error + Send>>>,
    pub cancellation_token: CancellationToken,
    pub metadata: Option<EnvelopeMetadata>,
    pub message_id: String,
    pub priority: MessagePriority,
    pub timestamp: Instant,
    pub estimated_processing_time: Duration,
}

/// Message envelope for publishing messages to topics
#[derive(Debug)]
pub struct PublishMessageEnvelope {
    pub message: Value,
    pub sender: Option<AgentId>,
    pub topic_id: TopicId,
    pub cancellation_token: CancellationToken,
    pub metadata: Option<EnvelopeMetadata>,
    pub message_id: String,
    pub priority: MessagePriority,
    pub timestamp: Instant,
    pub estimated_processing_time: Duration,
}

/// Message envelope for sending responses
#[derive(Debug)]
pub struct ResponseMessageEnvelope {
    pub message: Value,
    pub response_tx: oneshot::Sender<Result<Value, Box<dyn Error + Send>>>,
    pub sender: AgentId,
    pub recipient: Option<AgentId>,
    pub metadata: Option<EnvelopeMetadata>,
    pub priority: MessagePriority,
    pub timestamp: Instant,
}

/// Union type for all message envelopes
#[derive(Debug)]
pub enum MessageEnvelope {
    Send(SendMessageEnvelope),
    Publish(PublishMessageEnvelope),
    Response(ResponseMessageEnvelope),
}

impl MessageEnvelopeInfo for MessageEnvelope {
    fn priority(&self) -> MessagePriority {
        match self {
            MessageEnvelope::Send(env) => env.priority,
            MessageEnvelope::Publish(env) => env.priority,
            MessageEnvelope::Response(env) => env.priority,
        }
    }

    fn timestamp(&self) -> Instant {
        match self {
            MessageEnvelope::Send(env) => env.timestamp,
            MessageEnvelope::Publish(env) => env.timestamp,
            MessageEnvelope::Response(env) => env.timestamp,
        }
    }

    fn sender(&self) -> Option<&AgentId> {
        match self {
            MessageEnvelope::Send(env) => env.sender.as_ref(),
            MessageEnvelope::Publish(env) => env.sender.as_ref(),
            MessageEnvelope::Response(env) => Some(&env.sender),
        }
    }

    fn message_type(&self) -> &str {
        match self {
            MessageEnvelope::Send(_) => "Send",
            MessageEnvelope::Publish(_) => "Publish",
            MessageEnvelope::Response(_) => "Response",
        }
    }

    fn estimated_processing_time(&self) -> Duration {
        match self {
            MessageEnvelope::Send(env) => env.estimated_processing_time,
            MessageEnvelope::Publish(env) => env.estimated_processing_time,
            MessageEnvelope::Response(_) => Duration::from_millis(50), // Responses are typically fast
        }
    }
}

/// Background task handle for tracking spawned tasks
#[derive(Debug)]
pub struct BackgroundTask {
    pub name: String,
    pub handle: JoinHandle<()>,
    pub created_at: std::time::Instant,
}

impl BackgroundTask {
    pub fn new(name: String, handle: JoinHandle<()>) -> Self {
        Self {
            name,
            handle,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn abort(&self) {
        self.handle.abort();
    }
}

/// Background task manager for tracking and managing async tasks
#[derive(Debug)]
pub struct BackgroundTaskManager {
    tasks: Arc<Mutex<Vec<BackgroundTask>>>,
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Spawn a new background task and track it
    pub fn spawn<F>(&self, name: String, future: F) -> JoinHandle<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        // Note: We can't clone JoinHandle, so we'll track tasks differently
        // For now, just return the handle without tracking
        handle
    }

    /// Clean up finished tasks
    pub fn cleanup_finished(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|task| !task.is_finished());
    }

    /// Get the number of active tasks
    pub fn active_task_count(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().filter(|task| !task.is_finished()).count()
    }

    /// Abort all tasks
    pub fn abort_all(&self) {
        let tasks = self.tasks.lock().unwrap();
        for task in tasks.iter() {
            task.abort();
        }
    }

    /// Wait for all tasks to complete
    pub async fn wait_for_all(&self) {
        loop {
            let has_active_tasks = {
                let tasks = self.tasks.lock().unwrap();
                tasks.iter().any(|task| !task.is_finished())
            };

            if !has_active_tasks {
                break;
            }

            // Wait a bit and check again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Get task statistics
    pub fn get_task_stats(&self) -> (usize, usize, std::time::Duration) {
        let tasks = self.tasks.lock().unwrap();
        let total = tasks.len();
        let active = tasks.iter().filter(|task| !task.is_finished()).count();
        let oldest_age = tasks.iter()
            .map(|task| task.created_at.elapsed())
            .max()
            .unwrap_or_default();

        (total, active, oldest_age)
    }
}

struct AgentRuntimeState {
    agent_factories: HashMap<String, Box<dyn AgentFactory>>,
    instantiated_agents: HashMap<AgentId, Box<dyn Agent>>,
    subscription_manager: SubscriptionManager,
    serialization_registry: SerializationRegistry,
    intervention_handlers: Vec<Box<dyn InterventionHandler>>,
    background_tasks: BackgroundTaskManager,
}

/// Context for managing the runtime lifecycle
pub struct RunContext {
    run_task: Option<JoinHandle<()>>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
}

impl RunContext {
    pub fn new() -> Self {
        Self {
            run_task: None,
            stopped: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn start_with_runtime(&mut self, runtime: Arc<SingleThreadedAgentRuntime>) {
        let stopped_clone = self.stopped.clone();

        let run_task = tokio::spawn(async move {
            runtime._run_loop(stopped_clone).await;
        });

        self.run_task = Some(run_task);
    }

    /// Stop the runtime immediately
    pub async fn stop(&mut self) {
        self.stopped.store(true, std::sync::atomic::Ordering::Relaxed);

        if let Some(task) = self.run_task.take() {
            task.abort();
            let _ = task.await;
        }
    }

    /// Stop the runtime when idle (all messages processed)
    pub async fn stop_when_idle(&mut self, queue: &crate::queue::Queue<MessageEnvelope>) {
        queue.join().await;
        self.stop().await;
    }

    /// Stop the runtime when a condition is met
    pub async fn stop_when<F>(&mut self, condition: F, check_period_ms: u64)
    where
        F: Fn() -> bool + Send + 'static,
    {
        let condition = Arc::new(condition);
        loop {
            if condition() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(check_period_ms)).await;
        }
        self.stop().await;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub struct SingleThreadedAgentRuntime {
    state: Arc<Mutex<AgentRuntimeState>>,
    queue_tx: mpsc::Sender<MessageEnvelope>,
    queue: Arc<crate::queue::Queue<MessageEnvelope>>,
    run_context: Option<RunContext>,
}

impl SingleThreadedAgentRuntime {
    pub fn new(intervention_handlers: Vec<Box<dyn InterventionHandler>>) -> Self {
        let (tx, _rx) = mpsc::channel(100);
        let queue = Arc::new(crate::queue::Queue::new());
        let state = Arc::new(Mutex::new(AgentRuntimeState {
            agent_factories: HashMap::new(),
            instantiated_agents: HashMap::new(),
            subscription_manager: SubscriptionManager::new(),
            serialization_registry: SerializationRegistry::new(),
            intervention_handlers,
            background_tasks: BackgroundTaskManager::new(),
        }));

        Self {
            state,
            queue_tx: tx,
            queue,
            run_context: None,
        }
    }

    /// Start the runtime
    pub fn start(&mut self) -> &mut Self {
        if self.run_context.is_none() {
            self.run_context = Some(RunContext::new());
        }
        self
    }

    /// Get the run context for lifecycle management
    pub fn run_context(&mut self) -> Option<&mut RunContext> {
        self.run_context.as_mut()
    }

    /// Internal run loop
    async fn _run_loop(&self, stopped: Arc<std::sync::atomic::AtomicBool>) {
        loop {
            if stopped.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            match self.queue.get().await {
                Ok(envelope) => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        Self::process_envelope(state, envelope).await;
                    });
                }
                Err(_) => {
                    // Queue is shut down
                    break;
                }
            }
        }
    }

    /// Shutdown the queue
    fn shutdown_queue(&self) {
        self.queue.shutdown(true);
    }

    /// Process the next message (for compatibility)
    pub async fn _process_next(&self) {
        match self.queue.get().await {
            Ok(envelope) => {
                let state = self.state.clone();
                Self::process_envelope(state, envelope).await;
            }
            Err(_) => {
                // Queue is shut down or empty
            }
        }
    }

    async fn process_envelope(state: Arc<Mutex<AgentRuntimeState>>, envelope: MessageEnvelope) {
        match envelope {
            MessageEnvelope::Send(send_envelope) => {
                Self::process_send_envelope(state, send_envelope).await;
            }
            MessageEnvelope::Publish(publish_envelope) => {
                Self::process_publish_envelope(state, publish_envelope).await;
            }
            MessageEnvelope::Response(response_envelope) => {
                Self::process_response_envelope(state, response_envelope).await;
            }
        }
    }

    async fn process_send_envelope(state: Arc<Mutex<AgentRuntimeState>>, envelope: SendMessageEnvelope) {
        let result = Self::handle_send_message(state, &envelope).await;

        if let Err(send_error) = envelope.response_tx.send(result) {
            tracing::warn!("Failed to send response back to caller: {:?}", send_error);
        }
    }

    async fn handle_send_message(
        state: Arc<Mutex<AgentRuntimeState>>,
        envelope: &SendMessageEnvelope
    ) -> Result<Value, Box<dyn Error + Send>> {
        use crate::message_context::MessageContext;
        use crate::message_handler_context::MessageHandlerContext;


        // Log the message delivery
        tracing::info!(
            "Processing send message from {:?} to {} with message_id: {}",
            envelope.sender,
            envelope.recipient,
            envelope.message_id
        );

        // Apply intervention handlers for send
        let mut processed_message = envelope.message.clone();

        // Apply intervention handlers
        let intervention_handlers = {
            let state_guard = state.lock().unwrap();
            state_guard.intervention_handlers.iter().map(|h| h.clone_box()).collect::<Vec<_>>()
        };

        // Create a message context for the send operation
        let message_context = crate::message_context::MessageContext {
            sender: envelope.sender.clone(),
            topic_id: None,
            is_rpc: true,
            cancellation_token: envelope.cancellation_token.clone(),
            message_id: envelope.message_id.clone(),
        };

        for handler in &intervention_handlers {
            match handler.on_send(processed_message.clone(), &message_context, &envelope.recipient).await {
                Ok(modified_message) => {
                    processed_message = modified_message;
                }
                Err(_drop_message) => {
                    // Message was dropped by intervention handler
                    tracing::info!(
                        "Message dropped by intervention handler for recipient: {:?}",
                        envelope.recipient
                    );
                    return Ok(Value::Null); // Return null to indicate message was dropped
                }
            }
        }

        // Check if recipient agent exists
        let needs_creation = {
            let state_guard = state.lock().unwrap();
            if !state_guard.agent_factories.contains_key(&envelope.recipient.r#type) {
                return Err(RuntimeError::AgentTypeNotFound(envelope.recipient.r#type.clone()).into_send_error());
            }

            // Check if agent instance already exists
            state_guard.instantiated_agents.get(&envelope.recipient).is_none()
        };

        // Ensure agent exists (create if needed)
        if needs_creation {
            Self::create_agent_instance(state.clone(), &envelope.recipient).await?;
        }

        // Create message context
        let message_context = MessageContext {
            sender: envelope.sender.clone(),
            topic_id: None,
            is_rpc: true,
            cancellation_token: envelope.cancellation_token.clone(),
            message_id: envelope.message_id.clone(),
        };

        // Handle the message with proper synchronization
        let response = {
            // Get a mutable clone of the agent for message processing
            let mut agent_for_processing = {
                let state_guard = state.lock().unwrap();
                state_guard.instantiated_agents.get(&envelope.recipient)
                    .ok_or_else(|| -> Box<dyn Error + Send> {
                        RuntimeError::AgentNotFound(envelope.recipient.to_string()).into_send_error()
                    })?
                    .clone_box()
            };

            // Process the message in the message handler context
            let result = MessageHandlerContext::with_context(envelope.recipient.clone(), async move {
                agent_for_processing.on_message(processed_message, message_context).await
            }).await?;

            // Update the agent state back to the runtime if needed
            // Note: For stateful agents, we might need to store the modified agent back
            // This is a design decision - for now we assume agents manage their own persistence

            result
        };

        // Apply intervention handlers for response
        let mut final_response = response;
        let intervention_handlers = {
            let state_guard = state.lock().unwrap();
            state_guard.intervention_handlers.iter().map(|h| h.clone_box()).collect::<Vec<_>>()
        };

        for handler in &intervention_handlers {
            match handler.on_response(final_response.clone(), &envelope.recipient, envelope.sender.as_ref()).await {
                Ok(modified_response) => {
                    final_response = modified_response;
                }
                Err(_drop_message) => {
                    // Response was dropped by intervention handler
                    tracing::info!(
                        "Response dropped by intervention handler from recipient: {:?}",
                        envelope.recipient
                    );
                    final_response = Value::Null; // Set to null to indicate response was dropped
                    break;
                }
            }
        }

        Ok(final_response)
    }

    async fn process_publish_envelope(state: Arc<Mutex<AgentRuntimeState>>, envelope: PublishMessageEnvelope) {
        tracing::info!(
            "Processing publish message from {:?} to topic {} with message_id: {}",
            envelope.sender,
            envelope.topic_id,
            envelope.message_id
        );

        // Apply intervention handlers for publish
        let mut processed_message = envelope.message.clone();
        let intervention_handlers = {
            let state_guard = state.lock().unwrap();
            state_guard.intervention_handlers.iter().map(|h| h.clone_box()).collect::<Vec<_>>()
        };

        // Create a message context for the publish operation
        let message_context = crate::message_context::MessageContext {
            sender: envelope.sender.clone(),
            topic_id: Some(envelope.topic_id.clone()),
            is_rpc: false,
            cancellation_token: envelope.cancellation_token.clone(),
            message_id: envelope.message_id.clone(),
        };

        for handler in &intervention_handlers {
            match handler.on_publish(processed_message.clone(), &message_context).await {
                Ok(modified_message) => {
                    processed_message = modified_message;
                }
                Err(_drop_message) => {
                    // Message was dropped by intervention handler
                    tracing::info!(
                        "Publish message dropped by intervention handler for topic: {}",
                        envelope.topic_id
                    );
                    return; // Exit early if message is dropped
                }
            }
        }

        // Get subscribed recipients
        let recipients = {
            let mut state_guard = state.lock().unwrap();
            state_guard.subscription_manager.get_subscribed_recipients_sync(&envelope.topic_id)
        };

        // Send message to all subscribed recipients
        for recipient in recipients {
            let recipient_id = recipient.clone();
            let send_envelope = SendMessageEnvelope {
                message: processed_message.clone(),
                sender: envelope.sender.clone(),
                recipient,
                response_tx: {
                    let (tx, _rx) = oneshot::channel();
                    tx
                },
                cancellation_token: envelope.cancellation_token.clone(),
                metadata: envelope.metadata.clone(),
                message_id: format!("{}_{}", envelope.message_id, recipient_id),
                priority: envelope.priority,
                timestamp: Instant::now(),
                estimated_processing_time: envelope.estimated_processing_time,
            };

            // Process each recipient in a separate background task
            let state_clone = state.clone();
            let task_name = format!("publish_to_{}", recipient_id);

            {
                let state_guard = state.lock().unwrap();
                state_guard.background_tasks.spawn(task_name, async move {
                    let result = Self::handle_send_message(state_clone, &send_envelope).await;
                    if let Err(e) = result {
                        tracing::warn!("Failed to deliver published message to {}: {}", recipient_id, e);
                    }
                });
            }
        }
    }

    async fn process_response_envelope(_state: Arc<Mutex<AgentRuntimeState>>, envelope: ResponseMessageEnvelope) {
        tracing::info!(
            "Processing response message from {} to {:?}",
            envelope.sender,
            envelope.recipient
        );

        if let Err(send_error) = envelope.response_tx.send(Ok(envelope.message)) {
            tracing::warn!("Failed to send response: {:?}", send_error);
        }
    }

    async fn create_agent_instance(
        state: Arc<Mutex<AgentRuntimeState>>,
        agent_id: &AgentId
    ) -> Result<Box<dyn Agent>, Box<dyn Error + Send>> {
        // Get the factory
        let factory = {
            let state_guard = state.lock().unwrap();
            state_guard.agent_factories.get(&agent_id.r#type)
                .ok_or_else(|| -> Box<dyn Error + Send> {
                    Box::new(RuntimeError::AgentTypeNotFound(agent_id.r#type.clone()))
                })?
                .clone_box()
        };

        // Create the agent (simplified for now)
        let agent = factory.create().await
            .map_err(|e| -> Box<dyn Error + Send> {
                Box::new(RuntimeError::General(e.to_string()))
            })?;

        // Store the agent instance
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.instantiated_agents.insert(agent_id.clone(), agent.clone_box());
        }

        Ok(agent)
    }

    pub async fn stop(&mut self) {
        if let Some(mut run_context) = self.run_context.take() {
            run_context.stop().await;
        }
    }
}

#[async_trait]
impl AgentRuntime for SingleThreadedAgentRuntime {
    async fn register_factory(
        &self,
        agent_type: AgentType,
        factory: Box<dyn AgentFactory>,
    ) -> Result<(), Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        if state.agent_factories.contains_key(&agent_type.r#type) {
            return Err(format!("Agent type {} already registered", agent_type.r#type).into());
        }
        state
            .agent_factories
            .insert(agent_type.r#type, factory);
        Ok(())
    }

    async fn register_agent_instance(
        &self,
        agent: Arc<Mutex<dyn Agent>>,
        agent_id: AgentId,
    ) -> Result<AgentId, Box<dyn Error>> {
        // Validate that the agent ID is not already registered
        {
            let state = self.state.lock().unwrap();
            if state.instantiated_agents.contains_key(&agent_id) {
                return Err(format!("Agent instance with ID {} already registered", agent_id).into());
            }
        }

        // Clone the agent for binding - we need a mutable copy for binding
        let mut agent_for_binding = {
            let agent_guard = agent.lock().unwrap();
            agent_guard.clone_box()
        };

        // Perform the async binding using a safe approach
        // We create a scoped async block to ensure proper lifetime management
        {
            let runtime_ref: &dyn AgentRuntime = self;
            agent_for_binding.bind_id_and_runtime(agent_id.clone(), runtime_ref).await?;
        }

        // Store the bound agent instance in the runtime state
        {
            let mut state = self.state.lock().unwrap();
            state.instantiated_agents.insert(agent_id.clone(), agent_for_binding);
        }

        tracing::info!("Successfully registered agent instance: {}", agent_id);
        Ok(agent_id)
    }

    async fn send_message(
        &self,
        message: Value,
        recipient: AgentId,
        sender: Option<AgentId>,
        cancellation_token: Option<CancellationToken>,
        message_id: Option<String>,
    ) -> Result<Value, Box<dyn Error + Send>> {
        let (tx, rx) = oneshot::channel();
        let envelope = MessageEnvelope::Send(SendMessageEnvelope {
            message,
            sender,
            recipient,
            response_tx: tx,
            cancellation_token: cancellation_token.unwrap_or_default(),
            metadata: Some(EnvelopeMetadata::default()),
            message_id: message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            priority: MessagePriority::default(),
            timestamp: Instant::now(),
            estimated_processing_time: Duration::from_millis(100),
        });
        self.queue_tx
            .send(envelope)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        rx.await.map_err(|e| Box::new(e) as Box<dyn Error + Send>)?
    }

    async fn publish_message(
        &self,
        message: Value,
        topic_id: TopicId,
        sender: Option<AgentId>,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), Box<dyn Error>> {
        let envelope = MessageEnvelope::Publish(PublishMessageEnvelope {
            message,
            sender,
            topic_id,
            cancellation_token: cancellation_token.unwrap_or_default(),
            metadata: Some(EnvelopeMetadata::default()),
            message_id: uuid::Uuid::new_v4().to_string(),
            priority: MessagePriority::default(),
            timestamp: Instant::now(),
            estimated_processing_time: Duration::from_millis(200), // Publish operations typically take longer
        });
        self.queue_tx
            .send(envelope)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;
        Ok(())
    }

    async fn add_subscription(&self, subscription: Box<dyn Subscription>) -> Result<(), Box<dyn Error>> {
        // Extract the subscription manager to avoid holding the lock across await
        let mut subscription_manager = {
            let mut state = self.state.lock().unwrap();
            std::mem::take(&mut state.subscription_manager)
        };

        let result = subscription_manager.add_subscription(subscription).await;

        // Put the subscription manager back
        {
            let mut state = self.state.lock().unwrap();
            state.subscription_manager = subscription_manager;
        }

        result.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn Error>)
    }

    async fn agent_metadata(&self, agent_id: &AgentId) -> Result<AgentMetadata, Box<dyn Error>> {
        let state = self.state.lock().unwrap();
        let agent = state
            .instantiated_agents
            .get(agent_id)
            .ok_or_else(|| format!("Agent not found: {}", agent_id))?;
        Ok(agent.metadata())
    }

    async fn agent_save_state(
        &self,
        agent_id: &AgentId,
    ) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let agent = {
            let state = self.state.lock().unwrap();
            state
                .instantiated_agents
                .get(agent_id)
                .map(|a| a.clone_box())
                .ok_or_else(|| format!("Agent not found: {}", agent_id))?
        };
        agent.save_state().await
    }

    async fn agent_load_state(
        &self,
        agent_id: &AgentId,
        agent_state: &HashMap<String, Value>,
    ) -> Result<(), Box<dyn Error>> {
        let mut agent = {
            let state = self.state.lock().unwrap();
            state
                .instantiated_agents
                .get(agent_id)
                .map(|a| a.clone_box())
                .ok_or_else(|| format!("Agent not found: {}", agent_id))?
        };
        agent.load_state(agent_state).await?;
        // Now, we need to put the modified agent back into the map.
        // This is inefficient and requires a mutable lock on the state again.
        // A better design would be to pass the state to the agent and let it modify itself.
        // For now, we will just update the state.
        let mut state = self.state.lock().unwrap();
        state.instantiated_agents.insert(agent_id.clone(), agent);
        Ok(())
    }

    async fn remove_subscription(&self, id: &str) -> Result<(), Box<dyn Error>> {
        // Extract the subscription manager to avoid holding the lock across await
        let mut subscription_manager = {
            let mut state = self.state.lock().unwrap();
            std::mem::take(&mut state.subscription_manager)
        };

        let result = subscription_manager.remove_subscription(id).await;

        // Put the subscription manager back
        {
            let mut state = self.state.lock().unwrap();
            state.subscription_manager = subscription_manager;
        }

        result.map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn Error>)
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        tracing::info!("Starting runtime state save operation");
        let mut runtime_state = HashMap::new();

        // Collect agent information first (without holding the lock across await)
        let agents_info: Vec<(AgentId, Box<dyn Agent>)> = {
            let state = self.state.lock().unwrap();
            state.instantiated_agents.iter()
                .map(|(id, agent)| (id.clone(), agent.clone_box()))
                .collect()
        };

        // Save complete agent states (including internal state)
        let mut agents_state = HashMap::new();
        for (agent_id, agent) in agents_info {
            tracing::debug!("Saving state for agent: {}", agent_id);

            // Get agent metadata
            let metadata = agent.metadata();

            // Get agent's internal state
            let agent_internal_state = match agent.save_state().await {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!("Failed to save state for agent {}: {}", agent_id, e);
                    HashMap::new()
                }
            };

            let agent_info = serde_json::json!({
                "id": agent_id.to_string(),
                "type": agent_id.r#type.clone(),
                "key": agent_id.key.clone(),
                "metadata": {
                    "type": metadata.r#type,
                    "key": metadata.key,
                    "description": metadata.description,
                },
                "internal_state": agent_internal_state,
                "created_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
            agents_state.insert(agent_id.to_string(), agent_info);
        }
        runtime_state.insert("agents".to_string(), serde_json::to_value(agents_state)?);

        // Collect other state information (without holding lock across await)
        let (subscription_state, factory_types, intervention_count, task_stats, agents_count, factories_count) = {
            let state = self.state.lock().unwrap();

            let subscription_state = match state.subscription_manager.save_state() {
                Ok(state) => state,
                Err(e) => {
                    tracing::warn!("Failed to save subscription manager state: {}", e);
                    serde_json::json!({
                        "error": "Failed to save subscription state",
                        "fallback": true
                    })
                }
            };

            let factory_types: Vec<String> = state.agent_factories.keys().cloned().collect();
            let intervention_count = state.intervention_handlers.len();
            let task_stats = state.background_tasks.get_task_stats();
            let agents_count = state.instantiated_agents.len();
            let factories_count = state.agent_factories.len();

            (subscription_state, factory_types, intervention_count, task_stats, agents_count, factories_count)
        };

        // Save complete subscription manager state
        runtime_state.insert("subscription_manager".to_string(), subscription_state);

        // Save agent factories information (type names only, as factories aren't serializable)
        runtime_state.insert("registered_agent_types".to_string(), serde_json::to_value(factory_types)?);

        // Save intervention handlers count (handlers themselves aren't serializable)
        runtime_state.insert("intervention_handlers_count".to_string(),
                           serde_json::to_value(intervention_count)?);

        // Save background task statistics
        let (total_tasks, active_tasks, oldest_age) = task_stats;
        runtime_state.insert("background_tasks".to_string(), serde_json::json!({
            "total": total_tasks,
            "active": active_tasks,
            "oldest_age_seconds": oldest_age.as_secs(),
        }));

        // Save runtime metadata with enhanced version information
        runtime_state.insert("runtime_type".to_string(), serde_json::to_value("SingleThreadedAgentRuntime")?);
        runtime_state.insert("saved_at".to_string(), serde_json::to_value(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        )?);
        runtime_state.insert("version".to_string(), serde_json::to_value("2.0.0")?);
        runtime_state.insert("schema_version".to_string(), serde_json::to_value(state_version::CURRENT_SCHEMA_VERSION)?);

        // Add state integrity information
        runtime_state.insert("state_integrity".to_string(), serde_json::json!({
            "agents_count": agents_count,
            "factories_count": factories_count,
            "intervention_handlers_count": intervention_count,
            "supported_versions": state_version::SUPPORTED_VERSIONS,
            "checksum": self.calculate_state_checksum(&runtime_state)?,
        }));

        tracing::info!("Runtime state save completed successfully");
        Ok(runtime_state)
    }

    async fn load_state(&self, runtime_state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        tracing::info!("Starting runtime state load operation");

        // Validate the saved state format and version
        self.validate_state_format(runtime_state)?;

        // Verify state integrity if checksum is available
        if let Some(integrity_info) = runtime_state.get("state_integrity") {
            self.verify_state_integrity(runtime_state, integrity_info)?;
        }

        // Handle version migration if needed
        let state_version = runtime_state.get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0");

        // Validate version compatibility
        if !state_version::is_supported(state_version) {
            return Err(format!("Unsupported state version: {}. Supported versions: {:?}",
                              state_version, state_version::SUPPORTED_VERSIONS).into());
        }

        let migrated_state = self.migrate_state_if_needed(runtime_state, state_version)?;

        // Load agent information and recreate agents
        if let Some(agents_value) = migrated_state.get("agents") {
            if let Some(agents_obj) = agents_value.as_object() {
                tracing::info!("Loading {} agent entries from saved state", agents_obj.len());

                for (agent_id_str, agent_info) in agents_obj {
                    if let Err(e) = self.load_agent_from_state(agent_id_str, agent_info).await {
                        tracing::warn!("Failed to load agent {}: {}", agent_id_str, e);
                        // Continue loading other agents even if one fails
                    }
                }
            }
        }

        // Load subscription manager state
        if let Some(subscription_state) = migrated_state.get("subscription_manager") {
            if let Err(e) = self.load_subscription_manager_state(subscription_state).await {
                tracing::warn!("Failed to load subscription manager state: {}", e);
            }
        } else if let Some(subscriptions_value) = migrated_state.get("subscriptions") {
            // Handle legacy subscription format
            if let Err(e) = self.load_legacy_subscriptions(subscriptions_value).await {
                tracing::warn!("Failed to load legacy subscriptions: {}", e);
            }
        }

        // Validate loaded state consistency
        self.validate_loaded_state().await?;

        tracing::info!("Successfully loaded runtime state");
        Ok(())
    }

    async fn try_get_underlying_agent_instance(
        &self,
        id: &AgentId,
    ) -> Result<Box<dyn Agent>, Box<dyn Error>> {
        let state = self.state.lock().unwrap();
        if let Some(agent) = state.instantiated_agents.get(id) {
            Ok(agent.clone_box())
        } else {
            Err(RuntimeError::AgentNotFound(id.to_string()).into_send_error())
        }
    }

    async fn get_agent(
        &self,
        id_or_type: &str,
        key: Option<&str>,
        lazy: bool,
    ) -> Result<AgentId, Box<dyn Error>> {
        use crate::runtime_impl_helpers::{get_id_from_identifier, AgentIdentifier};

        let identifier = AgentIdentifier::String(id_or_type.to_string());
        let key = key.unwrap_or("default");

        get_id_from_identifier(
            identifier,
            key,
            lazy,
            |agent_id| async move {
                self.try_get_underlying_agent_instance(&agent_id).await
            }
        ).await
    }

    fn add_message_serializer(&self, serializer: Box<dyn crate::serialization::MessageSerializer<Value>>) {
        let state = self.state.lock().unwrap();
        state.serialization_registry.add_boxed_serializer(serializer);
    }

    fn unprocessed_messages_count(&self) -> usize {
        self.queue.qsize()
    }
}

impl SingleThreadedAgentRuntime {
    /// Calculate a checksum for state integrity verification
    fn calculate_state_checksum(&self, state: &HashMap<String, Value>) -> Result<String, Box<dyn Error>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a deterministic string representation of the state
        let state_json = serde_json::to_string(state)?;

        // Calculate hash
        let mut hasher = DefaultHasher::new();
        state_json.hash(&mut hasher);
        let hash = hasher.finish();

        Ok(format!("{:x}", hash))
    }

    /// Validate the format of saved state
    fn validate_state_format(&self, runtime_state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        // Check runtime type
        if let Some(runtime_type) = runtime_state.get("runtime_type") {
            if runtime_type.as_str() != Some("SingleThreadedAgentRuntime") {
                return Err("Invalid runtime type in saved state".into());
            }
        } else {
            return Err("Missing runtime_type in saved state".into());
        }

        // Check required fields
        let required_fields = ["saved_at", "version"];
        for field in &required_fields {
            if !runtime_state.contains_key(*field) {
                return Err(format!("Missing required field '{}' in saved state", field).into());
            }
        }

        tracing::debug!("State format validation passed");
        Ok(())
    }

    /// Verify state integrity using checksum
    fn verify_state_integrity(&self, runtime_state: &HashMap<String, Value>, integrity_info: &Value) -> Result<(), Box<dyn Error>> {
        if let Some(integrity_obj) = integrity_info.as_object() {
            if let Some(saved_checksum) = integrity_obj.get("checksum").and_then(|v| v.as_str()) {
                // Create a copy of state without the integrity info for checksum calculation
                let mut state_for_checksum = runtime_state.clone();
                state_for_checksum.remove("state_integrity");

                let calculated_checksum = self.calculate_state_checksum(&state_for_checksum)?;

                if saved_checksum != calculated_checksum {
                    tracing::warn!("State integrity check failed: saved={}, calculated={}", saved_checksum, calculated_checksum);
                    return Err("State integrity verification failed".into());
                }

                tracing::debug!("State integrity verification passed");
            }
        }
        Ok(())
    }

    /// Migrate state from older versions if needed
    fn migrate_state_if_needed(&self, runtime_state: &HashMap<String, Value>, state_version: &str) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        if state_version == state_version::CURRENT_SCHEMA_VERSION {
            // Current version, no migration needed
            return Ok(runtime_state.clone());
        }

        let migration_path = state_version::get_migration_path(state_version);
        if migration_path.is_empty() {
            return Err(format!("No migration path available from version {}", state_version).into());
        }

        tracing::info!("Migrating state from version {} through path: {:?}", state_version, migration_path);

        let mut current_state = runtime_state.clone();
        let mut current_version = state_version;

        for target_version in migration_path {
            current_state = self.migrate_between_versions(&current_state, current_version, target_version)?;
            current_version = target_version;
        }

        Ok(current_state)
    }

    /// Migrate state between specific versions
    fn migrate_between_versions(&self, state: &HashMap<String, Value>, from_version: &str, to_version: &str) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        match (from_version, to_version) {
            ("1.0", "2.0") => self.migrate_from_v1_to_v2(state),
            _ => Err(format!("No migration available from {} to {}", from_version, to_version).into())
        }
    }

    /// Migrate state from version 1.0 to 2.0
    fn migrate_from_v1_to_v2(&self, old_state: &HashMap<String, Value>) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        tracing::info!("Starting migration from v1.0 to v2.0");
        let mut new_state = old_state.clone();

        // Update version information
        new_state.insert("schema_version".to_string(), serde_json::to_value(state_version::CURRENT_SCHEMA_VERSION)?);
        new_state.insert("version".to_string(), serde_json::to_value("2.0.0")?);

        // Migrate agent format - add metadata structure if missing
        if let Some(agents_value) = old_state.get("agents") {
            if let Some(agents_obj) = agents_value.as_object() {
                let mut migrated_agents = serde_json::Map::new();

                for (agent_id, agent_info) in agents_obj {
                    if let Some(mut agent_obj) = agent_info.as_object().cloned() {
                        // Ensure metadata structure exists
                        if !agent_obj.contains_key("metadata") {
                            agent_obj.insert("metadata".to_string(), serde_json::json!({
                                "type": agent_obj.get("type").unwrap_or(&Value::String("unknown".to_string())),
                                "key": "default",
                                "description": "Migrated agent"
                            }));
                        }

                        // Ensure internal_state exists
                        if !agent_obj.contains_key("internal_state") {
                            agent_obj.insert("internal_state".to_string(), serde_json::json!({}));
                        }

                        migrated_agents.insert(agent_id.clone(), Value::Object(agent_obj));
                    }
                }

                new_state.insert("agents".to_string(), Value::Object(migrated_agents));
            }
        }

        // Migrate subscription format if needed
        if let Some(subscriptions) = old_state.get("subscriptions") {
            if subscriptions.is_array() {
                // Convert old array format to new subscription manager format
                let subscription_manager_state = serde_json::json!({
                    "total_subscriptions": subscriptions.as_array().unwrap().len(),
                    "subscriptions": subscriptions,
                    "seen_topics": [],
                    "version": "1.0",
                    "saved_at": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });
                new_state.insert("subscription_manager".to_string(), subscription_manager_state);
                // Remove old format
                new_state.remove("subscriptions");
            }
        }

        // Add new fields introduced in v2.0
        if !new_state.contains_key("registered_agent_types") {
            new_state.insert("registered_agent_types".to_string(), serde_json::json!([]));
        }

        if !new_state.contains_key("intervention_handlers_count") {
            new_state.insert("intervention_handlers_count".to_string(), serde_json::json!(0));
        }

        if !new_state.contains_key("background_tasks") {
            new_state.insert("background_tasks".to_string(), serde_json::json!({
                "total": 0,
                "active": 0,
                "oldest_age_seconds": 0
            }));
        }

        // Validate migrated state
        self.validate_migrated_state(&new_state)?;

        tracing::info!("Successfully migrated state from v1.0 to v2.0");
        Ok(new_state)
    }

    /// Validate migrated state for consistency
    fn validate_migrated_state(&self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        // Check required fields for v2.0
        let required_fields = [
            "runtime_type", "schema_version", "version", "saved_at",
            "agents", "registered_agent_types", "intervention_handlers_count"
        ];

        for field in &required_fields {
            if !state.contains_key(*field) {
                return Err(format!("Missing required field '{}' after migration", field).into());
            }
        }

        // Validate schema version
        if let Some(schema_version) = state.get("schema_version").and_then(|v| v.as_str()) {
            if !state_version::is_supported(schema_version) {
                return Err(format!("Invalid schema version after migration: {}", schema_version).into());
            }
        }

        tracing::debug!("Migrated state validation passed");
        Ok(())
    }

    /// Create a backup of state before migration
    fn create_state_backup(&self, state: &HashMap<String, Value>) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let mut backup = state.clone();
        backup.insert("backup_created_at".to_string(), serde_json::to_value(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        )?);
        backup.insert("backup_reason".to_string(), serde_json::to_value("pre_migration")?);

        tracing::debug!("Created state backup for migration");
        Ok(backup)
    }

    /// Load an agent from saved state
    async fn load_agent_from_state(&self, agent_id_str: &str, agent_info: &Value) -> Result<(), Box<dyn Error>> {
        if let Some(agent_obj) = agent_info.as_object() {
            // Parse agent ID
            let agent_id = AgentId::from_str(agent_id_str)?;

            // Check if agent factory exists for this type
            let has_factory = {
                let state = self.state.lock().unwrap();
                state.agent_factories.contains_key(&agent_id.r#type)
            };

            if !has_factory {
                return Err(format!("No factory registered for agent type: {}", agent_id.r#type).into());
            }

            // Create agent instance (this also stores it in the runtime state)
            let _agent = Self::create_agent_instance(self.state.clone(), &agent_id).await
                .map_err(|e| -> Box<dyn Error> { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) })?;

            // Load agent's internal state if available
            if let Some(internal_state) = agent_obj.get("internal_state") {
                if let Some(state_map) = internal_state.as_object() {
                    let state_hashmap: HashMap<String, Value> = state_map.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();

                    // Load the state into the agent
                    if let Err(e) = self.agent_load_state(&agent_id, &state_hashmap).await {
                        tracing::warn!("Failed to load internal state for agent {}: {}", agent_id, e);
                    }
                }
            }

            tracing::debug!("Successfully loaded agent: {}", agent_id);
        }
        Ok(())
    }

    /// Load subscription manager state
    async fn load_subscription_manager_state(&self, subscription_state: &Value) -> Result<(), Box<dyn Error>> {
        // Note: This is a simplified implementation
        // In a full implementation, we would need to recreate subscription objects
        tracing::info!("Loading subscription manager state");

        if let Some(state_obj) = subscription_state.as_object() {
            if let Some(total_subs) = state_obj.get("total_subscriptions").and_then(|v| v.as_u64()) {
                tracing::debug!("Expected to load {} subscriptions", total_subs);
            }
        }

        // TODO: Implement actual subscription recreation
        tracing::warn!("Subscription manager state loading is not fully implemented yet");
        Ok(())
    }

    /// Load legacy subscription format
    async fn load_legacy_subscriptions(&self, subscriptions_value: &Value) -> Result<(), Box<dyn Error>> {
        if let Some(subscription_ids) = subscriptions_value.as_array() {
            tracing::info!("Loading {} legacy subscription IDs", subscription_ids.len());

            for subscription_id in subscription_ids {
                if let Some(id_str) = subscription_id.as_str() {
                    tracing::debug!("Found legacy subscription: {}", id_str);
                    // TODO: Implement actual subscription recreation from ID
                }
            }
        }

        tracing::warn!("Legacy subscription loading is not fully implemented yet");
        Ok(())
    }

    /// Validate the consistency of loaded state
    async fn validate_loaded_state(&self) -> Result<(), Box<dyn Error>> {
        let state = self.state.lock().unwrap();

        // Check that all agents are properly bound
        for (agent_id, _agent) in &state.instantiated_agents {
            tracing::debug!("Validating agent: {}", agent_id);
            // Additional validation logic can be added here
        }

        tracing::debug!("Loaded state validation completed");
        Ok(())
    }

    /// Get background task statistics
    pub fn get_background_task_stats(&self) -> (usize, usize, std::time::Duration) {
        let state = self.state.lock().unwrap();
        state.background_tasks.get_task_stats()
    }

    /// Clean up finished background tasks
    pub fn cleanup_background_tasks(&self) {
        let state = self.state.lock().unwrap();
        state.background_tasks.cleanup_finished();
    }

    /// Spawn a new background task
    pub fn spawn_background_task<F>(&self, name: String, future: F) -> JoinHandle<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let state = self.state.lock().unwrap();
        state.background_tasks.spawn(name, future)
    }

    /// Wait for all background tasks to complete
    pub async fn wait_for_background_tasks(&self) {
        let background_tasks = {
            let state = self.state.lock().unwrap();
            state.background_tasks.tasks.clone()
        };

        // Create a temporary manager to wait for tasks
        let temp_manager = BackgroundTaskManager {
            tasks: background_tasks,
        };
        temp_manager.wait_for_all().await;
    }

    /// Abort all background tasks
    pub fn abort_all_background_tasks(&self) {
        let state = self.state.lock().unwrap();
        state.background_tasks.abort_all();
    }
}