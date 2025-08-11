use anyhow::{anyhow, Result};
use jsonschema::JSONSchema;
use serde_json::Value;

/// Validates a JSON value against a JSON schema.
///
/// # Arguments
///
/// * `instance` - The JSON value to validate.
/// * `schema` - The JSON schema to validate against.
///
/// # Returns
///
/// A `Result` that is `Ok(())` if the validation succeeds, or an `Err` with the validation errors.
pub fn validate_json_with_schema(instance: &Value, schema: &Value) -> Result<()> {
    let compiled_schema = JSONSchema::compile(schema)
        .map_err(|e| anyhow!("Failed to compile JSON schema: {}", e))?;

    let validation_result = compiled_schema.validate(instance);

    match validation_result {
        Ok(_) => Ok(()),
        Err(errors) => {
            let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
            Err(anyhow!(
                "JSON validation failed:\n{}",
                error_messages.join("\n")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_json_with_schema_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name", "age"]
        });

        let instance = json!({
            "name": "John Doe",
            "age": 42
        });

        let result = validate_json_with_schema(&instance, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_with_schema_invalid_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name", "age"]
        });

        let instance = json!({
            "name": "John Doe",
            "age": "42"
        });

        let result = validate_json_with_schema(&instance, &schema);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("is not of type \"number\""));
    }

    #[test]
    fn test_validate_json_with_schema_missing_required_property() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name", "age"]
        });

        let instance = json!({
            "name": "John Doe"
        });

        let result = validate_json_with_schema(&instance, &schema);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("\"age\" is a required property"));
    }

    #[test]
    fn test_validate_json_with_invalid_schema() {
        let schema = json!({
            "type": "invalid_type"
        });

        let instance = json!({
            "name": "John Doe"
        });

        let result = validate_json_with_schema(&instance, &schema);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Failed to compile JSON schema"));
    }
}
