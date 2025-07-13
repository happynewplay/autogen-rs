#[cfg(test)]
mod tests {
    use super::*;
    use crate::exceptions::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[derive(Debug)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test error: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    impl From<TestError> for AutogenError {
        fn from(err: TestError) -> Self {
            AutogenError::General {
                message: err.0,
                context: ErrorContext::default(),
            }
        }
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Critical > ErrorSeverity::High);
        assert!(ErrorSeverity::High > ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium > ErrorSeverity::Low);
    }

    #[test]
    fn test_retry_strategy_delay_calculation() {
        let strategy = RetryStrategy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for predictable testing
            exponential_backoff: true,
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(0));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(400));
    }

    #[test]
    fn test_retry_strategy_max_delay() {
        let strategy = RetryStrategy {
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
            exponential_backoff: true,
        };

        // Should be capped at max_delay
        assert_eq!(strategy.calculate_delay(10), Duration::from_millis(500));
    }

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext {
            severity: ErrorSeverity::High,
            category: ErrorCategory::Network,
            recoverability: RecoverabilityType::Recoverable,
            operation: "test_operation".to_string(),
            component: "test_component".to_string(),
            ..Default::default()
        };

        assert_eq!(context.severity, ErrorSeverity::High);
        assert_eq!(context.category, ErrorCategory::Network);
        assert_eq!(context.recoverability, RecoverabilityType::Recoverable);
    }

    #[test]
    fn test_autogen_error_creation() {
        let error = AutogenError::Network {
            message: "Connection timeout".to_string(),
            context: ErrorContext {
                severity: ErrorSeverity::Medium,
                category: ErrorCategory::Network,
                recoverability: RecoverabilityType::Recoverable,
                ..Default::default()
            },
        };

        assert!(error.is_retryable());
        assert_eq!(error.context().category, ErrorCategory::Network);
    }

    #[test]
    fn test_error_context_modification() {
        let mut error = AutogenError::General {
            message: "Test error".to_string(),
            context: ErrorContext::default(),
        };

        error = error.with_metadata("key".to_string(), serde_json::Value::String("value".to_string()));
        error = error.with_correlation_id("test-correlation-id".to_string());

        assert_eq!(
            error.context().metadata.get("key"),
            Some(&serde_json::Value::String("value".to_string()))
        );
        assert_eq!(
            error.context().correlation_id,
            Some("test-correlation-id".to_string())
        );
    }

    #[tokio::test]
    async fn test_retry_executor_success() {
        let executor = RetryExecutor::with_defaults();
        let mut attempt_count = 0;

        let result = executor
            .execute(|| {
                attempt_count += 1;
                async move {
                    if attempt_count < 3 {
                        Err(TestError("Temporary failure".to_string()))
                    } else {
                        Ok("Success".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count, 3);
    }

    #[tokio::test]
    async fn test_retry_executor_exhaustion() {
        let executor = RetryExecutor::new(RetryStrategy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_multiplier: 1.0,
            jitter_factor: 0.0,
            exponential_backoff: false,
        });

        let result: Result<String, _> = executor
            .execute(|| async { Err(TestError("Persistent failure".to_string())) })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AutogenError::RetryExhausted { .. } => {
                // Expected
            }
            _ => panic!("Expected RetryExhausted error"),
        }
    }

    #[test]
    fn test_error_classifier() {
        let classifier = ErrorClassifier;

        // Test network error classification
        let network_error = TestError("Connection timeout occurred".to_string());
        let context = ErrorClassifier::classify_error(&network_error);
        assert_eq!(context.category, ErrorCategory::Network);
        assert_eq!(context.recoverability, RecoverabilityType::Recoverable);

        // Test authentication error classification
        let auth_error = TestError("Unauthorized access denied".to_string());
        let context = ErrorClassifier::classify_error(&auth_error);
        assert_eq!(context.category, ErrorCategory::Authentication);
        assert_eq!(context.recoverability, RecoverabilityType::ManualIntervention);
    }

    #[test]
    fn test_advanced_error_classifier() {
        let classifier = AdvancedErrorClassifier::new();

        // Test detailed classification
        let error = TestError("API rate limit exceeded".to_string());
        let classification = classifier.classify_detailed(&error);

        assert_eq!(classification.category, ErrorCategory::ExternalService);
        assert!(classification.confidence > 0.8);
        assert!(!classification.tags.is_empty());
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let circuit_breaker = CircuitBreaker::new(2, Duration::from_millis(100));
        let mut failure_count = 0;

        // First failure
        let result: Result<String, _> = circuit_breaker
            .execute(|| {
                failure_count += 1;
                async { Err(TestError("Service failure".to_string())) }
            })
            .await;
        assert!(result.is_err());

        // Second failure - should trip the circuit breaker
        let result: Result<String, _> = circuit_breaker
            .execute(|| {
                failure_count += 1;
                async { Err(TestError("Service failure".to_string())) }
            })
            .await;
        assert!(result.is_err());

        // Third attempt - circuit breaker should be open
        let result: Result<String, _> = circuit_breaker
            .execute(|| {
                failure_count += 1;
                async { Ok::<String, TestError>("Success".to_string()) }
            })
            .await;
        assert!(result.is_err());
        assert_eq!(failure_count, 2); // Third operation shouldn't execute
    }

    #[tokio::test]
    async fn test_degradation_manager() {
        let manager = DegradationManager::new();

        assert!(!manager.is_degraded("feature1"));

        manager.degrade_feature("feature1".to_string()).await;
        assert!(manager.is_degraded("feature1"));

        let degraded_features = manager.get_degraded_features();
        assert!(degraded_features.contains(&"feature1".to_string()));

        manager.restore_feature("feature1").await;
        assert!(!manager.is_degraded("feature1"));
    }

    #[test]
    fn test_error_chain() {
        let mut chain = ErrorChain::new("test-correlation-id".to_string());
        
        let error1 = AutogenError::Network {
            message: "Network error".to_string(),
            context: ErrorContext::default(),
        };
        
        let error2 = AutogenError::General {
            message: "General error".to_string(),
            context: ErrorContext::default(),
        };

        chain.add_error(&error1, None);
        chain.add_error(&error2, None);

        assert_eq!(chain.depth(), 2);
        assert!(chain.root_cause().is_some());
        assert!(chain.latest_error().is_some());

        let formatted = chain.format_chain();
        assert!(formatted.contains("test-correlation-id"));
        assert!(formatted.contains("Network error"));
        assert!(formatted.contains("General error"));
    }

    #[tokio::test]
    async fn test_error_handling_strategy_manager() {
        let mut manager = ErrorHandlingStrategyManager::new();
        
        let network_error = TestError("Connection timeout".to_string());
        let action = manager.handle_error(&network_error, "test_service").await.unwrap();

        match action {
            RecoveryAction::Retry(_) => {
                // Expected for network errors
            }
            _ => panic!("Expected retry action for network error"),
        }

        // Check metrics were updated
        let metrics = manager.get_metrics();
        assert_eq!(metrics.total_errors.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that old exception types still work
        let old_error = CantHandleException("Test message".to_string());
        let new_error: AutogenError = old_error.into();

        match new_error {
            AutogenError::CantHandle { message, .. } => {
                assert_eq!(message, "Test message");
            }
            _ => panic!("Expected CantHandle variant"),
        }
    }

    #[test]
    fn test_utility_functions() {
        let network_err = utils::network_error("Network failure");
        assert_eq!(network_err.context().category, ErrorCategory::Network);

        let auth_err = utils::auth_error("Authentication failed");
        assert_eq!(auth_err.context().category, ErrorCategory::Authentication);

        let config_err = utils::config_error("Invalid configuration");
        assert_eq!(config_err.context().category, ErrorCategory::Configuration);
    }
}
