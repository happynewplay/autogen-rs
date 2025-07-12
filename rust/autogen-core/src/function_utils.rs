use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::any::TypeId;
use thiserror::Error;
use crate::type_helpers::{normalize_type_name, get_type_name_by_id};

/// Errors that can occur during function operations
#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Invalid function signature: {0}")]
    InvalidSignature(String),
    #[error("Type annotation missing: {0}")]
    MissingTypeAnnotation(String),
    #[error("Schema generation failed: {0}")]
    SchemaGenerationFailed(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parameters {
    #[serde(rename = "type")]
    pub type_of: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Function {
    pub description: String,
    pub name: String,
    pub parameters: Parameters,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    #[serde(rename = "type")]
    pub type_of: String,
    pub function: Function,
}

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub type_id: TypeId,
    pub type_name: String,
    pub is_required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

/// Information about a function
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type_id: Option<TypeId>,
    pub return_type_name: Option<String>,
    pub is_async: bool,
}

/// Normalize annotated types (equivalent to Python's normalize_annotated_type)
pub fn normalize_annotated_type(type_name: &str) -> String {
    normalize_type_name(type_name)
}

/// Generate JSON schema for a type (simplified version)
pub fn generate_type_schema(type_id: TypeId) -> Result<serde_json::Value, FunctionError> {
    let type_name = get_type_name_by_id(type_id)
        .ok_or_else(|| FunctionError::MissingTypeAnnotation(format!("{:?}", type_id)))?;

    // This is a simplified schema generation - in practice, you'd want more sophisticated logic
    let schema = match type_name.as_str() {
        "String" | "str" => serde_json::json!({
            "type": "string"
        }),
        "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => serde_json::json!({
            "type": "integer"
        }),
        "f32" | "f64" => serde_json::json!({
            "type": "number"
        }),
        "bool" => serde_json::json!({
            "type": "boolean"
        }),
        _ => serde_json::json!({
            "type": "object",
            "description": format!("Custom type: {}", type_name)
        })
    };

    Ok(schema)
}

/// Create a Parameters object from function info
pub fn create_parameters_from_info(func_info: &FunctionInfo) -> Result<Parameters, FunctionError> {
    let mut properties = HashMap::new();
    let mut required = Vec::new();

    for param in &func_info.parameters {
        let schema = generate_type_schema(param.type_id)?;
        properties.insert(param.name.clone(), schema);

        if param.is_required {
            required.push(param.name.clone());
        }
    }

    Ok(Parameters {
        type_of: "object".to_string(),
        properties,
        required,
        additional_properties: Some(false),
    })
}

/// Create a Function object from function info
pub fn create_function_from_info(func_info: &FunctionInfo) -> Result<Function, FunctionError> {
    let parameters = create_parameters_from_info(func_info)?;

    Ok(Function {
        name: func_info.name.clone(),
        description: func_info.description.clone().unwrap_or_default(),
        parameters,
        strict: None,
    })
}

/// Create a ToolFunction object from function info
pub fn create_tool_function_from_info(func_info: &FunctionInfo) -> Result<ToolFunction, FunctionError> {
    let function = create_function_from_info(func_info)?;

    Ok(ToolFunction {
        type_of: "function".to_string(),
        function,
    })
}

/// Helper macro for creating function info at compile time
#[macro_export]
macro_rules! function_info {
    (
        name: $name:expr,
        description: $desc:expr,
        parameters: [$(
            $param_name:expr => {
                type: $param_type:ty,
                required: $required:expr
                $(, default: $default:expr)?
                $(, description: $param_desc:expr)?
            }
        ),*],
        return_type: $return_type:ty,
        is_async: $is_async:expr
    ) => {
        {
            use std::any::TypeId;
            let mut parameters = Vec::new();

            $(
                let default_value = None $(.or(Some(serde_json::to_value($default).unwrap())))?;
                let description = None $(.or(Some($param_desc.to_string())))?;

                parameters.push(ParameterInfo {
                    name: $param_name.to_string(),
                    type_id: TypeId::of::<$param_type>(),
                    type_name: std::any::type_name::<$param_type>().to_string(),
                    is_required: $required,
                    default_value,
                    description,
                });
            )*

            FunctionInfo {
                name: $name.to_string(),
                description: Some($desc.to_string()),
                parameters,
                return_type_id: Some(TypeId::of::<$return_type>()),
                return_type_name: Some(std::any::type_name::<$return_type>().to_string()),
                is_async: $is_async,
            }
        }
    };
}