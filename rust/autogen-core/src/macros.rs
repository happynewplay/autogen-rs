//! Macros for implementing Python-like decorators in Rust
//! 
//! This module provides macros that simulate the behavior of Python decorators
//! used in the autogen-core Python implementation.

use std::any::TypeId;
use crate::serialization::MessageSerializer;

/// Macro to implement the `handles` decorator functionality
/// 
/// This macro adds type handling capabilities to an agent, similar to
/// Python's `@handles` decorator.
/// 
/// # Example
/// ```rust
/// use autogen_core::handles;
/// 
/// #[derive(Debug)]
/// struct MyAgent;
/// 
/// handles!(MyAgent, String); // Agent can handle String messages
/// ```
#[macro_export]
macro_rules! handles {
    ($agent_type:ty, $message_type:ty) => {
        impl $crate::base_agent::HandlesType<$message_type> for $agent_type {
            fn message_type_id() -> std::any::TypeId {
                std::any::TypeId::of::<$message_type>()
            }
            
            fn message_type_name() -> &'static str {
                std::any::type_name::<$message_type>()
            }
        }
        
        // Register the type globally
        $crate::base_agent::register_handled_type::<$agent_type, $message_type>();
    };
    
    ($agent_type:ty, $message_type:ty, $serializer:expr) => {
        impl $crate::base_agent::HandlesType<$message_type> for $agent_type {
            fn message_type_id() -> std::any::TypeId {
                std::any::TypeId::of::<$message_type>()
            }
            
            fn message_type_name() -> &'static str {
                std::any::type_name::<$message_type>()
            }
        }
        
        // Register the type with custom serializer
        $crate::base_agent::register_handled_type_with_serializer::<$agent_type, $message_type>($serializer);
    };
}

/// Macro to implement subscription factory functionality
/// 
/// This macro adds subscription capabilities to an agent, similar to
/// Python's subscription decorators.
/// 
/// # Example
/// ```rust
/// use autogen_core::subscription_factory;
/// 
/// subscription_factory!(MyAgent, "topic_name", MyMessageType);
/// ```
#[macro_export]
macro_rules! subscription_factory {
    ($agent_type:ty, $topic:expr, $message_type:ty) => {
        impl $crate::base_agent::HasSubscription for $agent_type {
            fn get_subscriptions() -> Vec<Box<dyn $crate::subscription::Subscription>> {
                vec![
                    Box::new($crate::type_subscription::TypeSubscription::new(
                        $topic.to_string(),
                        $crate::agent_type::AgentType {
                            r#type: std::any::type_name::<$agent_type>().to_string()
                        },
                    ))
                ]
            }
        }

        // Register the subscription globally
        $crate::base_agent::register_subscription::<$agent_type>($topic, std::any::TypeId::of::<$message_type>());
    };
}

/// Macro to implement message handler functionality
/// 
/// This macro creates message handlers for agents, similar to
/// Python's `@message_handler` decorator.
/// 
/// # Example
/// ```rust
/// use autogen_core::message_handler;
/// 
/// message_handler!(MyAgent, handle_string, String, {
///     |agent: &mut MyAgent, msg: String, ctx: MessageContext| async move {
///         // Handle the message
///         Ok(format!("Received: {}", msg))
///     }
/// });
/// ```
#[macro_export]
macro_rules! message_handler {
    ($agent_type:ty, $handler_name:ident, $message_type:ty, $handler:expr) => {
        impl $agent_type {
            pub async fn $handler_name(
                &mut self,
                message: $message_type,
                ctx: $crate::message_context::MessageContext,
            ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send>> {
                let handler = $handler;
                handler(self, message, ctx).await
            }
        }
        
        // Register the handler
        $crate::base_agent::register_message_handler::<$agent_type, $message_type>(
            stringify!($handler_name),
            |agent, msg, ctx| {
                Box::pin(async move {
                    if let Ok(typed_agent) = agent.downcast_mut::<$agent_type>() {
                        if let Ok(typed_msg) = serde_json::from_value::<$message_type>(msg) {
                            return typed_agent.$handler_name(typed_msg, ctx).await;
                        }
                    }
                    Err("Type conversion failed".into())
                })
            }
        );
    };
}

/// Macro to implement event handler functionality
/// 
/// This macro creates event handlers for agents, similar to
/// Python's `@event` decorator.
#[macro_export]
macro_rules! event_handler {
    ($agent_type:ty, $handler_name:ident, $event_type:ty, $handler:expr) => {
        impl $agent_type {
            pub async fn $handler_name(
                &mut self,
                event: $event_type,
                ctx: $crate::message_context::MessageContext,
            ) -> Result<(), Box<dyn std::error::Error + Send>> {
                let handler = $handler;
                handler(self, event, ctx).await
            }
        }
        
        // Register the event handler
        $crate::base_agent::register_event_handler::<$agent_type, $event_type>(
            stringify!($handler_name),
            |agent, event, ctx| {
                Box::pin(async move {
                    if let Ok(typed_agent) = agent.downcast_mut::<$agent_type>() {
                        if let Ok(typed_event) = serde_json::from_value::<$event_type>(event) {
                            return typed_agent.$handler_name(typed_event, ctx).await;
                        }
                    }
                    Err("Type conversion failed".into())
                })
            }
        );
    };
}

/// Macro to implement RPC handler functionality
/// 
/// This macro creates RPC handlers for agents, similar to
/// Python's `@rpc` decorator.
#[macro_export]
macro_rules! rpc_handler {
    ($agent_type:ty, $handler_name:ident, $request_type:ty, $response_type:ty, $handler:expr) => {
        impl $agent_type {
            pub async fn $handler_name(
                &mut self,
                request: $request_type,
                ctx: $crate::message_context::MessageContext,
            ) -> Result<$response_type, Box<dyn std::error::Error + Send>> {
                let handler = $handler;
                handler(self, request, ctx).await
            }
        }
        
        // Register the RPC handler
        $crate::base_agent::register_rpc_handler::<$agent_type, $request_type, $response_type>(
            stringify!($handler_name),
            |agent, request, ctx| {
                Box::pin(async move {
                    if let Ok(typed_agent) = agent.downcast_mut::<$agent_type>() {
                        if let Ok(typed_request) = serde_json::from_value::<$request_type>(request) {
                            let response = typed_agent.$handler_name(typed_request, ctx).await?;
                            return Ok(serde_json::to_value(response)?);
                        }
                    }
                    Err("Type conversion failed".into())
                })
            }
        );
    };
}

/// Helper macro to define an agent with all its handlers at once
/// 
/// This macro provides a convenient way to define an agent with multiple
/// message handlers, similar to how Python agents are defined with decorators.
#[macro_export]
macro_rules! define_agent {
    (
        $agent_type:ty,
        handles: [$($handle_type:ty),*],
        subscriptions: [$($sub_topic:expr => $sub_type:ty),*],
        handlers: {
            $(
                $handler_name:ident($msg_type:ty) -> $ret_type:ty = $handler:expr
            ),*
        }
    ) => {
        // Register all handled types
        $(
            handles!($agent_type, $handle_type);
        )*
        
        // Register all subscriptions
        $(
            subscription_factory!($agent_type, $sub_topic, $sub_type);
        )*
        
        // Register all message handlers
        $(
            message_handler!($agent_type, $handler_name, $msg_type, $handler);
        )*
    };
}
