//! Compliance screening and selective disclosure.
//!
//! Screening is fail-closed: a sanctioned or unregistered party cannot be
//! included in a block. A Groth16 proof shows the payment is under a public
//! limit and sanctions-clear without revealing the amount. The regulator
//! envelope still opens the underlying payment.

use std::collections::{BTreeMap, BTreeSet};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use swift_rs_blockchain::hashutil::sha256;
use swift_rs_blockchain::{Address, Hash, TxBody};
use thiserror::Error;

pub mod snark;

const DAY_MS: u64 = 86_400_000;
const MONTH_MS: u64 = DAY_MS * 30;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrivacyError {
    #[error("party is on a sanctions list")]
    Sanctioned,
    #[error("party is not in the KYC registry")]
    NotKyc,
    #[error("daily limit exceeded")]
    DailyLimit,
    #[error("monthly limit exceeded")]
    MonthlyLimit,
    #[error("purpose is not allowed")]
    Purpose,
    #[error("timelock is still closed")]
    Timelock,
    #[error("disclosure could not be opened")]
    Crypto,
    #[error("proof error: {0}")]
    Snark(String),
}

#[derive(Clone, Debug)]
pub struct PartyPolicy {
    pub daily_limit: u128,
    pub monthly_limit: u128,
    pub not_before_ms: Option<u64>,
    pub purposes: Option<BTreeSet<String>>,
}

#[derive(Clone, Debug, Default)]
struct Usage {
    day: u64,
    day_spent: u128,
    month: u64,
    month_spent: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    pub sanctions_clear: bool,
    pub kyc_clear: bool,
    pub limits_clear: bool,
    pub purpose_clear: bool,
    pub timelock_clear: bool,
}

#[derive(Clone, Debug)]
pub struct Disclosure {
    pub commitment: Hash,
    pub asset: String,
    pub purpose: String,
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevealedTransfer {
    pub from: Address,
    pub to: Address,
    pub asset: String,
    pub amount: u128,
    pub purpose: String,
}

#[derive(Clone, Debug)]
pub struct ComplianceEngine {
    pub kyc: BTreeSet<Address>,
    pub sanctions: BTreeSet<Address>,
    policies: BTreeMap<Address, PartyPolicy>,
    default_policy: PartyPolicy,
    usage: BTreeMap<Address, Usage>,
}

impl ComplianceEngine {
    pub fn new(default_policy: PartyPolicy) -> Self {
        Self {
            kyc: BTreeSet::new(),
            sanctions: BTreeSet::new(),
            policies: BTreeMap::new(),
            default_policy,
            usage: BTreeMap::new(),
        }
    }

    pub fn set_policy(&mut self, address: Address, policy: PartyPolicy) {
        self.policies.insert(address, policy);
    }

    pub fn check(&self, tx: &TxBody) -> Result<Attestation, PrivacyError> {
        if self.sanctions.contains(&tx.from) || self.sanctions.contains(&tx.to) {
            return Err(PrivacyError::Sanctioned);
        }
        if !self.kyc.contains(&tx.from) || !self.kyc.contains(&tx.to) {
            return Err(PrivacyError::NotKyc);
        }
        let policy = self.policies.get(&tx.from).unwrap_or(&self.default_policy);
        if let Some(not_before) = policy.not_before_ms {
            if tx.timestamp_ms < not_before {
                return Err(PrivacyError::Timelock);
            }
        }
        if let Some(purposes) = &policy.purposes {
            if !purposes.contains(&tx.purpose) {
                return Err(PrivacyError::Purpose);
            }
        }
        let day = tx.timestamp_ms / DAY_MS;
        let month = tx.timestamp_ms / MONTH_MS;
        let usage = self.usage.get(&tx.from).cloned().unwrap_or_default();
        let day_spent = if usage.day == day { usage.day_spent } else { 0 };
        let month_spent = if usage.month == month { usage.month_spent } else { 0 };
        if day_spent.saturating_add(tx.amount) > policy.daily_limit {
            return Err(PrivacyError::DailyLimit);
        }
        if month_spent.saturating_add(tx.amount) > policy.monthly_limit {
            return Err(PrivacyError::MonthlyLimit);
        }
        Ok(Attestation {
            sanctions_clear: true,
            kyc_clear: true,
            limits_clear: true,
            purpose_clear: true,
            timelock_clear: true,
        })
    }

    pub fn daily_limit_of(&self, address: &Address) -> u128 {
        self.policies
            .get(address)
            .unwrap_or(&self.default_policy)
            .daily_limit
    }

    pub fn purpose_allowed(&self, address: &Address, purpose: &str) -> bool {
        match self.policies.get(address).unwrap_or(&self.default_policy).purposes.as_ref() {
            Some(purposes) => purposes.contains(purpose),
            None => true,
        }
    }

    pub fn daily_spent(&self, address: &Address, now_ms: u64) -> u128 {
        let day = now_ms / DAY_MS;
        self.usage
            .get(address)
            .filter(|usage| usage.day == day)
            .map(|usage| usage.day_spent)
            .unwrap_or(0)
    }

    pub fn commit(&mut self, tx: &TxBody) {
        let day = tx.timestamp_ms / DAY_MS;
        let month = tx.timestamp_ms / MONTH_MS;
        let usage = self.usage.entry(tx.from).or_default();
        if usage.day != day {
            usage.day = day;
            usage.day_spent = 0;
        }
        if usage.month != month {
            usage.month = month;
            usage.month_spent = 0;
        }
        usage.day_spent = usage.day_spent.saturating_add(tx.amount);
        usage.month_spent = usage.month_spent.saturating_add(tx.amount);
    }
}

pub struct PrivacyVault {
    key: [u8; 32],
    counter: u64,
}

impl PrivacyVault {
    pub fn from_regulator_secret(secret: &[u8]) -> Self {
        Self {
            key: sha256(secret),
            counter: 1,
        }
    }

    pub fn seal(&mut self, tx: &TxBody) -> Result<Disclosure, PrivacyError> {
        let plaintext = canonical_private(tx);
        let blinding = sha256(&{
            let mut seed = self.counter.to_be_bytes().to_vec();
            seed.extend_from_slice(&plaintext);
            seed
        });
        let commitment = commit(&plaintext, &blinding);
        let mut body = plaintext;
        body.extend_from_slice(&blinding);
        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&self.counter.to_be_bytes());
        self.counter = self.counter.saturating_add(1);
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| PrivacyError::Crypto)?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), body.as_ref())
            .map_err(|_| PrivacyError::Crypto)?;
        Ok(Disclosure {
            commitment,
            asset: tx.asset.0.clone(),
            purpose: tx.purpose.clone(),
            ciphertext,
            nonce,
        })
    }

    pub fn open(&self, disclosure: &Disclosure) -> Result<RevealedTransfer, PrivacyError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| PrivacyError::Crypto)?;
        let body = cipher
            .decrypt(Nonce::from_slice(&disclosure.nonce), disclosure.ciphertext.as_ref())
            .map_err(|_| PrivacyError::Crypto)?;
        if body.len() < 32 {
            return Err(PrivacyError::Crypto);
        }
        let (plaintext, blinding) = body.split_at(body.len() - 32);
        let mut blind = [0u8; 32];
        blind.copy_from_slice(blinding);
        if commit(plaintext, &blind) != disclosure.commitment {
            return Err(PrivacyError::Crypto);
        }
        decode_private(plaintext)
    }
}

fn commit(plaintext: &[u8], blinding: &[u8; 32]) -> Hash {
    let mut buf = b"SCBPS-COMMIT-v1".to_vec();
    buf.extend_from_slice(plaintext);
    buf.extend_from_slice(blinding);
    Hash::sha256(&buf)
}

fn canonical_private(tx: &TxBody) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&tx.from.0);
    out.extend_from_slice(&tx.to.0);
    out.extend_from_slice(&tx.amount.to_be_bytes());
    let asset = tx.asset.0.as_bytes();
    out.extend_from_slice(&(asset.len() as u16).to_be_bytes());
    out.extend_from_slice(asset);
    let purpose = tx.purpose.as_bytes();
    out.extend_from_slice(&(purpose.len() as u16).to_be_bytes());
    out.extend_from_slice(purpose);
    out
}

fn decode_private(plaintext: &[u8]) -> Result<RevealedTransfer, PrivacyError> {
    if plaintext.len() < 32 + 32 + 16 + 2 {
        return Err(PrivacyError::Crypto);
    }
    let mut from = [0u8; 32];
    let mut to = [0u8; 32];
    from.copy_from_slice(&plaintext[0..32]);
    to.copy_from_slice(&plaintext[32..64]);
    let mut amount_bytes = [0u8; 16];
    amount_bytes.copy_from_slice(&plaintext[64..80]);
    let amount = u128::from_be_bytes(amount_bytes);
    let asset_len = u16::from_be_bytes([plaintext[80], plaintext[81]]) as usize;
    let asset_start = 82;
    let asset_end = asset_start + asset_len;
    if plaintext.len() < asset_end + 2 {
        return Err(PrivacyError::Crypto);
    }
    let asset = std::str::from_utf8(&plaintext[asset_start..asset_end]).map_err(|_| PrivacyError::Crypto)?;
    let purpose_len = u16::from_be_bytes([plaintext[asset_end], plaintext[asset_end + 1]]) as usize;
    let purpose_start = asset_end + 2;
    let purpose_end = purpose_start + purpose_len;
    if plaintext.len() != purpose_end {
        return Err(PrivacyError::Crypto);
    }
    let purpose = std::str::from_utf8(&plaintext[purpose_start..purpose_end]).map_err(|_| PrivacyError::Crypto)?;
    Ok(RevealedTransfer {
        from: Address(from),
        to: Address(to),
        asset: asset.to_string(),
        amount,
        purpose: purpose.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use swift_rs_blockchain::{AssetId, KeyPair};

    fn tx(purpose: &str, amount: u128, timestamp_ms: u64) -> (TxBody, KeyPair, KeyPair) {
        let from = KeyPair::from_seed(b"from");
        let to = KeyPair::from_seed(b"to");
        let body = TxBody {
            from: from.address,
            to: to.address,
            asset: AssetId::new("cbdc:brl").unwrap(),
            amount,
            fee: 1,
            nonce: 0,
            purpose: purpose.into(),
            timestamp_ms,
        };
        (body, from, to)
    }

    #[test]
    fn sanctions_kyc_limits_purpose_and_timelock() {
        let (body, from, to) = tx("trade", 30, 10 * DAY_MS);
        let mut engine = ComplianceEngine::new(PartyPolicy {
            daily_limit: 100,
            monthly_limit: 150,
            not_before_ms: None,
            purposes: None,
        });
        assert_eq!(engine.check(&body), Err(PrivacyError::NotKyc));
        engine.kyc.insert(from.address);
        engine.kyc.insert(to.address);
        engine.check(&body).unwrap();
        engine.sanctions.insert(to.address);
        assert_eq!(engine.check(&body), Err(PrivacyError::Sanctioned));
        engine.sanctions.clear();
        engine.commit(&body);
        let (again, _, _) = tx("trade", 80, 10 * DAY_MS);
        let mut again = again;
        again.from = from.address;
        again.to = to.address;
        assert_eq!(engine.check(&again), Err(PrivacyError::DailyLimit));
        engine.set_policy(
            from.address,
            PartyPolicy {
                daily_limit: 1_000,
                monthly_limit: 1_000,
                not_before_ms: Some(20 * DAY_MS),
                purposes: Some(BTreeSet::from(["oil-settlement".into()])),
            },
        );
        assert_eq!(engine.check(&again), Err(PrivacyError::Timelock));
        again.timestamp_ms = 21 * DAY_MS;
        assert_eq!(engine.check(&again), Err(PrivacyError::Purpose));
        again.purpose = "oil-settlement".into();
        assert!(engine.check(&again).unwrap().purpose_clear);
    }

    #[test]
    fn regulator_can_open_what_the_public_record_hides() {
        let (body, _, _) = tx("trade", 42, 1);
        let mut vault = PrivacyVault::from_regulator_secret(b"regulator");
        let disclosure = vault.seal(&body).unwrap();
        assert!(!disclosure.ciphertext.is_empty());
        let public_blob = format!("{}:{}", disclosure.asset, disclosure.purpose);
        assert!(!public_blob.contains(&body.from.to_hex()));
        let revealed = vault.open(&disclosure).unwrap();
        assert_eq!(revealed.amount, 42);
        assert_eq!(revealed.from, body.from);
        let mut forged = disclosure.clone();
        forged.ciphertext[0] ^= 0x01;
        assert_eq!(vault.open(&forged), Err(PrivacyError::Crypto));
    }
}
