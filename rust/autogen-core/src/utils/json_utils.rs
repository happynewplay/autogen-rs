//! JSON utility functions for parsing and extracting JSON from strings.

use regex::Regex;
use serde_json::{Value, Map};
use thiserror::Error;

/// Errors that can occur during JSON extraction
#[derive(Debug, Error)]
pub enum JsonExtractionError {
    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    ParseError(#[from] serde_json::Error),
    /// Invalid language specified in code block
    #[error("Expected JSON object, but found language: {0}")]
    InvalidLanguage(String),
    /// No JSON found in the input
    #[error("No JSON content found in the input string")]
    NoJsonFound,
}

/// Extract JSON objects from a string. Supports backtick enclosed JSON objects.
///
/// This function can extract JSON from:
/// 1. Plain JSON strings
/// 2. JSON enclosed in markdown code blocks (```json ... ```)
/// 3. JSON in code blocks without language specification (``` ... ```)
///
/// # Arguments
/// * `content` - The string content to extract JSON from
///
/// # Returns
/// A vector of JSON values extracted from the string
///
/// # Errors
/// Returns `JsonExtractionError` if:
/// - The JSON is malformed
/// - A code block specifies a non-JSON language
/// - No JSON content is found
///
/// # Example
///
/// ```rust
/// use autogen_core::utils::extract_json_from_str;
/// use serde_json::json;
///
/// // Extract from plain JSON
/// let result = extract_json_from_str(r#"{"name": "test", "value": 42}"#).unwrap();
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0]["name"], "test");
///
/// // Extract from markdown code block
/// let markdown = r#"
/// Here's some JSON:
/// ```json
/// {"name": "test", "value": 42}
/// ```
/// "#;
/// let result = extract_json_from_str(markdown).unwrap();
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0]["name"], "test");
/// ```
pub fn extract_json_from_str(content: &str) -> Result<Vec<Value>, JsonExtractionError> {
    // Regex pattern to match code blocks with optional language specification
    let pattern = Regex::new(r"```(?:\s*([\w\+\-]+))?\n([\s\S]*?)```").unwrap();
    let matches: Vec<_> = pattern.captures_iter(content).collect();
    let mut results = Vec::new();

    // If no matches found, assume the entire content is a JSON object
    if matches.is_empty() {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            let json_value: Value = serde_json::from_str(trimmed)?;
            results.push(json_value);
        } else {
            return Err(JsonExtractionError::NoJsonFound);
        }
    } else {
        // Process each code block match
        for capture in matches {
            let language = capture.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let json_content = capture.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check if language is specified and is not JSON
            if !language.is_empty() && language.to_lowercase() != "json" {
                return Err(JsonExtractionError::InvalidLanguage(language.to_string()));
            }

            // Parse the JSON content
            let json_value: Value = serde_json::from_str(json_content)?;
            results.push(json_value);
        }
    }

    if results.is_empty() {
        Err(JsonExtractionError::NoJsonFound)
    } else {
        Ok(results)
    }
}

/// Extract a single JSON object from a string.
///
/// This is a convenience function that extracts the first JSON object found.
///
/// # Arguments
/// * `content` - The string content to extract JSON from
///
/// # Returns
/// The first JSON value found in the string
///
/// # Errors
/// Returns `JsonExtractionError` if no JSON is found or parsing fails
pub fn extract_single_json_from_str(content: &str) -> Result<Value, JsonExtractionError> {
    let results = extract_json_from_str(content)?;
    results.into_iter().next().ok_or(JsonExtractionError::NoJsonFound)
}

/// Check if a string contains valid JSON
///
/// # Arguments
/// * `content` - The string to check
///
/// # Returns
/// True if the string contains valid JSON, false otherwise
pub fn is_valid_json(content: &str) -> bool {
    serde_json::from_str::<Value>(content).is_ok()
}

/// Pretty print a JSON value as a formatted string
///
/// # Arguments
/// * `value` - The JSON value to format
///
/// # Returns
/// A pretty-printed JSON string
pub fn pretty_print_json(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Merge two JSON objects recursively
///
/// # Arguments
/// * `base` - The base JSON object
/// * `overlay` - The JSON object to merge on top
///
/// # Returns
/// A new JSON object with merged values
pub fn merge_json_objects(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            let mut result = base_map.clone();
            for (key, value) in overlay_map {
                if let Some(base_value) = result.get(key) {
                    result.insert(key.clone(), merge_json_objects(base_value, value));
                } else {
                    result.insert(key.clone(), value.clone());
                }
            }
            Value::Object(result)
        }
        _ => overlay.clone(),
    }
}

/// Flatten a nested JSON object into a flat structure with dot-separated keys
///
/// # Arguments
/// * `value` - The JSON value to flatten
/// * `prefix` - Optional prefix for keys
///
/// # Returns
/// A flattened JSON object
pub fn flatten_json(value: &Value, prefix: Option<&str>) -> Map<String, Value> {
    let mut result = Map::new();
    
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_key = if let Some(p) = prefix {
                    format!("{}.{}", p, key)
                } else {
                    key.clone()
                };
                
                match val {
                    Value::Object(_) => {
                        let flattened = flatten_json(val, Some(&new_key));
                        result.extend(flattened);
                    }
                    _ => {
                        result.insert(new_key, val.clone());
                    }
                }
            }
        }
        _ => {
            let key = prefix.unwrap_or("value").to_string();
            result.insert(key, value.clone());
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_plain_json() {
        let content = r#"{"name": "test", "value": 42}"#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test");
        assert_eq!(result[0]["value"], 42);
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let content = r#"
        Here's some JSON:
        ```json
        {"name": "test", "value": 42}
        ```
        "#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test");
    }

    #[test]
    fn test_extract_json_from_code_block_no_language() {
        let content = r#"
        ```
        {"name": "test", "value": 42}
        ```
        "#;
        let result = extract_json_from_str(content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test");
    }

    #[test]
    fn test_invalid_language_error() {
        let content = r#"
        ```python
        {"name": "test", "value": 42}
        ```
        "#;
        let result = extract_json_from_str(content);
        assert!(matches!(result, Err(JsonExtractionError::InvalidLanguage(_))));
    }

    #[test]
    fn test_merge_json_objects() {
        let base = json!({"a": 1, "b": {"c": 2}});
        let overlay = json!({"b": {"d": 3}, "e": 4});
        let result = merge_json_objects(&base, &overlay);
        
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"]["c"], 2);
        assert_eq!(result["b"]["d"], 3);
        assert_eq!(result["e"], 4);
    }

    #[test]
    fn test_flatten_json() {
        let value = json!({"a": {"b": {"c": 1}}, "d": 2});
        let result = flatten_json(&value, None);
        
        assert_eq!(result.get("a.b.c"), Some(&json!(1)));
        assert_eq!(result.get("d"), Some(&json!(2)));
    }
}
