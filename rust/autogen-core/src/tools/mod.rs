//! Tool system for function calling and code execution.
//!
//! This module provides abstractions for creating and managing tools that can be
//! called by language models, including function tools, workbenches, and tool schemas.

mod base_tool;
mod function_tool;
mod workbench;
mod static_workbench;

pub use base_tool::{
    Tool, BaseTool, BaseToolWithState,
    ToolSchema, ParametersSchema, ToolResult, ToolCallEvent,
};
#[cfg(feature = "runtime")]
pub use base_tool::{StreamTool, BaseStreamTool};
pub use function_tool::{FunctionTool, FunctionToolConfig, AsyncToolFunction};
pub use workbench::{
    Workbench, TextResultContent, ImageResultContent,
    WorkbenchResult, WorkbenchStreamItem,
};
#[cfg(feature = "runtime")]
pub use workbench::StreamWorkbench;
pub use static_workbench::{StaticWorkbench, StaticWorkbenchConfig, StaticStreamWorkbench};
