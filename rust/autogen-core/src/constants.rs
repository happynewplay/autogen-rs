//! Constants used in the autogen-core crate.

/// Logger name used for root logger
pub const ROOT_LOGGER_NAME: &str = "autogen_core";

/// Logger name used for structured event logging
pub const EVENT_LOGGER_NAME: &str = "autogen_core.events";

/// Logger name used for developer intended trace logging. The content and format of this log should not be depended upon.
pub const TRACE_LOGGER_NAME: &str = "autogen_core.trace";