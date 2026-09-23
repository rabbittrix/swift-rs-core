use ed25519_dalek::VerifyingKey;

use crate::error::ChainError;
use crate::hashutil::push_len_bytes;
use crate::keys::{self, KeyPair};
use crate::types::{Address, AssetId, Hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxBody {
    pub from: Address,
    pub to: Address,
    pub asset: AssetId,
    pub amount: u128,
    pub fee: u128,
    pub nonce: u64,
    pub purpose: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedTransaction {
    pub body: TxBody,
    pub signature: [u8; 64],
    /// Cached `TxBody::id()` so batch finalization does not re-hash every transaction twice.
    pub id: Hash,
}

impl TxBody {
    pub fn signing_bytes(&self) -> Result<Vec<u8>, ChainError> {
        let mut out = b"SCBPS-TX-v1".to_vec();
        out.extend_from_slice(&self.from.0);
        out.extend_from_slice(&self.to.0);
        push_len_bytes(&mut out, self.asset.0.as_bytes())
            .map_err(|e| ChainError::InvalidTransaction(e.into()))?;
        out.extend_from_slice(&self.amount.to_be_bytes());
        out.extend_from_slice(&self.fee.to_be_bytes());
        out.extend_from_slice(&self.nonce.to_be_bytes());
        out.extend_from_slice(&self.timestamp_ms.to_be_bytes());
        push_len_bytes(&mut out, self.purpose.as_bytes())
            .map_err(|e| ChainError::InvalidTransaction(e.into()))?;
        Ok(out)
    }

    pub fn id(&self) -> Result<Hash, ChainError> {
        Ok(Hash::sha256(&self.signing_bytes()?))
    }

    pub fn validate_shape(&self) -> Result<(), ChainError> {
        if self.amount == 0 {
            return Err(ChainError::InvalidTransaction("amount must be positive".into()));
        }
        if self.from == self.to {
            return Err(ChainError::InvalidTransaction("self transfer".into()));
        }
        if self.purpose.is_empty() || self.purpose.len() > 64 {
            return Err(ChainError::InvalidTransaction("invalid purpose".into()));
        }
        if !self
            .purpose
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(ChainError::InvalidTransaction("invalid purpose".into()));
        }
        Ok(())
    }
}

impl SignedTransaction {
    pub fn sign(body: TxBody, key: &KeyPair) -> Result<Self, ChainError> {
        body.validate_shape()?;
        if key.address != body.from {
            return Err(ChainError::BadSignature);
        }
        let signing_bytes = body.signing_bytes()?;
        let signature = key.sign(&signing_bytes);
        let id = Hash::sha256(&signing_bytes);
        Ok(Self {
            body,
            signature,
            id,
        })
    }

    pub fn verify(&self, key: &VerifyingKey) -> Result<(), ChainError> {
        self.body.validate_shape()?;
        let message = self.body.signing_bytes()?;
        if keys::verify(key, &message, &self.signature) {
            Ok(())
        } else {
            Err(ChainError::BadSignature)
        }
    }
}
