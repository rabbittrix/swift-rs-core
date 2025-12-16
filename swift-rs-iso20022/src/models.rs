//! ISO 20022 message models

use serde::{Deserialize, Serialize};

/// ISO 20022 Document wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Document")]
pub struct Document {
    #[serde(rename = "xmlns")]
    pub xmlns: String,
    #[serde(rename = "xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "xsi:schemaLocation")]
    pub schema_location: String,
}

/// Credit Transfer (pacs.008) message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransfer {
    pub grp_hdr: GroupHeader,
    pub cdt_trf_tx_inf: CreditTransferTransactionInformation,
}

/// Group Header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupHeader {
    pub msg_id: String,
    pub cre_dt_tm: String,
    pub nb_of_txs: String,
    pub ctrl_sum: Option<String>,
}

/// Credit Transfer Transaction Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferTransactionInformation {
    pub pmt_id: PaymentIdentification,
    pub amt: Amount,
    pub cdtr: Creditor,
    pub cdtr_acct: CreditorAccount,
}

/// Payment Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIdentification {
    pub instr_id: Option<String>,
    pub end_to_end_id: String,
}

/// Amount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    #[serde(rename = "Ccy")]
    pub currency: String,
    #[serde(rename = "$value")]
    pub value: String,
}

/// Creditor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creditor {
    pub nm: String,
}

/// Creditor Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditorAccount {
    pub id: AccountIdentification,
}

/// Account Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountIdentification {
    pub iban: String,
}
