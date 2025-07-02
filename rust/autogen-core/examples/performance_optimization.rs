//! Performance optimization example for autogen-core
//!
//! This example demonstrates various performance optimization techniques
//! including efficient message passing, memory management, and runtime configuration.

use std::time::Instant;
use autogen_core::{
    AgentId, TextMessage, MessageContext, CancellationToken, Result,
    message_v2::{EfficientMessage, EfficientMessageEnvelope, EfficientMessageRouter, MessagePerformanceMetrics},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 AutoGen Core - Performance Optimization Example");
    
    // Demonstrate efficient message passing
    efficient_message_demo().await?;
    
    // Demonstrate memory optimization
    memory_optimization_demo().await?;
    
    // Demonstrate batch processing
    batch_processing_demo().await?;
    
    // Performance comparison
    performance_comparison_demo().await?;
    
    Ok(())
}

/// Demonstrate efficient message passing with reduced allocations
async fn efficient_message_demo() -> Result<()> {
    println!("\n=== Efficient Message Passing ===");
    
    let mut metrics = MessagePerformanceMetrics::default();
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Create messages of different sizes to test inline vs heap allocation
    let small_message = TextMessage {
        content: "Hello".to_string(), // Small message - should be inline
    };
    
    let large_message = TextMessage {
        content: "A".repeat(1000), // Large message - will use heap
    };
    
    // Test small message (should use inline storage)
    let start = Instant::now();
    let small_envelope = EfficientMessageEnvelope::new(small_message, context.clone())?;
    let small_duration = start.elapsed();
    metrics.record_message(&small_envelope);
    
    println!("Small message envelope created in: {:?}", small_duration);
    println!("Message type: {:?}", match small_envelope {
        EfficientMessageEnvelope::Inline { .. } => "Inline (no allocation)",
        EfficientMessageEnvelope::Heap { .. } => "Heap allocated",
        EfficientMessageEnvelope::Legacy { .. } => "Legacy",
    });
    
    // Test large message (should use heap storage)
    let start = Instant::now();
    let large_envelope = EfficientMessageEnvelope::new(large_message, context)?;
    let large_duration = start.elapsed();
    metrics.record_message(&large_envelope);
    
    println!("Large message envelope created in: {:?}", large_duration);
    println!("Message type: {:?}", match large_envelope {
        EfficientMessageEnvelope::Inline { .. } => "Inline (no allocation)",
        EfficientMessageEnvelope::Heap { .. } => "Heap allocated",
        EfficientMessageEnvelope::Legacy { .. } => "Legacy",
    });
    
    println!("Allocation avoidance rate: {:.1}%", metrics.allocation_avoidance_rate());
    
    Ok(())
}

/// Demonstrate memory optimization techniques
async fn memory_optimization_demo() -> Result<()> {
    println!("\n=== Memory Optimization ===");
    
    // Pre-allocate collections to avoid repeated allocations
    let mut message_buffer = Vec::with_capacity(1000);
    let mut router = EfficientMessageRouter::new();
    
    // Register handlers efficiently
    router.register_handler::<TextMessage, _>(|_msg, _ctx| Ok(None))?;
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Batch create messages to demonstrate efficient memory usage
    let start = Instant::now();
    for i in 0..1000 {
        let message = TextMessage {
            content: format!("Message {}", i),
        };
        let envelope = EfficientMessageEnvelope::new(message, context.clone())?;
        message_buffer.push(envelope);
    }
    let creation_time = start.elapsed();
    
    println!("Created 1000 messages in: {:?}", creation_time);
    println!("Average time per message: {:?}", creation_time / 1000);
    
    // Process messages efficiently
    let start = Instant::now();
    let mut processed = 0;
    for envelope in &message_buffer {
        if envelope.is_type::<TextMessage>() {
            processed += 1;
        }
    }
    let processing_time = start.elapsed();
    
    println!("Processed {} messages in: {:?}", processed, processing_time);
    println!("Average processing time per message: {:?}", processing_time / processed);
    
    Ok(())
}

/// Demonstrate batch processing for improved throughput
async fn batch_processing_demo() -> Result<()> {
    println!("\n=== Batch Processing ===");
    
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Create a batch of messages
    let batch_size = 100;
    let mut batch = Vec::with_capacity(batch_size);
    
    for i in 0..batch_size {
        let message = TextMessage {
            content: format!("Batch message {}", i),
        };
        batch.push(message);
    }
    
    // Process individually (slower)
    let start = Instant::now();
    for message in &batch {
        let envelope = EfficientMessageEnvelope::new(message.clone(), context.clone())?;
        // Simulate processing
        let _type_check = envelope.is_type::<TextMessage>();
    }
    let individual_time = start.elapsed();
    
    // Process in batch (faster)
    let start = Instant::now();
    let envelopes: Result<Vec<_>> = batch.iter()
        .map(|msg| EfficientMessageEnvelope::new(msg.clone(), context.clone()))
        .collect();
    let envelopes = envelopes?;
    
    // Batch type checking
    let text_messages: Vec<_> = envelopes.iter()
        .filter(|env| env.is_type::<TextMessage>())
        .collect();
    
    let batch_time = start.elapsed();
    
    println!("Individual processing time: {:?}", individual_time);
    println!("Batch processing time: {:?}", batch_time);
    println!("Speedup: {:.2}x", individual_time.as_nanos() as f64 / batch_time.as_nanos() as f64);
    println!("Processed {} text messages", text_messages.len());
    
    Ok(())
}

/// Compare performance between different message handling approaches
async fn performance_comparison_demo() -> Result<()> {
    println!("\n=== Performance Comparison ===");
    
    let iterations = 10_000;
    let token = CancellationToken::new();
    let context = MessageContext::direct_message(None, token);
    
    // Test efficient message system
    let start = Instant::now();
    let mut efficient_metrics = MessagePerformanceMetrics::default();
    
    for i in 0..iterations {
        let message = TextMessage {
            content: format!("Efficient message {}", i),
        };
        let envelope = EfficientMessageEnvelope::new(message, context.clone())?;
        efficient_metrics.record_message(&envelope);
        
        // Simulate message processing
        let _is_text = envelope.is_type::<TextMessage>();
    }
    
    let efficient_time = start.elapsed();
    
    println!("Efficient message system:");
    println!("  Time for {} messages: {:?}", iterations, efficient_time);
    println!("  Average time per message: {:?}", efficient_time / iterations);
    println!("  Allocation avoidance rate: {:.1}%", efficient_metrics.allocation_avoidance_rate());
    println!("  Inline messages: {}", efficient_metrics.inline_messages);
    println!("  Heap messages: {}", efficient_metrics.heap_messages);
    println!("  Bytes saved: {}", efficient_metrics.bytes_saved);
    
    // Memory usage summary
    println!("\n=== Memory Usage Summary ===");
    println!("Total messages processed: {}", iterations);
    println!("Messages using inline storage: {}", efficient_metrics.inline_messages);
    println!("Messages using heap storage: {}", efficient_metrics.heap_messages);
    println!("Memory allocation avoidance: {:.1}%", efficient_metrics.allocation_avoidance_rate());
    println!("Estimated bytes saved: {} KB", efficient_metrics.bytes_saved / 1024);
    
    Ok(())
}

/// Custom efficient message for demonstration
#[derive(Debug, Clone)]
struct CustomMessage {
    id: u64,
    data: String,
    priority: u8,
}

#[cfg(feature = "json")]
impl EfficientMessage for CustomMessage {
    type Response = CustomResponse;
    
    fn serialize(&self) -> Result<Vec<u8>> {
        use serde_json;
        serde_json::to_vec(&serde_json::json!({
            "id": self.id,
            "data": self.data,
            "priority": self.priority
        })).map_err(|e| autogen_core::AutoGenError::other(format!("Serialization failed: {}", e)))
    }
    
    fn deserialize(data: &[u8]) -> Result<Self> {
        use serde_json;
        let value: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| autogen_core::AutoGenError::other(format!("Deserialization failed: {}", e)))?;
        
        Ok(CustomMessage {
            id: value["id"].as_u64().unwrap_or(0),
            data: value["data"].as_str().unwrap_or("").to_string(),
            priority: value["priority"].as_u64().unwrap_or(0) as u8,
        })
    }
    
    fn validate(&self) -> Result<()> {
        if self.data.is_empty() {
            return Err(autogen_core::AutoGenError::other("CustomMessage data cannot be empty"));
        }
        if self.priority > 10 {
            return Err(autogen_core::AutoGenError::other("Priority must be between 0 and 10"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CustomResponse {
    success: bool,
    message: String,
}

#[cfg(feature = "json")]
impl EfficientMessage for CustomResponse {
    type Response = autogen_core::NoResponse;
    
    fn serialize(&self) -> Result<Vec<u8>> {
        use serde_json;
        serde_json::to_vec(&serde_json::json!({
            "success": self.success,
            "message": self.message
        })).map_err(|e| autogen_core::AutoGenError::other(format!("Serialization failed: {}", e)))
    }
    
    fn deserialize(data: &[u8]) -> Result<Self> {
        use serde_json;
        let value: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| autogen_core::AutoGenError::other(format!("Deserialization failed: {}", e)))?;
        
        Ok(CustomResponse {
            success: value["success"].as_bool().unwrap_or(false),
            message: value["message"].as_str().unwrap_or("").to_string(),
        })
    }
}
