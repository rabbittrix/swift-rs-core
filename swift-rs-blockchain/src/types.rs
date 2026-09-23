use serde::{Deserialize, Serialize};
use std::fmt;

use crate::hashutil::{self, HashAlgo};

pub const QUORUM_NUMERATOR: u128 = 67;
pub const QUORUM_DENOMINATOR: u128 = 100;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl Address {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self, String> {
        let bytes = hex::decode(value).map_err(|e| e.to_string())?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "address must be 32 bytes".to_string())?;
        Ok(Self(bytes))
    }
}

impl Hash {
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn of(algo: HashAlgo, data: &[u8]) -> Self {
        Self(algo.digest(data))
    }

    pub fn sha256(data: &[u8]) -> Self {
        Self(hashutil::sha256(data))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl AssetId {
    pub fn new(value: &str) -> Result<Self, String> {
        if value.is_empty() || value.len() > 32 {
            return Err("asset id length must be 1..=32".into());
        }
        if !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b':' || b == b'-')
        {
            return Err("asset id has invalid characters".into());
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", &self.to_hex()[..8])
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", &self.to_hex()[..8])
    }
}

impl fmt::Debug for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AssetId({})", self.0)
    }
}

/// Stake-weighted finality threshold from the protocol spec (67%).
pub fn meets_quorum(voted_stake: u128, total_stake: u128) -> bool {
    if total_stake == 0 {
        return false;
    }
    voted_stake.saturating_mul(QUORUM_DENOMINATOR) >= total_stake.saturating_mul(QUORUM_NUMERATOR)
}
