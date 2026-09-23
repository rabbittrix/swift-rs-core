use ed25519_dalek::VerifyingKey;
use std::collections::BTreeMap;

use crate::error::ChainError;
use crate::hashutil::sha256;
use crate::keys::address_from_verifying_key;
use crate::tx::SignedTransaction;
use crate::types::{Address, AssetId, Hash};

#[derive(Clone, Debug)]
pub struct Account {
    pub verifying_key: VerifyingKey,
    pub nonce: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Ledger {
    pub accounts: BTreeMap<Address, Account>,
    pub balances: BTreeMap<(Address, String), u128>,
    pub supply: BTreeMap<String, u128>,
    pub burned: BTreeMap<String, u128>,
}

impl Ledger {
    pub fn register(&mut self, key: &VerifyingKey) -> Address {
        let address = address_from_verifying_key(key);
        self.accounts.entry(address).or_insert(Account {
            verifying_key: *key,
            nonce: 0,
        });
        address
    }

    pub fn balance(&self, address: &Address, asset: &AssetId) -> u128 {
        self.balances
            .get(&(address.clone(), asset.0.clone()))
            .copied()
            .unwrap_or(0)
    }

    pub fn nonce(&self, address: &Address) -> u64 {
        self.accounts.get(address).map(|a| a.nonce).unwrap_or(0)
    }

    pub fn mint(&mut self, to: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        if amount == 0 {
            return Err(ChainError::InvalidTransaction("mint amount".into()));
        }
        if !self.accounts.contains_key(to) {
            return Err(ChainError::UnknownAccount);
        }
        let balance = self.balance(to, asset);
        let next = balance
            .checked_add(amount)
            .ok_or_else(|| ChainError::InvalidTransaction("balance overflow".into()))?;
        self.balances.insert((to.clone(), asset.0.clone()), next);
        let supply = self.supply.get(&asset.0).copied().unwrap_or(0);
        self.supply.insert(
            asset.0.clone(),
            supply
                .checked_add(amount)
                .ok_or_else(|| ChainError::InvalidTransaction("supply overflow".into()))?,
        );
        Ok(())
    }

    pub fn burn(&mut self, from: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        if self.balance(from, asset) < amount {
            return Err(ChainError::InsufficientBalance);
        }
        let supply = self.supply.get(&asset.0).copied().unwrap_or(0);
        if supply < amount {
            return Err(ChainError::InvalidTransaction("supply underflow".into()));
        }
        self.debit(from, asset, amount)?;
        self.supply.insert(asset.0.clone(), supply - amount);
        let burned = self.burned.get(&asset.0).copied().unwrap_or(0);
        self.burned
            .insert(asset.0.clone(), burned.saturating_add(amount));
        Ok(())
    }

    pub fn apply(
        &mut self,
        tx: &SignedTransaction,
        proposer: &Address,
        base_fee: u128,
    ) -> Result<(), ChainError> {
        let (verifying_key, nonce) = {
            let account = self
                .accounts
                .get(&tx.body.from)
                .ok_or(ChainError::UnknownAccount)?;
            (account.verifying_key, account.nonce)
        };
        tx.verify(&verifying_key)?;
        if !self.accounts.contains_key(&tx.body.to) {
            return Err(ChainError::UnknownAccount);
        }
        if tx.body.nonce != nonce {
            return Err(ChainError::BadNonce);
        }
        if tx.body.fee < base_fee {
            return Err(ChainError::InvalidTransaction("fee below base fee".into()));
        }
        let total = tx
            .body
            .amount
            .checked_add(tx.body.fee)
            .ok_or_else(|| ChainError::InvalidTransaction("amount overflow".into()))?;
        if self.balance(&tx.body.from, &tx.body.asset) < total {
            return Err(ChainError::InsufficientBalance);
        }
        if base_fee > 0 {
            let supply = self.supply.get(&tx.body.asset.0).copied().unwrap_or(0);
            if supply < base_fee {
                return Err(ChainError::InvalidTransaction("fee exceeds supply".into()));
            }
        }
        self.debit(&tx.body.from, &tx.body.asset, total)?;
        self.credit(&tx.body.to, &tx.body.asset, tx.body.amount)?;
        let tip = tx.body.fee - base_fee;
        if tip > 0 {
            if !self.accounts.contains_key(proposer) {
                return Err(ChainError::UnknownAccount);
            }
            self.credit(proposer, &tx.body.asset, tip)?;
        }
        if base_fee > 0 {
            let supply = self.supply.get(&tx.body.asset.0).copied().unwrap_or(0);
            self.supply
                .insert(tx.body.asset.0.clone(), supply - base_fee);
            let burned = self.burned.get(&tx.body.asset.0).copied().unwrap_or(0);
            self.burned
                .insert(tx.body.asset.0.clone(), burned.saturating_add(base_fee));
        }
        let account = self
            .accounts
            .get_mut(&tx.body.from)
            .ok_or(ChainError::UnknownAccount)?;
        account.nonce = account.nonce.saturating_add(1);
        Ok(())
    }

    pub fn state_root(&self) -> Hash {
        let mut buf = Vec::new();
        for ((address, asset), amount) in &self.balances {
            buf.extend_from_slice(&address.0);
            buf.extend_from_slice(asset.as_bytes());
            buf.extend_from_slice(&amount.to_be_bytes());
        }
        for (address, account) in &self.accounts {
            buf.extend_from_slice(&address.0);
            buf.extend_from_slice(&account.nonce.to_be_bytes());
        }
        Hash(sha256(&buf))
    }

    /// Sum of balances equals supply for every asset.
    pub fn conserved(&self) -> bool {
        let mut totals: BTreeMap<String, u128> = BTreeMap::new();
        for ((_, asset), amount) in &self.balances {
            let entry = totals.entry(asset.clone()).or_insert(0);
            *entry = entry.saturating_add(*amount);
        }
        for (asset, supply) in &self.supply {
            if totals.get(asset).copied().unwrap_or(0) != *supply {
                return false;
            }
        }
        totals.keys().all(|asset| self.supply.contains_key(asset))
    }

    fn debit(&mut self, address: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        let current = self.balance(address, asset);
        if current < amount {
            return Err(ChainError::InsufficientBalance);
        }
        let next = current - amount;
        if next == 0 {
            self.balances.remove(&(address.clone(), asset.0.clone()));
        } else {
            self.balances.insert((address.clone(), asset.0.clone()), next);
        }
        Ok(())
    }

    fn credit(&mut self, address: &Address, asset: &AssetId, amount: u128) -> Result<(), ChainError> {
        let next = self
            .balance(address, asset)
            .checked_add(amount)
            .ok_or_else(|| ChainError::InvalidTransaction("balance overflow".into()))?;
        self.balances.insert((address.clone(), asset.0.clone()), next);
        Ok(())
    }
}
