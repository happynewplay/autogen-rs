//! Schema utilities for JSON Schema validation and processing.

use std::collections::HashMap;
use jsonschema::JSONSchema;
use serde_json::{Value, Map};
use thiserror::Error;

/// Errors that can occur during schema processing
#[derive(Debug, Error)]
pub enum SchemaToStructError {
    /// JSON Schema compilation error
    #[error("JSON Schema compilation error: {0}")]
    SchemaCompilationError(String),
    /// Unsupported schema feature
    #[error("Unsupported schema feature: {0}")]
    UnsupportedFeature(String),
    /// Invalid schema format
    #[error("Invalid schema format: {0}")]
    InvalidSchema(String),
    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),
}

/// Errors that can occur during JSON validation
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Schema compilation error
    #[error("Schema compilation error: {0}")]
    SchemaError(String),
    /// Validation failed
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// JSON Schema processor for converting schemas to Rust-compatible structures
#[derive(Debug)]
pub struct JsonSchemaProcessor {
    /// Cache of compiled schemas
    schema_cache: HashMap<String, JSONSchema>,
}

impl JsonSchemaProcessor {
    /// Create a new JSON Schema processor
    pub fn new() -> Self {
        Self {
            schema_cache: HashMap::new(),
        }
    }

    /// Process a JSON Schema and extract type information
    ///
    /// # Arguments
    /// * `schema` - The JSON Schema to process
    /// * `name` - The name for the generated structure
    ///
    /// # Returns
    /// A map containing field information for the schema
    pub fn process_schema(
        &mut self,
        schema: &Value,
        name: &str,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        self.process_schema_recursive(schema, name, schema)
    }

    fn process_schema_recursive(
        &mut self,
        schema: &Value,
        name: &str,
        root_schema: &Value,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        // Handle $ref references
        if let Some(ref_value) = schema.get("$ref") {
            if let Some(ref_str) = ref_value.as_str() {
                return self.resolve_reference(ref_str, root_schema, name);
            }
        }

        // Handle allOf composition
        if let Some(all_of) = schema.get("allOf") {
            return self.handle_all_of(all_of, name, root_schema);
        }

        // Handle oneOf/anyOf unions
        if let Some(one_of) = schema.get("oneOf") {
            return self.handle_union(one_of, name, root_schema, "oneOf");
        }
        if let Some(any_of) = schema.get("anyOf") {
            return self.handle_union(any_of, name, root_schema, "anyOf");
        }

        // Handle object types
        if let Some(schema_type) = schema.get("type") {
            match schema_type.as_str() {
                Some("object") => self.handle_object_schema(schema, name, root_schema),
                Some("array") => self.handle_array_schema(schema, name, root_schema),
                Some("string") => Ok(self.handle_string_schema(schema)),
                Some("number") | Some("integer") => Ok(self.handle_number_schema(schema)),
                Some("boolean") => Ok(SchemaInfo::primitive("bool")),
                Some("null") => Ok(SchemaInfo::primitive("Option<()>")),
                _ => Err(SchemaToStructError::UnsupportedFeature(
                    format!("Unsupported type: {:?}", schema_type)
                )),
            }
        } else {
            // No type specified, try to infer from properties
            if schema.get("properties").is_some() {
                self.handle_object_schema(schema, name, root_schema)
            } else {
                Ok(SchemaInfo::primitive("serde_json::Value"))
            }
        }
    }

    fn resolve_reference(
        &mut self,
        ref_str: &str,
        root_schema: &Value,
        name: &str,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        // Simple $ref resolution for #/definitions/... or #/$defs/...
        if ref_str.starts_with("#/") {
            let path_parts: Vec<&str> = ref_str[2..].split('/').collect();
            let mut current = root_schema;
            
            for part in path_parts {
                current = current.get(part).ok_or_else(|| {
                    SchemaToStructError::InvalidSchema(
                        format!("Reference not found: {}", ref_str)
                    )
                })?;
            }
            
            self.process_schema_recursive(current, name, root_schema)
        } else {
            Err(SchemaToStructError::UnsupportedFeature(
                format!("External references not supported: {}", ref_str)
            ))
        }
    }

    fn handle_all_of(
        &mut self,
        all_of: &Value,
        name: &str,
        root_schema: &Value,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        if let Some(schemas) = all_of.as_array() {
            let mut merged_properties = Map::new();
            let mut merged_required = Vec::new();

            for schema in schemas {
                let info = self.process_schema_recursive(schema, name, root_schema)?;
                if let SchemaInfo::Object { properties, required, .. } = info {
                    merged_properties.extend(properties);
                    merged_required.extend(required);
                }
            }

            Ok(SchemaInfo::Object {
                name: name.to_string(),
                properties: merged_properties,
                required: merged_required,
            })
        } else {
            Err(SchemaToStructError::InvalidSchema("allOf must be an array".to_string()))
        }
    }

    fn handle_union(
        &mut self,
        union_schemas: &Value,
        name: &str,
        root_schema: &Value,
        union_type: &str,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        if let Some(schemas) = union_schemas.as_array() {
            let mut variants = Vec::new();
            
            for (i, schema) in schemas.iter().enumerate() {
                let variant_name = format!("{}Variant{}", name, i);
                let variant_info = self.process_schema_recursive(schema, &variant_name, root_schema)?;
                variants.push(variant_info);
            }

            Ok(SchemaInfo::Union {
                name: name.to_string(),
                variants,
                union_type: union_type.to_string(),
            })
        } else {
            Err(SchemaToStructError::InvalidSchema(
                format!("{} must be an array", union_type)
            ))
        }
    }

    fn handle_object_schema(
        &mut self,
        schema: &Value,
        name: &str,
        root_schema: &Value,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        let properties = schema.get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();

        let required = schema.get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let mut processed_properties = Map::new();
        
        for (prop_name, prop_schema) in properties {
            let field_name = format!("{}_{}", name, prop_name);
            let field_info = self.process_schema_recursive(&prop_schema, &field_name, root_schema)?;
            processed_properties.insert(prop_name, Value::String(field_info.to_type_string()));
        }

        Ok(SchemaInfo::Object {
            name: name.to_string(),
            properties: processed_properties,
            required,
        })
    }

    fn handle_array_schema(
        &mut self,
        schema: &Value,
        name: &str,
        root_schema: &Value,
    ) -> Result<SchemaInfo, SchemaToStructError> {
        if let Some(items_schema) = schema.get("items") {
            let item_name = format!("{}Item", name);
            let item_info = self.process_schema_recursive(items_schema, &item_name, root_schema)?;
            Ok(SchemaInfo::Array {
                item_type: Box::new(item_info),
            })
        } else {
            Ok(SchemaInfo::Array {
                item_type: Box::new(SchemaInfo::primitive("serde_json::Value")),
            })
        }
    }

    fn handle_string_schema(&self, schema: &Value) -> SchemaInfo {
        if let Some(enum_values) = schema.get("enum") {
            if let Some(values) = enum_values.as_array() {
                let variants: Vec<String> = values
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                return SchemaInfo::Enum { variants };
            }
        }

        // Check for format constraints
        if let Some(format) = schema.get("format").and_then(|f| f.as_str()) {
            match format {
                "email" => SchemaInfo::primitive("String"), // Could use email validation type
                "uri" | "url" => SchemaInfo::primitive("String"), // Could use URL type
                "date-time" => SchemaInfo::primitive("chrono::DateTime<chrono::Utc>"),
                "date" => SchemaInfo::primitive("chrono::NaiveDate"),
                "time" => SchemaInfo::primitive("chrono::NaiveTime"),
                "uuid" => SchemaInfo::primitive("uuid::Uuid"),
                _ => SchemaInfo::primitive("String"),
            }
        } else {
            SchemaInfo::primitive("String")
        }
    }

    fn handle_number_schema(&self, schema: &Value) -> SchemaInfo {
        let schema_type = schema.get("type").and_then(|t| t.as_str()).unwrap_or("number");
        
        match schema_type {
            "integer" => {
                // Check for format constraints
                if let Some(format) = schema.get("format").and_then(|f| f.as_str()) {
                    match format {
                        "int32" => SchemaInfo::primitive("i32"),
                        "int64" => SchemaInfo::primitive("i64"),
                        _ => SchemaInfo::primitive("i64"),
                    }
                } else {
                    SchemaInfo::primitive("i64")
                }
            }
            "number" => {
                if let Some(format) = schema.get("format").and_then(|f| f.as_str()) {
                    match format {
                        "float" => SchemaInfo::primitive("f32"),
                        "double" => SchemaInfo::primitive("f64"),
                        _ => SchemaInfo::primitive("f64"),
                    }
                } else {
                    SchemaInfo::primitive("f64")
                }
            }
            _ => SchemaInfo::primitive("f64"),
        }
    }
}

impl Default for JsonSchemaProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Information extracted from a JSON Schema
#[derive(Debug, Clone)]
pub enum SchemaInfo {
    /// Primitive type
    Primitive { type_name: String },
    /// Object type with properties
    Object {
        name: String,
        properties: Map<String, Value>,
        required: Vec<String>,
    },
    /// Array type
    Array { item_type: Box<SchemaInfo> },
    /// Enum type
    Enum { variants: Vec<String> },
    /// Union type (oneOf/anyOf)
    Union {
        name: String,
        variants: Vec<SchemaInfo>,
        union_type: String,
    },
}

impl SchemaInfo {
    /// Create a primitive schema info
    pub fn primitive(type_name: &str) -> Self {
        Self::Primitive {
            type_name: type_name.to_string(),
        }
    }

    /// Convert to a Rust type string
    pub fn to_type_string(&self) -> String {
        match self {
            SchemaInfo::Primitive { type_name } => type_name.clone(),
            SchemaInfo::Object { name, .. } => name.clone(),
            SchemaInfo::Array { item_type } => format!("Vec<{}>", item_type.to_type_string()),
            SchemaInfo::Enum { .. } => "String".to_string(), // Could generate enum type
            SchemaInfo::Union { name, .. } => name.clone(),
        }
    }
}

/// Convert a JSON Schema to a Rust struct-like representation
///
/// # Arguments
/// * `schema` - The JSON Schema to convert
/// * `name` - The name for the generated structure
///
/// # Returns
/// Schema information that can be used to generate Rust code
pub fn schema_to_struct(schema: &Value, name: &str) -> Result<SchemaInfo, SchemaToStructError> {
    let mut processor = JsonSchemaProcessor::new();
    processor.process_schema(schema, name)
}

/// Validate JSON data against a JSON Schema
///
/// # Arguments
/// * `schema` - The JSON Schema to validate against
/// * `data` - The JSON data to validate
///
/// # Returns
/// Ok(()) if validation succeeds, Err with validation details if it fails
pub fn validate_json_against_schema(schema: &Value, data: &Value) -> Result<(), ValidationError> {
    let compiled_schema = JSONSchema::compile(schema)
        .map_err(|e| ValidationError::SchemaError(e.to_string()))?;

    let result = compiled_schema.validate(data);
    if let Err(errors) = result {
        let error_messages: Vec<String> = errors
            .map(|e| e.to_string())
            .collect();
        return Err(ValidationError::ValidationFailed(error_messages.join("; ")));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_object_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        let result = schema_to_struct(&schema, "Person").unwrap();
        if let SchemaInfo::Object { name, properties, required } = result {
            assert_eq!(name, "Person");
            assert!(properties.contains_key("name"));
            assert!(properties.contains_key("age"));
            assert_eq!(required, vec!["name"]);
        } else {
            panic!("Expected object schema");
        }
    }

    #[test]
    fn test_validate_json() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });

        let valid_data = json!({"name": "Alice", "age": 30});
        assert!(validate_json_against_schema(&schema, &valid_data).is_ok());

        let invalid_data = json!({"age": 30}); // missing required "name"
        assert!(validate_json_against_schema(&schema, &invalid_data).is_err());
    }
}
