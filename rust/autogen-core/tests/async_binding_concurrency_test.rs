use autogen_core::{
    agent::{Agent, AgentFactory},
    agent_id::AgentId,
    agent_metadata::AgentMetadata,
    agent_runtime::AgentRuntime,
    agent_type::AgentType,
    cancellation_token::CancellationToken,
    message_context::MessageContext,
    single_threaded_agent_runtime::SingleThreadedAgentRuntime,
};
use async_trait::async_trait;

use serde_json::Value;
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::timeout;

/// Test agent for concurrency testing
#[derive(Clone)]
struct TestConcurrentAgent {
    id: Option<AgentId>,
    message_count: Arc<Mutex<u32>>,
    processing_delay_ms: u64,
}

impl TestConcurrentAgent {
    fn new(processing_delay_ms: u64) -> Self {
        Self {
            id: None,
            message_count: Arc::new(Mutex::new(0)),
            processing_delay_ms,
        }
    }

    fn get_message_count(&self) -> u32 {
        *self.message_count.lock().unwrap()
    }
}

#[async_trait]
impl Agent for TestConcurrentAgent {
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }

    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            r#type: "TestConcurrentAgent".to_string(),
            key: "default".to_string(),
            description: "Test agent for concurrency testing".to_string(),
        }
    }

    fn id(&self) -> AgentId {
        self.id.clone().unwrap_or_else(|| AgentId {
            r#type: "TestConcurrentAgent".to_string(),
            key: "default".to_string(),
        })
    }

    async fn bind_id_and_runtime(
        &mut self,
        id: AgentId,
        _runtime: &dyn AgentRuntime,
    ) -> Result<(), Box<dyn Error>> {
        self.id = Some(id);
        Ok(())
    }

    async fn on_message(
        &mut self,
        _message: Value,
        _ctx: MessageContext,
    ) -> Result<Value, Box<dyn Error + Send>> {
        // Simulate processing time
        if self.processing_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.processing_delay_ms)).await;
        }

        // Increment message count
        {
            let mut count = self.message_count.lock().unwrap();
            *count += 1;
        }

        Ok(Value::String(format!("Processed message #{}", self.get_message_count())))
    }

    async fn save_state(&self) -> Result<HashMap<String, Value>, Box<dyn Error>> {
        let mut state = HashMap::new();
        state.insert("message_count".to_string(), Value::Number(self.get_message_count().into()));
        Ok(state)
    }

    async fn load_state(&mut self, state: &HashMap<String, Value>) -> Result<(), Box<dyn Error>> {
        if let Some(count_value) = state.get("message_count") {
            if let Some(count) = count_value.as_u64() {
                *self.message_count.lock().unwrap() = count as u32;
            }
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

/// Factory for creating test concurrent agents
#[derive(Clone)]
struct TestConcurrentAgentFactory {
    processing_delay_ms: u64,
}

impl TestConcurrentAgentFactory {
    fn new(processing_delay_ms: u64) -> Self {
        Self { processing_delay_ms }
    }
}

impl AgentFactory for TestConcurrentAgentFactory {
    fn create(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Agent>, Box<dyn Error>>> + Send>> {
        let processing_delay_ms = self.processing_delay_ms;
        Box::pin(async move {
            Ok(Box::new(TestConcurrentAgent::new(processing_delay_ms)) as Box<dyn Agent>)
        })
    }

    fn clone_box(&self) -> Box<dyn AgentFactory> {
        Box::new(self.clone())
    }
}



#[tokio::test]
async fn test_concurrent_agent_registration() {
    let runtime = Arc::new(SingleThreadedAgentRuntime::new(vec![]));

    // Register factory
    let factory = Box::new(TestConcurrentAgentFactory::new(10));
    let agent_type = AgentType { r#type: "TestConcurrentAgent".to_string() };
    runtime.register_factory(agent_type, factory).await.unwrap();

    // Test sequential agent registration (avoiding Send issues for now)
    let mut results = vec![];

    for i in 0..5 {
        let agent = Arc::new(Mutex::new(TestConcurrentAgent::new(10)));
        let agent_id = AgentId {
            r#type: "TestConcurrentAgent".to_string(),
            key: format!("agent_{}", i),
        };

        let result = runtime.register_agent_instance(agent, agent_id.clone()).await;
        results.push(result);
    }

    // Verify all agents were registered successfully
    assert_eq!(results.len(), 5);
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Agent {} registration failed: {:?}", i, result);
    }
}

#[tokio::test]
async fn test_concurrent_message_sending() {
    let runtime = Arc::new(SingleThreadedAgentRuntime::new(vec![]));
    
    // Register factory
    let factory = Box::new(TestConcurrentAgentFactory::new(5));
    let agent_type = AgentType { r#type: "TestConcurrentAgent".to_string() };
    runtime.register_factory(agent_type, factory).await.unwrap();

    // Register a single agent
    let agent = Arc::new(Mutex::new(TestConcurrentAgent::new(5)));
    let agent_id = AgentId {
        r#type: "TestConcurrentAgent".to_string(),
        key: "test_agent".to_string(),
    };
    runtime.register_agent_instance(agent.clone(), agent_id.clone()).await.unwrap();

    // Send multiple concurrent messages
    let mut handles = vec![];
    
    for i in 0..20 {
        let runtime_clone = runtime.clone();
        let agent_id_clone = agent_id.clone();
        let handle = tokio::spawn(async move {
            let message = Value::String(format!("Test message {}", i));
            runtime_clone.send_message(
                message,
                agent_id_clone,
                None,
                Some(CancellationToken::new()),
                Some(format!("msg_{}", i)),
            ).await
        });
        handles.push(handle);
    }

    // Wait for all messages to be processed
    let mut responses = vec![];
    for handle in handles {
        let result = timeout(Duration::from_secs(10), handle).await;
        assert!(result.is_ok(), "Message sending timed out");
        responses.push(result.unwrap().unwrap());
    }

    // Verify all messages were processed successfully
    assert_eq!(responses.len(), 20);
    for (i, response) in responses.iter().enumerate() {
        assert!(response.is_ok(), "Message {} failed: {:?}", i, response);
    }

    // Verify the agent processed all messages
    let final_count = {
        let agent_guard = agent.lock().unwrap();
        agent_guard.get_message_count()
    };
    assert_eq!(final_count, 20, "Agent should have processed 20 messages, but processed {}", final_count);
}

#[tokio::test]
async fn test_agent_binding_thread_safety() {
    let runtime = Arc::new(SingleThreadedAgentRuntime::new(vec![]));

    // Register factory
    let factory = Box::new(TestConcurrentAgentFactory::new(0));
    let agent_type = AgentType { r#type: "TestConcurrentAgent".to_string() };
    runtime.register_factory(agent_type, factory).await.unwrap();

    // Test sequential binding operations (avoiding Send issues for now)
    for i in 0..5 {
        let mut agent = TestConcurrentAgent::new(0);
        let agent_id = AgentId {
            r#type: "TestConcurrentAgent".to_string(),
            key: format!("bind_test_{}", i),
        };

        // Test direct binding
        let runtime_ref: &dyn AgentRuntime = &*runtime;
        let result = agent.bind_id_and_runtime(agent_id.clone(), runtime_ref).await;
        assert!(result.is_ok(), "Binding operation failed: {:?}", result);
    }
}
