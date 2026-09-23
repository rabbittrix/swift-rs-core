//! Optimistic rollup.
//!
//! The sequencer applies a batch immediately and posts the resulting state root.
//! During the challenge window anyone can replay the batch. A mismatched root
//! rolls the batch back. After the window the batch is final.

use crate::error::ChainError;
use crate::ledger::Ledger;
use crate::tx::SignedTransaction;
use crate::types::{Address, Hash};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,
    Final,
    Rejected,
}

#[derive(Clone, Debug)]
struct Batch {
    parent_root: Hash,
    claimed_root: Hash,
    transactions: Vec<SignedTransaction>,
    proposer: Address,
    base_fee: u128,
    posted_ms: u64,
    preimage: Ledger,
    status: BatchStatus,
}

pub struct Rollup {
    ledger: Ledger,
    batches: Vec<Batch>,
    window_ms: u64,
}

impl Rollup {
    pub fn open(ledger: Ledger, window_ms: u64) -> Self {
        Self {
            ledger,
            batches: Vec::new(),
            window_ms,
        }
    }

    pub fn state_root(&self) -> Hash {
        self.ledger.state_root()
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    pub fn sequence(
        &mut self,
        transactions: Vec<SignedTransaction>,
        proposer: Address,
        base_fee: u128,
        now_ms: u64,
    ) -> Result<usize, ChainError> {
        if transactions.is_empty() {
            return Err(ChainError::EmptyBatch);
        }
        if self.batches.iter().any(|batch| batch.status == BatchStatus::Pending) {
            return Err(ChainError::Rollup("a batch is still pending".into()));
        }
        let preimage = self.ledger.clone();
        let parent_root = preimage.state_root();
        for tx in &transactions {
            self.ledger.apply(tx, &proposer, base_fee)?;
        }
        let index = self.batches.len();
        self.batches.push(Batch {
            parent_root,
            claimed_root: self.ledger.state_root(),
            transactions,
            proposer,
            base_fee,
            posted_ms: now_ms,
            preimage,
            status: BatchStatus::Pending,
        });
        Ok(index)
    }

    /// Replay the batch. `true` means the posted root was false and the state was reverted.
    pub fn challenge(&mut self, index: usize) -> Result<bool, ChainError> {
        let batch = self
            .batches
            .get(index)
            .ok_or_else(|| ChainError::Rollup("unknown batch".into()))?;
        if batch.status != BatchStatus::Pending {
            return Err(ChainError::Rollup("batch is not pending".into()));
        }
        if batch.preimage.state_root() != batch.parent_root {
            return Err(ChainError::Rollup("parent root mismatch".into()));
        }
        let mut replay = batch.preimage.clone();
        for tx in &batch.transactions {
            replay.apply(tx, &batch.proposer, batch.base_fee)?;
        }
        let fraud = replay.state_root() != batch.claimed_root;
        if fraud {
            self.ledger = self.batches[index].preimage.clone();
            self.batches[index].status = BatchStatus::Rejected;
        }
        Ok(fraud)
    }

    pub fn finalize(&mut self, index: usize, now_ms: u64) -> Result<Hash, ChainError> {
        let batch = self
            .batches
            .get_mut(index)
            .ok_or_else(|| ChainError::Rollup("unknown batch".into()))?;
        if batch.status != BatchStatus::Pending {
            return Err(ChainError::Rollup("batch is not pending".into()));
        }
        if now_ms < batch.posted_ms.saturating_add(self.window_ms) {
            return Err(ChainError::Rollup("challenge window is open".into()));
        }
        batch.status = BatchStatus::Final;
        Ok(batch.claimed_root)
    }

    /// Test hook: replace the posted root so a challenge can detect fraud.
    pub fn tamper_root(&mut self, index: usize, claimed_root: Hash) -> Result<(), ChainError> {
        let batch = self
            .batches
            .get_mut(index)
            .ok_or_else(|| ChainError::Rollup("unknown batch".into()))?;
        if batch.status != BatchStatus::Pending {
            return Err(ChainError::Rollup("batch is not pending".into()));
        }
        batch.claimed_root = claimed_root;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyPair;
    use crate::tx::TxBody;
    use crate::types::AssetId;

    fn funded() -> (Rollup, KeyPair, KeyPair) {
        let alice = KeyPair::from_seed(b"rollup-alice");
        let bob = KeyPair::from_seed(b"rollup-bob");
        let mut ledger = Ledger::default();
        ledger.register(&alice.verifying);
        ledger.register(&bob.verifying);
        let asset = AssetId::new("cbdc:brl").unwrap();
        ledger.mint(&alice.address, &asset, 1_000).unwrap();
        (Rollup::open(ledger, 1_000), alice, bob)
    }

    fn payment(alice: &KeyPair, bob: &KeyPair, nonce: u64) -> SignedTransaction {
        SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: AssetId::new("cbdc:brl").unwrap(),
                amount: 10,
                fee: 1,
                nonce,
                purpose: "trade".into(),
                timestamp_ms: 50,
            },
            alice,
        )
        .unwrap()
    }

    #[test]
    fn honest_batch_finalizes_after_the_window() {
        let (mut rollup, alice, bob) = funded();
        let before = rollup.ledger().balance(&bob.address, &AssetId::new("cbdc:brl").unwrap());
        let index = rollup
            .sequence(vec![payment(&alice, &bob, 0)], alice.address, 1, 100)
            .unwrap();
        assert!(!rollup.challenge(index).unwrap());
        assert!(rollup.finalize(index, 1_099).is_err());
        let root = rollup.finalize(index, 1_100).unwrap();
        assert_eq!(root, rollup.state_root());
        assert_eq!(
            rollup.ledger().balance(&bob.address, &AssetId::new("cbdc:brl").unwrap()),
            before + 10
        );
    }

    #[test]
    fn tampered_root_is_reverted() {
        let (mut rollup, alice, bob) = funded();
        let asset = AssetId::new("cbdc:brl").unwrap();
        let alice_before = rollup.ledger().balance(&alice.address, &asset);
        let index = rollup
            .sequence(vec![payment(&alice, &bob, 0)], alice.address, 1, 100)
            .unwrap();
        rollup.tamper_root(index, Hash::zero()).unwrap();
        assert!(rollup.challenge(index).unwrap());
        assert_eq!(rollup.ledger().balance(&alice.address, &asset), alice_before);
        assert!(rollup.finalize(index, 5_000).is_err());
    }
}
