//! # AutoGen Core
//!
//! AutoGen core offers an easy way to quickly build event-driven, distributed, 
//! scalable, resilient AI agent systems. Agents are developed using the Actor model.
//! You can build and run your agent system locally and easily move to a distributed 
//! system in the cloud when you are ready.
//!
//! ## Features
//!
//! - **Event-driven architecture**: Built on async/await and message passing
//! - **Type-safe agents**: Leverage Rust's type system for reliable agent interactions
//! - **Flexible runtime**: Support for single-threaded and distributed deployments
//! - **LLM integration**: Unified interface for various language models
//! - **Tool calling**: Built-in support for function calling and code execution
//! - **Observability**: Integrated tracing and telemetry
//!
//! ## Quick Start
//!
//! ```rust
//! use autogen_core::{AgentId, TopicId, MessageContext, CancellationToken};
//!
//! // Create an agent ID
//! let agent_id = AgentId::new("assistant", "main").unwrap();
//! assert_eq!(agent_id.agent_type(), "assistant");
//! assert_eq!(agent_id.key(), "main");
//!
//! // Create a topic ID
//! let topic_id = TopicId::new("user.message", "session_1").unwrap();
//! assert_eq!(topic_id.topic_type(), "user.message");
//! assert_eq!(topic_id.source(), "session_1");
//!
//! // Create a message context
//! let token = CancellationToken::new();
//! let context = MessageContext::direct_message(Some(agent_id), token);
//! ```

#![allow(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::module_inception)]

// Error types (must be first for other modules to use)
pub mod error;

// Core traits and types
pub mod agent;
pub mod agent_id;
pub mod topic;
pub mod message;
pub mod message_v2; // High-performance message system
pub mod cancellation;
pub mod macros; // Procedural macros for convenient development
pub mod agent_factory; // Improved agent factory system
pub mod base_agent; // Base agent implementation
pub mod closure_agent; // Closure-based agent implementation
pub mod cache; // Caching system
pub mod state_manager; // State management and persistence

// === Serialization System ===
// Requires at least one serialization feature
#[cfg(any(feature = "json", feature = "protobuf"))]
pub mod serialization;

// === Subscription System ===
// Core subscription functionality (always available)
pub mod subscription;

// === Runtime System ===
// Agent runtime and execution (requires runtime feature)
#[cfg(feature = "runtime")]
pub mod agent_runtime;
#[cfg(feature = "runtime")]
pub mod single_threaded_runtime;

// === Core Modules ===
// Always available core functionality
pub mod memory;
pub mod models;
pub mod model_context;
pub mod tools;
pub mod tool_agent;
pub mod utils;

// These modules will be implemented in subsequent tasks
// pub mod routed_agent;
// pub mod component;
// pub mod telemetry;

// === Core Re-exports ===
// Always available core types and traits
pub use error::{AutoGenError, Result};
pub use agent_id::{AgentId, AgentType};
pub use topic::{DefaultTopicId, TopicId};
pub use message::{
    MessageContext, MessageHandler, FunctionCall, Message,
    TypeSafeMessage, TypeSafeMessageEnvelope, TypeSafeMessageRouter,
    TextMessage, RequestMessage, ResponseMessage, NoResponse
};
pub use cancellation::CancellationToken;
pub use agent::{Agent, AgentMetadata, AgentProxy, RuntimeHandle};
pub use agent_factory::{AgentFactory, AgentFactoryRegistry, ClosureAgentFactory, AsyncClosureAgentFactory};
pub use base_agent::{BaseAgent, BaseAgentBuilder};
pub use closure_agent::{ClosureAgent, ClosureAgentBuilder, ClosureContext, closure_agent};
pub use cache::{CacheStore, InMemoryStore, TypedCache};
pub use state_manager::{AgentState, StateMetadata, StateStore, StateManager, FileSystemStateStore};
pub use subscription::{
    Subscription, DefaultSubscription, TypeSubscription,
    TopicSubscription, TypePrefixSubscription, SubscriptionRegistry
};

// === Serialization Re-exports ===
#[cfg(feature = "json")]
pub use serialization::{
    MessageSerializer, JsonMessageSerializer, SerializedMessage, JSON_DATA_CONTENT_TYPE
};
#[cfg(feature = "protobuf")]
pub use serialization::PROTOBUF_DATA_CONTENT_TYPE;

// === Runtime Re-exports ===
#[cfg(feature = "runtime")]
pub use agent_runtime::{
    AgentRuntime, RuntimeConfig, RuntimeStats, RuntimeEvent,
    RuntimeEventHandler, LoggingEventHandler
};
#[cfg(feature = "runtime")]
pub use single_threaded_runtime::SingleThreadedAgentRuntime;

// These will be implemented in subsequent tasks
// pub use component::{Component, ComponentConfig};
// pub use models::{ChatCompletionClient, LLMMessage};
// pub use routed_agent::RoutedAgent;

// Macros for agent development (will be implemented later)
// pub use autogen_core_macros::{agent, message_handler, rpc};

/// Current version of autogen-core
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Root logger name for the autogen-core library
pub const ROOT_LOGGER_NAME: &str = "autogen_core";

/// Event logger name for agent events
pub const EVENT_LOGGER_NAME: &str = "autogen_core.events";

/// Trace logger name for distributed tracing
pub const TRACE_LOGGER_NAME: &str = "autogen_core.trace";
