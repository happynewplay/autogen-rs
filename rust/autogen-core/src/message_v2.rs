//! High-performance message system with reduced allocations
//!
//! This module provides an optimized message system that minimizes the use of
//! Box<dyn Any> and reduces memory allocations for better performance.

use std::any::{Any, TypeId};
use std::fmt;
use crate::{MessageContext, Result, AutoGenError};

/// Trait for messages that can be efficiently serialized and routed
pub trait EfficientMessage: Send + Sync + fmt::Debug + 'static {
    /// The response type for this message
    type Response: EfficientMessage;
    
    /// Get a unique type identifier for this message
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
    /// Get the type name for debugging
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Serialize the message to bytes for efficient transport
    fn serialize(&self) -> Result<Vec<u8>>;
    
    /// Deserialize from bytes
    fn deserialize(data: &[u8]) -> Result<Self> where Self: Sized;
    
    /// Validate the message content
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// Efficient message envelope that avoids Box<dyn Any> for common cases
#[derive(Debug)]
pub enum EfficientMessageEnvelope {
    /// Small message that fits inline (no allocation)
    Inline {
        /// Serialized message data (up to 64 bytes)
        data: [u8; 64],
        /// Actual length of the serialized data
        len: usize,
        /// Type identifier for safe downcasting
        type_id: TypeId,
        /// Type name for debugging
        type_name: &'static str,
        /// Message context
        context: MessageContext,
    },
    /// Larger message stored in a Vec
    Heap {
        /// Serialized message data
        data: Vec<u8>,
        /// Type identifier for safe downcasting
        type_id: TypeId,
        /// Type name for debugging
        type_name: &'static str,
        /// Message context
        context: MessageContext,
    },
    /// Legacy fallback for compatibility
    Legacy {
        /// Legacy message as trait object
        message: Box<dyn Any + Send + Sync>,
        /// Type identifier for safe downcasting
        type_id: TypeId,
        /// Type name for debugging
        type_name: &'static str,
        /// Message context
        context: MessageContext,
    },
}

impl EfficientMessageEnvelope {
    /// Create a new envelope from an efficient message
    pub fn new<M: EfficientMessage>(message: M, context: MessageContext) -> Result<Self> {
        let serialized = message.serialize()?;
        let type_id = message.type_id();
        let type_name = message.type_name();
        
        if serialized.len() <= 64 {
            let mut data = [0u8; 64];
            data[..serialized.len()].copy_from_slice(&serialized);
            Ok(Self::Inline {
                data,
                len: serialized.len(),
                type_id,
                type_name,
                context,
            })
        } else {
            Ok(Self::Heap {
                data: serialized,
                type_id,
                type_name,
                context,
            })
        }
    }
    
    /// Create from legacy Box<dyn Any> for compatibility
    pub fn from_legacy(
        message: Box<dyn Any + Send + Sync>,
        type_id: TypeId,
        type_name: &'static str,
        context: MessageContext,
    ) -> Self {
        Self::Legacy {
            message,
            type_id,
            type_name,
            context,
        }
    }
    
    /// Get the type ID
    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Inline { type_id, .. } => *type_id,
            Self::Heap { type_id, .. } => *type_id,
            Self::Legacy { type_id, .. } => *type_id,
        }
    }
    
    /// Get the type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Inline { type_name, .. } => type_name,
            Self::Heap { type_name, .. } => type_name,
            Self::Legacy { type_name, .. } => type_name,
        }
    }
    
    /// Get the message context
    pub fn context(&self) -> &MessageContext {
        match self {
            Self::Inline { context, .. } => context,
            Self::Heap { context, .. } => context,
            Self::Legacy { context, .. } => context,
        }
    }
    
    /// Deserialize to a specific message type
    pub fn deserialize<M: EfficientMessage>(&self) -> Result<M> {
        if self.type_id() != TypeId::of::<M>() {
            return Err(AutoGenError::other(format!(
                "Type mismatch: expected {}, got {}",
                std::any::type_name::<M>(),
                self.type_name()
            )));
        }
        
        match self {
            Self::Inline { data, len, .. } => {
                M::deserialize(&data[..*len])
            }
            Self::Heap { data, .. } => {
                M::deserialize(data)
            }
            Self::Legacy { message, .. } => {
                // Try to downcast for legacy compatibility
                if let Some(_typed_message) = message.downcast_ref::<M>() {
                    // For legacy messages, we need to clone since we can't move out of the box
                    // This is not ideal but necessary for compatibility
                    Err(AutoGenError::other(
                        "Legacy message deserialization not supported - use EfficientMessage trait"
                    ))
                } else {
                    Err(AutoGenError::other("Failed to downcast legacy message"))
                }
            }
        }
    }
    
    /// Check if this envelope contains a message of type M
    pub fn is_type<M: EfficientMessage>(&self) -> bool {
        self.type_id() == TypeId::of::<M>()
    }
}

/// High-performance message router with reduced allocations
pub struct EfficientMessageRouter {
    /// Handlers indexed by type ID
    handlers: std::collections::HashMap<TypeId, HandlerInfo>,
}

/// Handler information for efficient dispatch
struct HandlerInfo {
    /// Function pointer for zero-allocation dispatch
    handler_fn: fn(&EfficientMessageEnvelope) -> Result<Option<EfficientMessageEnvelope>>,
    /// Type name for debugging
    type_name: &'static str,
}

impl fmt::Debug for EfficientMessageRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfficientMessageRouter")
            .field("handler_count", &self.handlers.len())
            .finish()
    }
}

impl EfficientMessageRouter {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }
    
    /// Register a handler for a specific message type
    pub fn register_handler<M: EfficientMessage>(&mut self) -> Result<()>
    where
        M::Response: EfficientMessage,
        M: 'static,
        M::Response: 'static,
    {
        let type_id = TypeId::of::<M>();

        // Create a wrapper function that handles the deserialization
        fn dispatch_wrapper<M: EfficientMessage>(
            _envelope: &EfficientMessageEnvelope
        ) -> Result<Option<EfficientMessageEnvelope>>
        where
            M::Response: EfficientMessage,
            M: 'static,
            M::Response: 'static,
        {
            // For now, return None - this will be implemented with proper handler storage
            // This is a placeholder implementation for the efficient message system
            Ok(None)
        }

        let info = HandlerInfo {
            handler_fn: dispatch_wrapper::<M>,
            type_name: std::any::type_name::<M>(),
        };

        if self.handlers.insert(type_id, info).is_some() {
            return Err(AutoGenError::other(format!(
                "Handler for type {} already registered",
                std::any::type_name::<M>()
            )));
        }

        Ok(())
    }
    
    /// Route a message to the appropriate handler
    pub fn route(&self, envelope: &EfficientMessageEnvelope) -> Result<Option<EfficientMessageEnvelope>> {
        if let Some(handler_info) = self.handlers.get(&envelope.type_id()) {
            (handler_info.handler_fn)(envelope)
        } else {
            Err(AutoGenError::other(format!(
                "No handler registered for message type: {}",
                envelope.type_name()
            )))
        }
    }
    
    /// Get the number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
    
    /// Check if a handler is registered for a specific type
    pub fn has_handler<M: EfficientMessage>(&self) -> bool {
        self.handlers.contains_key(&TypeId::of::<M>())
    }
}

impl Default for EfficientMessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for the efficient message system
#[derive(Debug, Default)]
pub struct MessagePerformanceMetrics {
    /// Number of inline messages (no allocation)
    pub inline_messages: u64,
    /// Number of heap-allocated messages
    pub heap_messages: u64,
    /// Number of legacy messages (Box<dyn Any>)
    pub legacy_messages: u64,
    /// Total bytes saved by avoiding allocations
    pub bytes_saved: u64,
    /// Average message size
    pub avg_message_size: f64,
}

impl MessagePerformanceMetrics {
    /// Record a new message
    pub fn record_message(&mut self, envelope: &EfficientMessageEnvelope) {
        match envelope {
            EfficientMessageEnvelope::Inline { len, .. } => {
                self.inline_messages += 1;
                self.bytes_saved += 64 - *len as u64; // Saved allocation overhead
                self.update_avg_size(*len as f64);
            }
            EfficientMessageEnvelope::Heap { data, .. } => {
                self.heap_messages += 1;
                self.update_avg_size(data.len() as f64);
            }
            EfficientMessageEnvelope::Legacy { .. } => {
                self.legacy_messages += 1;
                // Assume average size for legacy messages
                self.update_avg_size(128.0);
            }
        }
    }
    
    fn update_avg_size(&mut self, size: f64) {
        let total_messages = self.inline_messages + self.heap_messages + self.legacy_messages;
        if total_messages > 0 {
            self.avg_message_size = (self.avg_message_size * (total_messages - 1) as f64 + size) / total_messages as f64;
        }
    }
    
    /// Get the percentage of messages that avoided allocation
    pub fn allocation_avoidance_rate(&self) -> f64 {
        let total = self.inline_messages + self.heap_messages + self.legacy_messages;
        if total > 0 {
            self.inline_messages as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    }
}
