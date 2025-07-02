//! Tool agent system for executing function calls.
//!
//! This module provides agents that can execute tools and function calls,
//! along with caller loops for managing tool execution workflows.

mod tool_agent;
mod caller_loop;

pub use tool_agent::{
    ToolAgent, ToolException, ToolNotFoundException, InvalidToolArgumentsException,
    ToolExecutionException,
};
#[cfg(feature = "runtime")]
pub use caller_loop::tool_agent_caller_loop;
