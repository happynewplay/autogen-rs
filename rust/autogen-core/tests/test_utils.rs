//! Test utilities and helpers for autogen-core tests

use autogen_core::*;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create a test agent ID with a unique suffix
pub fn create_test_agent_id(base_name: &str) -> AgentId {
    let unique_id = uuid::Uuid::new_v4().to_string();
    AgentId::new("test", &format!("{}_{}", base_name, &unique_id[..8])).unwrap()
}

/// Create a test topic ID with a unique suffix
pub fn create_test_topic_id(base_name: &str) -> TopicId {
    let unique_id = uuid::Uuid::new_v4().to_string();
    TopicId::new("test.topic", &format!("{}_{}", base_name, &unique_id[..8])).unwrap()
}

/// Create a test message context
pub fn create_test_context() -> MessageContext {
    let token = CancellationToken::new();
    MessageContext::direct_message(None, token)
}

/// Create a test message context with sender
pub fn create_test_context_with_sender(sender: AgentId) -> MessageContext {
    let token = CancellationToken::new();
    MessageContext::direct_message(Some(sender), token)
}

/// Mock agent that records all received messages
#[derive(Debug)]
pub struct RecordingAgent {
    pub id: AgentId,
    pub messages: Arc<Mutex<Vec<String>>>,
    pub call_count: Arc<Mutex<usize>>,
}

impl RecordingAgent {
    pub fn new(name: &str) -> Self {
        Self {
            id: create_test_agent_id(name),
            messages: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn get_messages(&self) -> Vec<String> {
        self.messages.lock().await.clone()
    }

    pub async fn get_call_count(&self) -> usize {
        *self.call_count.lock().await
    }

    pub async fn clear(&self) {
        self.messages.lock().await.clear();
        *self.call_count.lock().await = 0;
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for RecordingAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let mut messages = self.messages.lock().await;
        messages.push(message.content);
        
        let mut count = self.call_count.lock().await;
        *count += 1;
        
        Ok(Some(NoResponse))
    }
}

/// Mock agent that can simulate failures
#[derive(Debug)]
pub struct FailingAgent {
    pub id: AgentId,
    pub should_fail: Arc<Mutex<bool>>,
    pub failure_message: String,
}

impl FailingAgent {
    pub fn new(name: &str, failure_message: &str) -> Self {
        Self {
            id: create_test_agent_id(name),
            should_fail: Arc::new(Mutex::new(false)),
            failure_message: failure_message.to_string(),
        }
    }

    pub async fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().await = should_fail;
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for FailingAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        _message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        let should_fail = *self.should_fail.lock().await;
        if should_fail {
            Err(AutoGenError::other(&self.failure_message))
        } else {
            Ok(Some(NoResponse))
        }
    }
}

/// Mock agent that simulates slow processing
#[derive(Debug)]
pub struct SlowAgent {
    pub id: AgentId,
    pub delay_ms: u64,
}

impl SlowAgent {
    pub fn new(name: &str, delay_ms: u64) -> Self {
        Self {
            id: create_test_agent_id(name),
            delay_ms,
        }
    }
}

#[async_trait::async_trait]
impl TypedAgent<TextMessage> for SlowAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn handle_message(
        &mut self,
        _message: TextMessage,
        _context: &MessageContext,
    ) -> Result<Option<NoResponse>> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        Ok(Some(NoResponse))
    }
}

/// Test helper for creating memory content with validation
pub fn create_test_memory_content(content: &str) -> autogen_core::memory::MemoryContent {
    use autogen_core::memory::{MemoryContent, ContentLimits};
    
    let content = MemoryContent::text(content);
    let limits = ContentLimits::default();
    content.validate(&limits).expect("Test content should be valid");
    content
}

/// Test helper for creating large memory content that exceeds limits
pub fn create_oversized_memory_content() -> autogen_core::memory::MemoryContent {
    use autogen_core::memory::MemoryContent;
    
    // Create content that's definitely too large
    let large_content = "x".repeat(2_000_000); // 2MB of text
    MemoryContent::text(large_content)
}

/// Test helper for measuring execution time
pub async fn measure_execution_time<F, Fut, T>(f: F) -> (T, std::time::Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = std::time::Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    (result, duration)
}

/// Test helper for running concurrent operations
pub async fn run_concurrent_operations<F, Fut, T>(
    operations: Vec<F>,
) -> Vec<std::result::Result<T, tokio::task::JoinError>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handles: Vec<_> = operations
        .into_iter()
        .map(|op| tokio::spawn(op()))
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }
    results
}

/// Test helper for creating a batch of test messages
pub fn create_test_messages(count: usize, prefix: &str) -> Vec<TextMessage> {
    (0..count)
        .map(|i| TextMessage {
            content: format!("{} {}", prefix, i),
        })
        .collect()
}

/// Test helper for asserting error types
pub fn assert_error_type(result: &Result<()>, expected_error_substring: &str) {
    match result {
        Ok(_) => panic!("Expected error but got Ok"),
        Err(e) => {
            let error_string = e.to_string();
            assert!(
                error_string.contains(expected_error_substring),
                "Error '{}' does not contain expected substring '{}'",
                error_string,
                expected_error_substring
            );
        }
    }
}

/// Test helper for creating cancellation scenarios
pub struct CancellationScenario {
    pub parent_token: CancellationToken,
    pub child_tokens: Vec<CancellationToken>,
}

impl CancellationScenario {
    pub fn new(num_children: usize) -> Self {
        let parent_token = CancellationToken::new();
        let child_tokens = (0..num_children)
            .map(|_| parent_token.child_token())
            .collect();

        Self {
            parent_token,
            child_tokens,
        }
    }

    pub fn cancel_parent(&self) {
        self.parent_token.cancel();
    }

    pub fn all_cancelled(&self) -> bool {
        self.parent_token.is_cancelled()
            && self.child_tokens.iter().all(|token| token.is_cancelled())
    }
}

/// Test helper for stress testing
pub async fn stress_test_operation<F, Fut, T>(
    operation: F,
    iterations: usize,
    concurrency: usize,
) -> Vec<std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    T: Send + 'static,
{
    let operation = Arc::new(operation);
    let mut results = Vec::new();
    
    for chunk_start in (0..iterations).step_by(concurrency) {
        let chunk_end = std::cmp::min(chunk_start + concurrency, iterations);
        let chunk_size = chunk_end - chunk_start;
        
        let mut handles = Vec::new();
        for _ in 0..chunk_size {
            let op = operation.clone();
            let handle = tokio::spawn(async move { op().await });
            handles.push(handle);
        }
        
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)),
            }
        }
    }
    
    results
}
