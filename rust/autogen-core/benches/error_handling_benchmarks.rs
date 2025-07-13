use criterion::{black_box, criterion_group, criterion_main, Criterion};
use autogen_core::exceptions::*;
use std::time::Duration;

fn benchmark_error_classification(c: &mut Criterion) {
    let classifier = AdvancedErrorClassifier::new();
    
    #[derive(Debug)]
    struct BenchError(String);
    
    impl std::fmt::Display for BenchError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    
    impl std::error::Error for BenchError {}
    
    let errors = vec![
        BenchError("Connection timeout occurred".to_string()),
        BenchError("Authentication failed - invalid credentials".to_string()),
        BenchError("Memory allocation failed - out of memory".to_string()),
        BenchError("Configuration error - invalid parameter".to_string()),
        BenchError("External API service unavailable".to_string()),
    ];
    
    c.bench_function("error_classification", |b| {
        b.iter(|| {
            for error in &errors {
                black_box(classifier.classify_detailed(error));
            }
        })
    });
}

fn benchmark_retry_strategy_calculation(c: &mut Criterion) {
    let strategy = RetryStrategy {
        max_attempts: 10,
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        exponential_backoff: true,
    };
    
    c.bench_function("retry_delay_calculation", |b| {
        b.iter(|| {
            for attempt in 0..10 {
                black_box(strategy.calculate_delay(black_box(attempt)));
            }
        })
    });
}

fn benchmark_error_context_creation(c: &mut Criterion) {
    c.bench_function("error_context_creation", |b| {
        b.iter(|| {
            black_box(ErrorContext {
                severity: ErrorSeverity::High,
                category: ErrorCategory::Network,
                recoverability: RecoverabilityType::Recoverable,
                timestamp: chrono::Utc::now(),
                operation: "benchmark_operation".to_string(),
                component: "benchmark_component".to_string(),
                metadata: std::collections::HashMap::new(),
                correlation_id: Some("bench-correlation-id".to_string()),
                retry_count: 0,
            })
        })
    });
}

fn benchmark_autogen_error_creation(c: &mut Criterion) {
    c.bench_function("autogen_error_creation", |b| {
        b.iter(|| {
            black_box(AutogenError::Network {
                message: "Benchmark network error".to_string(),
                context: ErrorContext {
                    severity: ErrorSeverity::Medium,
                    category: ErrorCategory::Network,
                    recoverability: RecoverabilityType::Recoverable,
                    timestamp: chrono::Utc::now(),
                    operation: "benchmark".to_string(),
                    component: "benchmark".to_string(),
                    metadata: std::collections::HashMap::new(),
                    correlation_id: None,
                    retry_count: 0,
                },
            })
        })
    });
}

fn benchmark_error_chain_operations(c: &mut Criterion) {
    c.bench_function("error_chain_creation", |b| {
        b.iter(|| {
            let mut chain = black_box(ErrorChain::new("bench-correlation".to_string()));
            
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
            
            black_box(chain.format_chain());
        })
    });
}

fn benchmark_circuit_breaker_state_check(c: &mut Criterion) {
    let circuit_breaker = CircuitBreaker::new(5, Duration::from_secs(60));
    
    c.bench_function("circuit_breaker_state_check", |b| {
        b.iter(|| {
            // This is a simplified benchmark since should_allow_request is private
            // In a real scenario, we'd benchmark the execute method
            black_box(&circuit_breaker);
        })
    });
}

fn benchmark_degradation_manager_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("degradation_manager_operations", |b| {
        b.iter(|| {
            rt.block_on(async {
                let manager = black_box(DegradationManager::new());
                
                manager.degrade_feature("feature1".to_string()).await;
                black_box(manager.is_degraded("feature1"));
                black_box(manager.get_degraded_features());
                manager.restore_feature("feature1").await;
            })
        })
    });
}

fn benchmark_error_conversion(c: &mut Criterion) {
    c.bench_function("error_conversion", |b| {
        b.iter(|| {
            let old_error = black_box(CantHandleException("Test error".to_string()));
            let new_error: AutogenError = black_box(old_error.into());
            black_box(new_error);
        })
    });
}

fn benchmark_utility_functions(c: &mut Criterion) {
    c.bench_function("utility_error_creation", |b| {
        b.iter(|| {
            black_box(utils::network_error("Network failure"));
            black_box(utils::auth_error("Auth failure"));
            black_box(utils::config_error("Config failure"));
            black_box(utils::resource_error("Resource failure"));
            black_box(utils::external_service_error("Service failure"));
        })
    });
}

fn benchmark_error_metadata_operations(c: &mut Criterion) {
    c.bench_function("error_metadata_operations", |b| {
        b.iter(|| {
            let error = black_box(utils::network_error("Test error"))
                .with_metadata("key1".to_string(), serde_json::Value::String("value1".to_string()))
                .with_metadata("key2".to_string(), serde_json::Value::Number(serde_json::Number::from(42)))
                .with_correlation_id("test-correlation".to_string());
            
            black_box(error.context());
        })
    });
}

fn benchmark_strategy_manager_error_handling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("strategy_manager_error_handling", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut manager = black_box(ErrorHandlingStrategyManager::new());
                
                #[derive(Debug)]
                struct BenchError(String);
                
                impl std::fmt::Display for BenchError {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{}", self.0)
                    }
                }
                
                impl std::error::Error for BenchError {}
                
                let error = BenchError("Benchmark error".to_string());
                let action = manager.handle_error(&error, "bench_service").await.unwrap();
                black_box(action);
            })
        })
    });
}

criterion_group!(
    benches,
    benchmark_error_classification,
    benchmark_retry_strategy_calculation,
    benchmark_error_context_creation,
    benchmark_autogen_error_creation,
    benchmark_error_chain_operations,
    benchmark_circuit_breaker_state_check,
    benchmark_degradation_manager_operations,
    benchmark_error_conversion,
    benchmark_utility_functions,
    benchmark_error_metadata_operations,
    benchmark_strategy_manager_error_handling
);

criterion_main!(benches);
