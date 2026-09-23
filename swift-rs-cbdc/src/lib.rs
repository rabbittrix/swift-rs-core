//! Full-reserve CBDC bridge.
//!
//! Each currency is pre-issued only up to its declared backing. A cross-border
//! payment locks the source units in escrow and releases destination units from
//! that currency's inventory in the same atomic batch.

use std::collections::BTreeMap;


use swift_rs_blockchain::merkle::{self, MerkleProof};
use swift_rs_blockchain::wasm::within_limit;
use swift_rs_blockchain::{
    Address, AssetId, ChainError, Hash, KeyPair, SignedTransaction, TxBody,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CbdcError {
    #[error("quote is zero")]
    ZeroQuote,
    #[error("overflow")]
    Overflow,
    #[error("escrow inventory is short")]
    ShortInventory,
    #[error("mint would exceed backing")]
    ExceedsBacking,
    #[error("unknown receipt")]
    UnknownReceipt,
    #[error("receipt already redeemed")]
    AlreadyRedeemed,
    #[error("chain: {0}")]
    Chain(String),
    #[error("amount exceeds the programmable limit")]
    OverLimit,
}

#[derive(Clone, Debug)]
pub struct CurrencyReserve {
    pub asset: AssetId,
    pub issuer: Address,
    pub backing: u128,
    pub minimum_escrow: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockReceipt {
    pub id: Hash,
    pub sender: Address,
    pub recipient: Address,
    pub source_asset: AssetId,
    pub source_amount: u128,
    pub dest_asset: AssetId,
    pub dest_amount: u128,
    pub redeemed: bool,
}

#[derive(Clone, Debug)]
pub struct ReserveLeaf {
    pub asset: String,
    pub backing: u128,
    pub circulating: u128,
}

#[derive(Clone, Debug)]
pub struct RebalanceSignal {
    pub asset: String,
    pub escrow_balance: u128,
    pub minimum_escrow: u128,
}

pub fn quote(source_amount: u128, rate_e8: u128) -> Result<u128, CbdcError> {
    if rate_e8 == 0 || source_amount == 0 {
        return Err(CbdcError::ZeroQuote);
    }
    let dest = source_amount
        .checked_mul(rate_e8)
        .ok_or(CbdcError::Overflow)?
        / 100_000_000;
    if dest == 0 {
        Err(CbdcError::ZeroQuote)
    } else {
        Ok(dest)
    }
}

pub fn assert_backing(supply_after: u128, backing: u128) -> Result<(), CbdcError> {
    if supply_after > backing {
        Err(CbdcError::ExceedsBacking)
    } else {
        Ok(())
    }
}

pub fn lock_and_release(
    escrow: &KeyPair,
    sender: &KeyPair,
    recipient: &Address,
    source: &AssetId,
    dest: &AssetId,
    source_amount: u128,
    rate_e8: u128,
    sender_nonce: u64,
    escrow_nonce: u64,
    fee: u128,
    timestamp_ms: u64,
    escrow_dest_balance: u128,
    purpose: &str,
    spending_limit: u128,
) -> Result<(Vec<SignedTransaction>, LockReceipt), CbdcError> {
    if !within_limit(source_amount, spending_limit).map_err(|e| CbdcError::Chain(e.to_string()))? {
        return Err(CbdcError::OverLimit);
    }
    let dest_amount = quote(source_amount, rate_e8)?;
    let escrow_need = dest_amount
        .checked_add(fee)
        .ok_or(CbdcError::Overflow)?;
    if escrow_dest_balance < escrow_need {
        return Err(CbdcError::ShortInventory);
    }
    let lock = SignedTransaction::sign(
        TxBody {
            from: sender.address,
            to: escrow.address,
            asset: source.clone(),
            amount: source_amount,
            fee,
            nonce: sender_nonce,
            purpose: purpose.to_string(),
            timestamp_ms,
        },
        sender,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    let release = SignedTransaction::sign(
        TxBody {
            from: escrow.address,
            to: *recipient,
            asset: dest.clone(),
            amount: dest_amount,
            fee,
            nonce: escrow_nonce,
            purpose: purpose.to_string(),
            timestamp_ms,
        },
        escrow,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    let mut id_material = lock.body.id().map_err(|e| CbdcError::Chain(e.to_string()))?.0.to_vec();
    id_material.extend_from_slice(&release.body.id().map_err(|e| CbdcError::Chain(e.to_string()))?.0);
    Ok((
        vec![lock, release],
        LockReceipt {
            id: Hash::sha256(&id_material),
            sender: sender.address,
            recipient: *recipient,
            source_asset: source.clone(),
            source_amount,
            dest_asset: dest.clone(),
            dest_amount,
            redeemed: false,
        },
    ))
}

pub fn redeem(
    escrow: &KeyPair,
    recipient: &KeyPair,
    receipt: &LockReceipt,
    recipient_nonce: u64,
    escrow_nonce: u64,
    fee: u128,
    timestamp_ms: u64,
    escrow_source_balance: u128,
) -> Result<Vec<SignedTransaction>, CbdcError> {
    if receipt.redeemed {
        return Err(CbdcError::AlreadyRedeemed);
    }
    if recipient.address != receipt.recipient {
        return Err(CbdcError::UnknownReceipt);
    }
    let unlock_need = receipt
        .source_amount
        .checked_add(fee)
        .ok_or(CbdcError::Overflow)?;
    if escrow_source_balance < unlock_need {
        return Err(CbdcError::ShortInventory);
    }
    let burn = SignedTransaction::sign(
        TxBody {
            from: recipient.address,
            to: escrow.address,
            asset: receipt.dest_asset.clone(),
            amount: receipt.dest_amount,
            fee,
            nonce: recipient_nonce,
            purpose: "bridge-redeem".into(),
            timestamp_ms,
        },
        recipient,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    let unlock = SignedTransaction::sign(
        TxBody {
            from: escrow.address,
            to: receipt.sender,
            asset: receipt.source_asset.clone(),
            amount: receipt.source_amount,
            fee,
            nonce: escrow_nonce,
            purpose: "bridge-redeem".into(),
            timestamp_ms,
        },
        escrow,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    Ok(vec![burn, unlock])
}

pub fn atomic_swap(
    party_a: &KeyPair,
    party_b: &KeyPair,
    asset_a: &AssetId,
    asset_b: &AssetId,
    amount_a: u128,
    amount_b: u128,
    nonce_a: u64,
    nonce_b: u64,
    fee: u128,
    timestamp_ms: u64,
) -> Result<Vec<SignedTransaction>, CbdcError> {
    let left = SignedTransaction::sign(
        TxBody {
            from: party_a.address,
            to: party_b.address,
            asset: asset_a.clone(),
            amount: amount_a,
            fee,
            nonce: nonce_a,
            purpose: "atomic-swap".into(),
            timestamp_ms,
        },
        party_a,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    let right = SignedTransaction::sign(
        TxBody {
            from: party_b.address,
            to: party_a.address,
            asset: asset_b.clone(),
            amount: amount_b,
            fee,
            nonce: nonce_b,
            purpose: "atomic-swap".into(),
            timestamp_ms,
        },
        party_b,
    )
    .map_err(|e: ChainError| CbdcError::Chain(e.to_string()))?;
    Ok(vec![left, right])
}

pub fn reserve_root(leaves: &[ReserveLeaf]) -> Hash {
    let hashed = leaves
        .iter()
        .map(|leaf| {
            let mut buf = leaf.asset.as_bytes().to_vec();
            buf.extend_from_slice(&leaf.backing.to_be_bytes());
            buf.extend_from_slice(&leaf.circulating.to_be_bytes());
            Hash::sha256(&buf)
        })
        .collect::<Vec<_>>();
    merkle::merkle_root(&hashed)
}

pub fn reserve_proof(leaves: &[ReserveLeaf], index: usize) -> Option<(Hash, MerkleProof)> {
    let hashed = leaves
        .iter()
        .map(|leaf| {
            let mut buf = leaf.asset.as_bytes().to_vec();
            buf.extend_from_slice(&leaf.backing.to_be_bytes());
            buf.extend_from_slice(&leaf.circulating.to_be_bytes());
            Hash::sha256(&buf)
        })
        .collect::<Vec<_>>();
    let proof = merkle::merkle_proof(&hashed, index)?;
    Some((merkle::merkle_root(&hashed), proof))
}

pub fn rebalance_signals(
    reserves: &BTreeMap<String, CurrencyReserve>,
    escrow_balance: impl Fn(&AssetId) -> u128,
) -> Vec<RebalanceSignal> {
    reserves
        .values()
        .filter_map(|reserve| {
            let balance = escrow_balance(&reserve.asset);
            if balance < reserve.minimum_escrow {
                Some(RebalanceSignal {
                    asset: reserve.asset.0.clone(),
                    escrow_balance: balance,
                    minimum_escrow: reserve.minimum_escrow,
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use swift_rs_blockchain::merkle::verify_proof;

    #[test]
    fn quote_and_backing() {
        assert_eq!(quote(1_000, 120_000_000).unwrap(), 1_200);
        assert_eq!(assert_backing(10, 10), Ok(()));
        assert_eq!(assert_backing(11, 10), Err(CbdcError::ExceedsBacking));
    }

    #[test]
    fn reserve_proof_verifies() {
        let leaves = vec![
            ReserveLeaf {
                asset: "cbdc:brl".into(),
                backing: 100,
                circulating: 40,
            },
            ReserveLeaf {
                asset: "cbdc:cny".into(),
                backing: 80,
                circulating: 10,
            },
        ];
        let (root, proof) = reserve_proof(&leaves, 1).unwrap();
        let mut buf = b"cbdc:cny".to_vec();
        buf.extend_from_slice(&80u128.to_be_bytes());
        buf.extend_from_slice(&10u128.to_be_bytes());
        assert!(verify_proof(&Hash::sha256(&buf), &proof, &root));
    }
}
