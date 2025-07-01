//! Utility functions and helpers for autogen-core.
//!
//! This module provides various utility functions for JSON processing,
//! schema validation, and other common operations.

mod json_utils;
mod schema_utils;

pub use json_utils::{extract_json_from_str, JsonExtractionError};
pub use schema_utils::{
    schema_to_struct, SchemaToStructError, JsonSchemaProcessor,
    validate_json_against_schema, ValidationError,
};
