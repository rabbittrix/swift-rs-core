//! Event store trait

use async_trait::async_trait;
use thiserror::Error;

use crate::aggregate::AggregateId;
use crate::event::DomainEvent;

#[derive(Error, Debug)]
pub enum EventStoreError {
    #[error("Aggregate not found: {0}")]
    AggregateNotFound(AggregateId),

    #[error("Concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: u64, actual: u64 },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Event store trait
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Load all events for an aggregate
    async fn load_events(&self, aggregate_id: AggregateId) -> Result<Vec<DomainEvent>, EventStoreError>;

    /// Append events to an aggregate
    async fn append_events(
        &self,
        aggregate_id: AggregateId,
        expected_version: u64,
        events: Vec<DomainEvent>,
    ) -> Result<(), EventStoreError>;

    /// Get all events of a specific type
    async fn get_events_by_type(&self, event_type: &str) -> Result<Vec<DomainEvent>, EventStoreError>;
}

