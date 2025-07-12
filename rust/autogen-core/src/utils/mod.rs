//! Utility functions and helpers for autogen-core
//!
//! This module provides various utility functions that are used throughout
//! the autogen-core library, including JSON processing, type utilities,
//! and validation helpers.

pub mod json_to_pydantic;
pub mod load_json;

// Re-export commonly used items
pub use json_to_pydantic::{JSONSchemaToRust, SchemaConversionError};
pub use load_json::extract_json_from_str;

use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type utilities for runtime type checking and conversion
pub struct TypeUtils;

impl TypeUtils {
    /// Check if a value is of a specific type
    pub fn is_type<T: 'static>(value: &dyn Any) -> bool {
        value.type_id() == TypeId::of::<T>()
    }

    /// Convert a JSON value to a specific type if possible
    pub fn json_to_type<T>(value: &Value) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_value(value.clone())
    }

    /// Convert any serializable type to JSON
    pub fn type_to_json<T>(value: &T) -> Result<Value, serde_json::Error>
    where
        T: serde::Serialize,
    {
        serde_json::to_value(value)
    }
}

/// JSON utilities for common operations
pub struct JsonUtils;

impl JsonUtils {
    /// Merge two JSON objects recursively
    pub fn merge_objects(a: &mut Value, b: &Value) {
        match (a, b) {
            (Value::Object(ref mut a), Value::Object(b)) => {
                for (k, v) in b {
                    Self::merge_objects(a.entry(k.clone()).or_insert(Value::Null), v);
                }
            }
            (a, b) => {
                *a = b.clone();
            }
        }
    }

    /// Deep clone a JSON value
    pub fn deep_clone(value: &Value) -> Value {
        value.clone()
    }

    /// Flatten a nested JSON object with dot notation
    pub fn flatten_object(obj: &Value) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        Self::flatten_recursive(obj, String::new(), &mut result);
        result
    }

    fn flatten_recursive(value: &Value, prefix: String, result: &mut HashMap<String, Value>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_recursive(val, new_key, result);
                }
            }
            _ => {
                result.insert(prefix, value.clone());
            }
        }
    }

    /// Unflatten a flattened JSON object
    pub fn unflatten_object(flattened: &HashMap<String, Value>) -> Value {
        let mut result = serde_json::Map::new();

        for (key, value) in flattened {
            let parts: Vec<&str> = key.split('.').collect();
            Self::set_nested_value(&mut result, &parts, value.clone());
        }

        Value::Object(result)
    }

    fn set_nested_value(obj: &mut serde_json::Map<String, Value>, path: &[&str], value: Value) {
        if path.is_empty() {
            return;
        }

        if path.len() == 1 {
            obj.insert(path[0].to_string(), value);
            return;
        }

        let key = path[0];
        let nested = obj.entry(key.to_string()).or_insert_with(|| Value::Object(serde_json::Map::new()));

        if let Value::Object(ref mut nested_obj) = nested {
            Self::set_nested_value(nested_obj, &path[1..], value);
        }
    }
}