use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Notify;
use crate::routing::MessagePriority;

/// A priority-aware message envelope wrapper for ordering in the priority queue
#[derive(Debug)]
pub struct PriorityMessageEnvelope<T> {
    pub envelope: T,
    pub priority: MessagePriority,
    pub timestamp: Instant,
}

impl<T> PriorityMessageEnvelope<T> {
    pub fn new(envelope: T, priority: MessagePriority, timestamp: Instant) -> Self {
        Self {
            envelope,
            priority,
            timestamp,
        }
    }
}

impl<T> PartialEq for PriorityMessageEnvelope<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp == other.timestamp
    }
}

impl<T> Eq for PriorityMessageEnvelope<T> {}

impl<T> PartialOrd for PriorityMessageEnvelope<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PriorityMessageEnvelope<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier timestamp (FIFO within same priority)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.timestamp.cmp(&self.timestamp), // Earlier timestamp first
            other => other, // Higher priority first
        }
    }
}

/// An async priority queue with backpressure control
#[derive(Debug)]
pub struct PriorityQueue<T> {
    inner: Arc<Mutex<PriorityQueueInner<T>>>,
    notify: Arc<Notify>,
}

#[derive(Debug)]
struct PriorityQueueInner<T> {
    heap: BinaryHeap<PriorityMessageEnvelope<T>>,
    is_shutdown: bool,
    maxsize: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct QueueShutDown;

impl std::fmt::Display for QueueShutDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Priority queue is shut down")
    }
}

impl std::error::Error for QueueShutDown {}

impl<T> PriorityQueue<T> {
    /// Creates a new unbounded priority queue
    pub fn new() -> Self {
        Self::with_maxsize(0)
    }

    /// Creates a new priority queue with a maximum size
    /// If maxsize is 0, the queue is unbounded
    pub fn with_maxsize(maxsize: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PriorityQueueInner {
                heap: BinaryHeap::new(),
                is_shutdown: false,
                maxsize,
            })),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Puts an item into the queue without blocking
    pub fn put_nowait(&self, item: T, priority: MessagePriority) -> Result<(), QueueShutDown> {
        let mut inner = self.inner.lock().unwrap();
        
        if inner.is_shutdown {
            return Err(QueueShutDown);
        }

        // Check if queue is full (for bounded queues)
        if inner.maxsize > 0 && inner.heap.len() >= inner.maxsize {
            return Err(QueueShutDown); // Using QueueShutDown as a generic error for now
        }

        let envelope = PriorityMessageEnvelope::new(item, priority, Instant::now());
        inner.heap.push(envelope);
        
        // Notify waiting consumers
        self.notify.notify_one();
        
        Ok(())
    }

    /// Puts an item into the queue asynchronously
    pub async fn put(&self, item: T, priority: MessagePriority) -> Result<(), QueueShutDown> {
        loop {
            {
                let mut inner = self.inner.lock().unwrap();
                
                if inner.is_shutdown {
                    return Err(QueueShutDown);
                }

                // Check if we can insert
                if inner.maxsize == 0 || inner.heap.len() < inner.maxsize {
                    let envelope = PriorityMessageEnvelope::new(item, priority, Instant::now());
                    inner.heap.push(envelope);
                    
                    // Notify waiting consumers
                    self.notify.notify_one();
                    
                    return Ok(());
                }
            }

            // Wait a bit and retry (simple backpressure)
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }

    /// Gets an item from the queue without blocking
    pub fn get_nowait(&self) -> Result<T, QueueShutDown> {
        let mut inner = self.inner.lock().unwrap();
        
        if inner.is_shutdown {
            return Err(QueueShutDown);
        }

        match inner.heap.pop() {
            Some(envelope) => Ok(envelope.envelope),
            None => Err(QueueShutDown),
        }
    }

    /// Gets an item from the queue asynchronously
    pub async fn get(&self) -> Result<T, QueueShutDown> {
        loop {
            {
                let mut inner = self.inner.lock().unwrap();
                
                if inner.is_shutdown {
                    return Err(QueueShutDown);
                }

                if let Some(envelope) = inner.heap.pop() {
                    return Ok(envelope.envelope);
                }
            }

            // Wait for notification
            self.notify.notified().await;
        }
    }

    /// Returns the number of items currently in the queue
    pub fn qsize(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.heap.len()
    }

    /// Returns true if the queue is empty
    pub fn empty(&self) -> bool {
        self.qsize() == 0
    }

    /// Returns true if the queue is full (only meaningful for bounded queues)
    pub fn full(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        if inner.maxsize == 0 {
            false
        } else {
            inner.heap.len() >= inner.maxsize
        }
    }

    /// Returns the maximum size of the queue (0 means unbounded)
    pub fn maxsize(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.maxsize
    }

    /// Shuts down the queue immediately
    pub fn shutdown(&self, _immediate: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.is_shutdown = true;
        
        // Notify all waiting consumers
        self.notify.notify_waiters();
    }

    /// Peek at the highest priority item without removing it
    pub fn peek(&self) -> Option<MessagePriority> {
        let inner = self.inner.lock().unwrap();
        inner.heap.peek().map(|envelope| envelope.priority)
    }

    /// Get statistics about the queue contents
    pub fn get_priority_stats(&self) -> PriorityStats {
        let inner = self.inner.lock().unwrap();
        let mut stats = PriorityStats::default();
        
        for envelope in &inner.heap {
            match envelope.priority {
                MessagePriority::Low => stats.low_count += 1,
                MessagePriority::Normal => stats.normal_count += 1,
                MessagePriority::High => stats.high_count += 1,
                MessagePriority::Critical => stats.critical_count += 1,
            }
        }
        
        stats.total_count = inner.heap.len();
        stats
    }
}

impl<T> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about priority queue contents
#[derive(Debug, Default, Clone)]
pub struct PriorityStats {
    pub total_count: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub normal_count: usize,
    pub low_count: usize,
}

impl PriorityStats {
    /// Get the percentage of high-priority messages (High + Critical)
    pub fn high_priority_percentage(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            ((self.high_count + self.critical_count) as f64 / self.total_count as f64) * 100.0
        }
    }

    /// Check if the queue is dominated by high-priority messages
    pub fn is_high_priority_dominated(&self) -> bool {
        self.high_priority_percentage() > 70.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = PriorityQueue::new();
        
        // Add messages with different priorities
        queue.put_nowait("low", MessagePriority::Low).unwrap();
        queue.put_nowait("normal", MessagePriority::Normal).unwrap();
        queue.put_nowait("high", MessagePriority::High).unwrap();
        queue.put_nowait("critical", MessagePriority::Critical).unwrap();
        
        // Should get them in priority order
        assert_eq!(queue.get_nowait().unwrap(), "critical");
        assert_eq!(queue.get_nowait().unwrap(), "high");
        assert_eq!(queue.get_nowait().unwrap(), "normal");
        assert_eq!(queue.get_nowait().unwrap(), "low");
    }

    #[tokio::test]
    async fn test_fifo_within_priority() {
        let queue = PriorityQueue::new();
        
        // Add multiple messages with same priority
        queue.put_nowait("first", MessagePriority::Normal).unwrap();
        sleep(Duration::from_millis(1)).await; // Ensure different timestamps
        queue.put_nowait("second", MessagePriority::Normal).unwrap();
        sleep(Duration::from_millis(1)).await;
        queue.put_nowait("third", MessagePriority::Normal).unwrap();
        
        // Should get them in FIFO order within same priority
        assert_eq!(queue.get_nowait().unwrap(), "first");
        assert_eq!(queue.get_nowait().unwrap(), "second");
        assert_eq!(queue.get_nowait().unwrap(), "third");
    }
}
