//! Message translation for external payment rails and a threshold FX oracle.
//!
//! SWIFT, CIPS, SPFS, and TIPS adapters all emit ISO 20022 credit transfers.
//! Importing a message does not settle it. The caller still has to pass
//! compliance screening and BFT finality.

use ed25519_dalek::VerifyingKey;

use swift_rs_blockchain::keys::{self, KeyPair};
use swift_rs_blockchain::{Address, AssetId, TxBody};
use swift_rs_iso20022::models::{
    AccountIdentification, Amount, CreditTransfer, CreditTransferTransactionInformation, Creditor,
    CreditorAccount, GroupHeader, PaymentIdentification,
};
use swift_rs_iso20022::Iso20022Serializer;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BridgeError {
    #[error("unsupported asset")]
    UnsupportedAsset,
    #[error("bad amount")]
    BadAmount,
    #[error("oracle rejected the update")]
    OracleRejected,
    #[error("serialization failed: {0}")]
    Serialization(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rail {
    Swift,
    Cips,
    Spfs,
    Tips,
}

impl Rail {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Swift => "swift",
            Self::Cips => "cips",
            Self::Spfs => "spfs",
            Self::Tips => "tips",
        }
    }
}

pub fn currency_code(asset: &AssetId) -> Result<&'static str, BridgeError> {
    match asset.as_str() {
        "cbdc:brl" => Ok("BRL"),
        "cbdc:cny" => Ok("CNY"),
        "cbdc:inr" => Ok("INR"),
        "cbdc:aed" => Ok("AED"),
        "stable:usd" | "fiat:usd" => Ok("USD"),
        _ => Err(BridgeError::UnsupportedAsset),
    }
}

pub fn format_minor(minor: u128) -> String {
    format!("{}.{:02}", minor / 100, minor % 100)
}

pub fn parse_minor(value: &str) -> Result<u128, BridgeError> {
    let (whole, frac) = value.split_once('.').unwrap_or((value, "00"));
    if frac.len() != 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(BridgeError::BadAmount);
    }
    let whole: u128 = whole.parse().map_err(|_| BridgeError::BadAmount)?;
    let frac: u128 = frac.parse().map_err(|_| BridgeError::BadAmount)?;
    whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(frac))
        .ok_or(BridgeError::BadAmount)
}

pub fn account_ref(address: &Address) -> String {
    format!("ACCT{}", &address.to_hex()[..16])
}

pub fn to_credit_transfer(rail: Rail, tx: &TxBody, msg_id: &str) -> Result<CreditTransfer, BridgeError> {
    Ok(CreditTransfer {
        grp_hdr: GroupHeader {
            msg_id: format!("{}:{}", rail.as_str(), msg_id),
            cre_dt_tm: tx.timestamp_ms.to_string(),
            nb_of_txs: "1".into(),
            ctrl_sum: Some(format_minor(tx.amount)),
        },
        cdt_trf_tx_inf: CreditTransferTransactionInformation {
            pmt_id: PaymentIdentification {
                instr_id: Some(tx.purpose.clone()),
                end_to_end_id: msg_id.to_string(),
            },
            amt: Amount {
                currency: currency_code(&tx.asset)?.into(),
                value: format_minor(tx.amount),
            },
            cdtr: Creditor {
                nm: account_ref(&tx.to),
            },
            cdtr_acct: CreditorAccount {
                id: AccountIdentification {
                    iban: account_ref(&tx.to),
                },
            },
        },
    })
}

pub fn to_xml(rail: Rail, tx: &TxBody, msg_id: &str) -> Result<String, BridgeError> {
    let message = to_credit_transfer(rail, tx, msg_id)?;
    let bytes = Iso20022Serializer::serialize_mx(&message)
        .map_err(|e| BridgeError::Serialization(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| BridgeError::Serialization(e.to_string()))
}

pub fn draft_from_transfer(
    message: &CreditTransfer,
    asset: AssetId,
    from: Address,
    to: Address,
    nonce: u64,
    fee: u128,
) -> Result<TxBody, BridgeError> {
    let timestamp_ms = message
        .grp_hdr
        .cre_dt_tm
        .parse()
        .map_err(|_| BridgeError::BadAmount)?;
    Ok(TxBody {
        from,
        to,
        asset,
        amount: parse_minor(&message.cdt_trf_tx_inf.amt.value)?,
        fee,
        nonce,
        purpose: message
            .cdt_trf_tx_inf
            .pmt_id
            .instr_id
            .clone()
            .unwrap_or_else(|| "trade".into()),
        timestamp_ms,
    })
}

#[derive(Clone, Debug)]
pub struct PriceOracle {
    pub pair: String,
    pub value_e8: u128,
    pub epoch: u64,
    pub threshold: usize,
    pub signers: Vec<VerifyingKey>,
}

impl PriceOracle {
    pub fn message(pair: &str, value_e8: u128, epoch: u64) -> Vec<u8> {
        let mut out = b"SCBPS-ORACLE-v1".to_vec();
        out.extend_from_slice(pair.as_bytes());
        out.extend_from_slice(&value_e8.to_be_bytes());
        out.extend_from_slice(&epoch.to_be_bytes());
        out
    }

    pub fn update(
        &mut self,
        value_e8: u128,
        epoch: u64,
        signatures: &[(usize, [u8; 64])],
    ) -> Result<(), BridgeError> {
        if epoch <= self.epoch || value_e8 == 0 {
            return Err(BridgeError::OracleRejected);
        }
        let message = Self::message(&self.pair, value_e8, epoch);
        let mut seen = Vec::new();
        let mut valid = 0usize;
        for (index, signature) in signatures {
            if !seen.contains(index) {
                if let Some(key) = self.signers.get(*index) {
                    if keys::verify(key, &message, signature) {
                        seen.push(*index);
                        valid += 1;
                    }
                }
            }
        }
        if valid < self.threshold {
            return Err(BridgeError::OracleRejected);
        }
        self.value_e8 = value_e8;
        self.epoch = epoch;
        Ok(())
    }
}

pub fn sign_oracle(key: &KeyPair, pair: &str, value_e8: u128, epoch: u64) -> [u8; 64] {
    key.sign(&PriceOracle::message(pair, value_e8, epoch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_roundtrip_keeps_amount_and_purpose() {
        let from = KeyPair::from_seed(b"debtor");
        let to = KeyPair::from_seed(b"creditor");
        let body = TxBody {
            from: from.address,
            to: to.address,
            asset: AssetId::new("cbdc:brl").unwrap(),
            amount: 12_550,
            fee: 1,
            nonce: 3,
            purpose: "trade".into(),
            timestamp_ms: 1_700_000_000_000,
        };
        let message = to_credit_transfer(Rail::Cips, &body, "e2e-1").unwrap();
        assert_eq!(message.grp_hdr.msg_id, "cips:e2e-1");
        assert_eq!(message.cdt_trf_tx_inf.amt.currency, "BRL");
        let draft = draft_from_transfer(&message, body.asset.clone(), from.address, to.address, 3, 1).unwrap();
        assert_eq!(draft.amount, 12_550);
        assert_eq!(draft.purpose, "trade");
        let xml = to_xml(Rail::Swift, &body, "e2e-1").unwrap();
        assert!(xml.contains("BRL"));
    }

    #[test]
    fn oracle_requires_the_threshold() {
        let keys = vec![
            KeyPair::from_seed(b"o1"),
            KeyPair::from_seed(b"o2"),
            KeyPair::from_seed(b"o3"),
        ];
        let mut oracle = PriceOracle {
            pair: "brl/cny".into(),
            value_e8: 100_000_000,
            epoch: 1,
            threshold: 2,
            signers: keys.iter().map(|k| k.verifying).collect(),
        };
        let one = vec![(0, sign_oracle(&keys[0], "brl/cny", 120_000_000, 2))];
        assert_eq!(oracle.update(120_000_000, 2, &one), Err(BridgeError::OracleRejected));
        let two = vec![
            (0, sign_oracle(&keys[0], "brl/cny", 120_000_000, 2)),
            (1, sign_oracle(&keys[1], "brl/cny", 120_000_000, 2)),
        ];
        oracle.update(120_000_000, 2, &two).unwrap();
        assert_eq!(oracle.value_e8, 120_000_000);
    }
}
