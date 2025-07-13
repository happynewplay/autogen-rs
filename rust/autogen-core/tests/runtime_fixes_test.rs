use autogen_core::agent::{Agent, AgentFactory};
use autogen_core::agent_id::AgentId;
use autogen_core::agent_metadata::AgentMetadata;
use autogen_core::agent_runtime::AgentRuntime;
use autogen_core::agent_type::AgentType;
use autogen_core::intervention::{DefaultInterventionHandler, InterventionHandler};
use autogen_core::message_context::MessageContext;
use autogen_core::single_threaded_agent_runtime::SingleThreadedAgentRuntime;
use autogen_core::subscription::Subscription;
use autogen_core::type_subscription::TypeSubscription;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio;

// Test agent implementation
#[derive(Clone)]
struct TestAgent {
    id: AgentId,
    metadata: AgentMetadata,
}

impl TestAgent {
    fn new(id: AgentId) -> Self {
        Self {
            id: id.clone(),
            metadata: AgentMetadata {
                r#type: "test_agent".to_string(),
                key: id.to_string(),
                description: "Test agent for runtime fixes".to_string(),
            },
        }
    }
}

#[async_trait]
impl Agent for TestAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }

    fn metadata(&self) -> AgentMetadata {
        self.metadata.clone()
    }

    fn id(&self) -> AgentId {
        self.id.clone()
    }

    async fn bind_id_and_runtime(
        &mut self,
        _id: AgentId,
        _runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        // Simple binding implementation for testing
        Ok(())
    }

    async fn on_message(
        &mut self,
        message: Value,
        _context: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        // Echo the message back
        Ok(message)
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let mut state = HashMap::new();
        state.insert("agent_id".to_string(), serde_json::to_value(&self.id)?);
        Ok(state)
    }

    async fn load_state(&mut self, _state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        // Simple load state implementation for testing
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        // Simple close implementation for testing
        Ok(())
    }
}

// Test agent factory implementation
#[derive(Clone)]
struct TestAgentFactory;

impl TestAgentFactory {
    fn new() -> Self {
        Self
    }
}

impl AgentFactory for TestAgentFactory {
    fn clone_box(&self) -> Box<dyn AgentFactory> {
        Box::new(self.clone())
    }

    fn create(&self) -> Pin<Box<dyn Future<Output = Result<Box<dyn Agent>, Box<dyn Error>>> + Send>> {
        Box::pin(async move {
            let agent_id = AgentId {
                r#type: "test_agent".to_string(),
                key: "default".to_string(),
            };
            Ok(Box::new(TestAgent::new(agent_id)) as Box<dyn Agent>)
        })
    }
}

// Test intervention handler
#[derive(Clone)]
struct TestInterventionHandler {
    name: String,
}

impl TestInterventionHandler {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl InterventionHandler for TestInterventionHandler {
    fn clone_box(&self) -> Box<dyn InterventionHandler> {
        Box::new(self.clone())
    }

    async fn on_send(
        &self,
        message: Value,
        _message_context: &MessageContext,
        _recipient: &AgentId,
    ) -> Result<Value, autogen_core::intervention::DropMessage> {
        // Add a marker to show intervention was applied
        if let Value::Object(mut obj) = message {
            obj.insert("intervention_applied".to_string(), Value::String(self.name.clone()));
            Ok(Value::Object(obj))
        } else {
            Ok(message)
        }
    }
}

#[tokio::test]
async fn test_intervention_handler_integration() {
    // Create runtime with intervention handler
    let intervention_handler = Box::new(TestInterventionHandler::new("test_handler"));
    let runtime = SingleThreadedAgentRuntime::new(vec![intervention_handler]);

    // Register a test agent
    let agent_type = AgentType { r#type: "test_agent".to_string() };
    let agent_id = AgentId::from_str("test_agent/agent_1").unwrap();

    // Create a simple agent factory
    #[derive(Clone)]
    struct TestAgentFactory {
        agent_id: AgentId,
    }

    #[async_trait]
    impl AgentFactory for TestAgentFactory {
        fn create(&self) -> Pin<Box<dyn Future<Output = Result<Box<dyn Agent>, Box<dyn Error>>> + Send>> {
            let agent_id = self.agent_id.clone();
            Box::pin(async move {
                let agent = TestAgent::new(agent_id);
                Ok(Box::new(agent) as Box<dyn Agent>)
            })
        }

        fn clone_box(&self) -> Box<dyn AgentFactory> {
            Box::new(self.clone())
        }
    }

    runtime.register_factory(
        agent_type,
        Box::new(TestAgentFactory { agent_id }),
    ).await.unwrap();

    // Test that the runtime was created successfully
    assert_eq!(runtime.unprocessed_messages_count(), 0);
}

#[tokio::test]
async fn test_subscription_management() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);
    
    // Create a test subscription
    let agent_type = AgentType { r#type: "test_agent".to_string() };
    let subscription = TypeSubscription::new("test_topic".to_string(), agent_type);
    
    // Test adding subscription
    let subscription_id = subscription.id().to_string();
    let result = runtime.add_subscription(Box::new(subscription)).await;
    assert!(result.is_ok(), "Failed to add subscription: {:?}", result);

    // Test removing subscription
    let result = runtime.remove_subscription(&subscription_id).await;
    assert!(result.is_ok(), "Failed to remove subscription: {:?}", result);
}

#[tokio::test]
async fn test_state_persistence() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Test saving state
    let saved_state = runtime.save_state().await;
    assert!(saved_state.is_ok(), "Failed to save state: {:?}", saved_state);

    let state = saved_state.unwrap();
    assert!(state.contains_key("agents"));
    assert!(state.contains_key("subscription_manager"));
    assert!(state.contains_key("runtime_type"));
    assert!(state.contains_key("saved_at"));
    assert!(state.contains_key("version"));
    assert!(state.contains_key("schema_version"));
    assert!(state.contains_key("state_integrity"));

    // Test loading state
    let result = runtime.load_state(&state).await;
    assert!(result.is_ok(), "Failed to load state: {:?}", result);
}

#[tokio::test]
async fn test_enhanced_state_persistence() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Register a test agent factory
    let agent_type = AgentType { r#type: "test_agent".to_string() };
    let factory = TestAgentFactory::new();
    runtime.register_factory(agent_type, Box::new(factory)).await.unwrap();

    // Create and register an agent instance
    let agent_id = AgentId {
        r#type: "test_agent".to_string(),
        key: "test_key".to_string(),
    };
    let test_agent = TestAgent::new(agent_id.clone());
    runtime.register_agent_instance(
        Arc::new(Mutex::new(test_agent)),
        agent_id.clone()
    ).await.unwrap();

    // Save state with agent
    let saved_state = runtime.save_state().await.unwrap();

    // Verify enhanced state structure
    assert!(saved_state.contains_key("agents"));
    assert!(saved_state.contains_key("registered_agent_types"));
    assert!(saved_state.contains_key("intervention_handlers_count"));
    assert!(saved_state.contains_key("background_tasks"));
    assert!(saved_state.contains_key("state_integrity"));

    // Check agent state details
    let agents = saved_state.get("agents").unwrap().as_object().unwrap();
    assert!(!agents.is_empty());

    let agent_state = agents.get(&agent_id.to_string()).unwrap().as_object().unwrap();
    assert!(agent_state.contains_key("id"));
    assert!(agent_state.contains_key("type"));
    assert!(agent_state.contains_key("metadata"));
    assert!(agent_state.contains_key("internal_state"));

    // Check state integrity
    let integrity = saved_state.get("state_integrity").unwrap().as_object().unwrap();
    assert!(integrity.contains_key("checksum"));
    assert!(integrity.contains_key("agents_count"));
    assert_eq!(integrity.get("agents_count").unwrap().as_u64().unwrap(), 1);

    // Test loading the enhanced state
    let result = runtime.load_state(&saved_state).await;
    assert!(result.is_ok(), "Failed to load enhanced state: {:?}", result);
}

#[tokio::test]
async fn test_state_version_management() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Test saving state with current version
    let saved_state = runtime.save_state().await.unwrap();

    // Check version information
    assert_eq!(saved_state.get("schema_version").unwrap().as_str().unwrap(), "2.0");
    assert_eq!(saved_state.get("version").unwrap().as_str().unwrap(), "2.0.0");

    // Check supported versions in integrity info
    let integrity = saved_state.get("state_integrity").unwrap().as_object().unwrap();
    let supported_versions = integrity.get("supported_versions").unwrap().as_array().unwrap();
    assert!(supported_versions.contains(&serde_json::Value::String("1.0".to_string())));
    assert!(supported_versions.contains(&serde_json::Value::String("2.0".to_string())));

    // Test loading current version state
    let result = runtime.load_state(&saved_state).await;
    assert!(result.is_ok(), "Failed to load current version state: {:?}", result);
}

#[tokio::test]
async fn test_state_migration_from_v1_to_v2() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Create a mock v1.0 state
    let mut v1_state = HashMap::new();
    v1_state.insert("runtime_type".to_string(), serde_json::Value::String("SingleThreadedAgentRuntime".to_string()));
    v1_state.insert("schema_version".to_string(), serde_json::Value::String("1.0".to_string()));
    v1_state.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
    v1_state.insert("saved_at".to_string(), serde_json::Value::Number(1234567890.into()));

    // Add v1.0 format agents
    let mut agents = serde_json::Map::new();
    agents.insert("test_agent/test_key".to_string(), serde_json::json!({
        "id": "test_agent/test_key",
        "type": "test_agent",
        "created_at": 1234567890
    }));
    v1_state.insert("agents".to_string(), serde_json::Value::Object(agents));

    // Add v1.0 format subscriptions (array format)
    v1_state.insert("subscriptions".to_string(), serde_json::json!(["sub1", "sub2"]));

    // Test loading v1.0 state (should trigger migration)
    let result = runtime.load_state(&v1_state).await;
    assert!(result.is_ok(), "Failed to migrate and load v1.0 state: {:?}", result);
}

#[tokio::test]
async fn test_state_integrity_verification() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Save state with integrity checksum
    let mut saved_state = runtime.save_state().await.unwrap();

    // Verify integrity info exists
    assert!(saved_state.contains_key("state_integrity"));
    let integrity = saved_state.get("state_integrity").unwrap().as_object().unwrap();
    assert!(integrity.contains_key("checksum"));

    // Test loading with valid checksum
    let result = runtime.load_state(&saved_state).await;
    assert!(result.is_ok(), "Failed to load state with valid checksum: {:?}", result);

    // Tamper with the state (but keep integrity info)
    saved_state.insert("tampered_field".to_string(), serde_json::Value::String("tampered".to_string()));

    // Test loading with invalid checksum (should fail due to integrity check)
    let result = runtime.load_state(&saved_state).await;
    assert!(result.is_err(), "Should fail to load state with checksum mismatch: {:?}", result);

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("State integrity verification failed"));
}

#[tokio::test]
async fn test_unsupported_version_handling() {
    let runtime = SingleThreadedAgentRuntime::new(vec![]);

    // Create state with unsupported version
    let mut unsupported_state = HashMap::new();
    unsupported_state.insert("runtime_type".to_string(), serde_json::Value::String("SingleThreadedAgentRuntime".to_string()));
    unsupported_state.insert("schema_version".to_string(), serde_json::Value::String("99.0".to_string()));
    unsupported_state.insert("version".to_string(), serde_json::Value::String("99.0.0".to_string()));
    unsupported_state.insert("saved_at".to_string(), serde_json::Value::Number(1234567890.into()));

    // Test loading unsupported version (should fail)
    let result = runtime.load_state(&unsupported_state).await;
    assert!(result.is_err(), "Should fail to load unsupported version");

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Unsupported state version"));
}

#[tokio::test]
async fn test_default_intervention_handler() {
    let default_handler = DefaultInterventionHandler::new();
    let cloned = default_handler.clone_box();
    
    // Test that default handler doesn't modify messages
    let test_message = serde_json::json!({"test": "message"});
    let message_context = MessageContext {
        sender: None,
        topic_id: None,
        is_rpc: false,
        cancellation_token: Default::default(),
        message_id: "test_id".to_string(),
    };
    let agent_id = AgentId::from_str("test_agent/agent_1").unwrap();
    
    let result = cloned.on_send(test_message.clone(), &message_context, &agent_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), test_message);
}
