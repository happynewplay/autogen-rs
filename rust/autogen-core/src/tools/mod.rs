//! Tool system for function calling and code execution.
//!
//! This module provides abstractions for creating and managing tools that can be
//! called by language models, including function tools, workbenches, and tool schemas.

mod base_tool;
mod function_tool;
mod workbench;
mod static_workbench;

pub use base_tool::{
    Tool, StreamTool, BaseTool, BaseToolWithState, BaseStreamTool,
    ToolSchema, ParametersSchema, ToolResult, ToolCallEvent,
};
pub use function_tool::{FunctionTool, FunctionToolConfig};
pub use workbench::{
    Workbench, StreamWorkbench, TextResultContent, ImageResultContent,
    WorkbenchResult,
};
pub use static_workbench::{StaticWorkbench, StaticWorkbenchConfig, StaticStreamWorkbench};
