use image::{DynamicImage, ImageError, ImageFormat};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::Cursor;
use std::path::Path;

/// Represents an image.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Image {
    #[serde(with = "dynamic_image_serializer")]
    image: DynamicImage,
}

mod dynamic_image_serializer {
    use super::*;
    use serde::ser::Error;

    pub fn serialize<S>(image: &DynamicImage, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, ImageFormat::Png)
            .map_err(S::Error::custom)?;
        let base64_str = base64::encode(buffer.get_ref());
        serializer.serialize_str(&base64_str)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DynamicImage, D::Error>
    where
        D: Deserializer<'de>,
    {
        let base64_str = String::deserialize(deserializer)?;
        let decoded = base64::decode(&base64_str).map_err(serde::de::Error::custom)?;
        image::load_from_memory(&decoded).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Image {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Image::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Image {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Image::deserialize(deserializer)
    }
}

impl Image {
    /// Creates an `Image` from a `DynamicImage`.
    pub fn from_dynamic_image(image: DynamicImage) -> Self {
        Self { image }
    }

    /// Creates an `Image` from a URI.
    pub fn from_uri(uri: &str) -> Result<Self, String> {
        if !uri.starts_with("data:image/") || !uri.contains(";base64,") {
            return Err("Invalid URI format. It should be a base64 encoded image URI.".to_string());
        }
        let parts: Vec<_> = uri.split(";base64,").collect();
        let base64_data = parts[1];
        let decoded = base64::decode(base64_data).map_err(|e| e.to_string())?;
        let image = image::load_from_memory(&decoded).map_err(|e| e.to_string())?;
        Ok(Self { image })
    }

    /// Converts the image to a base64 string (PNG format).
    pub fn to_base64(&self) -> Result<String, ImageError> {
        let mut buffer = Cursor::new(Vec::new());
        self.image.write_to(&mut buffer, ImageFormat::Png)?;
        Ok(base64::encode(buffer.get_ref()))
    }

    /// Creates an `Image` from a file path.
    pub fn from_file(file_path: &Path) -> Result<Self, ImageError> {
        let image = image::open(file_path)?;
        Ok(Self { image })
    }

    /// Returns the data URI of the image.
    pub fn data_uri(&self) -> Result<String, ImageError> {
        let base64_image = self.to_base64()?;
        let mime_type = "image/png"; // For simplicity, always use png
        Ok(format!("data:{};base64,{}", mime_type, base64_image))
    }
}