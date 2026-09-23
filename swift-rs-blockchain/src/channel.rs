use crate::error::ChainError;
use crate::keys::KeyPair;
use crate::tx::{SignedTransaction, TxBody};
use crate::types::{Address, AssetId, Hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: Hash,
    pub party_a: Address,
    pub party_b: Address,
    pub asset: AssetId,
    pub bal_a: u128,
    pub bal_b: u128,
    pub nonce: u64,
    pub open: bool,
}

#[derive(Clone, Debug)]
pub struct SignedChannelState {
    pub channel_id: Hash,
    pub bal_a: u128,
    pub bal_b: u128,
    pub nonce: u64,
    pub sig_a: [u8; 64],
    pub sig_b: [u8; 64],
}

fn state_message(channel_id: &Hash, bal_a: u128, bal_b: u128, nonce: u64) -> Vec<u8> {
    let mut out = b"SCBPS-CHANNEL-v1".to_vec();
    out.extend_from_slice(&channel_id.0);
    out.extend_from_slice(&bal_a.to_be_bytes());
    out.extend_from_slice(&bal_b.to_be_bytes());
    out.extend_from_slice(&nonce.to_be_bytes());
    out
}

impl Channel {
    pub fn sign_state(&self, party_a: &KeyPair, party_b: &KeyPair) -> Result<SignedChannelState, ChainError> {
        if party_a.address != self.party_a || party_b.address != self.party_b {
            return Err(ChainError::Channel("signer is not a channel party".into()));
        }
        let message = state_message(&self.id, self.bal_a, self.bal_b, self.nonce);
        Ok(SignedChannelState {
            channel_id: self.id,
            bal_a: self.bal_a,
            bal_b: self.bal_b,
            nonce: self.nonce,
            sig_a: party_a.sign(&message),
            sig_b: party_b.sign(&message),
        })
    }
}

pub fn verify_state(state: &SignedChannelState, party_a: &KeyPair, party_b: &KeyPair) -> Result<(), ChainError> {
    let message = state_message(&state.channel_id, state.bal_a, state.bal_b, state.nonce);
    if crate::keys::verify(&party_a.verifying, &message, &state.sig_a)
        && crate::keys::verify(&party_b.verifying, &message, &state.sig_b)
    {
        Ok(())
    } else {
        Err(ChainError::BadSignature)
    }
}

/// Higher nonce wins. The caller settles that state on the ledger.
pub fn resolve_dispute(
    left: &SignedChannelState,
    right: &SignedChannelState,
    party_a: &KeyPair,
    party_b: &KeyPair,
) -> Result<SignedChannelState, ChainError> {
    verify_state(left, party_a, party_b)?;
    verify_state(right, party_a, party_b)?;
    if left.channel_id != right.channel_id {
        return Err(ChainError::Channel("channel id mismatch".into()));
    }
    if right.nonce > left.nonce {
        Ok(right.clone())
    } else {
        Ok(left.clone())
    }
}

pub fn open_funding_txs(
    vault: &Address,
    party_a: &KeyPair,
    party_b: &KeyPair,
    asset: &AssetId,
    amount_a: u128,
    amount_b: u128,
    nonce_a: u64,
    nonce_b: u64,
    fee: u128,
    timestamp_ms: u64,
) -> Result<(Channel, Vec<SignedTransaction>), ChainError> {
    let mut id_bytes = b"channel".to_vec();
    id_bytes.extend_from_slice(&party_a.address.0);
    id_bytes.extend_from_slice(&party_b.address.0);
    id_bytes.extend_from_slice(asset.0.as_bytes());
    id_bytes.extend_from_slice(&nonce_a.to_be_bytes());
    let channel = Channel {
        id: Hash::sha256(&id_bytes),
        party_a: party_a.address,
        party_b: party_b.address,
        asset: asset.clone(),
        bal_a: amount_a,
        bal_b: amount_b,
        nonce: 0,
        open: true,
    };
    let mut txs = Vec::new();
    if amount_a > 0 {
        txs.push(SignedTransaction::sign(
            TxBody {
                from: party_a.address,
                to: *vault,
                asset: asset.clone(),
                amount: amount_a,
                fee,
                nonce: nonce_a,
                purpose: "channel-open".into(),
                timestamp_ms,
            },
            party_a,
        )?);
    }
    if amount_b > 0 {
        txs.push(SignedTransaction::sign(
            TxBody {
                from: party_b.address,
                to: *vault,
                asset: asset.clone(),
                amount: amount_b,
                fee,
                nonce: nonce_b,
                purpose: "channel-open".into(),
                timestamp_ms,
            },
            party_b,
        )?);
    }
    if txs.is_empty() {
        return Err(ChainError::Channel("channel needs funds".into()));
    }
    Ok((channel, txs))
}

pub fn close_txs(
    channel: &Channel,
    vault: &KeyPair,
    vault_nonce: u64,
    fee: u128,
    timestamp_ms: u64,
) -> Result<Vec<SignedTransaction>, ChainError> {
    if !channel.open {
        return Err(ChainError::Channel("channel is closed".into()));
    }
    let mut txs = Vec::new();
    let mut nonce = vault_nonce;
    if channel.bal_a > 0 {
        txs.push(SignedTransaction::sign(
            TxBody {
                from: vault.address,
                to: channel.party_a,
                asset: channel.asset.clone(),
                amount: channel.bal_a,
                fee,
                nonce,
                purpose: "channel-close".into(),
                timestamp_ms,
            },
            vault,
        )?);
        nonce = nonce.saturating_add(1);
    }
    if channel.bal_b > 0 {
        txs.push(SignedTransaction::sign(
            TxBody {
                from: vault.address,
                to: channel.party_b,
                asset: channel.asset.clone(),
                amount: channel.bal_b,
                fee,
                nonce,
                purpose: "channel-close".into(),
                timestamp_ms,
            },
            vault,
        )?);
    }
    Ok(txs)
}
