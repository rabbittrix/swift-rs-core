//! Swift-RS ISO 20022: High-performance MX message serializer/deserializer
//!
//! This crate provides efficient parsing and serialization of ISO 20022 (MX) messages,
//! with support for common message types like pacs.008 (Credit Transfer).

pub mod models;
pub mod parser;
pub mod serializer;

pub use parser::Iso20022Parser;
pub use serializer::Iso20022Serializer;
