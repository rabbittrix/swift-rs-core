//! Swift-RS Connector: Secure connectivity layer
//!
//! Provides secure connections to SWIFT network via VPN, SNA, or AMQP protocols.

pub mod error;
pub mod protocols;
pub mod tls;

pub use error::ConnectorError;
pub use protocols::{ConnectionConfig, SwiftConnector};
