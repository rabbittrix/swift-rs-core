use ed25519_dalek::VerifyingKey;


use crate::block::{certificate_weight, vote_message, Block, BlockHeader, QuorumCertificate, Vote};
use crate::error::ChainError;
use crate::keys::{self, KeyPair};
use crate::tx::SignedTransaction;
use crate::types::{meets_quorum, Address, Hash};

#[derive(Clone)]
pub struct Validator {
    pub address: Address,
    pub stake: u128,
    pub verifying_key: VerifyingKey,
    pub signing_key: Option<KeyPair>,
    pub jailed: bool,
}

pub struct Consensus {
    pub validators: Vec<Validator>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlashReason {
    DoubleSign,
    Downtime,
}

impl Consensus {
    pub fn new(validators: Vec<Validator>) -> Self {
        Self { validators }
    }

    pub fn total_stake(&self) -> u128 {
        self.validators
            .iter()
            .map(|v| v.stake)
            .fold(0u128, u128::saturating_add)
    }

    pub fn next_proposer(&self, height: u64) -> Result<Address, ChainError> {
        let active: Vec<&Validator> = self
            .validators
            .iter()
            .filter(|v| !v.jailed && v.stake > 0)
            .collect();
        if active.is_empty() {
            return Err(ChainError::NoProposer);
        }
        let index = (height as usize) % active.len();
        Ok(active[index].address)
    }

    pub fn set_stake(&mut self, address: &Address, stake: u128) {
        if let Some(validator) = self.validators.iter_mut().find(|v| &v.address == address) {
            validator.stake = stake;
        }
    }

    pub fn jail(&mut self, address: &Address, jailed: bool) {
        if let Some(validator) = self.validators.iter_mut().find(|v| &v.address == address) {
            validator.jailed = jailed;
        }
    }

    /// Collect local validator signatures. Finality requires 67% of active stake.
    pub fn certify(&self, height: u64, block_hash: Hash) -> Result<QuorumCertificate, ChainError> {
        let message = vote_message(height, &block_hash);
        let mut votes = Vec::new();
        let mut weight = 0u128;
        for validator in &self.validators {
            if validator.jailed || validator.stake == 0 {
                continue;
            }
            let Some(key) = &validator.signing_key else {
                continue;
            };
            votes.push(Vote {
                voter: validator.address,
                signature: key.sign(&message),
            });
            weight = weight.saturating_add(validator.stake);
        }
        if !meets_quorum(weight, self.total_stake()) {
            return Err(ChainError::QuorumNotReached);
        }
        let certificate = QuorumCertificate {
            block_hash,
            height,
            votes,
        };
        self.verify_certificate(&certificate, block_hash)?;
        Ok(certificate)
    }

    pub fn verify_certificate(
        &self,
        certificate: &QuorumCertificate,
        block_hash: Hash,
    ) -> Result<u128, ChainError> {
        if certificate.block_hash != block_hash {
            return Err(ChainError::InvalidBlock("certificate hash mismatch".into()));
        }
        let view: Vec<_> = self
            .validators
            .iter()
            .map(|v| (v.address, v.stake, v.verifying_key, v.jailed))
            .collect();
        certificate_weight(certificate, &view)
    }

    /// Two conflicting votes at the same height from one validator.
    pub fn detect_double_sign(
        &self,
        height: u64,
        first: &Vote,
        first_hash: &Hash,
        second: &Vote,
        second_hash: &Hash,
    ) -> Result<Address, ChainError> {
        if first.voter != second.voter || first_hash == second_hash {
            return Err(ChainError::InvalidBlock("not an equivocation".into()));
        }
        let validator = self
            .validators
            .iter()
            .find(|v| v.address == first.voter)
            .ok_or_else(|| ChainError::InvalidBlock("unknown validator".into()))?;
        let first_msg = vote_message(height, first_hash);
        let second_msg = vote_message(height, second_hash);
        if keys::verify(&validator.verifying_key, &first_msg, &first.signature)
            && keys::verify(&validator.verifying_key, &second_msg, &second.signature)
        {
            Ok(first.voter)
        } else {
            Err(ChainError::BadSignature)
        }
    }

    pub fn build_block(
        &self,
        parent: Hash,
        height: u64,
        transactions: Vec<SignedTransaction>,
        state_root: Hash,
        timestamp_ms: u64,
        base_fee: u128,
    ) -> Result<Block, ChainError> {
        let proposer = self.next_proposer(height)?;
        let tx_count = u32::try_from(transactions.len())
            .map_err(|_| ChainError::InvalidBlock("too many transactions".into()))?;
        let header = BlockHeader {
            height,
            parent,
            merkle_root: Hash::zero(),
            state_root,
            timestamp_ms,
            proposer,
            tx_count,
            base_fee,
        };
        let mut block = Block {
            header,
            transactions,
            certificate: None,
        };
        block.header.merkle_root = block.tx_root()?;
        let hash = block.header.hash();
        block.certificate = Some(self.certify(height, hash)?);
        Ok(block)
    }
}
