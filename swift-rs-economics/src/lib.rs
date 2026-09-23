//! Fee market, slashing amounts, and the documented genesis allocation.
//!
//! The base fee follows an EIP-1559-style adjustment. The base fee itself is
//! burned by the ledger. A tip above the base fee is paid to the block proposer.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EcoError {
    #[error("fee is below the base fee")]
    FeeTooLow,
    #[error("allocation does not sum to 100%")]
    BadAllocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlashReason {
    DoubleSign,
    Downtime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationBps {
    pub validators: u32,
    pub ecosystem: u32,
    pub founding: u32,
    pub treasury: u32,
}

impl AllocationBps {
    pub const GENESIS: Self = Self {
        validators: 4_000,
        ecosystem: 3_000,
        founding: 2_000,
        treasury: 1_000,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub validators: u128,
    pub ecosystem: u128,
    pub founding: u128,
    pub treasury: u128,
}

pub fn next_base_fee(parent_base: u128, parent_tx_count: u64, target_tx_count: u64) -> u128 {
    let base = parent_base.max(1);
    if target_tx_count == 0 || parent_tx_count == target_tx_count {
        return base;
    }
    if parent_tx_count > target_tx_count {
        let delta = (base.saturating_mul(u128::from(parent_tx_count - target_tx_count))
            / u128::from(target_tx_count)
            / 8)
            .max(1);
        base.saturating_add(delta)
    } else {
        let delta = (base.saturating_mul(u128::from(target_tx_count - parent_tx_count))
            / u128::from(target_tx_count)
            / 8)
            .max(1);
        base.saturating_sub(delta).max(1)
    }
}

pub fn split_fee(fee: u128, base_fee: u128) -> Result<(u128, u128), EcoError> {
    if fee < base_fee {
        return Err(EcoError::FeeTooLow);
    }
    Ok((base_fee, fee - base_fee))
}

pub fn slash_amount(stake: u128, reason: SlashReason) -> u128 {
    let bps: u128 = match reason {
        SlashReason::DoubleSign => 500,
        SlashReason::Downtime => 100,
    };
    stake.saturating_mul(bps) / 10_000
}

pub fn eligible_validator(stake: u128, minimum: u128) -> bool {
    stake >= minimum && minimum > 0
}

pub fn allocate(supply: u128, bps: AllocationBps) -> Result<Allocation, EcoError> {
    let total = bps
        .validators
        .saturating_add(bps.ecosystem)
        .saturating_add(bps.founding)
        .saturating_add(bps.treasury);
    if total != 10_000 {
        return Err(EcoError::BadAllocation);
    }
    let share = |points: u32| supply.saturating_mul(u128::from(points)) / 10_000;
    let allocation = Allocation {
        validators: share(bps.validators),
        ecosystem: share(bps.ecosystem),
        founding: share(bps.founding),
        treasury: share(bps.treasury),
    };
    Ok(allocation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_fee_moves_toward_the_target() {
        assert!(next_base_fee(100, 20, 10) > 100);
        assert!(next_base_fee(100, 1, 10) < 100);
        assert_eq!(next_base_fee(1, 1, 10), 1);
    }

    #[test]
    fn slashing_and_allocation() {
        assert_eq!(slash_amount(1_000, SlashReason::DoubleSign), 50);
        assert_eq!(slash_amount(1_000, SlashReason::Downtime), 10);
        assert!(!eligible_validator(99, 100));
        let allocation = allocate(1_000, AllocationBps::GENESIS).unwrap();
        assert_eq!(allocation.validators, 400);
        assert_eq!(allocation.ecosystem, 300);
        assert_eq!(allocation.founding, 200);
        assert_eq!(allocation.treasury, 100);
        assert_eq!(split_fee(5, 2).unwrap(), (2, 3));
    }
}
