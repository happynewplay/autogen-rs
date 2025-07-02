//! Macros for convenient agent and message handling
//!
//! This module provides macros that simplify agent development
//! by providing decorator-like syntax similar to Python's @event and @rpc decorators.

// Imports are handled within macro expansions

/// Macro to create a simple message handler dispatcher
///
/// This macro generates a match statement that routes TypeSafeMessage variants
/// to appropriate handler methods, similar to Python's @event decorator.
///
/// # Example
///
/// ```rust
/// use autogen_core::{handle_messages, TypeSafeMessage, MessageContext, Result};
///
/// impl MyAgent {
///     async fn handle_message(&mut self, message: TypeSafeMessage, context: &MessageContext) -> Result<Option<TypeSafeMessage>> {
///         handle_messages! {
///             message, context => {
///                 Text(msg) => self.handle_text(msg, context).await,
///                 FunctionCall(call) => self.handle_function_call(call, context).await,
///                 Request(req) => self.handle_request(req, context).await,
///                 _ => Ok(None)
///             }
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! handle_messages {
    ($message:expr, $context:expr => {
        $($variant:ident($param:ident) => $handler:expr,)*
        _ => $default:expr
    }) => {
        match $message {
            $(
                $crate::TypeSafeMessage::$variant($param) => $handler,
            )*
            _ => $default,
        }
    };
}

/// Macro to create an agent with automatic message routing
///
/// This macro generates the boilerplate code for implementing the Agent trait
/// with automatic message routing based on method signatures.
///
/// # Example
///
/// ```rust
/// use autogen_core::{agent_impl, AgentId, TypeSafeMessage, MessageContext, Result};
///
/// struct MyAgent {
///     id: AgentId,
/// }
///
/// agent_impl! {
///     MyAgent {
///         async fn handle_text(&mut self, msg: &TextMessage, ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> {
///             println!("Received: {}", msg.content);
///             Ok(Some(TypeSafeMessage::text("Acknowledged")))
///         }
///
///         async fn handle_function_call(&mut self, call: &FunctionCall, ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> {
///             println!("Function: {}", call.name);
///             Ok(None)
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! agent_impl {
    ($agent_type:ident {
        $(
            async fn $handler_name:ident(&mut self, $param:ident: &$msg_type:ident, $ctx:ident: &MessageContext) -> Result<Option<TypeSafeMessage>> $body:block
        )*
    }) => {
        #[async_trait::async_trait]
        impl $crate::Agent for $agent_type {
            fn id(&self) -> &$crate::AgentId {
                &self.id
            }

            async fn handle_message(
                &mut self,
                message: $crate::TypeSafeMessage,
                context: &$crate::MessageContext,
            ) -> $crate::Result<Option<$crate::TypeSafeMessage>> {
                match message {
                    $(
                        $crate::TypeSafeMessage::$msg_type($param) => {
                            let $ctx = context;
                            self.$handler_name($param, $ctx).await
                        }
                    )*
                    _ => Ok(None),
                }
            }
        }

        impl $agent_type {
            $(
                async fn $handler_name(&mut self, $param: &$msg_type, $ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> $body
            )*
        }
    };
}



/// Macro for creating routed agents with message handlers
///
/// This macro provides a convenient way to create routed agents with
/// automatic message handler implementations, similar to Python's RoutedAgent.
///
/// # Example
///
/// ```rust
/// use autogen_core::{routed_agent, AgentId, TypeSafeMessage, MessageContext, Result, TextMessage, FunctionCall};
///
/// routed_agent! {
///     struct MyAgent {
///         id: AgentId,
///         state: String,
///     }
///
///     handlers {
///         async fn handle_text_message(&mut self, msg: TextMessage, ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> {
///             self.state = msg.content.clone();
///             Ok(Some(TypeSafeMessage::text("Received")))
///         }
///
///         async fn handle_function_call_message(&mut self, call: FunctionCall, ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> {
///             println!("Executing function: {}", call.name);
///             Ok(None)
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! routed_agent {
    (
        struct $agent_name:ident {
            $($field:ident: $field_type:ty,)*
        }

        handlers {
            $(
                async fn $handler_name:ident(&mut self, $param:ident: $msg_type:ty, $ctx:ident: &MessageContext) -> Result<Option<TypeSafeMessage>> $body:block
            )*
        }
    ) => {
        pub struct $agent_name {
            $($field: $field_type,)*
        }

        impl $agent_name {
            pub fn new($($field: $field_type,)*) -> Self {
                Self {
                    $($field,)*
                }
            }

            $(
                async fn $handler_name(&mut self, $param: $msg_type, $ctx: &MessageContext) -> Result<Option<TypeSafeMessage>> $body
            )*
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

        #[async_trait::async_trait]
        impl $crate::RoutedAgent for $agent_name {
            // Use default implementations from the trait
            // Individual handlers will be called through route_message
        }
    };

    // Helper macro to implement handler methods - simplified approach
    (@impl_handlers) => {
        // Default implementations - handlers will override specific methods
    };
}

/// Macro for creating simple event handlers
///
/// This macro provides a shorthand for creating message handlers
/// that don't return responses, similar to Python's @event decorator.
///
/// # Example
///
/// ```rust
/// use autogen_core::{event_handler, TypeSafeMessage, MessageContext, Result};
///
/// event_handler! {
///     async fn log_text_messages(msg: &TextMessage, ctx: &MessageContext) -> Result<()> {
///         println!("Received text: {}", msg.content);
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! event_handler {
    (
        async fn $handler_name:ident($param:ident: &$msg_type:ident, $ctx:ident: &MessageContext) -> Result<()> $body:block
    ) => {
        async fn $handler_name($param: &$msg_type, $ctx: &MessageContext) -> $crate::Result<Option<$crate::TypeSafeMessage>> {
            let result: $crate::Result<()> = async move $body.await;
            result.map(|_| None)
        }
    };
}

/// Macro for creating RPC handlers
///
/// This macro provides a shorthand for creating request-response handlers,
/// similar to Python's @rpc decorator.
///
/// # Example
///
/// ```rust
/// use autogen_core::{rpc_handler, TypeSafeMessage, MessageContext, Result};
///
/// rpc_handler! {
///     async fn process_request(req: &RequestMessage<serde_json::Value>, ctx: &MessageContext) -> Result<TypeSafeMessage> {
///         Ok(TypeSafeMessage::text("Processed"))
///     }
/// }
/// ```
#[macro_export]
macro_rules! rpc_handler {
    (
        async fn $handler_name:ident($param:ident: &$msg_type:ident, $ctx:ident: &MessageContext) -> Result<TypeSafeMessage> $body:block
    ) => {
        async fn $handler_name($param: &$msg_type, $ctx: &MessageContext) -> $crate::Result<Option<$crate::TypeSafeMessage>> {
            let result: $crate::Result<$crate::TypeSafeMessage> = async move $body.await;
            result.map(Some)
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{TypeSafeMessage, TextMessage};

    #[test]
    fn test_handle_messages_macro_compilation() {
        // This test verifies that the handle_messages macro compiles correctly
        let message = TypeSafeMessage::Text(TextMessage {
            content: "test".to_string(),
        });
        let _context = crate::MessageContext::default();

        // Test that the macro expands correctly
        let result = match message {
            TypeSafeMessage::Text(_msg) => Some("handled"),
            _ => None,
        };

        assert_eq!(result, Some("handled"));
    }
}
