use std::collections::HashSet;
use std::sync::Arc;

use crate::block::{Block, BlockHeader};
use crate::merkle::merkle_root;
use crate::consensus::Consensus;
use crate::error::ChainError;
use crate::ledger::Ledger;
use crate::tx::{SignedTransaction, TxBody};
use crate::types::{Address, AssetId, Hash};

pub type Admission = Arc<dyn Fn(&TxBody) -> Result<(), String> + Send + Sync>;

#[derive(Clone)]
pub struct GenesisAllocation {
    pub to: Address,
    pub asset: AssetId,
    pub amount: u128,
}

pub struct Node {
    pub ledger: Ledger,
    pub blocks: Vec<Block>,
    pub consensus: Consensus,
    pub base_fee: u128,
    pub admission: Option<Admission>,
    pub genesis_allocations: Vec<GenesisAllocation>,
    sealed: bool,
}

impl Node {
    pub fn new(consensus: Consensus, base_fee: u128) -> Self {
        Self {
            ledger: Ledger::default(),
            blocks: Vec::new(),
            consensus,
            base_fee,
            admission: None,
            genesis_allocations: Vec::new(),
            sealed: false,
        }
    }

    pub fn register(&mut self, key: &ed25519_dalek::VerifyingKey) -> Result<Address, ChainError> {
        if self.sealed {
            return Err(ChainError::InvalidBlock(
                "account set is frozen after genesis".into(),
            ));
        }
        Ok(self.ledger.register(key))
    }

    pub fn seal_genesis(
        &mut self,
        timestamp_ms: u64,
        allocations: Vec<GenesisAllocation>,
    ) -> Result<Block, ChainError> {
        if self.sealed {
            return Err(ChainError::InvalidBlock("genesis already sealed".into()));
        }
        for allocation in &allocations {
            self.ledger
                .mint(&allocation.to, &allocation.asset, allocation.amount)?;
        }
        let proposer = self.consensus.next_proposer(0)?;
        let header = BlockHeader {
            height: 0,
            parent: Hash::zero(),
            merkle_root: merkle_root(&[]),
            state_root: self.ledger.state_root(),
            timestamp_ms,
            proposer,
            tx_count: 0,
            base_fee: self.base_fee,
        };
        let block = Block {
            header,
            transactions: Vec::new(),
            certificate: None,
        };
        self.genesis_allocations = allocations;
        self.blocks.push(block.clone());
        self.sealed = true;
        Ok(block)
    }

    pub fn tip_hash(&self) -> Hash {
        self.blocks
            .last()
            .map(|block| block.header.hash())
            .unwrap_or_else(Hash::zero)
    }

    pub fn height(&self) -> u64 {
        self.blocks.last().map(|block| block.header.height).unwrap_or(0)
    }

    pub fn next_height(&self) -> u64 {
        if self.blocks.is_empty() {
            0
        } else {
            self.height().saturating_add(1)
        }
    }

    /// Apply every transaction or none of them, then finalize with a 67% certificate.
    pub fn finalize_batch(
        &mut self,
        txs: Vec<SignedTransaction>,
        timestamp_ms: u64,
    ) -> Result<Block, ChainError> {
        if !self.sealed {
            return Err(ChainError::InvalidBlock("genesis is not sealed".into()));
        }
        if txs.is_empty() {
            return Err(ChainError::EmptyBatch);
        }
        if let Some(parent) = self.blocks.last() {
            if timestamp_ms < parent.header.timestamp_ms {
                return Err(ChainError::InvalidBlock(
                    "timestamp went backwards".into(),
                ));
            }
        }
        let height = self.next_height();
        let parent = self.tip_hash();
        let proposer = self.consensus.next_proposer(height)?;
        let mut scratch = self.ledger.clone();
        let mut seen = HashSet::with_capacity(txs.len());
        for tx in &txs {
            if let Some(admit) = &self.admission {
                admit(&tx.body).map_err(ChainError::Rejected)?;
            }
            let id = tx.body.id()?;
            if !seen.insert(id) {
                return Err(ChainError::Duplicate);
            }
            if tx.body.timestamp_ms > timestamp_ms {
                return Err(ChainError::InvalidTransaction(
                    "timestamp is after the block".into(),
                ));
            }
            scratch.apply(tx, &proposer, self.base_fee)?;
        }
        let block = self.consensus.build_block(
            parent,
            height,
            txs,
            scratch.state_root(),
            timestamp_ms,
            self.base_fee,
        )?;
        self.validate_block(&block, parent)?;
        self.ledger = scratch;
        self.blocks.push(block.clone());
        Ok(block)
    }

    pub fn set_base_fee(&mut self, base_fee: u128) {
        self.base_fee = base_fee;
    }

    pub fn validate_block(&self, block: &Block, expected_parent: Hash) -> Result<(), ChainError> {
        if block.header.parent != expected_parent {
            return Err(ChainError::InvalidBlock("parent mismatch".into()));
        }
        if block.header.height != self.next_height() {
            return Err(ChainError::InvalidBlock("height mismatch".into()));
        }
        if block.tx_root()? != block.header.merkle_root {
            return Err(ChainError::InvalidBlock("merkle root mismatch".into()));
        }
        let certificate = block
            .certificate
            .as_ref()
            .ok_or_else(|| ChainError::InvalidBlock("missing certificate".into()))?;
        self.consensus
            .verify_certificate(certificate, block.header.hash())?;
        Ok(())
    }

    /// Replay genesis allocations and every finalized block.
    pub fn audit(&self) -> Result<(), ChainError> {
        if self.blocks.is_empty() {
            return Err(ChainError::InvalidBlock("empty chain".into()));
        }
        let mut ledger = Ledger::default();
        for account in self.ledger.accounts.values() {
            ledger.register(&account.verifying_key);
        }
        for allocation in &self.genesis_allocations {
            ledger.mint(&allocation.to, &allocation.asset, allocation.amount)?;
        }
        if ledger.state_root() != self.blocks[0].header.state_root {
            return Err(ChainError::InvalidBlock("genesis state root".into()));
        }
        let mut parent = self.blocks[0].header.hash();
        for block in self.blocks.iter().skip(1) {
            if block.header.parent != parent {
                return Err(ChainError::InvalidBlock("broken parent link".into()));
            }
            if block.tx_root()? != block.header.merkle_root {
                return Err(ChainError::InvalidBlock("merkle root mismatch".into()));
            }
            let certificate = block
                .certificate
                .as_ref()
                .ok_or_else(|| ChainError::InvalidBlock("missing certificate".into()))?;
            self.consensus
                .verify_certificate(certificate, block.header.hash())?;
            for tx in &block.transactions {
                ledger.apply(tx, &block.header.proposer, block.header.base_fee)?;
            }
            if ledger.state_root() != block.header.state_root {
                return Err(ChainError::InvalidBlock("state root mismatch".into()));
            }
            parent = block.header.hash();
        }
        if !ledger.conserved() || ledger.balances != self.ledger.balances {
            return Err(ChainError::InvalidBlock("replay diverged".into()));
        }
        Ok(())
    }
}
