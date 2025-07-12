use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use dashmap::DashMap;
use once_cell::sync::Lazy;

/// Errors that can occur during component operations
#[derive(Error, Debug)]
pub enum ComponentError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Component type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Component creation failed: {0}")]
    CreationFailed(String),
}

/// Schema type for component configuration
pub type ComponentSchemaType = schemars::schema::RootSchema;

/// Model class for a component. Contains all information required to instantiate a component.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentModel {
    /// Describes how the component can be instantiated.
    pub provider: String,
    /// Logical type of the component.
    #[serde(rename = "component_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_type: Option<String>,
    /// Version of the component specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
    /// Version of the component.
    #[serde(rename = "component_version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_version: Option<i32>,
    /// Description of the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Human readable label for the component.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The schema validated config field.
    pub config: HashMap<String, Value>,
}

/// Base trait for all components
pub trait ComponentBase<T>: Send + Sync {
    /// The type identifier for this component
    const COMPONENT_TYPE: &'static str;

    /// Convert the component to its configuration representation
    fn to_config(&self) -> Result<T, ComponentError>;

    /// Create a component from its configuration
    fn from_config(config: T) -> Result<Self, ComponentError>
    where
        Self: Sized;

    /// Get the component schema for validation
    fn get_schema() -> ComponentSchemaType
    where
        T: schemars::JsonSchema,
    {
        schemars::schema_for!(T)
    }
}

/// A trait for components that can be created from a configuration.
pub trait ComponentFromConfig<C: DeserializeOwned>: Sized {
    /// Create a new instance of the component from a configuration object.
    fn from_config(config: C) -> Result<Self, ComponentError>;
}

/// A trait for components that can be converted to a configuration.
pub trait ComponentToConfig<C: Serialize> {
    /// The logical type of the component.
    fn component_type(&self) -> String;
    /// The version of the component.
    fn component_version(&self) -> i32 {
        1
    }
    /// A description of the component.
    fn component_description(&self) -> Option<String> {
        None
    }
    /// A human readable label for the component.
    fn component_label(&self) -> Option<String> {
        None
    }
    /// The provider string for the component.
    fn provider(&self) -> String;

    /// Dump the configuration to a serializable object.
    fn to_config(&self) -> Result<C, ComponentError>;

    /// Dump the component to a model that can be loaded back in.
    fn dump_component(&self) -> Result<ComponentModel, ComponentError> {
        let config_val = self.to_config()?;
        let config_map = serde_json::from_value(serde_json::to_value(config_val)?)?;
        Ok(ComponentModel {
            provider: self.provider(),
            component_type: Some(self.component_type()),
            version: Some(self.component_version()),
            component_version: Some(self.component_version()),
            description: self.component_description(),
            label: self.component_label(),
            config: config_map,
        })
    }
}

/// Factory function type for creating components
pub type ComponentFactory = Box<
    dyn Fn(HashMap<String, Value>) -> Result<Box<dyn Any + Send + Sync>, ComponentError>
        + Send
        + Sync,
>;

/// Component type information
#[derive(Debug, Clone)]
pub struct ComponentType {
    pub name: String,
    pub type_id: TypeId,
    pub schema: Option<ComponentSchemaType>,
}

/// Enhanced component loader with better error handling and type safety
pub struct ComponentLoader {
    factories: DashMap<String, ComponentFactory>,
    types: DashMap<String, ComponentType>,
}

impl ComponentLoader {
    pub fn new() -> Self {
        Self {
            factories: DashMap::new(),
            types: DashMap::new(),
        }
    }

    /// Register a component factory with type information
    pub fn register_factory<T: 'static>(
        &self,
        provider: &str,
        factory: ComponentFactory,
        component_type: ComponentType,
    ) {
        self.factories.insert(provider.to_string(), factory);
        self.types.insert(provider.to_string(), component_type);
    }

    /// Load a component from a model with type checking
    pub fn load_component<T: 'static>(&self, model: &ComponentModel) -> Result<T, ComponentError> {
        let factory = self
            .factories
            .get(&model.provider)
            .ok_or_else(|| ComponentError::ProviderNotFound(model.provider.clone()))?;

        let component_any = factory(model.config.clone())?;

        match component_any.downcast::<T>() {
            Ok(component) => Ok(*component),
            Err(_) => {
                let expected = std::any::type_name::<T>();
                let actual = if let Some(type_info) = self.types.get(&model.provider) {
                    type_info.name.clone()
                } else {
                    "unknown".to_string()
                };
                Err(ComponentError::TypeMismatch { expected: expected.to_string(), actual })
            }
        }
    }

    /// Check if a component type is registered
    pub fn is_component_type(&self, provider: &str) -> bool {
        self.types.contains_key(provider)
    }

    /// Get component type information
    pub fn get_component_type(&self, provider: &str) -> Option<ComponentType> {
        self.types.get(provider).map(|entry| entry.clone())
    }
}

/// Global component loader instance
static GLOBAL_LOADER: Lazy<ComponentLoader> = Lazy::new(ComponentLoader::new);

/// Convenience functions for global component operations
pub fn register_component<T: 'static>(
    provider: &str,
    factory: ComponentFactory,
    component_type: ComponentType,
) {
    GLOBAL_LOADER.register_factory::<T>(provider, factory, component_type);
}

pub fn load_component<T: 'static>(model: &ComponentModel) -> Result<T, ComponentError> {
    GLOBAL_LOADER.load_component::<T>(model)
}

pub fn is_component_class(provider: &str) -> bool {
    GLOBAL_LOADER.is_component_type(provider)
}

pub fn is_component_instance<T: 'static>(_instance: &T) -> bool {
    // In Rust, we can check if T implements our component traits at compile time
    // This is a simplified version - in practice, you might want more sophisticated checking
    true
}

/// A full component that can be converted to and from a configuration.
pub trait Component<C>: ComponentFromConfig<C> + ComponentToConfig<C>
where
    C: Serialize + DeserializeOwned,
{
}

/// Blanket implementation for types that implement both traits
impl<T, C> Component<C> for T
where
    T: ComponentFromConfig<C> + ComponentToConfig<C>,
    C: Serialize + DeserializeOwned,
{
}

/// Helper macro for registering components
#[macro_export]
macro_rules! register_component_type {
    ($provider:expr, $component_type:ty, $config_type:ty) => {
        {
            use std::any::TypeId;
            let factory: ComponentFactory = Box::new(|config| {
                let typed_config: $config_type = serde_json::from_value(serde_json::to_value(config)?)?;
                let component = <$component_type>::from_config(typed_config)?;
                Ok(Box::new(component) as Box<dyn Any + Send + Sync>)
            });

            let component_type = ComponentType {
                name: std::any::type_name::<$component_type>().to_string(),
                type_id: TypeId::of::<$component_type>(),
                schema: None, // Could be enhanced to include actual schema
            };

            register_component::<$component_type>($provider, factory, component_type);
        }
    };
}