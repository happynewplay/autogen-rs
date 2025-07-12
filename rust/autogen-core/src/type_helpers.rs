use std::any::{Any, TypeId};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use thiserror::Error;

/// Errors that can occur during type operations
#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Type not registered: {0}")]
    TypeNotRegistered(String),
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("Union type validation failed: {0}")]
    UnionValidationFailed(String),
}

/// Information about a registered type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_union: bool,
    pub union_types: Vec<TypeId>,
    pub is_dataclass: bool,
    pub is_base_model: bool,
}

/// Global type registry for runtime type checking
static TYPE_REGISTRY: Lazy<DashMap<TypeId, TypeInfo>> = Lazy::new(DashMap::new);
static NAME_TO_TYPE: Lazy<DashMap<String, TypeId>> = Lazy::new(DashMap::new);

/// Register a type in the global registry
pub fn register_type<T: 'static>(name: &str) {
    let type_id = TypeId::of::<T>();
    let type_info = TypeInfo {
        name: name.to_string(),
        type_id,
        is_union: false,
        union_types: Vec::new(),
        is_dataclass: false, // In Rust, we'll use serde::Serialize + Deserialize as equivalent
        is_base_model: false, // We'll use specific traits to identify "base model" types
    };

    TYPE_REGISTRY.insert(type_id, type_info);
    NAME_TO_TYPE.insert(name.to_string(), type_id);
}

/// Register a union type (simplified version)
pub fn register_union_type<T: 'static>(name: &str, union_types: Vec<TypeId>) {
    let type_id = TypeId::of::<T>();
    let type_info = TypeInfo {
        name: name.to_string(),
        type_id,
        is_union: true,
        union_types,
        is_dataclass: false,
        is_base_model: false,
    };

    TYPE_REGISTRY.insert(type_id, type_info);
    NAME_TO_TYPE.insert(name.to_string(), type_id);
}

/// Check if a type is registered as a union type
pub fn is_union_type<T: 'static>() -> bool {
    let type_id = TypeId::of::<T>();
    TYPE_REGISTRY
        .get(&type_id)
        .map(|info| info.is_union)
        .unwrap_or(false)
}

/// Check if a type is registered as a union type by TypeId
pub fn is_union_type_id(type_id: TypeId) -> bool {
    TYPE_REGISTRY
        .get(&type_id)
        .map(|info| info.is_union)
        .unwrap_or(false)
}

/// Get type name for a registered type
pub fn get_type_name<T: 'static>() -> Option<String> {
    let type_id = TypeId::of::<T>();
    TYPE_REGISTRY
        .get(&type_id)
        .map(|info| info.name.clone())
}

/// Get type name by TypeId
pub fn get_type_name_by_id(type_id: TypeId) -> Option<String> {
    TYPE_REGISTRY
        .get(&type_id)
        .map(|info| info.name.clone())
}

/// Get TypeId by name
pub fn get_type_id_by_name(name: &str) -> Option<TypeId> {
    NAME_TO_TYPE.get(name).map(|entry| *entry)
}

/// Check if a type is registered
pub fn is_type_registered<T: 'static>() -> bool {
    let type_id = TypeId::of::<T>();
    TYPE_REGISTRY.contains_key(&type_id)
}

/// Validate that a value matches one of the union types (simplified)
pub fn validate_union_type(type_id: TypeId, value: &dyn Any) -> Result<(), TypeError> {
    if let Some(type_info) = TYPE_REGISTRY.get(&type_id) {
        if !type_info.is_union {
            return Err(TypeError::UnionValidationFailed(
                "Type is not a union type".to_string()
            ));
        }

        let value_type_id = value.type_id();
        if type_info.union_types.contains(&value_type_id) {
            Ok(())
        } else {
            Err(TypeError::UnionValidationFailed(format!(
                "Value type {:?} is not in union types {:?}",
                value_type_id, type_info.union_types
            )))
        }
    } else {
        Err(TypeError::TypeNotRegistered(format!("{:?}", type_id)))
    }
}

/// Trait to mark types as "dataclass-like" (equivalent to Python dataclass)
pub trait DataclassLike: serde::Serialize + serde::de::DeserializeOwned + Send + Sync {}

/// Trait to mark types as "base model-like" (equivalent to Pydantic BaseModel)
pub trait BaseModelLike: serde::Serialize + serde::de::DeserializeOwned + Send + Sync {
    fn validate(&self) -> Result<(), String> {
        Ok(()) // Default implementation - can be overridden
    }
}

/// Check if a type implements DataclassLike
pub fn is_dataclass_like<T: 'static>() -> bool {
    // In Rust, we use compile-time trait bounds instead of runtime checking
    // This is a placeholder - actual implementation would use trait objects or type registry
    std::any::type_name::<T>().contains("Dataclass") // Simplified heuristic
}

/// Check if a type implements BaseModelLike
pub fn is_base_model_like<T: 'static>() -> bool {
    // Similar to above - simplified heuristic
    std::any::type_name::<T>().contains("Model") // Simplified heuristic
}

/// Normalize annotated types (equivalent to Python's normalize_annotated_type)
pub fn normalize_type_name(type_name: &str) -> String {
    // Remove common Rust type annotations and generics
    type_name
        .replace("std::", "")
        .replace("alloc::", "")
        .replace("core::", "")
        .split('<')
        .next()
        .unwrap_or(type_name)
        .to_string()
}

/// Helper macro for registering dataclass-like types
#[macro_export]
macro_rules! register_dataclass_type {
    ($type:ty, $name:expr) => {
        {
            register_type::<$type>($name);
            // Mark as dataclass in registry
            if let Some(mut type_info) = TYPE_REGISTRY.get_mut(&std::any::TypeId::of::<$type>()) {
                type_info.is_dataclass = true;
            }
        }
    };
}

/// Helper macro for registering base model-like types
#[macro_export]
macro_rules! register_base_model_type {
    ($type:ty, $name:expr) => {
        {
            register_type::<$type>($name);
            // Mark as base model in registry
            if let Some(mut type_info) = TYPE_REGISTRY.get_mut(&std::any::TypeId::of::<$type>()) {
                type_info.is_base_model = true;
            }
        }
    };
}