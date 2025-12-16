//! Domain event types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::aggregate::AggregateId;

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: Uuid,
    pub aggregate_id: AggregateId,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
}

/// Domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub metadata: EventMetadata,
    pub payload: serde_json::Value,
}

impl DomainEvent {
    /// Create a new domain event
    pub fn new(
        aggregate_id: AggregateId,
        event_type: String,
        payload: serde_json::Value,
        version: u64,
    ) -> Self {
        Self {
            metadata: EventMetadata {
                event_id: Uuid::new_v4(),
                aggregate_id,
                event_type,
                occurred_at: Utc::now(),
                version,
            },
            payload,
        }
    }
}
