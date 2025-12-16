//! Message validation logic

use crate::domain::{Message, MessageType};
use crate::{SwiftError, SwiftResult};

/// Validates a message according to business rules
pub struct MessageValidator;

impl MessageValidator {
    /// Validate message structure and content
    pub fn validate(message: &Message) -> SwiftResult<()> {
        // Validate integrity
        if !message.validate_integrity() {
            return Err(SwiftError::Validation(
                "Message checksum mismatch".to_string(),
            ));
        }

        // Validate sender/receiver
        if message.metadata.sender.is_empty() {
            return Err(SwiftError::Validation("Sender cannot be empty".to_string()));
        }

        if message.metadata.receiver.is_empty() {
            return Err(SwiftError::Validation(
                "Receiver cannot be empty".to_string(),
            ));
        }

        // Validate payload based on message type
        match message.metadata.message_type {
            MessageType::Mt => Self::validate_mt(&message.payload)?,
            MessageType::Mx => Self::validate_mx(&message.payload)?,
        }

        Ok(())
    }

    fn validate_mt(payload: &[u8]) -> SwiftResult<()> {
        // Basic MT validation: should start with {1:
        if payload.len() < 4 {
            return Err(SwiftError::Validation("MT message too short".to_string()));
        }

        let header = String::from_utf8_lossy(&payload[..4.min(payload.len())]);
        if !header.starts_with("{1:") {
            return Err(SwiftError::Validation(
                "Invalid MT message format".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_mx(payload: &[u8]) -> SwiftResult<()> {
        // MX messages should be valid XML
        if payload.is_empty() {
            return Err(SwiftError::Validation(
                "MX message cannot be empty".to_string(),
            ));
        }

        // Basic XML validation: should start with <
        if payload[0] != b'<' {
            return Err(SwiftError::Validation(
                "MX message must be valid XML".to_string(),
            ));
        }

        Ok(())
    }
}
