use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify};
use std::sync::atomic::{AtomicBool, AtomicUsize};

/// An async multi-producer, multi-consumer queue with backpressure control.
#[derive(Debug)]
pub struct Queue<T> {
    inner: Arc<Inner<T>>,
}

#[derive(Debug)]
struct Inner<T> {
    sender: mpsc::UnboundedSender<T>,
    receiver: Mutex<Option<mpsc::UnboundedReceiver<T>>>,
    is_shutdown: AtomicBool,
    unfinished_tasks: AtomicUsize,
    finished_notify: Notify,
    maxsize: usize,
    current_size: AtomicUsize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct QueueShutDown;

impl std::fmt::Display for QueueShutDown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Queue is shut down")
    }
}

impl std::error::Error for QueueShutDown {}

impl<T> Queue<T> {
    /// Creates a new unbounded queue.
    pub fn new() -> Self {
        Self::with_maxsize(0)
    }

    /// Creates a new queue with a maximum size.
    /// If maxsize is 0, the queue is unbounded.
    pub fn with_maxsize(maxsize: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            inner: Arc::new(Inner {
                sender,
                receiver: Mutex::new(Some(receiver)),
                is_shutdown: AtomicBool::new(false),
                unfinished_tasks: AtomicUsize::new(0),
                finished_notify: Notify::new(),
                maxsize,
                current_size: AtomicUsize::new(0),
            }),
        }
    }

    /// Puts an item into the queue without blocking.
    ///
    /// If the queue is shut down or full, this will return an error immediately.
    pub fn put_nowait(&self, item: T) -> Result<(), QueueShutDown> {
        if self.inner.is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(QueueShutDown);
        }

        // Check if queue is full (for bounded queues)
        if self.inner.maxsize > 0 {
            let current = self.inner.current_size.load(std::sync::atomic::Ordering::Relaxed);
            if current >= self.inner.maxsize {
                return Err(QueueShutDown); // Using QueueShutDown as a generic error for now
            }
        }

        self.inner.sender.send(item).map_err(|_| QueueShutDown)?;
        self.inner.unfinished_tasks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.current_size.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Puts an item into the queue asynchronously.
    ///
    /// If the queue is shut down, this will return an error.
    /// If the queue has a maxsize and is full, this will wait until space is available.
    pub async fn put(&self, item: T) -> Result<(), QueueShutDown> {
        if self.inner.is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(QueueShutDown);
        }

        // Check maxsize if bounded
        if self.inner.maxsize > 0 {
            loop {
                let current = self.inner.current_size.load(std::sync::atomic::Ordering::Relaxed);
                if current < self.inner.maxsize {
                    break;
                }
                if self.inner.is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(QueueShutDown);
                }
                // Wait a bit and retry (simple backpressure)
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }

        self.inner.sender.send(item).map_err(|_| QueueShutDown)?;
        self.inner.unfinished_tasks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.current_size.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Gets an item from the queue without blocking.
    ///
    /// If no item is available or the queue is shut down, this will return an error immediately.
    pub fn get_nowait(&self) -> Result<T, QueueShutDown> {
        if self.inner.is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(QueueShutDown);
        }

        let mut receiver = self.inner.receiver.lock().unwrap().take()
            .ok_or(QueueShutDown)?;

        match receiver.try_recv() {
            Ok(item) => {
                self.inner.current_size.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                // Put receiver back
                *self.inner.receiver.lock().unwrap() = Some(receiver);
                Ok(item)
            }
            Err(_) => {
                // Put receiver back
                *self.inner.receiver.lock().unwrap() = Some(receiver);
                Err(QueueShutDown)
            }
        }
    }

    /// Gets an item from the queue asynchronously.
    ///
    /// This will wait until an item is available or the queue is shut down.
    pub async fn get(&self) -> Result<T, QueueShutDown> {
        let mut receiver = self.inner.receiver.lock().unwrap().take()
            .ok_or(QueueShutDown)?;

        loop {
            tokio::select! {
                item = receiver.recv() => {
                    match item {
                        Some(item) => {
                            self.inner.current_size.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                            // Put receiver back
                            *self.inner.receiver.lock().unwrap() = Some(receiver);
                            return Ok(item);
                        }
                        None => {
                            return Err(QueueShutDown);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                    if self.inner.is_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                        return Err(QueueShutDown);
                    }
                }
            }
        }
    }

    /// Indicates that a formerly enqueued task is complete.
    pub fn task_done(&self) {
        let prev = self.inner.unfinished_tasks.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        if prev == 0 {
            panic!("task_done() called too many times");
        }
        if prev == 1 {
            self.inner.finished_notify.notify_waiters();
        }
    }

    /// Waits until all items in the queue have been gotten and processed.
    pub async fn join(&self) {
        loop {
            if self.inner.unfinished_tasks.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                break;
            }
            self.inner.finished_notify.notified().await;
        }
    }

    /// Shuts down the queue immediately.
    pub fn shutdown(&self, immediate: bool) {
        self.inner.is_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

        if immediate {
            // Mark all remaining tasks as done
            let remaining = self.inner.unfinished_tasks.swap(0, std::sync::atomic::Ordering::Relaxed);
            if remaining > 0 {
                self.inner.finished_notify.notify_waiters();
            }
        }

        // Close the sender to wake up any waiting receivers
        // This is done by dropping the sender, but we can't do that here
        // Instead, we rely on the is_shutdown flag
    }

    /// Returns the number of items currently in the queue.
    pub fn qsize(&self) -> usize {
        self.inner.current_size.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns true if the queue is empty.
    pub fn empty(&self) -> bool {
        self.qsize() == 0
    }

    /// Returns true if the queue is full (only meaningful for bounded queues).
    pub fn full(&self) -> bool {
        if self.inner.maxsize == 0 {
            false
        } else {
            self.qsize() >= self.inner.maxsize
        }
    }

    /// Returns the maximum size of the queue (0 means unbounded).
    pub fn maxsize(&self) -> usize {
        self.inner.maxsize
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}