//! Command trait

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Command trait for CQRS
#[async_trait]
pub trait Command: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// Command type identifier
    fn command_type(&self) -> &'static str;
}
