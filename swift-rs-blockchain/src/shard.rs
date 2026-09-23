use std::collections::BTreeMap;

use crate::error::ChainError;
use crate::types::{Address, AssetId};

#[derive(Clone, Debug)]
pub struct Shard {
    pub id: u16,
    balances: BTreeMap<(Address, String), u128>,
}

impl Shard {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            balances: BTreeMap::new(),
        }
    }

    pub fn credit(&mut self, address: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        let key = (address.clone(), asset.0.clone());
        let next = self
            .balances
            .get(&key)
            .copied()
            .unwrap_or(0)
            .checked_add(amount)
            .ok_or_else(|| ChainError::ShardAbort("overflow".into()))?;
        self.balances.insert(key, next);
        Ok(())
    }

    pub fn balance(&self, address: &Address, asset: &AssetId) -> u128 {
        self.balances
            .get(&(address.clone(), asset.0.clone()))
            .copied()
            .unwrap_or(0)
    }

    fn debit(&mut self, address: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        let key = (address.clone(), asset.0.clone());
        let current = self.balances.get(&key).copied().unwrap_or(0);
        if current < amount {
            return Err(ChainError::ShardAbort("insufficient balance".into()));
        }
        let next = current - amount;
        if next == 0 {
            self.balances.remove(&key);
        } else {
            self.balances.insert(key, next);
        }
        Ok(())
    }
}

pub struct ShardLayout {
    pub shards: Vec<Shard>,
}

impl ShardLayout {
    pub fn new(count: u16) -> Self {
        let shards = (0..count).map(Shard::new).collect();
        Self { shards }
    }

    pub fn shard_index(&self, asset: &AssetId) -> usize {
        let bytes = asset.0.as_bytes();
        let first = bytes.first().copied().unwrap_or(0) as usize;
        first % self.shards.len()
    }

    /// Two-phase transfer. A failed credit rolls the debit back.
    pub fn transfer(
        &mut self,
        from_shard: usize,
        to_shard: usize,
        from: &Address,
        to: &Address,
        asset: &AssetId,
        amount: u128,
    ) -> Result<(), ChainError> {
        if from_shard >= self.shards.len() || to_shard >= self.shards.len() {
            return Err(ChainError::ShardAbort("shard missing".into()));
        }
        self.shards[from_shard].debit(from, asset, amount)?;
        if let Err(err) = self.shards[to_shard].credit(to, asset, amount) {
            let _ = self.shards[from_shard].credit(from, asset, amount);
            return Err(err);
        }
        Ok(())
    }

    /// Prepare both debits, then commit both credits. Either asset failing aborts both.
    pub fn atomic_swap(
        &mut self,
        a_shard: usize,
        b_shard: usize,
        party_a: &Address,
        party_b: &Address,
        asset_a: &AssetId,
        asset_b: &AssetId,
        amount_a: u128,
        amount_b: u128,
    ) -> Result<(), ChainError> {
        let snap_a = self.shards[a_shard].balances.clone();
        let snap_b = self.shards[b_shard].balances.clone();
        let result = (|| {
            self.shards[a_shard].debit(party_a, asset_a, amount_a)?;
            self.shards[b_shard].debit(party_b, asset_b, amount_b)?;
            self.shards[b_shard].credit(party_b, asset_a, amount_a)?;
            self.shards[a_shard].credit(party_a, asset_b, amount_b)?;
            Ok(())
        })();
        if result.is_err() {
            self.shards[a_shard].balances = snap_a;
            self.shards[b_shard].balances = snap_b;
        }
        result
    }
}
