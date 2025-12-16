//! ISO 20022 XML parser

use crate::models::CreditTransfer;
use swift_rs_core::SwiftError;
use tracing::debug;

/// High-performance ISO 20022 parser
pub struct Iso20022Parser;

impl Iso20022Parser {
    /// Parse ISO 20022 XML message
    pub fn parse_mx(xml: &[u8]) -> Result<CreditTransfer, SwiftError> {
        debug!("Parsing ISO 20022 MX message ({} bytes)", xml.len());

        let xml_str = std::str::from_utf8(xml)
            .map_err(|e| SwiftError::Format(format!("Invalid UTF-8: {}", e)))?;

        // Use serde-xml-rs for parsing
        // In production, this would use a more efficient streaming parser
        let doc: CreditTransfer = serde_xml_rs::from_str(xml_str)
            .map_err(|e| SwiftError::Format(format!("XML parse error: {}", e)))?;

        Ok(doc)
    }

    /// Validate XML structure without full parsing
    pub fn validate_structure(xml: &[u8]) -> Result<(), SwiftError> {
        if xml.is_empty() {
            return Err(SwiftError::Format("Empty XML document".to_string()));
        }

        let xml_str = std::str::from_utf8(xml)
            .map_err(|e| SwiftError::Format(format!("Invalid UTF-8: {}", e)))?;

        // Basic XML structure validation
        if !xml_str.trim_start().starts_with('<') {
            return Err(SwiftError::Format("Not a valid XML document".to_string()));
        }

        Ok(())
    }
}
