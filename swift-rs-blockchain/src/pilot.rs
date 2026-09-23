//! Local pilot of five central-bank validators.
//!
//! Each bank signs the block with ed25519 and ML-DSA. The round is accepted
//! only when the BFT certificate reaches 67% and every hybrid signature verifies.
//! This is an in-process consortium, not a connection to a live central bank.

use crate::block::vote_message;
use crate::consensus::{Consensus, Validator};
use crate::error::ChainError;
use crate::keys::KeyPair;
use crate::node::{GenesisAllocation, Node};
use crate::pq::{sign_hybrid, verify_hybrid, HybridSignature, MlDsaKey};
use crate::tx::{SignedTransaction, TxBody};
use crate::types::AssetId;

pub struct PilotBank {
    pub name: &'static str,
    pub classical: KeyPair,
    pub ml_dsa: MlDsaKey,
}

impl PilotBank {
    fn new(name: &'static str, seed: u64) -> Result<Self, ChainError> {
        let ml_dsa = MlDsaKey::generate(seed).map_err(|err| ChainError::InvalidBlock(err.into()))?;
        Ok(Self {
            name,
            classical: KeyPair::from_seed(name.as_bytes()),
            ml_dsa,
        })
    }
}

pub struct PilotRound {
    pub height: u64,
    pub banks: usize,
    pub hybrid_signatures: usize,
}

pub fn run_five_bank_pilot() -> Result<PilotRound, ChainError> {
    let banks = [
        PilotBank::new("br-cb", 1)?,
        PilotBank::new("cn-cb", 2)?,
        PilotBank::new("in-cb", 3)?,
        PilotBank::new("ae-cb", 4)?,
        PilotBank::new("eu-cb", 5)?,
    ];
    let payer = KeyPair::from_seed(b"pilot-payer");
    let payee = KeyPair::from_seed(b"pilot-payee");
    let validators = banks
        .iter()
        .map(|bank| Validator {
            address: bank.classical.address,
            stake: 100,
            verifying_key: bank.classical.verifying,
            signing_key: Some(bank.classical.clone()),
            jailed: false,
        })
        .collect();
    let mut node = Node::new(Consensus::new(validators), 1);
    for bank in &banks {
        node.register(&bank.classical.verifying)?;
    }
    node.register(&payer.verifying)?;
    node.register(&payee.verifying)?;
    let asset = AssetId::new("cbdc:brl").map_err(ChainError::InvalidTransaction)?;
    node.seal_genesis(
        1_700_000_000_000,
        vec![GenesisAllocation {
            to: payer.address,
            asset: asset.clone(),
            amount: 10_000,
        }],
    )?;
    let tx = SignedTransaction::sign(
        TxBody {
            from: payer.address,
            to: payee.address,
            asset: asset.clone(),
            amount: 500,
            fee: 1,
            nonce: 0,
            purpose: "pilot".into(),
            timestamp_ms: 1_700_000_000_100,
        },
        &payer,
    )?;
    let block = node.finalize_batch(vec![tx], 1_700_000_000_100)?;
    let certificate = block
        .certificate
        .as_ref()
        .ok_or_else(|| ChainError::InvalidBlock("missing certificate".into()))?;
    for bank in &banks {
        node.consensus
            .verify_certificate(certificate, block.header.hash())?;
        let _ = bank;
    }
    let message = vote_message(block.header.height, &block.header.hash());
    let hybrid: Vec<HybridSignature> = banks
        .iter()
        .map(|bank| {
            sign_hybrid(&bank.classical, &bank.ml_dsa, &message)
                .map_err(|err| ChainError::InvalidBlock(err.into()))
        })
        .collect::<Result<Vec<_>, ChainError>>()?;
    for (bank, signature) in banks.iter().zip(&hybrid) {
        if !verify_hybrid(
            &bank.classical.verifying,
            &bank.ml_dsa.public_key,
            &message,
            signature,
        ) {
            return Err(ChainError::BadSignature);
        }
    }
    if node.ledger.balance(&payee.address, &asset) != 500 {
        return Err(ChainError::InvalidBlock("pilot payment did not settle".into()));
    }
    node.audit()?;
    Ok(PilotRound {
        height: block.header.height,
        banks: banks.len(),
        hybrid_signatures: hybrid.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_central_banks_finalize_a_hybrid_signed_payment() {
        let round = run_five_bank_pilot().unwrap();
        assert_eq!(round.banks, 5);
        assert_eq!(round.hybrid_signatures, 5);
        assert_eq!(round.height, 1);
    }
}
