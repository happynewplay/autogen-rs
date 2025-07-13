use autogen_core::routing_benchmarks::{RoutingBenchmarks, BenchmarkConfig, print_benchmark_results};
use std::time::Duration;

#[tokio::test]
async fn test_routing_performance_small_scale() {
    let config = BenchmarkConfig {
        agent_count: 5,
        message_count: 100,
        sustained_load_duration: Duration::from_secs(5),
        concurrent_senders: 2,
        priority_distribution: [0.3, 0.4, 0.2, 0.1], // More low priority for testing
    };

    let benchmarks = RoutingBenchmarks::new(config);
    let results = benchmarks.run_all_benchmarks().await;

    // Print results for manual inspection
    print_benchmark_results(&results);

    // Basic assertions
    assert!(results.throughput_test.messages_per_second > 0.0);
    assert!(results.throughput_test.successful_routings > 0);
    assert!(results.latency_test.average_latency < Duration::from_millis(100));
    assert!(results.load_balancing_test.load_distribution_variance < 1000.0);
}

#[tokio::test]
async fn test_routing_performance_medium_scale() {
    let config = BenchmarkConfig {
        agent_count: 10,
        message_count: 500,
        sustained_load_duration: Duration::from_secs(10),
        concurrent_senders: 3,
        priority_distribution: [0.2, 0.5, 0.25, 0.05], // Default distribution
    };

    let benchmarks = RoutingBenchmarks::new(config);
    let results = benchmarks.run_all_benchmarks().await;

    // Print results for manual inspection
    print_benchmark_results(&results);

    // Performance assertions
    assert!(results.throughput_test.messages_per_second > 10.0, 
        "Throughput should be at least 10 msg/s, got {}", results.throughput_test.messages_per_second);
    
    assert!(results.latency_test.average_latency < Duration::from_millis(50),
        "Average latency should be under 50ms, got {}ms", results.latency_test.average_latency.as_millis());
    
    assert!(results.latency_test.p99_latency < Duration::from_millis(200),
        "P99 latency should be under 200ms, got {}ms", results.latency_test.p99_latency.as_millis());

    // Load balancing assertions
    assert!(results.load_balancing_test.overloaded_agents <= 2,
        "Should have at most 2 overloaded agents, got {}", results.load_balancing_test.overloaded_agents);

    // Priority handling assertions
    assert!(results.priority_test.critical_message_latency < Duration::from_millis(20),
        "Critical messages should be processed quickly, got {}ms", 
        results.priority_test.critical_message_latency.as_millis());
    
    assert!(results.priority_test.low_priority_starvation_rate < 0.5,
        "Low priority starvation rate should be under 50%, got {:.2}%", 
        results.priority_test.low_priority_starvation_rate * 100.0);
}

#[tokio::test]
async fn test_routing_performance_high_load() {
    let config = BenchmarkConfig {
        agent_count: 20,
        message_count: 2000,
        sustained_load_duration: Duration::from_secs(20),
        concurrent_senders: 5,
        priority_distribution: [0.1, 0.6, 0.25, 0.05], // More normal priority messages
    };

    let benchmarks = RoutingBenchmarks::new(config);
    let results = benchmarks.run_all_benchmarks().await;

    // Print results for manual inspection
    print_benchmark_results(&results);

    // High-load performance assertions
    assert!(results.throughput_test.messages_per_second > 50.0,
        "High-load throughput should be at least 50 msg/s, got {}", results.throughput_test.messages_per_second);
    
    let success_rate = results.throughput_test.successful_routings as f64 / results.throughput_test.total_messages as f64;
    assert!(success_rate > 0.95,
        "Success rate should be above 95%, got {:.2}%", success_rate * 100.0);

    // Latency should still be reasonable under high load
    assert!(results.latency_test.average_latency < Duration::from_millis(100),
        "Average latency under high load should be under 100ms, got {}ms", 
        results.latency_test.average_latency.as_millis());

    // Load distribution should be reasonably balanced
    assert!(results.load_balancing_test.load_distribution_variance < 5000.0,
        "Load distribution variance should be reasonable, got {:.2}", 
        results.load_balancing_test.load_distribution_variance);
}

#[tokio::test]
async fn test_priority_message_handling() {
    let config = BenchmarkConfig {
        agent_count: 8,
        message_count: 400,
        sustained_load_duration: Duration::from_secs(8),
        concurrent_senders: 3,
        priority_distribution: [0.1, 0.3, 0.4, 0.2], // More high priority messages
    };

    let benchmarks = RoutingBenchmarks::new(config);
    let results = benchmarks.run_all_benchmarks().await;

    // Print results for manual inspection
    print_benchmark_results(&results);

    // Priority-specific assertions
    assert!(results.priority_test.priority_ordering_accuracy > 0.9,
        "Priority ordering accuracy should be above 90%, got {:.2}%", 
        results.priority_test.priority_ordering_accuracy * 100.0);

    assert!(results.priority_test.critical_message_latency < Duration::from_millis(15),
        "Critical messages should have very low latency, got {}ms", 
        results.priority_test.critical_message_latency.as_millis());

    // Ensure low priority messages aren't completely starved
    assert!(results.priority_test.low_priority_starvation_rate < 0.3,
        "Low priority starvation should be under 30%, got {:.2}%", 
        results.priority_test.low_priority_starvation_rate * 100.0);
}

#[tokio::test]
async fn test_load_balancing_effectiveness() {
    let config = BenchmarkConfig {
        agent_count: 15,
        message_count: 1500,
        sustained_load_duration: Duration::from_secs(15),
        concurrent_senders: 4,
        priority_distribution: [0.2, 0.5, 0.25, 0.05], // Standard distribution
    };

    let expected_messages_per_agent = config.message_count as f64 / config.agent_count as f64;
    let agent_count = config.agent_count;

    let benchmarks = RoutingBenchmarks::new(config);
    let results = benchmarks.run_all_benchmarks().await;

    // Print results for manual inspection
    print_benchmark_results(&results);
    let max_acceptable_variance = expected_messages_per_agent * expected_messages_per_agent * 0.5; // 50% variance tolerance

    assert!(results.load_balancing_test.load_distribution_variance < max_acceptable_variance,
        "Load distribution variance should be under {:.2}, got {:.2}", 
        max_acceptable_variance, results.load_balancing_test.load_distribution_variance);

    // Check that no single agent is handling too much load
    let max_utilization = results.load_balancing_test.agent_utilization.values()
        .fold(0.0f64, |max, &util| max.max(util));
    
    assert!(max_utilization < 0.3, // No agent should handle more than 30% of total load
        "Maximum agent utilization should be under 30%, got {:.2}%", max_utilization * 100.0);

    // Ensure we don't have too many overloaded agents
    let overload_percentage = results.load_balancing_test.overloaded_agents as f64 / agent_count as f64;
    assert!(overload_percentage < 0.2,
        "Overloaded agents should be under 20%, got {:.2}%", overload_percentage * 100.0);
}

#[tokio::test]
async fn test_routing_consistency() {
    // Run the same test multiple times to ensure consistent performance
    let config = BenchmarkConfig {
        agent_count: 6,
        message_count: 300,
        sustained_load_duration: Duration::from_secs(6),
        concurrent_senders: 2,
        priority_distribution: [0.25, 0.5, 0.2, 0.05],
    };

    let mut throughputs = Vec::new();
    let mut latencies = Vec::new();

    for _ in 0..3 {
        let benchmarks = RoutingBenchmarks::new(config.clone());
        let results = benchmarks.run_all_benchmarks().await;
        
        throughputs.push(results.throughput_test.messages_per_second);
        latencies.push(results.latency_test.average_latency.as_millis());
    }

    // Calculate coefficient of variation for throughput
    let avg_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
    let throughput_variance = throughputs.iter()
        .map(|&t| (t - avg_throughput).powi(2))
        .sum::<f64>() / throughputs.len() as f64;
    let throughput_cv = throughput_variance.sqrt() / avg_throughput;

    assert!(throughput_cv < 0.2, // Coefficient of variation should be under 20%
        "Throughput should be consistent across runs, CV: {:.3}", throughput_cv);

    // Calculate coefficient of variation for latency
    let avg_latency = latencies.iter().sum::<u128>() as f64 / latencies.len() as f64;
    let latency_variance = latencies.iter()
        .map(|&l| (l as f64 - avg_latency).powi(2))
        .sum::<f64>() / latencies.len() as f64;
    let latency_cv = latency_variance.sqrt() / avg_latency;

    assert!(latency_cv < 0.3, // Latency CV should be under 30%
        "Latency should be consistent across runs, CV: {:.3}", latency_cv);

    println!("Consistency test passed - Throughput CV: {:.3}, Latency CV: {:.3}", throughput_cv, latency_cv);
}
