//! Swift-RS CQRS: Lightweight Event Sourcing implementation
//!
//! Provides traits and utilities for implementing CQRS (Command Query Responsibility Segregation)
//! and Event Sourcing patterns in Rust, inspired by Axon Framework.

pub mod aggregate;
pub mod command;
pub mod event;
pub mod event_store;

pub use aggregate::{Aggregate, AggregateId};
pub use command::Command;
pub use event::{DomainEvent, EventMetadata};
pub use event_store::{EventStore, EventStoreError};
