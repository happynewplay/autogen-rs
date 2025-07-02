//! Routed Agent Implementation
//!
//! This module provides a routed agent system similar to Python's RoutedAgent,
//! but leveraging Rust's type system for compile-time safety and zero-cost abstractions.
//!
//! The routed agent system allows defining message handlers using attribute macros
//! and automatically routes incoming messages to the appropriate handlers based on
//! message type and optional custom routing logic.

use crate::{Agent, AgentId, AgentMetadata, MessageContext, Result, TypeSafeMessage};
use async_trait::async_trait;
use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for message routing functions
pub type MessageRouter = Arc<
    dyn Fn(&TypeSafeMessage, &MessageContext) -> bool + Send + Sync,
>;

/// Handler metadata for message routing
#[derive(Clone)]
pub struct HandlerInfo {
    /// Optional custom routing logic
    pub router: Option<MessageRouter>,
    /// Whether this handler is for RPC messages only
    pub is_rpc_only: bool,
    /// Whether this handler is for event messages only
    pub is_event_only: bool,
    /// Handler name for debugging
    pub name: String,
}

impl HandlerInfo {
    /// Create a new handler info
    pub fn new(
        name: String,
        is_rpc_only: bool,
        is_event_only: bool,
        router: Option<MessageRouter>,
    ) -> Self {
        Self {
            router,
            is_rpc_only,
            is_event_only,
            name,
        }
    }

    /// Check if this handler can handle the given message context
    pub fn can_handle(&self, ctx: &MessageContext) -> bool {
        // Check RPC/event constraints
        if self.is_rpc_only && !ctx.is_rpc {
            return false;
        }
        if self.is_event_only && ctx.is_rpc {
            return false;
        }

        true
    }

    /// Check if custom router matches
    pub fn router_matches(&self, message: &TypeSafeMessage, ctx: &MessageContext) -> bool {
        if let Some(router) = &self.router {
            router(message, ctx)
        } else {
            true
        }
    }
}

/// Registry for message handlers
#[derive(Default)]
pub struct HandlerRegistry {
    /// Handlers organized by message type
    handlers: HashMap<&'static str, Vec<HandlerInfo>>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a specific message type
    pub fn register_handler(&mut self, message_type: &'static str, handler: HandlerInfo) {
        self.handlers
            .entry(message_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// Get handlers for a specific message type
    pub fn get_handlers(&self, message_type: &str) -> Option<&Vec<HandlerInfo>> {
        self.handlers.get(message_type)
    }

    /// Get all registered message types
    pub fn message_types(&self) -> Vec<&str> {
        self.handlers.keys().copied().collect()
    }
}

/// Trait for agents that support automatic message routing
///
/// This trait provides a simpler approach to message routing by using
/// method dispatch instead of complex handler registration.
#[async_trait]
pub trait RoutedAgent: Agent + Send + Sync {
    /// Route a message to the appropriate handler
    ///
    /// This method should be implemented by agents to provide custom
    /// message routing logic. The default implementation calls the
    /// individual handler methods based on message type.
    async fn route_message(
        &mut self,
        message: TypeSafeMessage,
        ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        match message {
            TypeSafeMessage::Text(text_msg) => {
                self.handle_text_message(text_msg, ctx).await
            }
            TypeSafeMessage::FunctionCall(func_call) => {
                self.handle_function_call_message(func_call, ctx).await
            }
            TypeSafeMessage::Request(request) => {
                self.handle_request_message(request, ctx).await
            }
            TypeSafeMessage::Response(response) => {
                self.handle_response_message(response, ctx).await
            }
            TypeSafeMessage::Custom { message_type, payload } => {
                self.handle_custom_message(message_type, payload, ctx).await
            }
            TypeSafeMessage::NoResponse(_) => {
                // NoResponse messages don't need handling
                Ok(None)
            }
        }
    }

    /// Handle text messages
    ///
    /// Override this method to provide custom text message handling.
    /// The default implementation returns None.
    async fn handle_text_message(
        &mut self,
        _message: crate::TextMessage,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        Ok(None)
    }

    /// Handle function call messages
    ///
    /// Override this method to provide custom function call handling.
    /// The default implementation returns None.
    async fn handle_function_call_message(
        &mut self,
        _message: crate::FunctionCall,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        Ok(None)
    }

    /// Handle request messages
    ///
    /// Override this method to provide custom request handling.
    /// The default implementation returns None.
    async fn handle_request_message(
        &mut self,
        _message: crate::RequestMessage<serde_json::Value>,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        Ok(None)
    }

    /// Handle response messages
    ///
    /// Override this method to provide custom response handling.
    /// The default implementation returns None.
    async fn handle_response_message(
        &mut self,
        _message: crate::ResponseMessage<serde_json::Value>,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        Ok(None)
    }

    /// Handle custom messages
    ///
    /// Override this method to provide custom message handling.
    /// The default implementation returns None.
    async fn handle_custom_message(
        &mut self,
        _message_type: String,
        _payload: serde_json::Value,
        _ctx: &MessageContext,
    ) -> Result<Option<TypeSafeMessage>> {
        Ok(None)
    }
}



/// Macro to create a simple routed agent implementation
///
/// This macro provides a convenient way to create a routed agent that
/// automatically implements the Agent trait and delegates to RoutedAgent.
#[macro_export]
macro_rules! simple_routed_agent {
    (
        struct $agent_name:ident {
            id: AgentId,
            $($field:ident: $field_type:ty,)*
        }
    ) => {
        pub struct $agent_name {
            id: AgentId,
            $($field: $field_type,)*
        }

        impl $agent_name {
            pub fn new(id: AgentId $(, $field: $field_type)*) -> Self {
                Self {
                    id,
                    $($field,)*
                }
            }
        }

        #[async_trait::async_trait]
        impl Agent for $agent_name {
            fn id(&self) -> &AgentId {
                &self.id
            }

            async fn handle_message(
                &mut self,
                message: TypeSafeMessage,
                context: &MessageContext,
            ) -> Result<Option<TypeSafeMessage>> {
                self.route_message(message, context).await
            }
        }

        // Note: RoutedAgent implementation should be provided by the user
    };
}

/// Macro to implement message handlers for a routed agent
///
/// This macro provides a convenient way to implement multiple message handlers
/// for a routed agent, similar to Python's @event and @rpc decorators.
#[macro_export]
macro_rules! impl_message_handlers {
    (
        impl $agent_type:ty {
            $(
                $(#[event])?
                $(#[rpc])?
                async fn $handler_name:ident(&mut self, $param:ident: $msg_type:ty, $ctx:ident: &MessageContext) -> Result<$return_type:ty> $body:block
            )*
        }
    ) => {
        #[async_trait::async_trait]
        impl RoutedAgent for $agent_type {
            $(
                async fn $handler_name(&mut self, $param: $msg_type, $ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> {
                    let result: $return_type = $body;
                    // Convert result to Option<TypeSafeMessage> based on return type
                    impl_message_handlers!(@convert_result result)
                }
            )*
        }
    };

    // Helper to convert different return types to Option<TypeSafeMessage>
    (@convert_result $result:expr) => {
        match $result {
            Ok(Some(msg)) => Ok(Some(msg)),
            Ok(None) => Ok(None),
            Ok(()) => Ok(None),
            Err(e) => Err(e),
        }
    };
}


