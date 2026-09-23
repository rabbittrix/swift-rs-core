use crate::error::ChainError;
use crate::keys;
use crate::merkle::merkle_root;
use crate::tx::SignedTransaction;
use crate::types::{meets_quorum, Address, Hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vote {
    pub voter: Address,
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuorumCertificate {
    pub block_hash: Hash,
    pub height: u64,
    pub votes: Vec<Vote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    pub height: u64,
    pub parent: Hash,
    pub merkle_root: Hash,
    pub state_root: Hash,
    pub timestamp_ms: u64,
    pub proposer: Address,
    pub tx_count: u32,
    pub base_fee: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<SignedTransaction>,
    pub certificate: Option<QuorumCertificate>,
}

impl BlockHeader {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = b"SCBPS-BLK-v1".to_vec();
        out.extend_from_slice(&self.height.to_be_bytes());
        out.extend_from_slice(&self.parent.0);
        out.extend_from_slice(&self.merkle_root.0);
        out.extend_from_slice(&self.state_root.0);
        out.extend_from_slice(&self.timestamp_ms.to_be_bytes());
        out.extend_from_slice(&self.proposer.0);
        out.extend_from_slice(&self.tx_count.to_be_bytes());
        out.extend_from_slice(&self.base_fee.to_be_bytes());
        out
    }

    pub fn hash(&self) -> Hash {
        Hash::sha256(&self.signing_bytes())
    }
}

pub fn vote_message(height: u64, block_hash: &Hash) -> Vec<u8> {
    let mut message = b"SCBPS-VOTE-v1".to_vec();
    message.extend_from_slice(&height.to_be_bytes());
    message.extend_from_slice(&block_hash.0);
    message
}

impl Block {
    pub fn tx_root(&self) -> Result<Hash, ChainError> {
        let mut leaves = Vec::with_capacity(self.transactions.len());
        for tx in &self.transactions {
            leaves.push(tx.id);
        }
        Ok(merkle_root(&leaves))
    }
}

pub fn certificate_weight(
    certificate: &QuorumCertificate,
    validators: &[(Address, u128, ed25519_dalek::VerifyingKey, bool)],
) -> Result<u128, ChainError> {
    if certificate.votes.is_empty() {
        return Err(ChainError::QuorumNotReached);
    }
    let message = vote_message(certificate.height, &certificate.block_hash);
    let mut seen = Vec::new();
    let mut weight = 0u128;
    for vote in &certificate.votes {
        if seen.contains(&vote.voter) {
            return Err(ChainError::InvalidBlock("duplicate vote".into()));
        }
        seen.push(vote.voter);
        let Some((_, stake, key, jailed)) = validators.iter().find(|(addr, _, _, _)| addr == &vote.voter)
        else {
            return Err(ChainError::InvalidBlock("vote from unknown validator".into()));
        };
        if *jailed || *stake == 0 {
            return Err(ChainError::InvalidBlock("vote from inactive validator".into()));
        }
        if !keys::verify(key, &message, &vote.signature) {
            return Err(ChainError::BadSignature);
        }
        weight = weight.saturating_add(*stake);
    }
    let total: u128 = validators
        .iter()
        .map(|(_, stake, _, _)| *stake)
        .fold(0u128, u128::saturating_add);
    if !meets_quorum(weight, total) {
        return Err(ChainError::QuorumNotReached);
    }
    Ok(weight)
}
