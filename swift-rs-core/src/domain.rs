//! Domain models for financial messages

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message types supported by Swift-RS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MessageType {
    /// SWIFT MT (FIN) message
    Mt,
    /// ISO 20022 MX message
    Mx,
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Normal,
    Urgent,
    System,
}

/// Financial message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub id: Uuid,
    pub message_type: MessageType,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub sender: String,
    pub receiver: String,
    pub message_reference: Option<String>,
}

/// Core message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub metadata: MessageMetadata,
    pub payload: Vec<u8>,
    pub checksum: String,
}

impl Message {
    /// Create a new message
    pub fn new(
        message_type: MessageType,
        sender: String,
        receiver: String,
        payload: Vec<u8>,
    ) -> Self {
        let metadata = MessageMetadata {
            id: Uuid::new_v4(),
            message_type,
            priority: Priority::Normal,
            created_at: Utc::now(),
            sender,
            receiver,
            message_reference: None,
        };

        let checksum = format!("{:x}", md5::compute(&payload));

        Self {
            metadata,
            payload,
            checksum,
        }
    }

    /// Validate message integrity
    pub fn validate_integrity(&self) -> bool {
        let computed = format!("{:x}", md5::compute(&self.payload));
        computed == self.checksum
    }
}
