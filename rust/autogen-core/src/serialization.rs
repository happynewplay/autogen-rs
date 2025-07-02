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

/// Versioned serialization format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationVersion {
    /// Version 1.0 - Basic JSON/Protobuf
    V1_0,
    /// Version 1.1 - Added compression support
    V1_1,
    /// Version 2.0 - Enhanced type safety
    V2_0,
}

impl SerializationVersion {
    /// Get the current version
    pub fn current() -> Self {
        Self::V2_0
    }

    /// Check if this version is compatible with another
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::V1_0, Self::V1_0) => true,
            (Self::V1_1, Self::V1_0 | Self::V1_1) => true,
            (Self::V1_1, Self::V2_0) => false, // V1.1 can't read V2.0
            (Self::V2_0, _) => true, // V2.0 is backward compatible
            (Self::V1_0, Self::V1_1 | Self::V2_0) => false, // V1.0 can't read newer formats
        }
    }

    /// Get version string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1_0 => "1.0",
            Self::V1_1 => "1.1",
            Self::V2_0 => "2.0",
        }
    }

    /// Parse version from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "1.0" => Ok(Self::V1_0),
            "1.1" => Ok(Self::V1_1),
            "2.0" => Ok(Self::V2_0),
            _ => Err(crate::AutoGenError::other(format!("Unknown serialization version: {}", s))),
        }
    }
}

/// Compression algorithm options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Gzip compression
    #[cfg(feature = "compression")]
    Gzip,
    /// LZ4 compression (fast)
    #[cfg(feature = "compression")]
    Lz4,
    /// Zstd compression (high ratio)
    #[cfg(feature = "compression")]
    Zstd,
}

impl CompressionAlgorithm {
    /// Get the algorithm identifier
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            #[cfg(feature = "compression")]
            Self::Gzip => "gzip",
            #[cfg(feature = "compression")]
            Self::Lz4 => "lz4",
            #[cfg(feature = "compression")]
            Self::Zstd => "zstd",
        }
    }

    /// Parse algorithm from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "none" => Ok(Self::None),
            #[cfg(feature = "compression")]
            "gzip" => Ok(Self::Gzip),
            #[cfg(feature = "compression")]
            "lz4" => Ok(Self::Lz4),
            #[cfg(feature = "compression")]
            "zstd" => Ok(Self::Zstd),
            _ => Err(crate::AutoGenError::other(format!("Unknown compression algorithm: {}", s))),
        }
    }

    /// Check if compression is beneficial for the given data size
    pub fn should_compress(&self, _data_size: usize) -> bool {
        match self {
            Self::None => false,
            #[cfg(feature = "compression")]
            Self::Gzip | Self::Lz4 | Self::Zstd => _data_size > 1024, // Only compress if > 1KB
        }
    }
}

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

impl std::fmt::Debug for JsonMessageSerializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonMessageSerializer")
            .field("type_registry_count", &self.type_registry.len())
            .finish()
    }
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
                .map_err(|e| AutoGenError::Serialization(crate::error::SerializationError::JsonSerialization {
                    details: e.to_string(),
                }))?;
            Ok((bytes, JSON_DATA_CONTENT_TYPE.to_string()))
        } else {
            Err(AutoGenError::other(format!("No serializer registered for type: {:?}", type_id)))
        }
    }

    fn deserialize(&self, data: &[u8], content_type: &str, type_id: TypeId) -> Result<Box<dyn Any + Send>> {
        if content_type != JSON_DATA_CONTENT_TYPE {
            return Err(AutoGenError::other(format!("Unsupported content type: {}", content_type)));
        }

        if let Some(serializer) = self.type_registry.get(&type_id) {
            let json_value: serde_json::Value = serde_json::from_slice(data)
                .map_err(|e| AutoGenError::Serialization(crate::error::SerializationError::JsonDeserialization {
                    details: e.to_string(),
                }))?;
            serializer.deserialize(&json_value)
        } else {
            Err(AutoGenError::other(format!("No deserializer registered for type: {:?}", type_id)))
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
                .map_err(|e| AutoGenError::Serialization(crate::error::SerializationError::JsonSerialization {
                    details: e.to_string(),
                }))
        } else {
            Err(AutoGenError::other("Type mismatch during serialization"))
        }
    }

    fn deserialize(&self, value: &serde_json::Value) -> Result<Box<dyn Any + Send>> {
        let typed_value: T = serde_json::from_value(value.clone())
            .map_err(|e| AutoGenError::Serialization(crate::error::SerializationError::JsonDeserialization {
                details: e.to_string(),
            }))?;
        Ok(Box::new(typed_value))
    }
}

/// Message envelope for serialized messages with version control and compression
///
/// This wraps a serialized message with metadata needed for routing and processing.
/// Supports versioning for backward compatibility and optional compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// The serialized message data (potentially compressed)
    pub data: Vec<u8>,

    /// Content type of the serialized data
    pub content_type: String,

    /// Type name of the original message
    pub type_name: String,

    /// Serialization format version
    #[serde(default = "default_version")]
    pub version: String,

    /// Compression algorithm used (if any)
    #[serde(default = "default_compression")]
    pub compression: String,

    /// Original data size (before compression)
    pub original_size: Option<usize>,

    /// Checksum for data integrity verification
    pub checksum: Option<String>,

    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Timestamp when the message was serialized
    pub serialized_at: Option<String>,
}

fn default_version() -> String {
    SerializationVersion::current().as_str().to_string()
}

fn default_compression() -> String {
    CompressionAlgorithm::None.as_str().to_string()
}

impl SerializedMessage {
    /// Create a new serialized message with current version
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
            version: default_version(),
            compression: default_compression(),
            original_size: None,
            checksum: None,
            metadata,
            serialized_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Create a new serialized message with compression
    pub fn new_with_compression(
        data: Vec<u8>,
        content_type: String,
        type_name: String,
        metadata: HashMap<String, serde_json::Value>,
        compression: CompressionAlgorithm,
    ) -> Result<Self> {
        let original_size = data.len();
        let (compressed_data, actual_compression) = Self::compress_data(data, compression)?;

        let checksum = Self::calculate_checksum(&compressed_data);

        Ok(Self {
            data: compressed_data,
            content_type,
            type_name,
            version: default_version(),
            compression: actual_compression.as_str().to_string(),
            original_size: Some(original_size),
            checksum: Some(checksum),
            metadata,
            serialized_at: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    /// Compress data using the specified algorithm
    fn compress_data(data: Vec<u8>, algorithm: CompressionAlgorithm) -> Result<(Vec<u8>, CompressionAlgorithm)> {
        if !algorithm.should_compress(data.len()) {
            return Ok((data, CompressionAlgorithm::None));
        }

        match algorithm {
            CompressionAlgorithm::None => Ok((data, CompressionAlgorithm::None)),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Gzip | CompressionAlgorithm::Lz4 | CompressionAlgorithm::Zstd => {
                // For now, just return uncompressed data if compression feature is not enabled
                // In a real implementation, you would add the compression libraries
                Ok((data, CompressionAlgorithm::None))
            }
        }
    }

    /// Calculate checksum for data integrity
    fn calculate_checksum(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Verify data integrity using checksum
    pub fn verify_integrity(&self) -> Result<bool> {
        if let Some(expected_checksum) = &self.checksum {
            let actual_checksum = Self::calculate_checksum(&self.data);
            Ok(actual_checksum == *expected_checksum)
        } else {
            Ok(true) // No checksum to verify
        }
    }

    /// Get the size of the serialized data
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the original size (before compression)
    pub fn original_size(&self) -> usize {
        self.original_size.unwrap_or(self.data.len())
    }

    /// Check if the message is compressed
    pub fn is_compressed(&self) -> bool {
        self.compression != CompressionAlgorithm::None.as_str()
    }

    /// Check if the message is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the serialization version
    pub fn version(&self) -> Result<SerializationVersion> {
        SerializationVersion::from_str(&self.version)
    }

    /// Check version compatibility
    pub fn is_compatible_with_version(&self, version: &SerializationVersion) -> bool {
        if let Ok(msg_version) = self.version() {
            version.is_compatible_with(&msg_version)
        } else {
            false
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

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Remove metadata entry
    pub fn remove_metadata(&mut self, key: &str) -> Option<serde_json::Value> {
        self.metadata.remove(key)
    }

    /// Decompress the message data if it's compressed
    pub fn decompress(&self) -> Result<Vec<u8>> {
        if !self.is_compressed() {
            return Ok(self.data.clone());
        }

        // For now, just return the data as-is since compression is not implemented
        // In a real implementation, you would decompress based on self.compression
        #[cfg(feature = "compression")]
        {
            // TODO: Implement actual decompression based on algorithm
            Ok(self.data.clone())
        }
        #[cfg(not(feature = "compression"))]
        {
            Ok(self.data.clone())
        }
    }

    /// Get the compression ratio (original_size / compressed_size)
    pub fn compression_ratio(&self) -> Option<f64> {
        if let Some(original_size) = self.original_size {
            if self.data.len() > 0 {
                Some(original_size as f64 / self.data.len() as f64)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Advanced message serializer with version control and compression
#[cfg(any(feature = "json", feature = "protobuf"))]
#[derive(Debug)]
pub struct AdvancedMessageSerializer {
    /// Base JSON serializer
    json_serializer: JsonMessageSerializer,
    /// Default compression algorithm
    default_compression: CompressionAlgorithm,
    /// Compression threshold (bytes)
    compression_threshold: usize,
    /// Enable integrity checking
    enable_checksums: bool,
}

#[cfg(any(feature = "json", feature = "protobuf"))]
impl AdvancedMessageSerializer {
    /// Create a new advanced serializer
    pub fn new() -> Self {
        Self {
            json_serializer: JsonMessageSerializer::new(),
            default_compression: CompressionAlgorithm::None,
            compression_threshold: 1024, // 1KB
            enable_checksums: true,
        }
    }

    /// Create with custom compression settings
    pub fn with_compression(compression: CompressionAlgorithm, threshold: usize) -> Self {
        Self {
            json_serializer: JsonMessageSerializer::new(),
            default_compression: compression,
            compression_threshold: threshold,
            enable_checksums: true,
        }
    }

    /// Set compression algorithm
    pub fn set_compression(&mut self, compression: CompressionAlgorithm) {
        self.default_compression = compression;
    }

    /// Set compression threshold
    pub fn set_compression_threshold(&mut self, threshold: usize) {
        self.compression_threshold = threshold;
    }

    /// Enable or disable checksums
    pub fn set_checksums_enabled(&mut self, enabled: bool) {
        self.enable_checksums = enabled;
    }

    /// Serialize a message with advanced features
    pub fn serialize_advanced(&self, message: &dyn Any) -> Result<SerializedMessage> {
        // First serialize using the base serializer
        let (data, content_type) = self.json_serializer.serialize(message)?;
        let type_name = format!("{:?}", message.type_id());

        let mut metadata = HashMap::new();
        metadata.insert("serializer".to_string(), serde_json::Value::String("advanced".to_string()));

        // Determine if we should compress
        let should_compress = self.default_compression != CompressionAlgorithm::None
            && data.len() >= self.compression_threshold;

        if should_compress {
            SerializedMessage::new_with_compression(
                data,
                content_type,
                type_name,
                metadata,
                self.default_compression,
            )
        } else {
            let mut msg = SerializedMessage::new(data, content_type, type_name, metadata);

            // Add checksum if enabled
            if self.enable_checksums {
                let checksum = SerializedMessage::calculate_checksum(&msg.data);
                msg.checksum = Some(checksum);
            }

            Ok(msg)
        }
    }

    /// Deserialize a message with version compatibility checking
    pub fn deserialize_advanced(&self, msg: &SerializedMessage, type_id: TypeId) -> Result<Box<dyn Any + Send>> {
        // Check version compatibility
        let current_version = SerializationVersion::current();
        if !msg.is_compatible_with_version(&current_version) {
            return Err(crate::AutoGenError::other(format!(
                "Incompatible serialization version: {} (current: {})",
                msg.version, current_version.as_str()
            )));
        }

        // Verify integrity if checksum is present
        if !msg.verify_integrity()? {
            return Err(crate::AutoGenError::other("Message integrity check failed"));
        }

        // Get the actual data (decompress if needed)
        let data = if msg.is_compressed() {
            msg.decompress()?
        } else {
            msg.data().to_vec()
        };

        // Deserialize using the base serializer
        self.json_serializer.deserialize(&data, msg.content_type(), type_id)
    }

    /// Get compression statistics for a message
    pub fn get_compression_stats(&self, msg: &SerializedMessage) -> CompressionStats {
        CompressionStats {
            original_size: msg.original_size(),
            compressed_size: msg.size(),
            compression_ratio: msg.compression_ratio().unwrap_or(1.0),
            algorithm: msg.compression.clone(),
            is_compressed: msg.is_compressed(),
        }
    }
}

#[cfg(any(feature = "json", feature = "protobuf"))]
impl Default for AdvancedMessageSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Original data size
    pub original_size: usize,
    /// Compressed data size
    pub compressed_size: usize,
    /// Compression ratio (compressed/original)
    pub compression_ratio: f64,
    /// Compression algorithm used
    pub algorithm: String,
    /// Whether the data is compressed
    pub is_compressed: bool,
}

impl CompressionStats {
    /// Get space saved in bytes
    pub fn space_saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }

    /// Get space saved as percentage
    pub fn space_saved_percent(&self) -> f64 {
        if self.original_size > 0 {
            (1.0 - self.compression_ratio) * 100.0
        } else {
            0.0
        }
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
