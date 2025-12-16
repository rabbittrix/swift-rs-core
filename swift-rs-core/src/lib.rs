//! Swift-RS Core: Domain logic and message validation
//!
//! This crate provides the core domain models, validation logic, and business rules
//! for financial messaging in the Swift-RS system.

pub mod domain;
pub mod error;
pub mod validation;

pub use error::{SwiftError, SwiftResult};
