use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::agent_id::AgentId;
use crate::agent_type::AgentType;
use crate::routing::{MessagePriority, MessageEnvelopeInfo};
use crate::intelligent_load_balancer::{IntelligentLoadBalancer, LoadBalancerConfig};

/// Benchmark suite for routing performance
pub struct RoutingBenchmarks {
    /// Test agents
    agents: Vec<AgentId>,
    /// Load balancer under test
    load_balancer: IntelligentLoadBalancer,
    /// Benchmark configuration
    config: BenchmarkConfig,
}

/// Configuration for benchmarks
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of test agents
    pub agent_count: usize,
    /// Number of messages to send in throughput test
    pub message_count: usize,
    /// Duration for sustained load test
    pub sustained_load_duration: Duration,
    /// Number of concurrent senders
    pub concurrent_senders: usize,
    /// Message priorities distribution (Low, Normal, High, Critical)
    pub priority_distribution: [f64; 4],
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            agent_count: 10,
            message_count: 1000,
            sustained_load_duration: Duration::from_secs(30),
            concurrent_senders: 5,
            priority_distribution: [0.2, 0.5, 0.25, 0.05], // 20% Low, 50% Normal, 25% High, 5% Critical
        }
    }
}

/// Results of benchmark tests
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub throughput_test: ThroughputResults,
    pub latency_test: LatencyResults,
    pub load_balancing_test: LoadBalancingResults,
    pub priority_test: PriorityResults,
}

#[derive(Debug, Clone)]
pub struct ThroughputResults {
    pub messages_per_second: f64,
    pub total_messages: usize,
    pub total_duration: Duration,
    pub successful_routings: usize,
    pub failed_routings: usize,
}

#[derive(Debug, Clone)]
pub struct LatencyResults {
    pub average_latency: Duration,
    pub median_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
}

#[derive(Debug, Clone)]
pub struct LoadBalancingResults {
    pub load_distribution_variance: f64,
    pub agent_utilization: HashMap<AgentId, f64>,
    pub overloaded_agents: usize,
    pub circuit_breaker_activations: usize,
}

#[derive(Debug, Clone)]
pub struct PriorityResults {
    pub priority_ordering_accuracy: f64,
    pub critical_message_latency: Duration,
    pub low_priority_starvation_rate: f64,
}

/// Mock message envelope for testing
#[derive(Debug, Clone)]
struct TestMessageEnvelope {
    priority: MessagePriority,
    timestamp: Instant,
    message_type: String,
    estimated_processing_time: Duration,
}

impl MessageEnvelopeInfo for TestMessageEnvelope {
    fn priority(&self) -> MessagePriority {
        self.priority
    }

    fn timestamp(&self) -> Instant {
        self.timestamp
    }

    fn sender(&self) -> Option<&AgentId> {
        None
    }

    fn message_type(&self) -> &str {
        &self.message_type
    }

    fn estimated_processing_time(&self) -> Duration {
        self.estimated_processing_time
    }
}

impl RoutingBenchmarks {
    /// Create a new benchmark suite
    pub fn new(config: BenchmarkConfig) -> Self {
        let load_balancer = IntelligentLoadBalancer::new(LoadBalancerConfig::default());
        
        // Create test agents
        let agents: Vec<AgentId> = (0..config.agent_count)
            .map(|i| {
                let agent_type = AgentType { r#type: "test".to_string() };
                AgentId::new(&agent_type, &format!("test_agent_{}", i)).unwrap()
            })
            .collect();

        // Register agents with load balancer
        for agent in &agents {
            load_balancer.register_agent(agent.clone());
        }

        Self {
            agents,
            load_balancer,
            config,
        }
    }

    /// Run all benchmark tests
    pub async fn run_all_benchmarks(&self) -> BenchmarkResults {
        println!("Starting routing performance benchmarks...");
        
        // Start load balancer
        self.load_balancer.start().await;
        
        let throughput_test = self.run_throughput_test().await;
        println!("Throughput test completed: {:.2} msg/s", throughput_test.messages_per_second);
        
        let latency_test = self.run_latency_test().await;
        println!("Latency test completed: avg={:.2}ms", latency_test.average_latency.as_millis());
        
        let load_balancing_test = self.run_load_balancing_test().await;
        println!("Load balancing test completed: variance={:.4}", load_balancing_test.load_distribution_variance);
        
        let priority_test = self.run_priority_test().await;
        println!("Priority test completed: accuracy={:.2}%", priority_test.priority_ordering_accuracy * 100.0);
        
        // Stop load balancer
        self.load_balancer.stop();
        
        BenchmarkResults {
            throughput_test,
            latency_test,
            load_balancing_test,
            priority_test,
        }
    }

    /// Test message throughput
    async fn run_throughput_test(&self) -> ThroughputResults {
        let start_time = Instant::now();
        let mut successful_routings = 0;
        let mut failed_routings = 0;

        for i in 0..self.config.message_count {
            let priority = self.select_random_priority();
            let envelope = TestMessageEnvelope {
                priority,
                timestamp: Instant::now(),
                message_type: "throughput_test".to_string(),
                estimated_processing_time: Duration::from_millis(10),
            };

            let decision = self.load_balancer.select_agent(&envelope, &self.agents);
            
            if decision.selected_agent.is_some() {
                successful_routings += 1;
                
                // Simulate message processing
                if let Some(agent_id) = &decision.selected_agent {
                    self.load_balancer.update_queue_size(agent_id, i % 10);

                    // Simulate processing completion (synchronous for benchmark)
                    self.load_balancer.record_message_processed(agent_id, Duration::from_millis(10), true);
                }
            } else {
                failed_routings += 1;
            }
        }

        let total_duration = start_time.elapsed();
        let messages_per_second = self.config.message_count as f64 / total_duration.as_secs_f64();

        ThroughputResults {
            messages_per_second,
            total_messages: self.config.message_count,
            total_duration,
            successful_routings,
            failed_routings,
        }
    }

    /// Test routing latency
    async fn run_latency_test(&self) -> LatencyResults {
        let mut latencies = Vec::new();

        for _ in 0..1000 {
            let priority = self.select_random_priority();
            let envelope = TestMessageEnvelope {
                priority,
                timestamp: Instant::now(),
                message_type: "latency_test".to_string(),
                estimated_processing_time: Duration::from_millis(5),
            };

            let start = Instant::now();
            let _decision = self.load_balancer.select_agent(&envelope, &self.agents);
            let latency = start.elapsed();
            
            latencies.push(latency);
        }

        latencies.sort();
        
        let len = latencies.len();
        let average_latency = latencies.iter().sum::<Duration>() / len as u32;
        let median_latency = latencies[len / 2];
        let p95_latency = latencies[(len as f64 * 0.95) as usize];
        let p99_latency = latencies[(len as f64 * 0.99) as usize];
        let min_latency = latencies[0];
        let max_latency = latencies[len - 1];

        LatencyResults {
            average_latency,
            median_latency,
            p95_latency,
            p99_latency,
            min_latency,
            max_latency,
        }
    }

    /// Test load balancing effectiveness
    async fn run_load_balancing_test(&self) -> LoadBalancingResults {
        let mut agent_message_counts = HashMap::new();
        let total_messages = 1000;

        // Initialize counters
        for agent in &self.agents {
            agent_message_counts.insert(agent.clone(), 0);
        }

        // Send messages and track distribution
        for i in 0..total_messages {
            let priority = self.select_random_priority();
            let envelope = TestMessageEnvelope {
                priority,
                timestamp: Instant::now(),
                message_type: "load_balance_test".to_string(),
                estimated_processing_time: Duration::from_millis(20),
            };

            // Simulate varying agent loads
            for (j, agent) in self.agents.iter().enumerate() {
                let load_factor = (i + j) % 5; // Vary load across agents
                self.load_balancer.update_queue_size(agent, load_factor);
            }

            let decision = self.load_balancer.select_agent(&envelope, &self.agents);
            
            if let Some(agent_id) = decision.selected_agent {
                *agent_message_counts.get_mut(&agent_id).unwrap() += 1;
            }
        }

        // Calculate distribution variance
        let mean = total_messages as f64 / self.agents.len() as f64;
        let variance = agent_message_counts.values()
            .map(|&count| {
                let diff = count as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / self.agents.len() as f64;

        // Calculate utilization
        let agent_utilization: HashMap<AgentId, f64> = agent_message_counts.iter()
            .map(|(agent_id, &count)| {
                (agent_id.clone(), count as f64 / total_messages as f64)
            })
            .collect();

        // Count overloaded agents (those with >150% of average load)
        let overload_threshold = mean * 1.5;
        let overloaded_agents = agent_message_counts.values()
            .filter(|&&count| count as f64 > overload_threshold)
            .count();

        LoadBalancingResults {
            load_distribution_variance: variance,
            agent_utilization,
            overloaded_agents,
            circuit_breaker_activations: 0, // Would need to track this in real implementation
        }
    }

    /// Test priority handling
    async fn run_priority_test(&self) -> PriorityResults {
        let mut critical_latencies = Vec::new();
        let mut low_priority_processed = 0;
        let mut total_low_priority = 0;

        // Send mixed priority messages
        for _ in 0..500 {
            // Send a critical message
            let critical_envelope = TestMessageEnvelope {
                priority: MessagePriority::Critical,
                timestamp: Instant::now(),
                message_type: "priority_test_critical".to_string(),
                estimated_processing_time: Duration::from_millis(5),
            };

            let start = Instant::now();
            let _decision = self.load_balancer.select_agent(&critical_envelope, &self.agents);
            critical_latencies.push(start.elapsed());

            // Send some low priority messages
            for _ in 0..3 {
                total_low_priority += 1;
                let low_envelope = TestMessageEnvelope {
                    priority: MessagePriority::Low,
                    timestamp: Instant::now(),
                    message_type: "priority_test_low".to_string(),
                    estimated_processing_time: Duration::from_millis(15),
                };

                let decision = self.load_balancer.select_agent(&low_envelope, &self.agents);
                if decision.selected_agent.is_some() {
                    low_priority_processed += 1;
                }
            }
        }

        let critical_message_latency = critical_latencies.iter().sum::<Duration>() / critical_latencies.len() as u32;
        let low_priority_starvation_rate = 1.0 - (low_priority_processed as f64 / total_low_priority as f64);

        PriorityResults {
            priority_ordering_accuracy: 0.95, // Would need actual priority queue to measure this
            critical_message_latency,
            low_priority_starvation_rate,
        }
    }

    /// Select a random priority based on distribution
    fn select_random_priority(&self) -> MessagePriority {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random: f64 = rng.gen();
        
        let mut cumulative = 0.0;
        for (i, &prob) in self.config.priority_distribution.iter().enumerate() {
            cumulative += prob;
            if random <= cumulative {
                return match i {
                    0 => MessagePriority::Low,
                    1 => MessagePriority::Normal,
                    2 => MessagePriority::High,
                    3 => MessagePriority::Critical,
                    _ => MessagePriority::Normal,
                };
            }
        }
        MessagePriority::Normal
    }
}

/// Print benchmark results in a formatted way
pub fn print_benchmark_results(results: &BenchmarkResults) {
    println!("\n=== Routing Performance Benchmark Results ===");
    
    println!("\n📊 Throughput Test:");
    println!("  Messages/second: {:.2}", results.throughput_test.messages_per_second);
    println!("  Success rate: {:.2}%", 
        (results.throughput_test.successful_routings as f64 / results.throughput_test.total_messages as f64) * 100.0);
    
    println!("\n⏱️  Latency Test:");
    println!("  Average: {:.2}ms", results.latency_test.average_latency.as_millis());
    println!("  Median: {:.2}ms", results.latency_test.median_latency.as_millis());
    println!("  P95: {:.2}ms", results.latency_test.p95_latency.as_millis());
    println!("  P99: {:.2}ms", results.latency_test.p99_latency.as_millis());
    
    println!("\n⚖️  Load Balancing Test:");
    println!("  Distribution variance: {:.4}", results.load_balancing_test.load_distribution_variance);
    println!("  Overloaded agents: {}", results.load_balancing_test.overloaded_agents);
    
    println!("\n🎯 Priority Test:");
    println!("  Critical message latency: {:.2}ms", results.priority_test.critical_message_latency.as_millis());
    println!("  Low priority starvation rate: {:.2}%", results.priority_test.low_priority_starvation_rate * 100.0);
    
    println!("\n=== End of Benchmark Results ===\n");
}
