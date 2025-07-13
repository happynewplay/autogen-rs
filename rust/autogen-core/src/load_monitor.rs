use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;
use crate::agent_id::AgentId;
use crate::routing::AgentLoad;

/// Load monitoring system for tracking agent performance
#[derive(Debug)]
pub struct LoadMonitor {
    /// Agent load data
    agent_loads: Arc<Mutex<HashMap<AgentId, AgentLoadData>>>,
    /// Monitoring configuration
    config: LoadMonitorConfig,
    /// Whether monitoring is active
    is_active: Arc<Mutex<bool>>,
}

/// Internal agent load data with additional tracking information
#[derive(Debug, Clone)]
struct AgentLoadData {
    /// Current load information
    pub load: AgentLoad,
    /// Message processing history (timestamp, processing_time)
    pub processing_history: Vec<(Instant, Duration)>,
    /// Queue size history (timestamp, size)
    pub queue_history: Vec<(Instant, usize)>,
    /// Last update timestamp
    pub last_update: Instant,
}

/// Configuration for load monitoring
#[derive(Debug, Clone)]
pub struct LoadMonitorConfig {
    /// How often to update load statistics
    pub update_interval: Duration,
    /// How long to keep historical data
    pub history_retention: Duration,
    /// Maximum number of history entries per agent
    pub max_history_entries: usize,
    /// CPU usage sampling interval
    pub cpu_sample_interval: Duration,
}

impl Default for LoadMonitorConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_secs(5),
            history_retention: Duration::from_secs(300), // 5 minutes
            max_history_entries: 100,
            cpu_sample_interval: Duration::from_secs(1),
        }
    }
}

impl LoadMonitor {
    /// Create a new load monitor
    pub fn new(config: LoadMonitorConfig) -> Self {
        Self {
            agent_loads: Arc::new(Mutex::new(HashMap::new())),
            config,
            is_active: Arc::new(Mutex::new(false)),
        }
    }

    /// Start monitoring
    pub async fn start(&self) {
        {
            let mut active = self.is_active.lock().unwrap();
            if *active {
                return; // Already running
            }
            *active = true;
        }

        let agent_loads = self.agent_loads.clone();
        let config = self.config.clone();
        let is_active = self.is_active.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.update_interval);
            
            loop {
                interval.tick().await;
                
                // Check if monitoring should continue
                {
                    let active = is_active.lock().unwrap();
                    if !*active {
                        break;
                    }
                }

                // Update load statistics
                Self::update_load_statistics(&agent_loads, &config).await;
            }
        });
    }

    /// Stop monitoring
    pub fn stop(&self) {
        let mut active = self.is_active.lock().unwrap();
        *active = false;
    }

    /// Register a new agent for monitoring
    pub fn register_agent(&self, agent_id: AgentId) {
        let mut loads = self.agent_loads.lock().unwrap();
        loads.insert(agent_id, AgentLoadData {
            load: AgentLoad::default(),
            processing_history: Vec::new(),
            queue_history: Vec::new(),
            last_update: Instant::now(),
        });
    }

    /// Unregister an agent from monitoring
    pub fn unregister_agent(&self, agent_id: &AgentId) {
        let mut loads = self.agent_loads.lock().unwrap();
        loads.remove(agent_id);
    }

    /// Record message processing completion
    pub fn record_message_processed(&self, agent_id: &AgentId, processing_time: Duration) {
        let mut loads = self.agent_loads.lock().unwrap();
        if let Some(data) = loads.get_mut(agent_id) {
            let now = Instant::now();
            
            // Update processing time
            data.load.update_after_processing(processing_time);
            
            // Add to history
            data.processing_history.push((now, processing_time));
            
            // Trim history if needed
            if data.processing_history.len() > self.config.max_history_entries {
                data.processing_history.remove(0);
            }
            
            // Remove old history entries
            let cutoff = now - self.config.history_retention;
            data.processing_history.retain(|(timestamp, _)| *timestamp > cutoff);
            
            data.last_update = now;
        }
    }

    /// Update queue size for an agent
    pub fn update_queue_size(&self, agent_id: &AgentId, queue_size: usize) {
        let mut loads = self.agent_loads.lock().unwrap();
        if let Some(data) = loads.get_mut(agent_id) {
            let now = Instant::now();
            
            data.load.queue_size = queue_size;
            data.queue_history.push((now, queue_size));
            
            // Trim history if needed
            if data.queue_history.len() > self.config.max_history_entries {
                data.queue_history.remove(0);
            }
            
            // Remove old history entries
            let cutoff = now - self.config.history_retention;
            data.queue_history.retain(|(timestamp, _)| *timestamp > cutoff);
            
            data.last_update = now;
        }
    }

    /// Get current load for an agent
    pub fn get_agent_load(&self, agent_id: &AgentId) -> Option<AgentLoad> {
        let loads = self.agent_loads.lock().unwrap();
        loads.get(agent_id).map(|data| data.load.clone())
    }

    /// Get all agent loads
    pub fn get_all_loads(&self) -> HashMap<AgentId, AgentLoad> {
        let loads = self.agent_loads.lock().unwrap();
        loads.iter()
            .map(|(id, data)| (id.clone(), data.load.clone()))
            .collect()
    }

    /// Get load statistics for an agent
    pub fn get_agent_statistics(&self, agent_id: &AgentId) -> Option<AgentStatistics> {
        let loads = self.agent_loads.lock().unwrap();
        loads.get(agent_id).map(|data| {
            let now = Instant::now();
            let recent_cutoff = now - Duration::from_secs(60); // Last minute
            
            // Calculate recent processing times
            let recent_times: Vec<Duration> = data.processing_history.iter()
                .filter(|(timestamp, _)| *timestamp > recent_cutoff)
                .map(|(_, duration)| *duration)
                .collect();
            
            let avg_processing_time = if recent_times.is_empty() {
                data.load.processing_time_avg
            } else {
                let total: Duration = recent_times.iter().sum();
                total / recent_times.len() as u32
            };
            
            // Calculate messages per minute
            let messages_per_minute = recent_times.len() as u32;
            
            // Calculate average queue size
            let recent_queue_sizes: Vec<usize> = data.queue_history.iter()
                .filter(|(timestamp, _)| *timestamp > recent_cutoff)
                .map(|(_, size)| *size)
                .collect();
            
            let avg_queue_size = if recent_queue_sizes.is_empty() {
                data.load.queue_size as f64
            } else {
                recent_queue_sizes.iter().sum::<usize>() as f64 / recent_queue_sizes.len() as f64
            };
            
            AgentStatistics {
                agent_id: agent_id.clone(),
                current_load: data.load.clone(),
                avg_processing_time,
                messages_per_minute,
                avg_queue_size,
                total_messages_processed: data.processing_history.len(),
                last_activity: data.load.last_activity,
                uptime: now.duration_since(data.last_update),
            }
        })
    }

    /// Get system-wide load statistics
    pub fn get_system_statistics(&self) -> SystemStatistics {
        let loads = self.agent_loads.lock().unwrap();
        let now = Instant::now();
        
        let mut total_agents = 0;
        let mut active_agents = 0;
        let mut overloaded_agents = 0;
        let mut total_queue_size = 0;
        let mut total_messages_processed = 0;
        
        for data in loads.values() {
            total_agents += 1;
            
            if now.duration_since(data.load.last_activity) < Duration::from_secs(60) {
                active_agents += 1;
            }
            
            if data.load.is_overloaded() {
                overloaded_agents += 1;
            }
            
            total_queue_size += data.load.queue_size;
            total_messages_processed += data.processing_history.len();
        }
        
        SystemStatistics {
            total_agents,
            active_agents,
            overloaded_agents,
            total_queue_size,
            total_messages_processed,
            average_load: if total_agents > 0 {
                loads.values().map(|data| data.load.load_score()).sum::<f64>() / total_agents as f64
            } else {
                0.0
            },
        }
    }

    /// Internal method to update load statistics
    async fn update_load_statistics(
        agent_loads: &Arc<Mutex<HashMap<AgentId, AgentLoadData>>>,
        _config: &LoadMonitorConfig,
    ) {
        let mut loads = agent_loads.lock().unwrap();
        let now = Instant::now();
        
        for data in loads.values_mut() {
            // Update messages per minute based on recent history
            let recent_cutoff = now - Duration::from_secs(60);
            let recent_messages = data.processing_history.iter()
                .filter(|(timestamp, _)| *timestamp > recent_cutoff)
                .count();
            data.load.messages_per_minute = recent_messages as u32;
            
            // Simulate CPU usage (in a real implementation, this would query actual CPU usage)
            // For now, we'll estimate based on load
            data.load.cpu_usage = (data.load.load_score() * 100.0) as f32;
        }
    }
}

/// Statistics for a single agent
#[derive(Debug, Clone)]
pub struct AgentStatistics {
    pub agent_id: AgentId,
    pub current_load: AgentLoad,
    pub avg_processing_time: Duration,
    pub messages_per_minute: u32,
    pub avg_queue_size: f64,
    pub total_messages_processed: usize,
    pub last_activity: Instant,
    pub uptime: Duration,
}

/// System-wide statistics
#[derive(Debug, Clone)]
pub struct SystemStatistics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub overloaded_agents: usize,
    pub total_queue_size: usize,
    pub total_messages_processed: usize,
    pub average_load: f64,
}

impl SystemStatistics {
    /// Get the percentage of overloaded agents
    pub fn overload_percentage(&self) -> f64 {
        if self.total_agents == 0 {
            0.0
        } else {
            (self.overloaded_agents as f64 / self.total_agents as f64) * 100.0
        }
    }

    /// Check if the system is under stress
    pub fn is_under_stress(&self) -> bool {
        self.overload_percentage() > 50.0 || self.average_load > 0.8
    }
}
