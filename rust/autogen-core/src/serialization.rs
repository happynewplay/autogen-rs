use prost::Message;
use prost_types::Any;
use serde::{de::DeserializeOwned, Serialize};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use crate::type_helpers::{DataclassLike, BaseModelLike};
use std::any::{TypeId, Any as StdAny};
use std::collections::HashMap;
use std::sync::Arc;

pub const JSON_DATA_CONTENT_TYPE: &str = "application/json";
pub const PROTOBUF_DATA_CONTENT_TYPE: &str = "application/x-protobuf";

/// Errors that can occur during serialization
#[derive(Debug)]
pub enum SerializationError {
    JsonError(serde_json::Error),
    ProtobufError(prost::DecodeError),
    TypeMismatch(String),
    UnknownType(String),
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationError::JsonError(e) => write!(f, "JSON error: {}", e),
            SerializationError::ProtobufError(e) => write!(f, "Protobuf error: {}", e),
            SerializationError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            SerializationError::UnknownType(msg) => write!(f, "Unknown type: {}", msg),
        }
    }
}

impl std::error::Error for SerializationError {}

impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        SerializationError::JsonError(err)
    }
}

impl From<prost::DecodeError> for SerializationError {
    fn from(err: prost::DecodeError) -> Self {
        SerializationError::ProtobufError(err)
    }
}

/// A trait for serializing and deserializing messages.
pub trait MessageSerializer<T>: Send + Sync {
    /// The content type of the serialized data (e.g., "application/json").
    fn data_content_type(&self) -> &str;
    /// The name of the type being serialized.
    fn type_name(&self) -> &str;
    /// Deserializes a payload into a message.
    fn deserialize(&self, payload: &[u8]) -> Result<T, SerializationError>;
    /// Serializes a message into a payload.
    fn serialize(&self, message: &T) -> Result<Vec<u8>, SerializationError>;
}

/// Enhanced trait for serializers that can be cloned
pub trait CloneableMessageSerializer<T>: MessageSerializer<T> {
    fn clone_box(&self) -> Box<dyn CloneableMessageSerializer<T>>;
}

/// A serializer for types that implement `serde::Serialize` and `serde::Deserialize`.
pub struct JsonSerializer<T: Serialize + DeserializeOwned> {
    type_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> JsonSerializer<T> {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize + DeserializeOwned + Send + Sync> MessageSerializer<T> for JsonSerializer<T> {
    fn data_content_type(&self) -> &str {
        JSON_DATA_CONTENT_TYPE
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn deserialize(&self, payload: &[u8]) -> Result<T, SerializationError> {
        serde_json::from_slice(payload).map_err(SerializationError::from)
    }
    fn serialize(&self, message: &T) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(message).map_err(SerializationError::from)
    }
}

/// A serializer for Protobuf messages.
pub struct ProtobufSerializer<T: Message + Default> {
    type_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Message + Default> ProtobufSerializer<T> {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Message + Default + Send + Sync> MessageSerializer<T> for ProtobufSerializer<T> {
    fn data_content_type(&self) -> &str {
        PROTOBUF_DATA_CONTENT_TYPE
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn deserialize(&self, payload: &[u8]) -> Result<T, SerializationError> {
        let any_proto = Any::decode(payload)?;
        // Note: This doesn't check the type_url. A robust implementation should.
        let message = T::decode(any_proto.value.as_slice())?;
        Ok(message)
    }
    fn serialize(&self, message: &T) -> Result<Vec<u8>, SerializationError> {
        let any_proto = Any {
            type_url: format!("type.googleapis.com/{}", self.type_name()),
            value: message.encode_to_vec(),
        };
        Ok(any_proto.encode_to_vec())
    }
}

/// Serializer for dataclass-like types (equivalent to Python's DataclassJsonMessageSerializer)
#[derive(Clone)]
pub struct DataclassJsonSerializer<T: DataclassLike> {
    type_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DataclassLike> DataclassJsonSerializer<T> {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: DataclassLike> MessageSerializer<T> for DataclassJsonSerializer<T> {
    fn data_content_type(&self) -> &str {
        JSON_DATA_CONTENT_TYPE
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn deserialize(&self, payload: &[u8]) -> Result<T, SerializationError> {
        serde_json::from_slice(payload).map_err(SerializationError::from)
    }
    fn serialize(&self, message: &T) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(message).map_err(SerializationError::from)
    }
}

/// Serializer for base model-like types (equivalent to Python's PydanticJsonMessageSerializer)
#[derive(Clone)]
pub struct BaseModelJsonSerializer<T: BaseModelLike> {
    type_name: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: BaseModelLike> BaseModelJsonSerializer<T> {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: BaseModelLike> MessageSerializer<T> for BaseModelJsonSerializer<T> {
    fn data_content_type(&self) -> &str {
        JSON_DATA_CONTENT_TYPE
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn deserialize(&self, payload: &[u8]) -> Result<T, SerializationError> {
        let result: T = serde_json::from_slice(payload)?;
        // Validate the deserialized object
        result.validate().map_err(|e| SerializationError::TypeMismatch(e))?;
        Ok(result)
    }
    fn serialize(&self, message: &T) -> Result<Vec<u8>, SerializationError> {
        // Validate before serializing
        message.validate().map_err(|e| SerializationError::TypeMismatch(e))?;
        serde_json::to_vec(message).map_err(SerializationError::from)
    }
}

#[derive(Debug)]
pub struct UnknownPayload {
    pub type_name: String,
    pub data_content_type: String,
    pub payload: Vec<u8>,
}

type DynMessageSerializer =
    Box<dyn Fn(&[u8]) -> Result<Box<dyn std::any::Any + Send>, SerializationError> + Send + Sync>;

type DynMessageSerializerFactory =
    Box<dyn Fn() -> Box<dyn std::any::Any + Send + Sync> + Send + Sync>;

/// Enhanced serialization registry with automatic type discovery
pub struct SerializationRegistry {
    serializers: DashMap<(String, String), DynMessageSerializer>,
    serializer_factories: DashMap<std::any::TypeId, DynMessageSerializerFactory>,
}

impl Default for SerializationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SerializationRegistry {
    pub fn new() -> Self {
        Self {
            serializers: DashMap::new(),
            serializer_factories: DashMap::new(),
        }
    }

    pub fn add_serializer<T: 'static + Send + Sync, S: MessageSerializer<T> + Send + Sync + 'static>(
        &self,
        serializer: S,
    ) {
        let key = (
            serializer.type_name().to_string(),
            serializer.data_content_type().to_string(),
        );
        let deserializer: DynMessageSerializer =
            Box::new(move |payload: &[u8]| -> Result<Box<dyn std::any::Any + Send>, SerializationError> {
                let msg = serializer.deserialize(payload)?;
                Ok(Box::new(msg))
            });
        self.serializers.insert(key, deserializer);
    }

    /// Add a boxed serializer
    pub fn add_boxed_serializer<T: 'static + Send + Sync>(
        &self,
        serializer: Box<dyn MessageSerializer<T>>,
    ) {
        let key = (
            serializer.type_name().to_string(),
            serializer.data_content_type().to_string(),
        );
        let deserializer: DynMessageSerializer =
            Box::new(move |payload: &[u8]| -> Result<Box<dyn std::any::Any + Send>, SerializationError> {
                let msg = serializer.deserialize(payload)?;
                Ok(Box::new(msg))
            });
        self.serializers.insert(key, deserializer);
    }

    /// Add multiple serializers at once
    pub fn add_serializers<T: 'static + Send + Sync>(&self, serializers: Vec<Box<dyn MessageSerializer<T>>>) {
        for serializer in serializers {
            self.add_boxed_serializer(serializer);
        }
    }

    /// Register a factory for automatic serializer creation
    pub fn register_serializer_factory<T: 'static>(&self, factory: DynMessageSerializerFactory) {
        self.serializer_factories.insert(std::any::TypeId::of::<T>(), factory);
    }

    pub fn deserialize(
        &self,
        payload: &[u8],
        type_name: &str,
        data_content_type: &str,
    ) -> Result<Box<dyn std::any::Any + Send>, UnknownPayload> {
        let key = (type_name.to_string(), data_content_type.to_string());
        if let Some(deserializer) = self.serializers.get(&key) {
            deserializer(payload).map_err(|_e| UnknownPayload {
                type_name: type_name.to_string(),
                data_content_type: data_content_type.to_string(),
                payload: payload.to_vec(),
            })
        } else {
            Err(UnknownPayload {
                type_name: type_name.to_string(),
                data_content_type: data_content_type.to_string(),
                payload: payload.to_vec(),
            })
        }
    }

    /// Serialize a message with automatic type detection
    pub fn serialize<T: 'static>(
        &self,
        message: &T,
        type_name: &str,
        data_content_type: &str,
    ) -> Result<Vec<u8>, SerializationError> {
        let key = (type_name.to_string(), data_content_type.to_string());
        // This is a simplified implementation - in practice, you'd need more sophisticated
        // type handling for serialization
        Err(SerializationError::UnknownType(format!(
            "Serialization not implemented for type {} with content type {}",
            type_name, data_content_type
        )))
    }

    /// Check if a type/content type combination is registered
    pub fn is_registered(&self, type_name: &str, data_content_type: &str) -> bool {
        let key = (type_name.to_string(), data_content_type.to_string());
        self.serializers.contains_key(&key)
    }

    /// Get the type name for a message (simplified)
    pub fn type_name<T: 'static>(&self) -> String {
        std::any::type_name::<T>().to_string()
    }
}

/// Try to get known serializers for a type (equivalent to Python's try_get_known_serializers_for_type)
/// This is a simplified version that returns an empty vector by default
/// In practice, you would register serializers explicitly for each type
pub fn try_get_known_serializers_for_type<T: 'static + Send + Sync>() -> Vec<Box<dyn MessageSerializer<T>>> {
    // In Rust, we can't do runtime trait checking like Python
    // Instead, we rely on explicit registration or compile-time trait bounds
    // This function serves as a placeholder for the Python equivalent
    Vec::new()
}

/// Specialized function for dataclass-like types
pub fn try_get_dataclass_serializers<T: DataclassLike + 'static>() -> Vec<Box<dyn MessageSerializer<T>>> {
    let mut serializers: Vec<Box<dyn MessageSerializer<T>>> = Vec::new();
    let serializer = DataclassJsonSerializer::<T>::new(std::any::type_name::<T>());
    serializers.push(Box::new(serializer));
    serializers
}

/// Specialized function for base model-like types
pub fn try_get_base_model_serializers<T: BaseModelLike + 'static>() -> Vec<Box<dyn MessageSerializer<T>>> {
    let mut serializers: Vec<Box<dyn MessageSerializer<T>>> = Vec::new();
    let serializer = BaseModelJsonSerializer::<T>::new(std::any::type_name::<T>());
    serializers.push(Box::new(serializer));
    serializers
}

/// Specialized function for protobuf types
pub fn try_get_protobuf_serializers<T: Message + Default + 'static>() -> Vec<Box<dyn MessageSerializer<T>>> {
    let mut serializers: Vec<Box<dyn MessageSerializer<T>>> = Vec::new();
    let serializer = ProtobufSerializer::<T>::new(std::any::type_name::<T>());
    serializers.push(Box::new(serializer));
    serializers
}

/// Specialized function for basic serde types
pub fn try_get_json_serializers<T: Serialize + DeserializeOwned + Send + Sync + 'static>() -> Vec<Box<dyn MessageSerializer<T>>> {
    let mut serializers: Vec<Box<dyn MessageSerializer<T>>> = Vec::new();
    let serializer = JsonSerializer::<T>::new(std::any::type_name::<T>());
    serializers.push(Box::new(serializer));
    serializers
}

/// Convenience functions for global registry operations
pub fn add_message_serializer<T: 'static + Send + Sync, S: MessageSerializer<T> + Send + Sync + 'static>(
    serializer: S,
) {
    GLOBAL_REGISTRY.add_serializer(serializer);
}

pub fn add_message_serializers<T: 'static + Send + Sync>(
    serializers: Vec<Box<dyn MessageSerializer<T>>>,
) {
    GLOBAL_REGISTRY.add_serializers(serializers);
}

pub fn deserialize_message(
    payload: &[u8],
    type_name: &str,
    data_content_type: &str,
) -> Result<Box<dyn std::any::Any + Send>, UnknownPayload> {
    GLOBAL_REGISTRY.deserialize(payload, type_name, data_content_type)
}

pub fn is_message_type_registered(type_name: &str, data_content_type: &str) -> bool {
    GLOBAL_REGISTRY.is_registered(type_name, data_content_type)
}

/// Global serialization registry
static GLOBAL_REGISTRY: Lazy<SerializationRegistry> = Lazy::new(SerializationRegistry::new);

/// Enhanced serialization registry with type safety
pub struct TypedSerializationRegistry {
    type_mappings: DashMap<TypeId, Vec<Arc<dyn StdAny + Send + Sync>>>,
    name_mappings: DashMap<String, TypeId>,
}

impl TypedSerializationRegistry {
    pub fn new() -> Self {
        Self {
            type_mappings: DashMap::new(),
            name_mappings: DashMap::new(),
        }
    }

    /// Register a serializer for a specific type with type safety
    pub fn register_typed_serializer<T: 'static>(
        &self,
        serializer: Arc<dyn MessageSerializer<T>>,
    ) {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        // Store the serializer in a type-erased way
        let any_serializer: Arc<dyn StdAny + Send + Sync> = Arc::new(serializer);

        self.type_mappings
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(any_serializer);

        self.name_mappings.insert(type_name, type_id);
    }

    /// Get serializers for a specific type
    pub fn get_serializers_for_type<T: 'static>(&self) -> Vec<Arc<dyn MessageSerializer<T>>> {
        let type_id = TypeId::of::<T>();
        if let Some(serializers) = self.type_mappings.get(&type_id) {
            serializers
                .iter()
                .filter_map(|s| {
                    s.downcast_ref::<Arc<dyn MessageSerializer<T>>>()
                        .map(|arc_ref| arc_ref.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a type has registered serializers
    pub fn has_serializers_for_type<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.type_mappings.contains_key(&type_id)
    }
}

/// Global typed serialization registry
static TYPED_REGISTRY: Lazy<TypedSerializationRegistry> = Lazy::new(TypedSerializationRegistry::new);

/// Register a typed serializer globally
pub fn register_typed_serializer<T: 'static>(serializer: Arc<dyn MessageSerializer<T>>) {
    TYPED_REGISTRY.register_typed_serializer(serializer);
}

/// Get typed serializers for a type
pub fn get_typed_serializers<T: 'static>() -> Vec<Arc<dyn MessageSerializer<T>>> {
    TYPED_REGISTRY.get_serializers_for_type::<T>()
}

/// Check if a type has registered serializers
pub fn has_typed_serializers<T: 'static>() -> bool {
    TYPED_REGISTRY.has_serializers_for_type::<T>()
}

/// Helper macro for automatically registering serializers for a type
#[macro_export]
macro_rules! register_message_serializers {
    ($type:ty) => {
        {
            let serializers = try_get_known_serializers_for_type::<$type>();
            add_message_serializers(serializers);
        }
    };
}