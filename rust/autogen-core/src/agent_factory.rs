//! Improved agent factory system
//!
//! This module provides a type-safe and flexible agent factory system
//! that simplifies agent registration and creation.

use crate::{Agent, AgentId, Result};
use async_trait::async_trait;
use std::collections::HashMap;


/// Type-safe agent factory trait
///
/// This trait provides a clean interface for creating agents with compile-time
/// type safety, eliminating the need for Box<dyn Any> and unsafe downcasting.
#[async_trait]
pub trait AgentFactory: Send + Sync {
    /// The type of agent this factory creates
    type AgentType: Agent;

    /// Create a new agent instance
    ///
    /// # Arguments
    /// * `agent_id` - The ID to assign to the new agent
    /// * `config` - Optional configuration for the agent
    ///
    /// # Returns
    /// A new agent instance
    async fn create_agent(
        &self,
        agent_id: AgentId,
        config: Option<serde_json::Value>,
    ) -> Result<Self::AgentType>;

    /// Get the agent type name
    fn agent_type_name(&self) -> &'static str {
        std::any::type_name::<Self::AgentType>()
    }

    /// Validate agent configuration
    fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// Simple closure-based agent factory
///
/// This factory allows creating agents using simple closures,
/// providing a convenient way to register agents without implementing
/// the full AgentFactory trait.
pub struct ClosureAgentFactory<A, F>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Result<A> + Send + Sync,
{
    factory_fn: F,
    _phantom: std::marker::PhantomData<A>,
}

impl<A, F> ClosureAgentFactory<A, F>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Result<A> + Send + Sync,
{
    /// Create a new closure-based factory
    pub fn new(factory_fn: F) -> Self {
        Self {
            factory_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<A, F> AgentFactory for ClosureAgentFactory<A, F>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Result<A> + Send + Sync,
{
    type AgentType = A;

    async fn create_agent(
        &self,
        agent_id: AgentId,
        config: Option<serde_json::Value>,
    ) -> Result<Self::AgentType> {
        (self.factory_fn)(agent_id, config)
    }
}

/// Async closure-based agent factory
///
/// This factory supports async agent creation, useful for agents that
/// need to perform async initialization.
pub struct AsyncClosureAgentFactory<A, F, Fut>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<A>> + Send,
{
    factory_fn: F,
    _phantom: std::marker::PhantomData<(A, Fut)>,
}

impl<A, F, Fut> AsyncClosureAgentFactory<A, F, Fut>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<A>> + Send,
{
    /// Create a new async closure-based factory
    pub fn new(factory_fn: F) -> Self {
        Self {
            factory_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<A, F, Fut> AgentFactory for AsyncClosureAgentFactory<A, F, Fut>
where
    A: Agent,
    F: Fn(AgentId, Option<serde_json::Value>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<A>> + Send + Sync,
{
    type AgentType = A;

    async fn create_agent(
        &self,
        agent_id: AgentId,
        config: Option<serde_json::Value>,
    ) -> Result<Self::AgentType> {
        (self.factory_fn)(agent_id, config).await
    }
}

/// Agent factory registry
///
/// This registry manages multiple agent factories and provides
/// a unified interface for agent creation.
pub struct AgentFactoryRegistry {
    /// Registered factories by type name
    factories: HashMap<String, Box<dyn Fn(AgentId, Option<serde_json::Value>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Agent>>> + Send>> + Send + Sync>>,
}

impl AgentFactoryRegistry {
    /// Create a new factory registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register an agent factory
    ///
    /// # Arguments
    /// * `type_name` - The type name for the agent
    /// * `factory` - The factory function
    pub fn register_factory<F, Fut>(&mut self, type_name: String, factory: F) -> Result<()>
    where
        F: Fn(AgentId, Option<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Box<dyn Agent>>> + Send + 'static,
    {
        let boxed_factory = Box::new(move |id, config| {
            Box::pin(factory(id, config)) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Agent>>> + Send>>
        });
        self.factories.insert(type_name, boxed_factory);
        Ok(())
    }

    /// Create an agent using a registered factory
    ///
    /// # Arguments
    /// * `type_name` - The type name of the agent to create
    /// * `agent_id` - The ID to assign to the new agent
    /// * `config` - Optional configuration for the agent
    ///
    /// # Returns
    /// A new agent instance
    pub async fn create_agent(
        &self,
        type_name: &str,
        agent_id: AgentId,
        config: Option<serde_json::Value>,
    ) -> Result<Box<dyn Agent>> {
        if let Some(factory) = self.factories.get(type_name) {
            factory(agent_id, config).await
        } else {
            Err(crate::AutoGenError::other(format!(
                "No factory registered for agent type: {}",
                type_name
            )))
        }
    }

    /// Check if a factory is registered for a type
    pub fn has_factory(&self, type_name: &str) -> bool {
        self.factories.contains_key(type_name)
    }

    /// Get all registered agent types
    pub fn registered_types(&self) -> Vec<&String> {
        self.factories.keys().collect()
    }
}

impl Default for AgentFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macro for creating simple agent factories
///
/// # Example
///
/// ```rust
/// use autogen_core::{agent_factory, AgentId, Result};
///
/// agent_factory! {
///     MyAgent => |id: AgentId, _config| {
///         Ok(MyAgent::new(id))
///     }
/// }
/// ```
#[macro_export]
macro_rules! agent_factory {
    ($agent_type:ty => |$id:ident: AgentId, $config:ident| $body:expr) => {
        $crate::agent_factory::ClosureAgentFactory::new(
            move |$id: $crate::AgentId, $config: Option<serde_json::Value>| -> $crate::Result<$agent_type> {
                $body
            }
        )
    };
}

/// Convenience macro for creating async agent factories
///
/// # Example
///
/// ```rust
/// use autogen_core::{async_agent_factory, AgentId, Result};
///
/// async_agent_factory! {
///     MyAgent => |id: AgentId, _config| async {
///         let agent = MyAgent::new(id).await?;
///         Ok(agent)
///     }
/// }
/// ```
#[macro_export]
macro_rules! async_agent_factory {
    ($agent_type:ty => |$id:ident: AgentId, $config:ident| $body:expr) => {
        $crate::agent_factory::AsyncClosureAgentFactory::new(
            move |$id: $crate::AgentId, $config: Option<serde_json::Value>| async move -> $crate::Result<$agent_type> {
                $body.await
            }
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TypeSafeMessage, MessageContext};

    // Test agent for factory testing
    struct TestAgent {
        id: AgentId,
        value: String,
    }

    impl TestAgent {
        fn new(id: AgentId, value: String) -> Self {
            Self { id, value }
        }
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn id(&self) -> &AgentId {
            &self.id
        }

        async fn handle_message(
            &mut self,
            _message: TypeSafeMessage,
            _context: &MessageContext,
        ) -> Result<Option<TypeSafeMessage>> {
            Ok(Some(TypeSafeMessage::text(format!("TestAgent: {}", self.value))))
        }
    }

    #[tokio::test]
    async fn test_closure_agent_factory() {
        let factory = ClosureAgentFactory::new(|id: AgentId, _config| {
            Ok(TestAgent::new(id, "test".to_string()))
        });

        let agent_id = AgentId::new("test", "agent").unwrap();
        let agent = factory.create_agent(agent_id.clone(), None).await.unwrap();

        assert_eq!(agent.id(), &agent_id);
    }

    #[tokio::test]
    async fn test_agent_factory_registry() {
        let mut registry = AgentFactoryRegistry::new();

        registry.register_factory("TestAgent".to_string(), |id: AgentId, _config| async move {
            Ok(Box::new(TestAgent::new(id, "registry_test".to_string())) as Box<dyn Agent>)
        }).unwrap();

        assert!(registry.has_factory("TestAgent"));

        let agent_id = AgentId::new("test", "registry").unwrap();
        let agent = registry.create_agent("TestAgent", agent_id.clone(), None).await.unwrap();

        assert_eq!(agent.id(), &agent_id);
    }
}
