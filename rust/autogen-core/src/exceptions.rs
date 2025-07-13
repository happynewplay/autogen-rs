use std::fmt;
use std::time::Duration;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tracing;
use tokio::time::sleep;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

/// Error severity levels for classification and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Low severity - informational errors that don't affect operation
    Low = 1,
    /// Medium severity - warnings that may affect performance
    Medium = 2,
    /// High severity - errors that affect functionality
    High = 3,
    /// Critical severity - errors that can cause system failure
    Critical = 4,
}

/// Error categories for different types of errors
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Network-related errors (timeouts, connection failures)
    Network,
    /// Authentication and authorization errors
    Authentication,
    /// Resource-related errors (memory, disk, etc.)
    Resource,
    /// Configuration errors
    Configuration,
    /// Business logic errors
    BusinessLogic,
    /// External service errors
    ExternalService,
    /// Internal system errors
    Internal,
}

/// Indicates whether an error is recoverable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoverabilityType {
    /// Error can be automatically recovered
    Recoverable,
    /// Error requires manual intervention
    ManualIntervention,
    /// Error is permanent and cannot be recovered
    Permanent,
}

/// Enhanced error context with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
    pub recoverability: RecoverabilityType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operation: String,
    pub component: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub correlation_id: Option<String>,
    pub retry_count: u32,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            severity: ErrorSeverity::Medium,
            category: ErrorCategory::Internal,
            recoverability: RecoverabilityType::Recoverable,
            timestamp: chrono::Utc::now(),
            operation: "unknown".to_string(),
            component: "unknown".to_string(),
            metadata: std::collections::HashMap::new(),
            correlation_id: None,
            retry_count: 0,
        }
    }
}

/// Retry strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor to add randomness (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Whether to use exponential backoff
    pub exponential_backoff: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            exponential_backoff: true,
        }
    }
}

impl RetryStrategy {
    /// Calculate delay for a specific retry attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let mut delay = if self.exponential_backoff {
            let multiplier = self.backoff_multiplier.powi((attempt - 1) as i32);
            Duration::from_millis((self.initial_delay.as_millis() as f64 * multiplier) as u64)
        } else {
            self.initial_delay
        };

        // Apply maximum delay limit
        if delay > self.max_delay {
            delay = self.max_delay;
        }

        // Add jitter to prevent thundering herd
        if self.jitter_factor > 0.0 {
            let jitter = (delay.as_millis() as f64 * self.jitter_factor * rand::random::<f64>()) as u64;
            delay = Duration::from_millis(delay.as_millis() as u64 + jitter);
        }

        delay
    }

    /// Check if we should retry based on attempt count
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Enhanced error type with context and retry capabilities
#[derive(Error, Debug, Clone)]
pub enum AutogenError {
    #[error("Can't handle exception: {message}")]
    CantHandle {
        message: String,
        context: ErrorContext,
    },

    #[error("Undeliverable exception: {message}")]
    Undeliverable {
        message: String,
        context: ErrorContext,
    },

    #[error("Message dropped exception: {message}")]
    MessageDropped {
        message: String,
        context: ErrorContext,
    },

    #[error("Not accessible error: {message}")]
    NotAccessible {
        message: String,
        context: ErrorContext,
    },

    #[error("General error: {message}")]
    General {
        message: String,
        context: ErrorContext,
    },

    #[error("Network error: {message}")]
    Network {
        message: String,
        context: ErrorContext,
    },

    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        context: ErrorContext,
    },

    #[error("Resource error: {message}")]
    Resource {
        message: String,
        context: ErrorContext,
    },

    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        context: ErrorContext,
    },

    #[error("External service error: {message}")]
    ExternalService {
        message: String,
        context: ErrorContext,
    },

    #[error("Retry exhausted: {message}")]
    RetryExhausted {
        message: String,
        context: ErrorContext,
        last_error: Option<String>,
    },
}

impl AutogenError {
    /// Get the error context
    pub fn context(&self) -> &ErrorContext {
        match self {
            AutogenError::CantHandle { context, .. } => context,
            AutogenError::Undeliverable { context, .. } => context,
            AutogenError::MessageDropped { context, .. } => context,
            AutogenError::NotAccessible { context, .. } => context,
            AutogenError::General { context, .. } => context,
            AutogenError::Network { context, .. } => context,
            AutogenError::Authentication { context, .. } => context,
            AutogenError::Resource { context, .. } => context,
            AutogenError::Configuration { context, .. } => context,
            AutogenError::ExternalService { context, .. } => context,
            AutogenError::RetryExhausted { context, .. } => context,
        }
    }

    /// Get mutable error context
    pub fn context_mut(&mut self) -> &mut ErrorContext {
        match self {
            AutogenError::CantHandle { context, .. } => context,
            AutogenError::Undeliverable { context, .. } => context,
            AutogenError::MessageDropped { context, .. } => context,
            AutogenError::NotAccessible { context, .. } => context,
            AutogenError::General { context, .. } => context,
            AutogenError::Network { context, .. } => context,
            AutogenError::Authentication { context, .. } => context,
            AutogenError::Resource { context, .. } => context,
            AutogenError::Configuration { context, .. } => context,
            AutogenError::ExternalService { context, .. } => context,
            AutogenError::RetryExhausted { context, .. } => context,
        }
    }

    /// Check if this error is retryable based on its context
    pub fn is_retryable(&self) -> bool {
        match self.context().recoverability {
            RecoverabilityType::Recoverable => true,
            RecoverabilityType::ManualIntervention => false,
            RecoverabilityType::Permanent => false,
        }
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.context_mut().retry_count += 1;
    }

    /// Create a new error with updated context
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        *self.context_mut() = context;
        self
    }

    /// Add metadata to the error context
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.context_mut().metadata.insert(key, value);
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.context_mut().correlation_id = Some(correlation_id);
        self
    }
}

// Backward compatibility types - these maintain the original API
/// Raised when a handler can't handle the exception.
#[derive(Debug)]
pub struct CantHandleException(pub String);

impl fmt::Display for CantHandleException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Can't handle exception: {}", self.0)
    }
}

impl std::error::Error for CantHandleException {}

/// Raised when a message can't be delivered.
#[derive(Debug)]
pub struct UndeliverableException(pub String);

impl fmt::Display for UndeliverableException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Undeliverable exception: {}", self.0)
    }
}

impl std::error::Error for UndeliverableException {}

/// Raised when a message is dropped.
#[derive(Debug)]
pub struct MessageDroppedException(pub String);

impl fmt::Display for MessageDroppedException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message dropped exception: {}", self.0)
    }
}

impl std::error::Error for MessageDroppedException {}

/// Tried to access a value that is not accessible.
/// For example if it is remote cannot be accessed locally.
#[derive(Debug)]
pub struct NotAccessibleError(pub String);

impl fmt::Display for NotAccessibleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not accessible error: {}", self.0)
    }
}

impl std::error::Error for NotAccessibleError {}

#[derive(Debug)]
pub struct GeneralError(pub String);

impl fmt::Display for GeneralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GeneralError {}

/// Error recovery strategy
#[derive(Clone)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    Retry(RetryStrategy),
    /// Fallback to alternative implementation
    Fallback(Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), AutogenError>> + Send>> + Send + Sync>),
    /// Graceful degradation - continue with reduced functionality
    Degrade,
    /// Circuit breaker - stop processing temporarily
    CircuitBreaker {
        failure_threshold: u32,
        recovery_timeout: Duration,
    },
    /// Manual intervention required
    Manual,
}

impl std::fmt::Debug for RecoveryStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryStrategy::Retry(strategy) => f.debug_tuple("Retry").field(strategy).finish(),
            RecoveryStrategy::Fallback(_) => f.debug_tuple("Fallback").field(&"<closure>").finish(),
            RecoveryStrategy::Degrade => write!(f, "Degrade"),
            RecoveryStrategy::CircuitBreaker { failure_threshold, recovery_timeout } => {
                f.debug_struct("CircuitBreaker")
                    .field("failure_threshold", failure_threshold)
                    .field("recovery_timeout", recovery_timeout)
                    .finish()
            }
            RecoveryStrategy::Manual => write!(f, "Manual"),
        }
    }
}

/// Retry executor with enhanced error handling
pub struct RetryExecutor {
    strategy: RetryStrategy,
}

impl RetryExecutor {
    pub fn new(strategy: RetryStrategy) -> Self {
        Self { strategy }
    }

    pub fn with_defaults() -> Self {
        Self::new(RetryStrategy::default())
    }

    /// Execute a function with retry logic
    pub async fn execute<F, Fut, T, E>(&self, mut operation: F) -> Result<T, AutogenError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Into<AutogenError> + std::error::Error + Send + Sync + 'static,
    {
        let mut last_error: Option<AutogenError> = None;

        for attempt in 0..=self.strategy.max_attempts {
            if attempt > 0 {
                let delay = self.strategy.calculate_delay(attempt);
                tracing::debug!("Retrying operation after {:?} (attempt {})", delay, attempt);
                sleep(delay).await;
            }

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!("Operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    let mut autogen_error = error.into();
                    autogen_error.increment_retry();

                    tracing::warn!(
                        "Operation failed on attempt {} with error: {}",
                        attempt + 1,
                        autogen_error
                    );

                    if !autogen_error.is_retryable() {
                        tracing::error!("Error is not retryable, aborting");
                        return Err(autogen_error);
                    }

                    if !self.strategy.should_retry(attempt + 1) {
                        tracing::error!("Max retry attempts reached");
                        return Err(AutogenError::RetryExhausted {
                            message: format!("Operation failed after {} attempts", attempt + 1),
                            context: ErrorContext {
                                severity: ErrorSeverity::High,
                                category: ErrorCategory::Internal,
                                recoverability: RecoverabilityType::ManualIntervention,
                                retry_count: attempt + 1,
                                ..Default::default()
                            },
                            last_error: Some(autogen_error.to_string()),
                        });
                    }

                    last_error = Some(autogen_error);
                }
            }
        }

        // This should never be reached due to the loop logic above
        unreachable!("Retry loop should have returned or errored")
    }
}

/// Error classifier for automatic error categorization
pub struct ErrorClassifier;

impl ErrorClassifier {
    /// Classify an error based on its characteristics
    pub fn classify_error(error: &dyn std::error::Error) -> ErrorContext {
        let error_string = error.to_string().to_lowercase();

        let (category, severity, recoverability) = if error_string.contains("timeout")
            || error_string.contains("connection")
            || error_string.contains("network") {
            (ErrorCategory::Network, ErrorSeverity::Medium, RecoverabilityType::Recoverable)
        } else if error_string.contains("auth")
            || error_string.contains("permission")
            || error_string.contains("unauthorized") {
            (ErrorCategory::Authentication, ErrorSeverity::High, RecoverabilityType::ManualIntervention)
        } else if error_string.contains("memory")
            || error_string.contains("disk")
            || error_string.contains("resource") {
            (ErrorCategory::Resource, ErrorSeverity::High, RecoverabilityType::Recoverable)
        } else if error_string.contains("config")
            || error_string.contains("setting") {
            (ErrorCategory::Configuration, ErrorSeverity::Medium, RecoverabilityType::ManualIntervention)
        } else if error_string.contains("external")
            || error_string.contains("service")
            || error_string.contains("api") {
            (ErrorCategory::ExternalService, ErrorSeverity::Medium, RecoverabilityType::Recoverable)
        } else {
            (ErrorCategory::Internal, ErrorSeverity::Medium, RecoverabilityType::Recoverable)
        };

        ErrorContext {
            severity,
            category,
            recoverability,
            timestamp: chrono::Utc::now(),
            operation: "unknown".to_string(),
            component: "unknown".to_string(),
            metadata: std::collections::HashMap::new(),
            correlation_id: None,
            retry_count: 0,
        }
    }

    /// Get recommended retry strategy for an error category
    pub fn get_retry_strategy(category: &ErrorCategory) -> RetryStrategy {
        match category {
            ErrorCategory::Network => RetryStrategy {
                max_attempts: 5,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 2.0,
                jitter_factor: 0.2,
                exponential_backoff: true,
            },
            ErrorCategory::ExternalService => RetryStrategy {
                max_attempts: 3,
                initial_delay: Duration::from_millis(1000),
                max_delay: Duration::from_secs(60),
                backoff_multiplier: 2.5,
                jitter_factor: 0.1,
                exponential_backoff: true,
            },
            ErrorCategory::Resource => RetryStrategy {
                max_attempts: 2,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
                jitter_factor: 0.0,
                exponential_backoff: false,
            },
            _ => RetryStrategy::default(),
        }
    }
}

/// Error recovery manager for coordinating recovery strategies
pub struct ErrorRecoveryManager {
    strategies: std::collections::HashMap<ErrorCategory, RecoveryStrategy>,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        let mut strategies = std::collections::HashMap::new();

        // Default strategies for different error categories
        strategies.insert(
            ErrorCategory::Network,
            RecoveryStrategy::Retry(ErrorClassifier::get_retry_strategy(&ErrorCategory::Network))
        );

        strategies.insert(
            ErrorCategory::ExternalService,
            RecoveryStrategy::Retry(ErrorClassifier::get_retry_strategy(&ErrorCategory::ExternalService))
        );

        strategies.insert(
            ErrorCategory::Resource,
            RecoveryStrategy::CircuitBreaker {
                failure_threshold: 5,
                recovery_timeout: Duration::from_secs(60),
            }
        );

        strategies.insert(
            ErrorCategory::Authentication,
            RecoveryStrategy::Manual
        );

        strategies.insert(
            ErrorCategory::Configuration,
            RecoveryStrategy::Manual
        );

        Self { strategies }
    }

    /// Get recovery strategy for a specific error category
    pub fn get_strategy(&self, category: &ErrorCategory) -> Option<&RecoveryStrategy> {
        self.strategies.get(category)
    }

    /// Set custom recovery strategy for an error category
    pub fn set_strategy(&mut self, category: ErrorCategory, strategy: RecoveryStrategy) {
        self.strategies.insert(category, strategy);
    }

    /// Execute recovery for a given error
    pub async fn recover(&self, error: &AutogenError) -> Result<(), AutogenError> {
        let category = &error.context().category;

        match self.get_strategy(category) {
            Some(RecoveryStrategy::Retry(retry_strategy)) => {
                tracing::info!("Attempting retry recovery for {:?} error", category);
                // The actual retry logic would be handled by the RetryExecutor
                Ok(())
            }
            Some(RecoveryStrategy::Degrade) => {
                tracing::warn!("Applying graceful degradation for {:?} error", category);
                // Implement degradation logic here
                Ok(())
            }
            Some(RecoveryStrategy::Manual) => {
                tracing::error!("Manual intervention required for {:?} error", category);
                Err(AutogenError::General {
                    message: "Manual intervention required".to_string(),
                    context: error.context().clone(),
                })
            }
            Some(RecoveryStrategy::CircuitBreaker { .. }) => {
                tracing::warn!("Circuit breaker activated for {:?} error", category);
                // Implement circuit breaker logic here
                Ok(())
            }
            Some(RecoveryStrategy::Fallback(_)) => {
                tracing::info!("Executing fallback strategy for {:?} error", category);
                // Fallback execution would be handled separately
                Ok(())
            }
            None => {
                tracing::warn!("No recovery strategy found for {:?} error", category);
                Err(error.clone())
            }
        }
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

// Conversion implementations for backward compatibility
impl From<CantHandleException> for AutogenError {
    fn from(err: CantHandleException) -> Self {
        AutogenError::CantHandle {
            message: err.0,
            context: ErrorContext {
                category: ErrorCategory::BusinessLogic,
                severity: ErrorSeverity::Medium,
                recoverability: RecoverabilityType::ManualIntervention,
                ..Default::default()
            },
        }
    }
}

impl From<UndeliverableException> for AutogenError {
    fn from(err: UndeliverableException) -> Self {
        AutogenError::Undeliverable {
            message: err.0,
            context: ErrorContext {
                category: ErrorCategory::Network,
                severity: ErrorSeverity::High,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        }
    }
}

impl From<MessageDroppedException> for AutogenError {
    fn from(err: MessageDroppedException) -> Self {
        AutogenError::MessageDropped {
            message: err.0,
            context: ErrorContext {
                category: ErrorCategory::Internal,
                severity: ErrorSeverity::Medium,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        }
    }
}

impl From<NotAccessibleError> for AutogenError {
    fn from(err: NotAccessibleError) -> Self {
        AutogenError::NotAccessible {
            message: err.0,
            context: ErrorContext {
                category: ErrorCategory::Authentication,
                severity: ErrorSeverity::High,
                recoverability: RecoverabilityType::ManualIntervention,
                ..Default::default()
            },
        }
    }
}

impl From<GeneralError> for AutogenError {
    fn from(err: GeneralError) -> Self {
        AutogenError::General {
            message: err.0,
            context: ErrorContext::default(),
        }
    }
}

// Utility functions for error handling
pub mod utils {
    use super::*;

    /// Create a retryable network error
    pub fn network_error(message: impl Into<String>) -> AutogenError {
        AutogenError::Network {
            message: message.into(),
            context: ErrorContext {
                category: ErrorCategory::Network,
                severity: ErrorSeverity::Medium,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> AutogenError {
        AutogenError::Configuration {
            message: message.into(),
            context: ErrorContext {
                category: ErrorCategory::Configuration,
                severity: ErrorSeverity::High,
                recoverability: RecoverabilityType::ManualIntervention,
                ..Default::default()
            },
        }
    }

    /// Create an authentication error
    pub fn auth_error(message: impl Into<String>) -> AutogenError {
        AutogenError::Authentication {
            message: message.into(),
            context: ErrorContext {
                category: ErrorCategory::Authentication,
                severity: ErrorSeverity::High,
                recoverability: RecoverabilityType::ManualIntervention,
                ..Default::default()
            },
        }
    }

    /// Create a resource error
    pub fn resource_error(message: impl Into<String>) -> AutogenError {
        AutogenError::Resource {
            message: message.into(),
            context: ErrorContext {
                category: ErrorCategory::Resource,
                severity: ErrorSeverity::High,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        }
    }

    /// Create an external service error
    pub fn external_service_error(message: impl Into<String>) -> AutogenError {
        AutogenError::ExternalService {
            message: message.into(),
            context: ErrorContext {
                category: ErrorCategory::ExternalService,
                severity: ErrorSeverity::Medium,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        }
    }
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    failure_count: std::sync::atomic::AtomicU32,
    last_failure_time: std::sync::Mutex<Option<std::time::Instant>>,
    state: std::sync::Mutex<CircuitBreakerState>,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            failure_count: std::sync::atomic::AtomicU32::new(0),
            last_failure_time: std::sync::Mutex::new(None),
            state: std::sync::Mutex::new(CircuitBreakerState::Closed),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, AutogenError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Into<AutogenError> + std::error::Error + Send + Sync + 'static,
    {
        // Check if circuit breaker should allow the operation
        if !self.should_allow_request().await {
            return Err(utils::external_service_error("Circuit breaker is open"));
        }

        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error.into())
            }
        }
    }

    async fn should_allow_request(&self) -> bool {
        let state = self.state.lock().unwrap().clone();

        match state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if enough time has passed to try half-open
                if let Some(last_failure) = *self.last_failure_time.lock().unwrap() {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        *self.state.lock().unwrap() = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    async fn on_success(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.state.lock().unwrap() = CircuitBreakerState::Closed;
    }

    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        *self.last_failure_time.lock().unwrap() = Some(std::time::Instant::now());

        if failures >= self.failure_threshold {
            *self.state.lock().unwrap() = CircuitBreakerState::Open;
        }
    }
}

/// Graceful degradation manager
pub struct DegradationManager {
    degraded_features: std::sync::RwLock<std::collections::HashSet<String>>,
}

impl DegradationManager {
    pub fn new() -> Self {
        Self {
            degraded_features: std::sync::RwLock::new(std::collections::HashSet::new()),
        }
    }

    /// Mark a feature as degraded
    pub async fn degrade_feature(&self, feature: String) {
        let mut degraded = self.degraded_features.write().unwrap();
        degraded.insert(feature.clone());
        tracing::warn!("Feature '{}' has been degraded", feature);
    }

    /// Restore a degraded feature
    pub async fn restore_feature(&self, feature: &str) {
        let mut degraded = self.degraded_features.write().unwrap();
        if degraded.remove(feature) {
            tracing::info!("Feature '{}' has been restored", feature);
        }
    }

    /// Check if a feature is currently degraded
    pub fn is_degraded(&self, feature: &str) -> bool {
        let degraded = self.degraded_features.read().unwrap();
        degraded.contains(feature)
    }

    /// Get all currently degraded features
    pub fn get_degraded_features(&self) -> Vec<String> {
        let degraded = self.degraded_features.read().unwrap();
        degraded.iter().cloned().collect()
    }
}

impl Default for DegradationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Error chain for tracking error propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorChain {
    pub errors: Vec<ErrorChainEntry>,
    pub root_correlation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorChainEntry {
    pub error_type: String,
    pub message: String,
    pub context: ErrorContext,
    pub stack_trace: Option<String>,
    pub source_location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub function: String,
}

impl ErrorChain {
    pub fn new(correlation_id: String) -> Self {
        Self {
            errors: Vec::new(),
            root_correlation_id: correlation_id,
        }
    }

    /// Add an error to the chain
    pub fn add_error(&mut self, error: &AutogenError, source_location: Option<SourceLocation>) {
        let entry = ErrorChainEntry {
            error_type: std::any::type_name::<AutogenError>().to_string(),
            message: error.to_string(),
            context: error.context().clone(),
            stack_trace: self.capture_stack_trace(),
            source_location,
        };
        self.errors.push(entry);
    }

    /// Get the root cause error
    pub fn root_cause(&self) -> Option<&ErrorChainEntry> {
        self.errors.first()
    }

    /// Get the most recent error
    pub fn latest_error(&self) -> Option<&ErrorChainEntry> {
        self.errors.last()
    }

    /// Get error chain depth
    pub fn depth(&self) -> usize {
        self.errors.len()
    }

    /// Capture stack trace (simplified implementation)
    fn capture_stack_trace(&self) -> Option<String> {
        // In a real implementation, you might use backtrace crate
        // For now, we'll return a placeholder
        Some("Stack trace capture not implemented".to_string())
    }

    /// Format the error chain for logging
    pub fn format_chain(&self) -> String {
        let mut result = format!("Error Chain ({}): ", self.root_correlation_id);

        for (i, entry) in self.errors.iter().enumerate() {
            result.push_str(&format!(
                "\n  {}. {} in {} ({}): {}",
                i + 1,
                entry.error_type,
                entry.context.component,
                entry.context.operation,
                entry.message
            ));

            if let Some(location) = &entry.source_location {
                result.push_str(&format!(
                    "\n     at {}:{}:{} in {}",
                    location.file, location.line, location.column, location.function
                ));
            }
        }

        result
    }
}

/// Enhanced AutogenError with error chain support
impl AutogenError {
    /// Create a new error with chain tracking
    pub fn with_chain(mut self, chain: ErrorChain) -> Self {
        self.context_mut().metadata.insert(
            "error_chain".to_string(),
            serde_json::to_value(chain).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Get the error chain if present
    pub fn get_chain(&self) -> Option<ErrorChain> {
        self.context()
            .metadata
            .get("error_chain")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Add this error to an existing chain
    pub fn add_to_chain(self, mut chain: ErrorChain, source_location: Option<SourceLocation>) -> Self {
        chain.add_error(&self, source_location);
        self.with_chain(chain)
    }

    /// Create a new error chain starting with this error
    pub fn start_chain(self, correlation_id: String, source_location: Option<SourceLocation>) -> Self {
        let mut chain = ErrorChain::new(correlation_id);
        chain.add_error(&self, source_location);
        self.with_chain(chain)
    }
}

/// Macro for creating source location information
#[macro_export]
macro_rules! source_location {
    () => {
        Some($crate::exceptions::SourceLocation {
            file: file!().to_string(),
            line: line!(),
            column: column!(),
            function: {
                fn f() {}
                fn type_name_of<T>(_: T) -> &'static str {
                    std::any::type_name::<T>()
                }
                let name = type_name_of(f);
                name.strip_suffix("::f").unwrap_or(name).to_string()
            },
        })
    };
}

/// Macro for propagating errors with chain tracking
#[macro_export]
macro_rules! propagate_error {
    ($result:expr, $correlation_id:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                let autogen_err: $crate::exceptions::AutogenError = err.into();
                return Err(autogen_err.start_chain($correlation_id, source_location!()));
            }
        }
    };
    ($result:expr, $chain:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => {
                let autogen_err: $crate::exceptions::AutogenError = err.into();
                return Err(autogen_err.add_to_chain($chain, source_location!()));
            }
        }
    };
}

/// Advanced error classification system
pub struct AdvancedErrorClassifier {
    classification_rules: Vec<ClassificationRule>,
}

#[derive(Debug, Clone)]
pub struct ClassificationRule {
    pub pattern: regex::Regex,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub recoverability: RecoverabilityType,
    pub recommended_strategy: RetryStrategy,
}

impl AdvancedErrorClassifier {
    pub fn new() -> Self {
        let mut classifier = Self {
            classification_rules: Vec::new(),
        };
        classifier.add_default_rules();
        classifier
    }

    fn add_default_rules(&mut self) {
        // Network errors
        self.add_rule(
            r"(?i)(timeout|connection|network|dns|socket)",
            ErrorCategory::Network,
            ErrorSeverity::Medium,
            RecoverabilityType::Recoverable,
            RetryStrategy {
                max_attempts: 5,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 2.0,
                jitter_factor: 0.2,
                exponential_backoff: true,
            },
        );

        // Authentication errors
        self.add_rule(
            r"(?i)(auth|unauthorized|forbidden|permission|credential)",
            ErrorCategory::Authentication,
            ErrorSeverity::High,
            RecoverabilityType::ManualIntervention,
            RetryStrategy {
                max_attempts: 1,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(1),
                backoff_multiplier: 1.0,
                jitter_factor: 0.0,
                exponential_backoff: false,
            },
        );

        // External service errors (check before resource errors to catch rate limits)
        self.add_rule(
            r"(?i)(service.*unavailable|api.*error|external.*service|rate.*limit)",
            ErrorCategory::ExternalService,
            ErrorSeverity::Medium,
            RecoverabilityType::Recoverable,
            RetryStrategy {
                max_attempts: 4,
                initial_delay: Duration::from_millis(1000),
                max_delay: Duration::from_secs(120),
                backoff_multiplier: 2.5,
                jitter_factor: 0.3,
                exponential_backoff: true,
            },
        );

        // Resource errors
        self.add_rule(
            r"(?i)(memory|disk|cpu|resource|quota|(?<!rate\s)limit)",
            ErrorCategory::Resource,
            ErrorSeverity::High,
            RecoverabilityType::Recoverable,
            RetryStrategy {
                max_attempts: 3,
                initial_delay: Duration::from_secs(2),
                max_delay: Duration::from_secs(60),
                backoff_multiplier: 3.0,
                jitter_factor: 0.1,
                exponential_backoff: true,
            },
        );

        // Configuration errors
        self.add_rule(
            r"(?i)(config|setting|parameter|invalid.*format)",
            ErrorCategory::Configuration,
            ErrorSeverity::High,
            RecoverabilityType::ManualIntervention,
            RetryStrategy {
                max_attempts: 1,
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(1),
                backoff_multiplier: 1.0,
                jitter_factor: 0.0,
                exponential_backoff: false,
            },
        );
    }

    pub fn add_rule(
        &mut self,
        pattern: &str,
        category: ErrorCategory,
        severity: ErrorSeverity,
        recoverability: RecoverabilityType,
        strategy: RetryStrategy,
    ) {
        if let Ok(regex) = regex::Regex::new(pattern) {
            self.classification_rules.push(ClassificationRule {
                pattern: regex,
                category,
                severity,
                recoverability,
                recommended_strategy: strategy,
            });
        }
    }

    /// Classify an error with detailed analysis
    pub fn classify_detailed(&self, error: &dyn std::error::Error) -> DetailedClassification {
        let error_message = error.to_string();

        // Try to match against classification rules
        for rule in &self.classification_rules {
            if rule.pattern.is_match(&error_message) {
                return DetailedClassification {
                    category: rule.category.clone(),
                    severity: rule.severity,
                    recoverability: rule.recoverability,
                    confidence: 0.9, // High confidence for pattern matches
                    recommended_strategy: rule.recommended_strategy.clone(),
                    reasoning: format!("Matched pattern: {}", rule.pattern.as_str()),
                    tags: self.extract_tags(&error_message),
                };
            }
        }

        // Fallback classification
        DetailedClassification {
            category: ErrorCategory::Internal,
            severity: ErrorSeverity::Medium,
            recoverability: RecoverabilityType::Recoverable,
            confidence: 0.3, // Low confidence for fallback
            recommended_strategy: RetryStrategy::default(),
            reasoning: "No specific pattern matched, using default classification".to_string(),
            tags: self.extract_tags(&error_message),
        }
    }

    fn extract_tags(&self, error_message: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let message_lower = error_message.to_lowercase();

        // Extract common error indicators as tags
        let indicators = [
            "timeout", "connection", "network", "auth", "permission",
            "memory", "disk", "cpu", "config", "api", "service",
            "rate_limit", "quota", "invalid", "not_found", "forbidden"
        ];

        for indicator in &indicators {
            if message_lower.contains(indicator) {
                tags.push(indicator.to_string());
            }
        }

        tags
    }
}

#[derive(Debug, Clone)]
pub struct DetailedClassification {
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub recoverability: RecoverabilityType,
    pub confidence: f64, // 0.0 to 1.0
    pub recommended_strategy: RetryStrategy,
    pub reasoning: String,
    pub tags: Vec<String>,
}

impl Default for AdvancedErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive error handling strategy manager
pub struct ErrorHandlingStrategyManager {
    classifier: AdvancedErrorClassifier,
    recovery_manager: ErrorRecoveryManager,
    circuit_breakers: std::collections::HashMap<String, Arc<CircuitBreaker>>,
    degradation_manager: Arc<DegradationManager>,
    metrics: ErrorMetrics,
}

#[derive(Debug, Default)]
pub struct ErrorMetrics {
    pub total_errors: std::sync::atomic::AtomicU64,
    pub errors_by_category: std::sync::RwLock<std::collections::HashMap<ErrorCategory, u64>>,
    pub errors_by_severity: std::sync::RwLock<std::collections::HashMap<ErrorSeverity, u64>>,
    pub recovery_success_rate: std::sync::atomic::AtomicU64,
    pub circuit_breaker_trips: std::sync::atomic::AtomicU64,
}

impl ErrorHandlingStrategyManager {
    pub fn new() -> Self {
        Self {
            classifier: AdvancedErrorClassifier::new(),
            recovery_manager: ErrorRecoveryManager::new(),
            circuit_breakers: std::collections::HashMap::new(),
            degradation_manager: Arc::new(DegradationManager::new()),
            metrics: ErrorMetrics::default(),
        }
    }

    /// Handle an error with comprehensive strategy
    pub async fn handle_error(&mut self, error: &dyn std::error::Error, context: &str) -> Result<RecoveryAction, AutogenError> {
        // Update metrics
        self.update_metrics(error).await;

        // Classify the error
        let classification = self.classifier.classify_detailed(error);

        tracing::error!(
            "Handling error in context '{}': {} (confidence: {:.2}, category: {:?})",
            context,
            error,
            classification.confidence,
            classification.category
        );

        // Determine recovery action based on classification
        let recovery_action = self.determine_recovery_action(&classification, context).await;

        // Execute recovery if needed
        match &recovery_action {
            RecoveryAction::Retry(strategy) => {
                tracing::info!("Initiating retry with strategy: {:?}", strategy);
            }
            RecoveryAction::CircuitBreaker(service) => {
                tracing::warn!("Activating circuit breaker for service: {}", service);
                self.activate_circuit_breaker(service).await;
            }
            RecoveryAction::Degrade(features) => {
                tracing::warn!("Degrading features: {:?}", features);
                for feature in features {
                    self.degradation_manager.degrade_feature(feature.clone()).await;
                }
            }
            RecoveryAction::Escalate => {
                tracing::error!("Escalating error for manual intervention");
            }
            RecoveryAction::Ignore => {
                tracing::debug!("Ignoring error as per strategy");
            }
        }

        Ok(recovery_action)
    }

    async fn update_metrics(&self, error: &dyn std::error::Error) {
        self.metrics.total_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let classification = self.classifier.classify_detailed(error);

        // Update category metrics
        {
            let mut category_metrics = self.metrics.errors_by_category.write().unwrap();
            *category_metrics.entry(classification.category).or_insert(0) += 1;
        }

        // Update severity metrics
        {
            let mut severity_metrics = self.metrics.errors_by_severity.write().unwrap();
            *severity_metrics.entry(classification.severity).or_insert(0) += 1;
        }
    }

    async fn determine_recovery_action(&self, classification: &DetailedClassification, context: &str) -> RecoveryAction {
        match (&classification.category, classification.severity) {
            (ErrorCategory::Network, ErrorSeverity::Medium | ErrorSeverity::Low) => {
                RecoveryAction::Retry(classification.recommended_strategy.clone())
            }
            (ErrorCategory::ExternalService, _) => {
                RecoveryAction::CircuitBreaker(context.to_string())
            }
            (ErrorCategory::Resource, ErrorSeverity::High | ErrorSeverity::Critical) => {
                RecoveryAction::Degrade(vec!["non_essential_features".to_string()])
            }
            (ErrorCategory::Authentication | ErrorCategory::Configuration, _) => {
                RecoveryAction::Escalate
            }
            (_, ErrorSeverity::Critical) => {
                RecoveryAction::Escalate
            }
            (_, ErrorSeverity::Low) => {
                RecoveryAction::Ignore
            }
            _ => {
                RecoveryAction::Retry(classification.recommended_strategy.clone())
            }
        }
    }

    async fn activate_circuit_breaker(&mut self, service: &str) {
        if !self.circuit_breakers.contains_key(service) {
            let circuit_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(60)));
            self.circuit_breakers.insert(service.to_string(), circuit_breaker);
        }

        self.metrics.circuit_breaker_trips.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get circuit breaker for a service
    pub fn get_circuit_breaker(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        self.circuit_breakers.get(service).cloned()
    }

    /// Get current error metrics
    pub fn get_metrics(&self) -> &ErrorMetrics {
        &self.metrics
    }

    /// Get degradation manager
    pub fn get_degradation_manager(&self) -> Arc<DegradationManager> {
        self.degradation_manager.clone()
    }
}

#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry with specified strategy
    Retry(RetryStrategy),
    /// Activate circuit breaker for service
    CircuitBreaker(String),
    /// Degrade specified features
    Degrade(Vec<String>),
    /// Escalate for manual intervention
    Escalate,
    /// Ignore the error
    Ignore,
}

impl Default for ErrorHandlingStrategyManager {
    fn default() -> Self {
        Self::new()
    }
}

// Include tests module
#[cfg(test)]
mod tests;