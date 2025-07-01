//! Message serialization system
//!
//! This module provides serialization and deserialization capabilities for messages
//! in the autogen system, following the Python autogen-core design.
//!
//! This module is only available when serialization features are enabled.

#[cfg(any(feature = "json", feature = "protobuf"))]
use crate::error::{AutoGenError, Result};
#[cfg(any(feature = "json", feature = "protobuf"))]
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "json", feature = "protobuf"))]
use std::any::{Any, TypeId};
#[cfg(any(feature = "json", feature = "protobuf"))]
use std::collections::HashMap;
#[cfg(any(feature = "json", feature = "protobuf"))]
use std::sync::Arc;

#[cfg(feature = "json")]
/// Content type constants for message serialization
pub const JSON_DATA_CONTENT_TYPE: &str = "application/json";

#[cfg(feature = "protobuf")]
/// Content type for Protocol Buffer serialization
pub const PROTOBUF_DATA_CONTENT_TYPE: &str = "application/x-protobuf";

#[cfg(any(feature = "json", feature = "protobuf"))]
/// Trait for message serialization
///
/// This trait defines the interface for serializing and deserializing messages
/// in different formats (JSON, Protobuf, etc.).
pub trait MessageSerializer: Send + Sync {
    /// Serialize a message to bytes
    ///
    /// # Arguments
    /// * `message` - The message to serialize
    ///
    /// # Returns
    /// Serialized message bytes and content type
    fn serialize(&self, message: &dyn Any) -> Result<(Vec<u8>, String)>;

    /// Deserialize bytes to a message
    ///
    /// # Arguments
    /// * `data` - The serialized data
    /// * `content_type` - The content type of the data
    /// * `type_id` - The expected type ID of the message
    ///
    /// # Returns
    /// Deserialized message
    fn deserialize(&self, data: &[u8], content_type: &str, type_id: TypeId) -> Result<Box<dyn Any + Send>>;

    /// Get the content type this serializer produces
    fn content_type(&self) -> &str;
}

/// JSON message serializer
///
/// Serializes messages to/from JSON format using serde_json.
/// This follows the Python autogen-core JSON serialization approach.
pub struct JsonMessageSerializer {
    /// Registry of type serializers
    type_registry: HashMap<TypeId, Arc<dyn TypeSerializer>>,
}

impl JsonMessageSerializer {
    /// Create a new JSON message serializer
    pub fn new() -> Self {
        Self {
            type_registry: HashMap::new(),
        }
    }

    /// Register a type for serialization
    ///
    /// # Arguments
    /// * `serializer` - The type serializer for this type
    pub fn register_type<T: 'static>(&mut self, serializer: Arc<dyn TypeSerializer>) {
        self.type_registry.insert(TypeId::of::<T>(), serializer);
    }
}

impl Default for JsonMessageSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageSerializer for JsonMessageSerializer {
    fn serialize(&self, message: &dyn Any) -> Result<(Vec<u8>, String)> {
        let type_id = message.type_id();
        
        if let Some(serializer) = self.type_registry.get(&type_id) {
            let json_value = serializer.serialize(message)?;
            let bytes = serde_json::to_vec(&json_value)
                .map_err(|e| AutoGenError::Serialization { source: e })?;
            Ok((bytes, JSON_DATA_CONTENT_TYPE.to_string()))
        } else {
            Err(AutoGenError::Other {
                message: format!("No serializer registered for type: {:?}", type_id),
            })
        }
    }

    fn deserialize(&self, data: &[u8], content_type: &str, type_id: TypeId) -> Result<Box<dyn Any + Send>> {
        if content_type != JSON_DATA_CONTENT_TYPE {
            return Err(AutoGenError::Other {
                message: format!("Unsupported content type: {}", content_type),
            });
        }

        if let Some(serializer) = self.type_registry.get(&type_id) {
            let json_value: serde_json::Value = serde_json::from_slice(data)
                .map_err(|e| AutoGenError::Serialization { source: e })?;
            serializer.deserialize(&json_value)
        } else {
            Err(AutoGenError::Other {
                message: format!("No deserializer registered for type: {:?}", type_id),
            })
        }
    }

    fn content_type(&self) -> &str {
        JSON_DATA_CONTENT_TYPE
    }
}

/// Trait for type-specific serialization
///
/// This trait allows registration of custom serializers for specific types.
pub trait TypeSerializer: Send + Sync {
    /// Serialize a value to JSON
    fn serialize(&self, value: &dyn Any) -> Result<serde_json::Value>;

    /// Deserialize from JSON
    fn deserialize(&self, value: &serde_json::Value) -> Result<Box<dyn Any + Send>>;
}

/// Generic type serializer for types that implement Serialize + Deserialize
pub struct SerdeTypeSerializer<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> SerdeTypeSerializer<T> {
    /// Create a new serde type serializer
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for SerdeTypeSerializer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TypeSerializer for SerdeTypeSerializer<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    fn serialize(&self, value: &dyn Any) -> Result<serde_json::Value> {
        if let Some(typed_value) = value.downcast_ref::<T>() {
            serde_json::to_value(typed_value)
                .map_err(|e| AutoGenError::Serialization { source: e })
        } else {
            Err(AutoGenError::Other {
                message: "Type mismatch during serialization".to_string(),
            })
        }
    }

    fn deserialize(&self, value: &serde_json::Value) -> Result<Box<dyn Any + Send>> {
        let typed_value: T = serde_json::from_value(value.clone())
            .map_err(|e| AutoGenError::Serialization { source: e })?;
        Ok(Box::new(typed_value))
    }
}

/// Message envelope for serialized messages
///
/// This wraps a serialized message with metadata needed for routing and processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// The serialized message data
    pub data: Vec<u8>,
    
    /// Content type of the serialized data
    pub content_type: String,
    
    /// Type name of the original message
    pub type_name: String,
    
    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SerializedMessage {
    /// Create a new serialized message
    pub fn new(
        data: Vec<u8>,
        content_type: String,
        type_name: String,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            data,
            content_type,
            type_name,
            metadata,
        }
    }

    /// Get the message data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the content type
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Get the type name
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Get the metadata
    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        content: String,
        id: u32,
    }

    #[test]
    fn test_json_serializer_creation() {
        let serializer = JsonMessageSerializer::new();
        assert_eq!(serializer.content_type(), JSON_DATA_CONTENT_TYPE);
    }

    #[test]
    fn test_serde_type_serializer() {
        let serializer = SerdeTypeSerializer::<TestMessage>::new();
        let message = TestMessage {
            content: "test".to_string(),
            id: 42,
        };

        let json_value = serializer.serialize(&message).unwrap();
        let deserialized = serializer.deserialize(&json_value).unwrap();
        let recovered = deserialized.downcast::<TestMessage>().unwrap();
        
        assert_eq!(*recovered, message);
    }

    #[test]
    fn test_serialized_message() {
        let data = b"test data".to_vec();
        let content_type = JSON_DATA_CONTENT_TYPE.to_string();
        let type_name = "TestMessage".to_string();
        let metadata = HashMap::new();

        let serialized = SerializedMessage::new(data.clone(), content_type.clone(), type_name.clone(), metadata);
        
        assert_eq!(serialized.data(), data.as_slice());
        assert_eq!(serialized.content_type(), content_type);
        assert_eq!(serialized.type_name(), type_name);
        assert!(serialized.metadata().is_empty());
    }
}
