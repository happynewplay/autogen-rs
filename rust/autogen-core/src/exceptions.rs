use std::error::Error;
use std::fmt;

/// Raised when a handler can't handle the exception.
#[derive(Debug)]
pub struct CantHandleException(pub String);

impl fmt::Display for CantHandleException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Can't handle exception: {}", self.0)
    }
}

impl Error for CantHandleException {}

/// Raised when a message can't be delivered.
#[derive(Debug)]
pub struct UndeliverableException(pub String);

impl fmt::Display for UndeliverableException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Undeliverable exception: {}", self.0)
    }
}

impl Error for UndeliverableException {}

/// Raised when a message is dropped.
#[derive(Debug)]
pub struct MessageDroppedException(pub String);

impl fmt::Display for MessageDroppedException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message dropped exception: {}", self.0)
    }
}

impl Error for MessageDroppedException {}

/// Tried to access a value that is not accessible.
/// For example if it is remote cannot be accessed locally.
#[derive(Debug)]
pub struct NotAccessibleError(pub String);

impl fmt::Display for NotAccessibleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not accessible error: {}", self.0)
    }
}

impl Error for NotAccessibleError {}

#[derive(Debug)]
pub struct GeneralError(pub String);

impl fmt::Display for GeneralError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for GeneralError {}