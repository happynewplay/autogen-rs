//! Base memory traits and types for the memory system.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{CancellationToken, error::Result};

/// Supported MIME types for memory content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryMimeType {
    /// Plain text content
    #[serde(rename = "text/plain")]
    Text,
    /// JSON content
    #[serde(rename = "application/json")]
    Json,
    /// Markdown content
    #[serde(rename = "text/markdown")]
    Markdown,
    /// Image content (any image format)
    #[serde(rename = "image/*")]
    Image,
    /// Binary content
    #[serde(rename = "application/octet-stream")]
    Binary,
    /// Custom MIME type
    #[serde(untagged)]
    Custom(String),
}

impl MemoryMimeType {
    /// Get the string representation of the MIME type
    pub fn as_str(&self) -> &str {
        match self {
            MemoryMimeType::Text => "text/plain",
            MemoryMimeType::Json => "application/json",
            MemoryMimeType::Markdown => "text/markdown",
            MemoryMimeType::Image => "image/*",
            MemoryMimeType::Binary => "application/octet-stream",
            MemoryMimeType::Custom(s) => s,
        }
    }

    /// Check if this MIME type is compatible with the given content type
    pub fn is_compatible_with(&self, content: &ContentType) -> bool {
        match (self, content) {
            (MemoryMimeType::Text | MemoryMimeType::Markdown, ContentType::Text(_)) => true,
            (MemoryMimeType::Json, ContentType::Json(_)) => true,
            (MemoryMimeType::Image, ContentType::Image { .. }) => true,
            (MemoryMimeType::Binary, ContentType::Binary(_)) => true,
            (MemoryMimeType::Custom(_), _) => true, // Allow custom types with any content
            _ => false,
        }
    }
}

/// Content type that can be stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ContentType {
    /// String content with size limit
    #[serde(rename = "text")]
    Text(String),
    /// Binary content with size limit
    #[serde(rename = "binary")]
    Binary(Vec<u8>),
    /// JSON object content
    #[serde(rename = "json")]
    Json(serde_json::Value),
    /// Image content with metadata
    #[serde(rename = "image")]
    Image {
        /// Base64 encoded image data
        data: String,
        /// Image format (png, jpg, etc.)
        format: String,
        /// Image dimensions (width, height)
        dimensions: Option<(u32, u32)>,
    },
}

/// Content size limits for memory items
#[derive(Debug, Clone)]
pub struct ContentLimits {
    /// Maximum size for text content (in characters)
    pub max_text_size: usize,
    /// Maximum size for binary content (in bytes)
    pub max_binary_size: usize,
    /// Maximum size for JSON content (in bytes when serialized)
    pub max_json_size: usize,
    /// Maximum size for image content (in bytes)
    pub max_image_size: usize,
}

impl Default for ContentLimits {
    fn default() -> Self {
        Self {
            max_text_size: 1_000_000,      // 1MB of text
            max_binary_size: 10_000_000,   // 10MB binary
            max_json_size: 1_000_000,      // 1MB JSON
            max_image_size: 5_000_000,     // 5MB images
        }
    }
}

impl ContentType {
    /// Validate content against size limits
    pub fn validate(&self, limits: &ContentLimits) -> crate::Result<()> {
        match self {
            ContentType::Text(text) => {
                if text.len() > limits.max_text_size {
                    return Err(crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "text")
                            .with_detail("size", text.len().to_string())
                            .with_detail("limit", limits.max_text_size.to_string()),
                        format!("Text content size {} exceeds limit {}", text.len(), limits.max_text_size)
                    ));
                }
            }
            ContentType::Binary(data) => {
                if data.len() > limits.max_binary_size {
                    return Err(crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "binary")
                            .with_detail("size", data.len().to_string())
                            .with_detail("limit", limits.max_binary_size.to_string()),
                        format!("Binary content size {} exceeds limit {}", data.len(), limits.max_binary_size)
                    ));
                }
            }
            ContentType::Json(value) => {
                let serialized_size = serde_json::to_string(value)
                    .map_err(|e| crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "json"),
                        format!("Failed to serialize JSON content: {}", e)
                    ))?
                    .len();
                if serialized_size > limits.max_json_size {
                    return Err(crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "json")
                            .with_detail("size", serialized_size.to_string())
                            .with_detail("limit", limits.max_json_size.to_string()),
                        format!("JSON content size {} exceeds limit {}", serialized_size, limits.max_json_size)
                    ));
                }
            }
            ContentType::Image { data, format, .. } => {
                // Validate base64 data
                let decoded_size = data.len() * 3 / 4; // Approximate decoded size
                if decoded_size > limits.max_image_size {
                    return Err(crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "image")
                            .with_detail("format", format)
                            .with_detail("size", decoded_size.to_string())
                            .with_detail("limit", limits.max_image_size.to_string()),
                        format!("Image content size {} exceeds limit {}", decoded_size, limits.max_image_size)
                    ));
                }

                // Validate image format
                if !["png", "jpg", "jpeg", "gif", "webp", "bmp"].contains(&format.to_lowercase().as_str()) {
                    return Err(crate::AutoGenError::validation(
                        crate::error::ErrorContext::new("content_validation")
                            .with_detail("content_type", "image")
                            .with_detail("format", format),
                        format!("Unsupported image format: {}", format)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get the approximate size of the content in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            ContentType::Text(text) => text.len(),
            ContentType::Binary(data) => data.len(),
            ContentType::Json(value) => {
                serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
            }
            ContentType::Image { data, .. } => data.len() * 3 / 4, // Approximate decoded size
        }
    }
}

/// A memory content item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    /// The content of the memory item
    pub content: ContentType,
    /// The MIME type of the memory content
    pub mime_type: MemoryMimeType,
    /// Metadata associated with the memory item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl MemoryContent {
    /// Create a new text memory content
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: ContentType::Text(content.into()),
            mime_type: MemoryMimeType::Text,
            metadata: None,
        }
    }

    /// Create a new JSON memory content
    pub fn json(content: serde_json::Value) -> Self {
        Self {
            content: ContentType::Json(content),
            mime_type: MemoryMimeType::Json,
            metadata: None,
        }
    }

    /// Create a new markdown memory content
    pub fn markdown(content: impl Into<String>) -> Self {
        Self {
            content: ContentType::Text(content.into()),
            mime_type: MemoryMimeType::Markdown,
            metadata: None,
        }
    }

    /// Create a new image memory content
    pub fn image(data: impl Into<String>, format: impl Into<String>, dimensions: Option<(u32, u32)>) -> Self {
        Self {
            content: ContentType::Image {
                data: data.into(),
                format: format.into(),
                dimensions,
            },
            mime_type: MemoryMimeType::Image,
            metadata: None,
        }
    }

    /// Create a new binary memory content
    pub fn binary(data: Vec<u8>) -> Self {
        Self {
            content: ContentType::Binary(data),
            mime_type: MemoryMimeType::Binary,
            metadata: None,
        }
    }

    /// Add metadata to this memory content
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Validate this memory content against limits
    pub fn validate(&self, limits: &ContentLimits) -> crate::Result<()> {
        // Validate content size
        self.content.validate(limits)?;

        // Validate MIME type compatibility
        if !self.mime_type.is_compatible_with(&self.content) {
            return Err(crate::AutoGenError::validation(
                crate::error::ErrorContext::new("memory_content_validation")
                    .with_detail("mime_type", self.mime_type.as_str())
                    .with_detail("content_type", format!("{:?}", self.content)),
                format!("MIME type {} is not compatible with content type", self.mime_type.as_str())
            ));
        }

        Ok(())
    }

    /// Get the size of this memory content in bytes
    pub fn size_bytes(&self) -> usize {
        self.content.size_bytes()
    }
}

/// Result of a memory query operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    /// The memory content results
    pub results: Vec<MemoryContent>,
}

impl MemoryQueryResult {
    /// Create a new empty query result
    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Create a new query result with the given results
    pub fn new(results: Vec<MemoryContent>) -> Self {
        Self { results }
    }
}

/// Result of a memory update_context operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContextResult {
    /// The memories that were used to update the context
    pub memories: MemoryQueryResult,
}

impl UpdateContextResult {
    /// Create a new update context result
    pub fn new(memories: MemoryQueryResult) -> Self {
        Self { memories }
    }

    /// Create an empty update context result
    pub fn empty() -> Self {
        Self {
            memories: MemoryQueryResult::empty(),
        }
    }
}

/// Query input for memory operations
#[derive(Debug, Clone)]
pub enum MemoryQuery {
    /// String query
    Text(String),
    /// Memory content query
    Content(MemoryContent),
}

impl From<String> for MemoryQuery {
    fn from(s: String) -> Self {
        MemoryQuery::Text(s)
    }
}

impl From<&str> for MemoryQuery {
    fn from(s: &str) -> Self {
        MemoryQuery::Text(s.to_string())
    }
}

impl From<MemoryContent> for MemoryQuery {
    fn from(content: MemoryContent) -> Self {
        MemoryQuery::Content(content)
    }
}

/// Protocol defining the interface for memory implementations.
///
/// A memory is the storage for data that can be used to enrich or modify the model context.
/// A memory implementation can use any storage mechanism, such as a list, a database, or a file system.
/// It can also use any retrieval mechanism, such as vector search or text search.
/// It is up to the implementation to decide how to store and retrieve data.
///
/// It is also a memory implementation's responsibility to update the model context
/// with relevant memory content based on the current model context and querying the memory store.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Get the content limits for this memory implementation
    fn content_limits(&self) -> ContentLimits {
        ContentLimits::default()
    }
    /// Update the provided model context using relevant memory content.
    ///
    /// # Arguments
    /// * `model_context` - The context to update
    ///
    /// # Returns
    /// UpdateContextResult containing relevant memories
    async fn update_context(
        &self,
        model_context: &mut dyn crate::model_context::ChatCompletionContext,
    ) -> Result<UpdateContextResult>;

    /// Query the memory store and return relevant entries.
    ///
    /// # Arguments
    /// * `query` - Query content item
    /// * `cancellation_token` - Optional token to cancel operation
    ///
    /// # Returns
    /// MemoryQueryResult containing memory entries
    async fn query(
        &self,
        query: impl Into<MemoryQuery> + Send,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<MemoryQueryResult>;

    /// Add a new content to memory.
    ///
    /// # Arguments
    /// * `content` - The memory content to add (will be validated)
    /// * `cancellation_token` - Optional token to cancel operation
    async fn add(
        &self,
        content: MemoryContent,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()> {
        // Validate content before adding
        content.validate(&self.content_limits())?;
        self.add_validated(content, cancellation_token).await
    }

    /// Add pre-validated content to memory (internal method)
    ///
    /// This method should be implemented by memory implementations
    /// and assumes the content has already been validated.
    async fn add_validated(
        &self,
        content: MemoryContent,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<()>;

    /// Clear all entries from memory.
    async fn clear(&self) -> Result<()>;

    /// Clean up any resources used by the memory implementation.
    async fn close(&self) -> Result<()>;
}
