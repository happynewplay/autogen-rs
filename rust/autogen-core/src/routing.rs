use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::agent_id::AgentId;


/// Message priority levels for intelligent routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority messages (background tasks, cleanup, etc.)
    Low = 0,
    /// Normal priority messages (default for most operations)
    Normal = 1,
    /// High priority messages (user interactions, important notifications)
    High = 2,
    /// Critical priority messages (system alerts, emergency responses)
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

impl MessagePriority {
    /// Get the numeric value of the priority
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Create priority from numeric value
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(MessagePriority::Low),
            1 => Some(MessagePriority::Normal),
            2 => Some(MessagePriority::High),
            3 => Some(MessagePriority::Critical),
            _ => None,
        }
    }
}

/// Agent load information for load balancing
#[derive(Debug, Clone)]
pub struct AgentLoad {
    /// Current number of messages in the agent's queue
    pub queue_size: usize,
    /// Average message processing time
    pub processing_time_avg: Duration,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Number of messages processed in the last minute
    pub messages_per_minute: u32,
    /// Current CPU usage percentage (0-100)
    pub cpu_usage: f32,
}

impl Default for AgentLoad {
    fn default() -> Self {
        Self {
            queue_size: 0,
            processing_time_avg: Duration::from_millis(100),
            last_activity: Instant::now(),
            messages_per_minute: 0,
            cpu_usage: 0.0,
        }
    }
}

impl AgentLoad {
    /// Calculate the load score (0.0 = no load, 1.0 = maximum load)
    pub fn load_score(&self) -> f64 {
        let queue_factor = (self.queue_size as f64 / 100.0).min(1.0);
        let time_factor = (self.processing_time_avg.as_millis() as f64 / 1000.0).min(1.0);
        let cpu_factor = (self.cpu_usage as f64 / 100.0).min(1.0);
        let activity_factor = if self.last_activity.elapsed() > Duration::from_secs(60) {
            0.1 // Reduce load score for inactive agents
        } else {
            1.0
        };

        (queue_factor * 0.4 + time_factor * 0.3 + cpu_factor * 0.3) * activity_factor
    }

    /// Check if the agent is overloaded
    pub fn is_overloaded(&self) -> bool {
        self.load_score() > 0.8
    }

    /// Update load statistics after processing a message
    pub fn update_after_processing(&mut self, processing_time: Duration) {
        self.last_activity = Instant::now();
        
        // Update average processing time using exponential moving average
        let alpha = 0.1; // Smoothing factor
        let new_time_ms = processing_time.as_millis() as f64;
        let old_time_ms = self.processing_time_avg.as_millis() as f64;
        let updated_time_ms = alpha * new_time_ms + (1.0 - alpha) * old_time_ms;
        self.processing_time_avg = Duration::from_millis(updated_time_ms as u64);
    }
}

/// Load balancing strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Least loaded agent first
    LeastLoaded,
    /// Weighted round-robin based on agent capacity
    WeightedRoundRobin,
    /// Random selection
    Random,
}

/// Routing rule trait for implementing custom routing logic
pub trait RoutingRule: Send + Sync + std::fmt::Debug {
    /// Check if this rule should be applied to the given message and agent
    fn should_route(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> bool;
    
    /// Calculate routing score (higher score = better match)
    fn calculate_score(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> f64;
    
    /// Get rule name for debugging
    fn name(&self) -> &str;
}

/// Trait for accessing message envelope information without exposing internal structure
pub trait MessageEnvelopeInfo {
    fn priority(&self) -> MessagePriority;
    fn timestamp(&self) -> Instant;
    fn sender(&self) -> Option<&AgentId>;
    fn message_type(&self) -> &str;
    fn estimated_processing_time(&self) -> Duration;
}

/// Priority-based routing rule
#[derive(Debug)]
pub struct PriorityRoutingRule {
    /// Minimum priority level to consider
    pub min_priority: MessagePriority,
}

impl PriorityRoutingRule {
    pub fn new(min_priority: MessagePriority) -> Self {
        Self { min_priority }
    }
}

impl RoutingRule for PriorityRoutingRule {
    fn should_route(&self, envelope: &dyn MessageEnvelopeInfo, _agent_load: &AgentLoad) -> bool {
        envelope.priority() >= self.min_priority
    }

    fn calculate_score(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> f64 {
        let priority_score = envelope.priority().value() as f64 / 3.0; // Normalize to 0-1
        let load_penalty = agent_load.load_score(); // Higher load = lower score
        priority_score * (1.0 - load_penalty * 0.5) // Reduce score based on load
    }

    fn name(&self) -> &str {
        "PriorityRoutingRule"
    }
}

/// Load-aware routing rule
#[derive(Debug)]
pub struct LoadAwareRoutingRule {
    /// Maximum acceptable load score
    pub max_load_score: f64,
}

impl LoadAwareRoutingRule {
    pub fn new(max_load_score: f64) -> Self {
        Self { max_load_score }
    }
}

impl RoutingRule for LoadAwareRoutingRule {
    fn should_route(&self, _envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> bool {
        agent_load.load_score() <= self.max_load_score
    }

    fn calculate_score(&self, _envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> f64 {
        1.0 - agent_load.load_score() // Higher load = lower score
    }

    fn name(&self) -> &str {
        "LoadAwareRoutingRule"
    }
}

/// Load balancer for intelligent message distribution
#[derive(Debug)]
pub struct LoadBalancer {
    /// Agent load information
    agent_loads: HashMap<AgentId, AgentLoad>,
    /// Load balancing strategy
    strategy: LoadBalancingStrategy,
    /// Round-robin counter for round-robin strategy
    round_robin_counter: usize,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            agent_loads: HashMap::new(),
            strategy,
            round_robin_counter: 0,
        }
    }

    /// Update agent load information
    pub fn update_agent_load(&mut self, agent_id: AgentId, load: AgentLoad) {
        self.agent_loads.insert(agent_id, load);
    }

    /// Get agent load information
    pub fn get_agent_load(&self, agent_id: &AgentId) -> Option<&AgentLoad> {
        self.agent_loads.get(agent_id)
    }

    /// Select the best agent for message delivery based on the strategy
    pub fn select_agent(&mut self, candidates: &[AgentId]) -> Option<AgentId> {
        if candidates.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let selected = candidates[self.round_robin_counter % candidates.len()].clone();
                self.round_robin_counter += 1;
                Some(selected)
            }
            LoadBalancingStrategy::LeastLoaded => {
                candidates
                    .iter()
                    .min_by(|&a, &b| {
                        let load_a = self.agent_loads.get(a).map(|l| l.load_score()).unwrap_or(0.0);
                        let load_b = self.agent_loads.get(b).map(|l| l.load_score()).unwrap_or(0.0);
                        load_a.partial_cmp(&load_b).unwrap_or(Ordering::Equal)
                    })
                    .cloned()
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                // For now, fall back to least loaded
                // TODO: Implement proper weighted round-robin
                self.select_agent_least_loaded(candidates)
            }
            LoadBalancingStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                candidates.choose(&mut rng).cloned()
            }
        }
    }

    fn select_agent_least_loaded(&self, candidates: &[AgentId]) -> Option<AgentId> {
        candidates
            .iter()
            .min_by(|&a, &b| {
                let load_a = self.agent_loads.get(a).map(|l| l.load_score()).unwrap_or(0.0);
                let load_b = self.agent_loads.get(b).map(|l| l.load_score()).unwrap_or(0.0);
                load_a.partial_cmp(&load_b).unwrap_or(Ordering::Equal)
            })
            .cloned()
    }

    /// Remove agent from load tracking
    pub fn remove_agent(&mut self, agent_id: &AgentId) {
        self.agent_loads.remove(agent_id);
    }

    /// Get all tracked agents
    pub fn get_tracked_agents(&self) -> Vec<AgentId> {
        self.agent_loads.keys().cloned().collect()
    }
}

/// Routing engine for intelligent message distribution
pub struct RoutingEngine {
    /// Routing rules in priority order
    rules: Vec<Box<dyn RoutingRule>>,
    /// Load balancer for agent selection
    load_balancer: LoadBalancer,
    /// Routing metrics
    metrics: RoutingMetrics,
}

impl RoutingEngine {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            rules: Vec::new(),
            load_balancer: LoadBalancer::new(strategy),
            metrics: RoutingMetrics::default(),
        }
    }

    /// Add a routing rule
    pub fn add_rule(&mut self, rule: Box<dyn RoutingRule>) {
        self.rules.push(rule);
    }

    /// Remove all routing rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Evaluate routing for a message envelope and candidate agents
    pub fn evaluate_routing(
        &mut self,
        envelope: &dyn MessageEnvelopeInfo,
        candidates: &[AgentId],
    ) -> RoutingDecision {
        let start_time = std::time::Instant::now();

        // Filter candidates based on routing rules
        let mut scored_candidates = Vec::new();

        for agent_id in candidates {
            let agent_load = self.load_balancer.get_agent_load(agent_id)
                .cloned()
                .unwrap_or_default();

            // Check if all rules allow routing to this agent
            let mut total_score = 0.0;
            let mut rule_count = 0;
            let mut should_route = true;

            for rule in &self.rules {
                if !rule.should_route(envelope, &agent_load) {
                    should_route = false;
                    break;
                }
                total_score += rule.calculate_score(envelope, &agent_load);
                rule_count += 1;
            }

            if should_route {
                let average_score = if rule_count > 0 {
                    total_score / rule_count as f64
                } else {
                    1.0 // Default score if no rules
                };

                scored_candidates.push(ScoredCandidate {
                    agent_id: agent_id.clone(),
                    score: average_score,
                    load: agent_load,
                });
            }
        }

        // Sort by score (highest first)
        scored_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Select the best agent using load balancer
        let selected_agent = if scored_candidates.is_empty() {
            // Fallback to load balancer if no candidates pass rules
            self.load_balancer.select_agent(candidates)
        } else {
            // Use the highest scored candidates for load balancing
            let top_candidates: Vec<AgentId> = scored_candidates
                .iter()
                .take(3) // Consider top 3 candidates for load balancing
                .map(|c| c.agent_id.clone())
                .collect();
            self.load_balancer.select_agent(&top_candidates)
        };

        // Update metrics
        self.metrics.total_decisions += 1;
        self.metrics.total_evaluation_time += start_time.elapsed();

        if let Some(ref agent) = selected_agent {
            self.metrics.successful_routings += 1;

            // Update agent load after routing decision
            if let Some(load) = self.load_balancer.get_agent_load(agent) {
                let mut updated_load = load.clone();
                updated_load.queue_size += 1; // Assume message will be queued
                self.load_balancer.update_agent_load(agent.clone(), updated_load);
            }
        }

        RoutingDecision {
            selected_agent,
            evaluated_candidates: scored_candidates,
            evaluation_time: start_time.elapsed(),
            rules_applied: self.rules.len(),
        }
    }

    /// Update agent load information
    pub fn update_agent_load(&mut self, agent_id: AgentId, load: AgentLoad) {
        self.load_balancer.update_agent_load(agent_id, load);
    }

    /// Get routing metrics
    pub fn get_metrics(&self) -> &RoutingMetrics {
        &self.metrics
    }

    /// Reset routing metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = RoutingMetrics::default();
    }

    /// Get load balancer reference
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Get mutable load balancer reference
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }
}

impl std::fmt::Debug for RoutingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingEngine")
            .field("rules_count", &self.rules.len())
            .field("load_balancer", &self.load_balancer)
            .field("metrics", &self.metrics)
            .finish()
    }
}

/// A candidate agent with routing score
#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub agent_id: AgentId,
    pub score: f64,
    pub load: AgentLoad,
}

/// Result of routing evaluation
#[derive(Debug)]
pub struct RoutingDecision {
    pub selected_agent: Option<AgentId>,
    pub evaluated_candidates: Vec<ScoredCandidate>,
    pub evaluation_time: Duration,
    pub rules_applied: usize,
}

/// Routing performance metrics
#[derive(Debug, Default)]
pub struct RoutingMetrics {
    pub total_decisions: u64,
    pub successful_routings: u64,
    pub total_evaluation_time: Duration,
    pub average_evaluation_time: Duration,
}

impl RoutingMetrics {
    /// Update average evaluation time
    pub fn update_average(&mut self) {
        if self.total_decisions > 0 {
            self.average_evaluation_time = self.total_evaluation_time / self.total_decisions as u32;
        }
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_decisions == 0 {
            0.0
        } else {
            (self.successful_routings as f64 / self.total_decisions as f64) * 100.0
        }
    }
}

/// Message type-based routing rule
#[derive(Debug)]
pub struct MessageTypeRoutingRule {
    /// Allowed message types
    pub allowed_types: Vec<String>,
    /// Whether to use whitelist (true) or blacklist (false)
    pub is_whitelist: bool,
}

impl MessageTypeRoutingRule {
    pub fn whitelist(allowed_types: Vec<String>) -> Self {
        Self {
            allowed_types,
            is_whitelist: true,
        }
    }

    pub fn blacklist(blocked_types: Vec<String>) -> Self {
        Self {
            allowed_types: blocked_types,
            is_whitelist: false,
        }
    }
}

impl RoutingRule for MessageTypeRoutingRule {
    fn should_route(&self, envelope: &dyn MessageEnvelopeInfo, _agent_load: &AgentLoad) -> bool {
        let message_type = envelope.message_type();
        let contains = self.allowed_types.iter().any(|t| t == message_type);

        if self.is_whitelist {
            contains
        } else {
            !contains
        }
    }

    fn calculate_score(&self, envelope: &dyn MessageEnvelopeInfo, _agent_load: &AgentLoad) -> f64 {
        if self.should_route(envelope, _agent_load) {
            1.0
        } else {
            0.0
        }
    }

    fn name(&self) -> &str {
        "MessageTypeRoutingRule"
    }
}

/// Time-based routing rule (e.g., business hours only)
#[derive(Debug)]
pub struct TimeBasedRoutingRule {
    /// Start hour (0-23)
    pub start_hour: u8,
    /// End hour (0-23)
    pub end_hour: u8,
    /// Allowed days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Vec<u8>,
}

impl TimeBasedRoutingRule {
    pub fn business_hours() -> Self {
        Self {
            start_hour: 9,
            end_hour: 17,
            allowed_days: vec![1, 2, 3, 4, 5], // Monday to Friday
        }
    }

    pub fn always_available() -> Self {
        Self {
            start_hour: 0,
            end_hour: 23,
            allowed_days: vec![0, 1, 2, 3, 4, 5, 6], // All days
        }
    }
}

impl RoutingRule for TimeBasedRoutingRule {
    fn should_route(&self, _envelope: &dyn MessageEnvelopeInfo, _agent_load: &AgentLoad) -> bool {
        use chrono::{Local, Datelike, Timelike};

        let now = Local::now();
        let current_hour = now.hour() as u8;
        let current_day = now.weekday().num_days_from_sunday() as u8;

        let hour_ok = if self.start_hour <= self.end_hour {
            current_hour >= self.start_hour && current_hour <= self.end_hour
        } else {
            // Handle overnight ranges (e.g., 22:00 to 06:00)
            current_hour >= self.start_hour || current_hour <= self.end_hour
        };

        let day_ok = self.allowed_days.contains(&current_day);

        hour_ok && day_ok
    }

    fn calculate_score(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> f64 {
        if self.should_route(envelope, agent_load) {
            1.0
        } else {
            0.1 // Low score for off-hours, but not completely blocked
        }
    }

    fn name(&self) -> &str {
        "TimeBasedRoutingRule"
    }
}

/// Composite routing rule that combines multiple rules
pub struct CompositeRoutingRule {
    /// Child rules
    pub rules: Vec<Box<dyn RoutingRule>>,
    /// Combination strategy
    pub strategy: CompositeStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum CompositeStrategy {
    /// All rules must pass
    And,
    /// At least one rule must pass
    Or,
    /// Weighted average of scores
    WeightedAverage,
}

impl CompositeRoutingRule {
    pub fn new(strategy: CompositeStrategy) -> Self {
        Self {
            rules: Vec::new(),
            strategy,
        }
    }

    pub fn add_rule(&mut self, rule: Box<dyn RoutingRule>) {
        self.rules.push(rule);
    }
}

impl RoutingRule for CompositeRoutingRule {
    fn should_route(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> bool {
        match self.strategy {
            CompositeStrategy::And => {
                self.rules.iter().all(|rule| rule.should_route(envelope, agent_load))
            }
            CompositeStrategy::Or => {
                self.rules.iter().any(|rule| rule.should_route(envelope, agent_load))
            }
            CompositeStrategy::WeightedAverage => {
                // For weighted average, we consider it should route if average score > 0.5
                let total_score: f64 = self.rules.iter()
                    .map(|rule| rule.calculate_score(envelope, agent_load))
                    .sum();
                let average_score = if self.rules.is_empty() {
                    0.0
                } else {
                    total_score / self.rules.len() as f64
                };
                average_score > 0.5
            }
        }
    }

    fn calculate_score(&self, envelope: &dyn MessageEnvelopeInfo, agent_load: &AgentLoad) -> f64 {
        if self.rules.is_empty() {
            return 1.0;
        }

        match self.strategy {
            CompositeStrategy::And => {
                // Minimum score of all rules
                self.rules.iter()
                    .map(|rule| rule.calculate_score(envelope, agent_load))
                    .fold(f64::INFINITY, f64::min)
            }
            CompositeStrategy::Or => {
                // Maximum score of all rules
                self.rules.iter()
                    .map(|rule| rule.calculate_score(envelope, agent_load))
                    .fold(f64::NEG_INFINITY, f64::max)
            }
            CompositeStrategy::WeightedAverage => {
                // Average score of all rules
                let total_score: f64 = self.rules.iter()
                    .map(|rule| rule.calculate_score(envelope, agent_load))
                    .sum();
                total_score / self.rules.len() as f64
            }
        }
    }

    fn name(&self) -> &str {
        "CompositeRoutingRule"
    }
}

impl std::fmt::Debug for CompositeRoutingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeRoutingRule")
            .field("strategy", &self.strategy)
            .field("rules_count", &self.rules.len())
            .finish()
    }
}
