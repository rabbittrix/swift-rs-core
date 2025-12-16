//! Swift-RS AI: AI/ML inference layer
//!
//! Provides fraud detection, anomaly detection, and intelligent message mapping
//! using local LLM/ML inference (ONNX/Torch).

pub mod fraud;
pub mod mapping;
pub mod models;

pub use fraud::FraudDetector;
pub use mapping::MessageMapper;
