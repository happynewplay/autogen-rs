use crate::agent::{Agent, AgentFactory};
use crate::agent_id::AgentId;
use crate::agent_metadata::AgentMetadata;
use crate::agent_runtime::AgentRuntime;
use crate::agent_type::AgentType;
use crate::cancellation_token::CancellationToken;
use crate::intervention::InterventionHandler;
use crate::queue::Queue;
use crate::runtime_impl_helpers::SubscriptionManager;
use crate::serialization::SerializationRegistry;
use crate::subscription::Subscription;
use crate::topic::TopicId;
use crate::telemetry::{TraceHelper, get_telemetry_envelope_metadata};
use crate::telemetry::tracing_config::MessageRuntimeTracingConfig;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use thiserror::Error;
use opentelemetry::trace::TracerProvider;

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
}

/// Message envelope for sending responses
#[derive(Debug)]
pub struct ResponseMessageEnvelope {
    pub message: Value,
    pub response_tx: oneshot::Sender<Result<Value, Box<dyn Error + Send>>>,
    pub sender: AgentId,
    pub recipient: Option<AgentId>,
    pub metadata: Option<EnvelopeMetadata>,
}

/// Union type for all message envelopes
#[derive(Debug)]
pub enum MessageEnvelope {
    Send(SendMessageEnvelope),
    Publish(PublishMessageEnvelope),
    Response(ResponseMessageEnvelope),
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
        use crate::intervention::DropMessage;

        // Log the message delivery
        tracing::info!(
            "Processing send message from {:?} to {} with message_id: {}",
            envelope.sender,
            envelope.recipient,
            envelope.message_id
        );

        // Apply intervention handlers for send
        let mut processed_message = envelope.message.clone();

        // 未实现: 简化intervention handler处理，暂时跳过
        // let message_context = MessageContext { ... };
        // for handler in &intervention_handlers { ... }

        // Check if recipient agent exists
        let needs_creation = {
            let state_guard = state.lock().unwrap();
            if !state_guard.agent_factories.contains_key(&envelope.recipient.r#type) {
                return Err(RuntimeError::AgentTypeNotFound(envelope.recipient.r#type.clone()).into_send_error());
            }

            // Check if agent instance already exists
            state_guard.instantiated_agents.get(&envelope.recipient).is_none()
        };

        let agent = if needs_creation {
            // Need to create agent instance
            Self::create_agent_instance(state.clone(), &envelope.recipient).await?
        } else {
            // Get existing agent instance
            let state_guard = state.lock().unwrap();
            state_guard.instantiated_agents.get(&envelope.recipient)
                .unwrap()
                .clone_box()
        };

        // Create message context
        let message_context = MessageContext {
            sender: envelope.sender.clone(),
            topic_id: None,
            is_rpc: true,
            cancellation_token: envelope.cancellation_token.clone(),
            message_id: envelope.message_id.clone(),
        };

        // Set message handler context and handle the message
        let response = MessageHandlerContext::with_context(envelope.recipient.clone(), async {
            // Handle the message using the correct method
            let mut agent_mut = agent;
            agent_mut.on_message(processed_message, message_context).await
        }).await?;

        // Apply intervention handlers for response
        let final_response = response;
        // 未实现: 简化intervention handler处理，暂时跳过

        Ok(final_response)
    }

    async fn process_publish_envelope(state: Arc<Mutex<AgentRuntimeState>>, envelope: PublishMessageEnvelope) {
        use crate::intervention::DropMessage;
        use crate::message_context::MessageContext;

        tracing::info!(
            "Processing publish message from {:?} to topic {} with message_id: {}",
            envelope.sender,
            envelope.topic_id,
            envelope.message_id
        );

        // Apply intervention handlers for publish
        let processed_message = envelope.message.clone();
        // 未实现: 简化intervention handler处理，暂时跳过

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
        _agent: Arc<Mutex<dyn Agent>>,
        _agent_id: AgentId,
    ) -> Result<AgentId, Box<dyn Error>> {
        unimplemented!()
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
        });
        self.queue_tx
            .send(envelope)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;
        Ok(())
    }

    async fn add_subscription(&self, subscription: Box<dyn Subscription>) -> Result<(), Box<dyn Error>> {
        // 未实现: 简化的subscription管理，暂时返回成功
        Ok(())
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

    async fn remove_subscription(&self, _id: &str) -> Result<(), Box<dyn Error>> {
        // 未实现: 简化的subscription管理，暂时返回成功
        Ok(())
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let mut runtime_state = HashMap::new();

        // 未实现: 简化的状态保存，暂时返回空状态
        runtime_state.insert("agents".to_string(), serde_json::to_value(HashMap::<String, Value>::new())?);
        runtime_state.insert("subscriptions".to_string(), serde_json::to_value(Vec::<String>::new())?);

        Ok(runtime_state)
    }

    async fn load_state(&self, runtime_state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        if let Some(agents_state) = runtime_state.get("agents") {
            if let Value::Object(agents_map) = agents_state {
                for (agent_id_str, agent_state_value) in agents_map {
                    let agent_id = AgentId::from_str(agent_id_str)?;
                    if let Value::Object(agent_state_map) = agent_state_value {
                        let agent_state: HashMap<String, Value> = agent_state_map.clone()
                            .into_iter()
                            .collect();
                        self.agent_load_state(&agent_id, &agent_state).await?;
                    }
                }
            }
        }
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
        let mut state = self.state.lock().unwrap();
        state.serialization_registry.add_boxed_serializer(serializer);
    }

    fn unprocessed_messages_count(&self) -> usize {
        self.queue.qsize()
    }
}

impl SingleThreadedAgentRuntime {
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