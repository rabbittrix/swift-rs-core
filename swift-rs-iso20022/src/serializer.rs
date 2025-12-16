//! ISO 20022 XML serializer

use crate::models::CreditTransfer;
use swift_rs_core::SwiftError;
use tracing::debug;

/// High-performance ISO 20022 serializer
pub struct Iso20022Serializer;

impl Iso20022Serializer {
    /// Serialize ISO 20022 message to XML
    pub fn serialize_mx(message: &CreditTransfer) -> Result<Vec<u8>, SwiftError> {
        debug!("Serializing ISO 20022 MX message");

        let xml = serde_xml_rs::to_string(message)
            .map_err(|e| SwiftError::Serialization(format!("XML serialization error: {}", e)))?;

        Ok(xml.into_bytes())
    }
}
