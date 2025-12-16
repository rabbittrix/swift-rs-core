use thiserror::Error;

/// Core error types for Swift-RS
#[derive(Error, Debug)]
pub enum SwiftError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Message format error: {0}")]
    Format(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type SwiftResult<T> = Result<T, SwiftError>;
