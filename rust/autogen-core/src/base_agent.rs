use crate::agent::{Agent, AgentFactory};
use crate::agent_id::AgentId;
use crate::agent_instantiation::AgentInstantiationContext;
use crate::agent_metadata::AgentMetadata;
use crate::agent_runtime::AgentRuntime;
use crate::agent_type::AgentType;
use crate::cancellation_token::CancellationToken;
use crate::exceptions::GeneralError;
use crate::message_context::MessageContext;
use crate::topic::TopicId;
use crate::subscription::Subscription;
use crate::type_subscription::TypeSubscription;
use crate::type_prefix_subscription::TypePrefixSubscription;
use crate::subscription_manager::{add_global_type_subscription, add_global_prefix_subscription};
use crate::serialization::MessageSerializer;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tracing::warn;
use std::any::{TypeId, Any};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use tracing::info;
use std::future::Future;
use std::pin::Pin;

/// Subscription information for an agent
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub subscription_id: String,
    pub topic_type: String,
    pub agent_type: String,
}

/// Global registry for unbound subscriptions (equivalent to Python's internal_unbound_subscriptions_list)
static UNBOUND_SUBSCRIPTIONS: Lazy<DashMap<String, Vec<SubscriptionInfo>>> =
    Lazy::new(DashMap::new);

/// Global registry for extra handle types (equivalent to Python's internal_extra_handles_types)
static EXTRA_HANDLES_TYPES: Lazy<DashMap<String, Vec<TypeId>>> =
    Lazy::new(DashMap::new);

/// Handler function type for message handlers
pub type MessageHandlerFn = Box<
    dyn Fn(&mut dyn Any, Value, MessageContext) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn Error + Send>>> + Send>>
        + Send
        + Sync,
>;

/// Handler function type for event handlers
pub type EventHandlerFn = Box<
    dyn Fn(&mut dyn Any, Value, MessageContext) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send>>> + Send>>
        + Send
        + Sync,
>;

/// Handler function type for RPC handlers
pub type RpcHandlerFn = Box<
    dyn Fn(&mut dyn Any, Value, MessageContext) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn Error + Send>>> + Send>>
        + Send
        + Sync,
>;

/// Global registry for message handlers
static MESSAGE_HANDLERS: Lazy<DashMap<String, MessageHandlerFn>> = Lazy::new(DashMap::new);

/// Global registry for event handlers
static EVENT_HANDLERS: Lazy<DashMap<String, EventHandlerFn>> = Lazy::new(DashMap::new);

/// Global registry for RPC handlers
static RPC_HANDLERS: Lazy<DashMap<String, RpcHandlerFn>> = Lazy::new(DashMap::new);

/// Trait for types that can handle specific message types
pub trait HandlesType<T> {
    fn message_type_id() -> TypeId;
    fn message_type_name() -> &'static str;
}

/// Trait for agents that have subscriptions
pub trait HasSubscription {
    fn get_subscriptions() -> Vec<Box<dyn Subscription>>;
}

/// Register a handled type for an agent
pub fn register_handled_type<A: 'static, T: 'static>() {
    let agent_type = std::any::type_name::<A>().to_string();
    let message_type_id = TypeId::of::<T>();

    EXTRA_HANDLES_TYPES
        .entry(agent_type)
        .or_insert_with(Vec::new)
        .push(message_type_id);
}

/// Register a handled type with custom serializer
pub fn register_handled_type_with_serializer<A: 'static, T: 'static>(
    _serializer: Box<dyn MessageSerializer<T>>,
) {
    // For now, just register the type - serializer integration would be more complex
    register_handled_type::<A, T>();
}

/// Register a subscription for an agent
pub fn register_subscription<A: 'static>(topic: &str, message_type_id: TypeId) {
    let agent_type = std::any::type_name::<A>().to_string();
    let sub_info = SubscriptionInfo {
        subscription_id: format!("{}:{}", agent_type, topic),
        topic_type: topic.to_string(),
        agent_type: agent_type.clone(),
    };

    UNBOUND_SUBSCRIPTIONS
        .entry(agent_type)
        .or_insert_with(Vec::new)
        .push(sub_info);
}

/// Register a message handler
pub fn register_message_handler<A: 'static, T: 'static>(
    handler_name: &str,
    handler: MessageHandlerFn,
) {
    let key = format!("{}::{}", std::any::type_name::<A>(), handler_name);
    MESSAGE_HANDLERS.insert(key, handler);
}

/// Register an event handler
pub fn register_event_handler<A: 'static, T: 'static>(
    handler_name: &str,
    handler: EventHandlerFn,
) {
    let key = format!("{}::{}", std::any::type_name::<A>(), handler_name);
    EVENT_HANDLERS.insert(key, handler);
}

/// Register an RPC handler
pub fn register_rpc_handler<A: 'static, Req: 'static, Res: 'static>(
    handler_name: &str,
    handler: RpcHandlerFn,
) {
    let key = format!("{}::{}", std::any::type_name::<A>(), handler_name);
    RPC_HANDLERS.insert(key, handler);
}

/// Base implementation for the `Agent` trait.
#[derive(Clone)]
pub struct BaseAgent {
    id: Option<AgentId>,
    runtime: Option<Arc<dyn AgentRuntime>>,
    description: String,
    /// Local subscriptions for this agent instance
    subscriptions: Arc<DashMap<String, Box<dyn Subscription>>>,
    /// Whether this agent has been bound to a runtime
    is_bound: bool,
}

impl BaseAgent {
    /// Creates a new `BaseAgent`.
    pub fn new(description: String) -> Self {
        let mut agent = Self {
            id: None,
            runtime: None,
            description,
            subscriptions: Arc::new(DashMap::new()),
            is_bound: false,
        };

        if AgentInstantiationContext::is_in_factory_call() {
            agent.id = Some(AgentInstantiationContext::current_agent_id());
            agent.runtime = Some(AgentInstantiationContext::current_runtime());
            agent.is_bound = true;
        }

        agent
    }

    /// Bind the agent to a runtime and agent ID (equivalent to Python's bind_id_and_runtime)
    pub fn bind_id_and_runtime(&mut self, agent_id: AgentId, runtime: Arc<dyn AgentRuntime>) {
        self.id = Some(agent_id);
        self.runtime = Some(runtime);
        self.is_bound = true;

        // Process any unbound subscriptions for this agent type
        if let Some(agent_id) = &self.id {
            self.process_unbound_subscriptions(&agent_id.r#type);
        }
    }

    /// Process unbound subscriptions for this agent type
    fn process_unbound_subscriptions(&self, agent_type: &str) {
        if let Some((_, subscriptions)) = UNBOUND_SUBSCRIPTIONS.remove(agent_type) {
            for sub_info in subscriptions {
                info!("Processing unbound subscription: {:?}", sub_info);

                // Create the appropriate subscription based on the topic type
                let subscription: Box<dyn Subscription> = if sub_info.topic_type.ends_with(':') {
                    // This is a prefix subscription
                    Box::new(TypePrefixSubscription::new(
                        sub_info.topic_type.trim_end_matches(':').to_string(),
                        AgentType { r#type: sub_info.agent_type.clone() }
                    ))
                } else {
                    // This is a regular type subscription
                    Box::new(TypeSubscription::new(
                        sub_info.topic_type.clone(),
                        AgentType { r#type: sub_info.agent_type.clone() }
                    ))
                };

                // Register with the runtime if available
                if let Some(runtime) = &self.runtime {
                    // In a real implementation, we would need an async context here
                    // For now, we'll store it locally and register later
                    let subscription_id = subscription.id().to_string();
                    self.subscriptions.insert(subscription_id, subscription);
                    info!("Registered subscription for agent type: {}", agent_type);
                }
            }
        }
    }

    /// Add a subscription to this agent
    pub fn add_subscription(&self, subscription: Box<dyn Subscription>) -> Result<(), Box<dyn Error>> {
        let subscription_id = subscription.id().to_string();

        // Extract topic type from subscription for proper categorization
        let topic_type = self.extract_topic_type_from_subscription(&subscription);

        self.subscriptions.insert(subscription_id.clone(), subscription);

        if self.is_bound {
            // If bound, register with the runtime immediately
            info!("Registering subscription {} for bound agent", subscription_id);
            // TODO: In a full async implementation, we would call:
            // runtime.add_subscription(subscription).await?;
        } else {
            // If not bound, add to unbound subscriptions
            if let Some(agent_id) = &self.id {
                let sub_info = SubscriptionInfo {
                    subscription_id,
                    topic_type,
                    agent_type: agent_id.r#type.clone(),
                };

                UNBOUND_SUBSCRIPTIONS
                    .entry(agent_id.r#type.clone())
                    .or_insert_with(Vec::new)
                    .push(sub_info);
            }
        }

        Ok(())
    }

    /// Extract topic type from subscription for categorization
    fn extract_topic_type_from_subscription(&self, subscription: &Box<dyn Subscription>) -> String {
        // This is a simplified implementation
        // In a real implementation, we would need to downcast or use a trait method
        // to extract the topic type from different subscription types
        format!("extracted_topic_{}", subscription.id())
    }

    /// Save state to a JSON value (equivalent to Python's save_state)
    pub fn save_state(&self) -> Result<Value, Box<dyn Error>> {
        warn!("BaseAgent save_state called - this should be overridden by subclasses for proper state management.");

        // Return basic agent information as default state
        let mut state = serde_json::Map::new();
        if let Some(id) = &self.id {
            state.insert("agent_id".to_string(), serde_json::to_value(id)?);
        }
        state.insert("description".to_string(), Value::String(self.description.clone()));
        state.insert("is_bound".to_string(), Value::Bool(self.is_bound));

        Ok(Value::Object(state))
    }

    /// Load state from a JSON value (equivalent to Python's load_state)
    pub fn load_state(&mut self, state: Value) -> Result<(), Box<dyn Error>> {
        warn!("BaseAgent load_state called - this should be overridden by subclasses for proper state management.");

        // Load basic agent information from state
        if let Value::Object(state_map) = state {
            if let Some(description_value) = state_map.get("description") {
                if let Some(description) = description_value.as_str() {
                    self.description = description.to_string();
                }
            }

            if let Some(is_bound_value) = state_map.get("is_bound") {
                if let Some(is_bound) = is_bound_value.as_bool() {
                    self.is_bound = is_bound;
                }
            }
        }

        Ok(())
    }

    /// Get the agent's description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Check if the agent is bound to a runtime
    pub fn is_bound(&self) -> bool {
        self.is_bound
    }

    /// Get the agent's ID if bound
    pub fn agent_id(&self) -> Option<&AgentId> {
        self.id.as_ref()
    }

    /// Get the runtime if bound
    pub fn runtime(&self) -> Option<Arc<dyn AgentRuntime>> {
        self.runtime.clone()
    }

    /// Sends a message to another agent.
    pub async fn send_message(
        &self,
        message: Value,
        recipient: AgentId,
        cancellation_token: Option<CancellationToken>,
        message_id: Option<String>,
    ) -> Result<Value, Box<dyn Error + Send>> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| GeneralError("Runtime not bound".to_string()))
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        let sender = self
            .id
            .as_ref()
            .ok_or_else(|| GeneralError("ID not bound".to_string()))
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        runtime
            .send_message(
                message,
                recipient,
                Some(sender.clone()),
                cancellation_token,
                message_id,
            )
            .await
    }

    /// Publishes a message to a topic.
    pub async fn publish_message(
        &self,
        message: Value,
        topic_id: TopicId,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(), Box<dyn Error>> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| GeneralError("Runtime not bound".to_string()))
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;
        let sender = self
            .id
            .as_ref()
            .ok_or_else(|| GeneralError("ID not bound".to_string()))
            .map_err(|e| Box::new(e) as Box<dyn Error>)?;
        runtime
            .publish_message(message, topic_id, Some(sender.clone()), cancellation_token)
            .await
    }

    /// Registers an instance of an agent with the runtime.
    pub async fn register_instance(
        agent: Arc<Mutex<dyn Agent>>,
        runtime: Arc<dyn AgentRuntime>,
        agent_id: AgentId,
    ) -> Result<AgentId, Box<dyn Error>> {
        let agent_id = runtime
            .register_agent_instance(agent, agent_id)
            .await?;
        // In a full implementation, you would also handle subscriptions here.
        Ok(agent_id)
    }

    /// Registers a factory for creating agents of this type.
    pub async fn register<F>(
        runtime: Arc<dyn AgentRuntime>,
        agent_type: String,
        factory: F,
    ) -> Result<AgentType, Box<dyn Error>>
    where
        F: AgentFactory + 'static,
    {
        let agent_type = AgentType { r#type: agent_type };
        runtime
            .register_factory(agent_type.clone(), Box::new(factory))
            .await?;
        // In a full implementation, you would also handle subscriptions here.
        Ok(agent_type)
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn metadata(&self) -> AgentMetadata {
        let id = self.id.as_ref().expect("ID not bound");
        AgentMetadata {
            r#type: id.r#type.clone(),
            key: id.key.clone(),
            description: self.description.clone(),
        }
    }

    fn id(&self) -> AgentId {
        self.id.as_ref().expect("ID not bound").clone()
    }

    async fn bind_id_and_runtime(
        &mut self,
        id: AgentId,
        _runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(existing_id) = &self.id {
            if existing_id != &id {
                return Err(Box::new(GeneralError(
                    "Agent is already bound to a different ID".to_string(),
                )));
            }
        }
        if self.runtime.is_some() {
            // Cannot compare runtimes directly.
        }
        self.id = Some(id);
        // self.runtime = Some(Arc::from(runtime)); // This is still problematic
        Ok(())
    }

    async fn on_message(
        &mut self,
        message: Value,
        ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        // Default implementation that provides better error handling
        // and logging for debugging purposes

        // Extract message type for logging
        let message_type = message.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Log the unhandled message for debugging
        warn!(
            "BaseAgent received unhandled message. Agent ID: {:?}, Message type: {}, Sender: {:?}, Topic: {:?}",
            self.id,
            message_type,
            ctx.sender,
            ctx.topic_id
        );

        // Return a more specific error that indicates this is expected behavior
        // for a base agent that should be subclassed
        Err(Box::new(crate::exceptions::CantHandleException(
            format!(
                "BaseAgent cannot handle message of type '{}'. This agent should be subclassed with specific message handlers.",
                message_type
            )
        )))
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        warn!("BaseAgent save_state called - this should be overridden by subclasses for proper state management");

        // Return basic agent information as default state
        let mut state = HashMap::new();
        if let Some(id) = &self.id {
            state.insert("agent_id".to_string(), serde_json::to_value(id)?);
        }
        state.insert("description".to_string(), Value::String(self.description.clone()));
        state.insert("is_bound".to_string(), Value::Bool(self.is_bound));

        Ok(state)
    }

    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        warn!("BaseAgent load_state called - this should be overridden by subclasses for proper state management");

        // Load basic agent information from state
        if let Some(description_value) = state.get("description") {
            if let Some(description) = description_value.as_str() {
                self.description = description.to_string();
            }
        }

        if let Some(is_bound_value) = state.get("is_bound") {
            if let Some(is_bound) = is_bound_value.as_bool() {
                self.is_bound = is_bound;
            }
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}