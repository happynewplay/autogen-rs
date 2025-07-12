use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug)]
pub enum SchemaConversionError {
    ReferenceNotFound(String),
    FormatNotSupported(String),
    UnsupportedKeyword(String),
}

impl std::fmt::Display for SchemaConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaConversionError::ReferenceNotFound(msg) => write!(f, "ReferenceNotFound: {}", msg),
            SchemaConversionError::FormatNotSupported(msg) => write!(f, "FormatNotSupported: {}", msg),
            SchemaConversionError::UnsupportedKeyword(msg) => write!(f, "UnsupportedKeyword: {}", msg),
        }
    }
}

impl Error for SchemaConversionError {}

// This is a simplified translation. Rust's static typing makes it hard to dynamically
// create types with validation like Pydantic. This implementation will focus on
// parsing the structure of the JSON schema.

pub struct JSONSchemaToRust;

impl JSONSchemaToRust {
    pub fn new() -> Self {
        JSONSchemaToRust
    }

    pub fn schema_to_rust_struct_string(&self, schema: &Value, model_name: &str) -> Result<String, Box<dyn Error>> {
        let mut struct_defs = String::new();
        self.generate_struct(schema, model_name, &mut struct_defs)?;
        Ok(struct_defs)
    }

    fn generate_struct(&self, schema: &Value, struct_name: &str, struct_defs: &mut String) -> Result<(), Box<dyn Error>> {
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            let mut fields = String::new();
            for (key, value) in properties {
                let field_type = self.json_type_to_rust_type(value, struct_name, struct_defs)?;
                fields.push_str(&format!("    pub {}: {},\n", key, field_type));
            }
            struct_defs.push_str(&format!("#[derive(Debug, serde::Serialize, serde::Deserialize)]\npub struct {} {{\n{}}}\n\n", struct_name, fields));
        }
        Ok(())
    }

    fn json_type_to_rust_type(&self, value: &Value, parent_name: &str, struct_defs: &mut String) -> Result<String, Box<dyn Error>> {
        if let Some(ref_path) = value.get("$ref").and_then(|r| r.as_str()) {
            let ref_name = ref_path.split('/').last().unwrap_or_default();
            return Ok(ref_name.to_string());
        }

        if let Some(json_type) = value.get("type").and_then(|t| t.as_str()) {
            match json_type {
                "string" => Ok("String".to_string()),
                "integer" => Ok("i64".to_string()),
                "number" => Ok("f64".to_string()),
                "boolean" => Ok("bool".to_string()),
                "array" => {
                    if let Some(items) = value.get("items") {
                        let item_type = self.json_type_to_rust_type(items, parent_name, struct_defs)?;
                        Ok(format!("Vec<{}>", item_type))
                    } else {
                        Ok("Vec<Value>".to_string())
                    }
                }
                "object" => {
                    let new_struct_name = format!("{}_{}", parent_name, "object");
                    self.generate_struct(value, &new_struct_name, struct_defs)?;
                    Ok(new_struct_name)
                }
                "null" => Ok("Option<()>".to_string()),
                _ => Err(Box::new(SchemaConversionError::UnsupportedKeyword(format!("Unsupported type: {}", json_type)))),
            }
        } else {
            Ok("Value".to_string())
        }
    }
}