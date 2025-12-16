//! Aggregate root trait

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::event::DomainEvent;

/// Aggregate identifier
pub type AggregateId = Uuid;

/// Aggregate root trait
#[async_trait]
pub trait Aggregate: Send + Sync {
    /// Get the aggregate ID
    fn id(&self) -> AggregateId;

    /// Get the current version
    fn version(&self) -> u64;

    /// Apply a domain event to the aggregate
    fn apply_event(&mut self, event: &DomainEvent);

    /// Get all uncommitted events
    fn uncommitted_events(&self) -> Vec<DomainEvent>;

    /// Mark events as committed
    fn mark_events_committed(&mut self);
}
