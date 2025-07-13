use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::agent_id::AgentId;
use crate::routing::{
    RoutingEngine, LoadBalancingStrategy, MessageEnvelopeInfo, RoutingDecision,
    PriorityRoutingRule, LoadAwareRoutingRule, MessagePriority, AgentLoad
};
use crate::load_monitor::{LoadMonitor, LoadMonitorConfig, SystemStatistics};

/// Intelligent load balancer that combines routing rules with load monitoring
pub struct IntelligentLoadBalancer {
    /// Routing engine for rule-based routing
    routing_engine: Arc<Mutex<RoutingEngine>>,
    /// Load monitor for tracking agent performance
    load_monitor: Arc<LoadMonitor>,
    /// Configuration
    config: LoadBalancerConfig,
    /// Adaptive strategy state
    adaptive_state: Arc<Mutex<AdaptiveState>>,
}

/// Configuration for the intelligent load balancer
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Base load balancing strategy
    pub base_strategy: LoadBalancingStrategy,
    /// Enable adaptive strategy switching
    pub enable_adaptive: bool,
    /// Threshold for switching to emergency mode
    pub emergency_load_threshold: f64,
    /// Minimum time between strategy changes
    pub strategy_change_cooldown: Duration,
    /// Enable circuit breaker for overloaded agents
    pub enable_circuit_breaker: bool,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: f64,
    /// Circuit breaker recovery time
    pub circuit_breaker_recovery_time: Duration,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            base_strategy: LoadBalancingStrategy::LeastLoaded,
            enable_adaptive: true,
            emergency_load_threshold: 0.8,
            strategy_change_cooldown: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 0.9,
            circuit_breaker_recovery_time: Duration::from_secs(60),
        }
    }
}

/// Adaptive strategy state
#[derive(Debug)]
struct AdaptiveState {
    /// Current strategy
    current_strategy: LoadBalancingStrategy,
    /// Last strategy change time
    last_strategy_change: Instant,
    /// Circuit breaker states for agents
    circuit_breakers: HashMap<AgentId, CircuitBreakerState>,
    /// Performance history for strategy evaluation
    strategy_performance: HashMap<LoadBalancingStrategy, StrategyPerformance>,
}

/// Circuit breaker state for an agent
#[derive(Debug, Clone)]
struct CircuitBreakerState {
    /// Whether the circuit is open (agent is blocked)
    is_open: bool,
    /// When the circuit was opened
    opened_at: Option<Instant>,
    /// Number of consecutive failures
    failure_count: u32,
    /// Last failure time
    last_failure: Option<Instant>,
}

/// Performance metrics for a load balancing strategy
#[derive(Debug, Clone)]
struct StrategyPerformance {
    /// Total decisions made
    total_decisions: u64,
    /// Successful routings
    successful_routings: u64,
    /// Average response time
    avg_response_time: Duration,
    /// Last evaluation time
    last_evaluation: Instant,
}

impl IntelligentLoadBalancer {
    /// Create a new intelligent load balancer
    pub fn new(config: LoadBalancerConfig) -> Self {
        let routing_engine = Arc::new(Mutex::new(RoutingEngine::new(config.base_strategy)));
        let load_monitor = Arc::new(LoadMonitor::new(LoadMonitorConfig::default()));
        
        // Add default routing rules
        {
            let mut engine = routing_engine.lock().unwrap();
            engine.add_rule(Box::new(LoadAwareRoutingRule::new(0.9))); // Don't route to heavily loaded agents
            engine.add_rule(Box::new(PriorityRoutingRule::new(MessagePriority::Low))); // Accept all priorities
        }

        let base_strategy = config.base_strategy;

        Self {
            routing_engine,
            load_monitor,
            config,
            adaptive_state: Arc::new(Mutex::new(AdaptiveState {
                current_strategy: base_strategy,
                last_strategy_change: Instant::now(),
                circuit_breakers: HashMap::new(),
                strategy_performance: HashMap::new(),
            })),
        }
    }

    /// Start the load balancer (starts monitoring)
    pub async fn start(&self) {
        self.load_monitor.start().await;
    }

    /// Stop the load balancer
    pub fn stop(&self) {
        self.load_monitor.stop();
    }

    /// Register an agent for load balancing
    pub fn register_agent(&self, agent_id: AgentId) {
        self.load_monitor.register_agent(agent_id.clone());
        
        // Initialize circuit breaker state
        let mut state = self.adaptive_state.lock().unwrap();
        state.circuit_breakers.insert(agent_id, CircuitBreakerState {
            is_open: false,
            opened_at: None,
            failure_count: 0,
            last_failure: None,
        });
    }

    /// Unregister an agent
    pub fn unregister_agent(&self, agent_id: &AgentId) {
        self.load_monitor.unregister_agent(agent_id);
        
        let mut state = self.adaptive_state.lock().unwrap();
        state.circuit_breakers.remove(agent_id);
    }

    /// Select the best agent for a message
    pub fn select_agent(
        &self,
        envelope: &dyn MessageEnvelopeInfo,
        candidates: &[AgentId],
    ) -> RoutingDecision {
        // Update adaptive strategy if needed
        if self.config.enable_adaptive {
            self.update_adaptive_strategy();
        }

        // Filter out agents with open circuit breakers
        let available_candidates = if self.config.enable_circuit_breaker {
            self.filter_circuit_breaker_candidates(candidates)
        } else {
            candidates.to_vec()
        };

        if available_candidates.is_empty() {
            return RoutingDecision {
                selected_agent: None,
                evaluated_candidates: Vec::new(),
                evaluation_time: Duration::from_millis(0),
                rules_applied: 0,
            };
        }

        // Update routing engine with current loads
        self.update_routing_engine_loads();

        // Use routing engine to make decision
        let mut engine = self.routing_engine.lock().unwrap();
        let decision = engine.evaluate_routing(envelope, &available_candidates);

        // Update circuit breaker and performance metrics
        if let Some(ref selected_agent) = decision.selected_agent {
            self.record_routing_decision(selected_agent, true);
        }

        decision
    }

    /// Record message processing completion
    pub fn record_message_processed(&self, agent_id: &AgentId, processing_time: Duration, success: bool) {
        self.load_monitor.record_message_processed(agent_id, processing_time);
        
        if !success {
            self.record_routing_decision(agent_id, false);
        }
    }

    /// Update queue size for an agent
    pub fn update_queue_size(&self, agent_id: &AgentId, queue_size: usize) {
        self.load_monitor.update_queue_size(agent_id, queue_size);
    }

    /// Get system statistics
    pub fn get_system_statistics(&self) -> SystemStatistics {
        self.load_monitor.get_system_statistics()
    }

    /// Get current load for an agent
    pub fn get_agent_load(&self, agent_id: &AgentId) -> Option<AgentLoad> {
        self.load_monitor.get_agent_load(agent_id)
    }

    /// Update adaptive strategy based on current system state
    fn update_adaptive_strategy(&self) {
        let mut state = self.adaptive_state.lock().unwrap();
        let now = Instant::now();
        
        // Check cooldown period
        if now.duration_since(state.last_strategy_change) < self.config.strategy_change_cooldown {
            return;
        }

        let system_stats = self.load_monitor.get_system_statistics();
        
        // Determine if we need to change strategy
        let new_strategy = if system_stats.is_under_stress() {
            // Under stress: prioritize load balancing
            LoadBalancingStrategy::LeastLoaded
        } else if system_stats.average_load < 0.3 {
            // Low load: can use round-robin for simplicity
            LoadBalancingStrategy::RoundRobin
        } else {
            // Normal load: use weighted approach
            LoadBalancingStrategy::WeightedRoundRobin
        };

        if new_strategy != state.current_strategy {
            tracing::info!(
                "Switching load balancing strategy from {:?} to {:?} (system load: {:.2})",
                state.current_strategy,
                new_strategy,
                system_stats.average_load
            );
            
            state.current_strategy = new_strategy;
            state.last_strategy_change = now;
            
            // Update routing engine strategy
            drop(state); // Release lock before acquiring routing engine lock
            let mut engine = self.routing_engine.lock().unwrap();
            *engine.load_balancer_mut() = crate::routing::LoadBalancer::new(new_strategy);
        }
    }

    /// Filter candidates based on circuit breaker states
    fn filter_circuit_breaker_candidates(&self, candidates: &[AgentId]) -> Vec<AgentId> {
        let mut state = self.adaptive_state.lock().unwrap();
        let now = Instant::now();
        
        candidates.iter()
            .filter(|agent_id| {
                if let Some(cb_state) = state.circuit_breakers.get_mut(agent_id) {
                    if cb_state.is_open {
                        // Check if circuit breaker should be closed (recovery time passed)
                        if let Some(opened_at) = cb_state.opened_at {
                            if now.duration_since(opened_at) > self.config.circuit_breaker_recovery_time {
                                cb_state.is_open = false;
                                cb_state.opened_at = None;
                                cb_state.failure_count = 0;
                                tracing::info!("Circuit breaker closed for agent: {}", agent_id);
                                return true;
                            }
                        }
                        false // Circuit is still open
                    } else {
                        true // Circuit is closed
                    }
                } else {
                    true // No circuit breaker state (shouldn't happen)
                }
            })
            .cloned()
            .collect()
    }

    /// Update routing engine with current agent loads
    fn update_routing_engine_loads(&self) {
        let loads = self.load_monitor.get_all_loads();
        let mut engine = self.routing_engine.lock().unwrap();
        
        for (agent_id, load) in loads {
            engine.update_agent_load(agent_id, load);
        }
    }

    /// Record a routing decision for circuit breaker and performance tracking
    fn record_routing_decision(&self, agent_id: &AgentId, success: bool) {
        let mut state = self.adaptive_state.lock().unwrap();
        let now = Instant::now();
        
        if let Some(cb_state) = state.circuit_breakers.get_mut(agent_id) {
            if success {
                // Reset failure count on success
                cb_state.failure_count = 0;
                cb_state.last_failure = None;
            } else {
                // Increment failure count
                cb_state.failure_count += 1;
                cb_state.last_failure = Some(now);
                
                // Check if we should open the circuit breaker
                if !cb_state.is_open {
                    let agent_load = self.load_monitor.get_agent_load(agent_id)
                        .map(|load| load.load_score())
                        .unwrap_or(0.0);
                    
                    if agent_load > self.config.circuit_breaker_threshold || cb_state.failure_count >= 5 {
                        cb_state.is_open = true;
                        cb_state.opened_at = Some(now);
                        tracing::warn!(
                            "Circuit breaker opened for agent: {} (load: {:.2}, failures: {})",
                            agent_id,
                            agent_load,
                            cb_state.failure_count
                        );
                    }
                }
            }
        }
    }

    /// Get circuit breaker status for all agents
    pub fn get_circuit_breaker_status(&self) -> HashMap<AgentId, bool> {
        let state = self.adaptive_state.lock().unwrap();
        state.circuit_breakers.iter()
            .map(|(agent_id, cb_state)| (agent_id.clone(), cb_state.is_open))
            .collect()
    }

    /// Manually open circuit breaker for an agent
    pub fn open_circuit_breaker(&self, agent_id: &AgentId) {
        let mut state = self.adaptive_state.lock().unwrap();
        if let Some(cb_state) = state.circuit_breakers.get_mut(agent_id) {
            cb_state.is_open = true;
            cb_state.opened_at = Some(Instant::now());
            tracing::info!("Manually opened circuit breaker for agent: {}", agent_id);
        }
    }

    /// Manually close circuit breaker for an agent
    pub fn close_circuit_breaker(&self, agent_id: &AgentId) {
        let mut state = self.adaptive_state.lock().unwrap();
        if let Some(cb_state) = state.circuit_breakers.get_mut(agent_id) {
            cb_state.is_open = false;
            cb_state.opened_at = None;
            cb_state.failure_count = 0;
            tracing::info!("Manually closed circuit breaker for agent: {}", agent_id);
        }
    }
}
