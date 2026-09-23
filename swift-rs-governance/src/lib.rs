//! Stake-weighted governance.
//!
//! A proposal passes only when yes-votes cover 67% of total stake. Treasury
//! spends need a distinct multi-signature threshold. One validator cannot
//! change the rule set or move the treasury alone.

use std::collections::BTreeSet;


use ed25519_dalek::VerifyingKey;
use swift_rs_blockchain::keys::{self, KeyPair};
use swift_rs_blockchain::wasm::quorum_met;
use swift_rs_blockchain::{Address, Hash};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GovError {
    #[error("unknown proposal")]
    UnknownProposal,
    #[error("voting is closed")]
    Closed,
    #[error("voter already voted")]
    AlreadyVoted,
    #[error("bad signature")]
    BadSignature,
    #[error("zero stake")]
    ZeroStake,
    #[error("treasury threshold not met")]
    Threshold,
    #[error("dispute is in the wrong stage")]
    BadStage,
    #[error("wasm: {0}")]
    Wasm(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    Open,
    Passed,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisputeStage {
    Open,
    Arbitrated,
    Appealed,
    Closed,
}

#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub payload: String,
    pub votes_for: u128,
    pub votes_against: u128,
    pub voters: BTreeSet<Address>,
    pub status: ProposalStatus,
    pub deadline_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Dispute {
    pub id: u64,
    pub claimant: Address,
    pub respondent: Address,
    pub evidence: Hash,
    pub stage: DisputeStage,
    pub award_bps: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct TreasurySpend {
    pub digest: Hash,
    pub to: Address,
    pub amount: u128,
    pub asset: String,
}

#[derive(Clone, Debug, Default)]
pub struct Governance {
    pub proposals: Vec<Proposal>,
    pub disputes: Vec<Dispute>,
    pub treasury_signers: Vec<Address>,
    pub treasury_threshold: usize,
    pub executed_spends: BTreeSet<Hash>,
    next_proposal: u64,
    next_dispute: u64,
}

impl Governance {
    pub fn with_treasury(signers: Vec<Address>, threshold: usize) -> Self {
        Self {
            treasury_signers: signers,
            treasury_threshold: threshold,
            ..Self::default()
        }
    }

    pub fn propose(
        &mut self,
        proposer: Address,
        title: impl Into<String>,
        payload: impl Into<String>,
        now_ms: u64,
    ) -> u64 {
        let id = self.next_proposal;
        self.next_proposal += 1;
        self.proposals.push(Proposal {
            id,
            proposer,
            title: title.into(),
            payload: payload.into(),
            votes_for: 0,
            votes_against: 0,
            voters: BTreeSet::new(),
            status: ProposalStatus::Open,
            deadline_ms: now_ms.saturating_add(86_400_000),
        });
        id
    }

    pub fn vote_message(proposal_id: u64, support: bool) -> Vec<u8> {
        let mut out = b"SCBPS-GOV-v1".to_vec();
        out.extend_from_slice(&proposal_id.to_be_bytes());
        out.push(u8::from(support));
        out
    }

    pub fn cast_vote(
        &mut self,
        proposal_id: u64,
        voter: &KeyPair,
        stake: u128,
        support: bool,
        now_ms: u64,
    ) -> Result<(), GovError> {
        if stake == 0 {
            return Err(GovError::ZeroStake);
        }
        let signature = voter.sign(&Self::vote_message(proposal_id, support));
        self.cast_vote_signed(proposal_id, voter.address, &voter.verifying, stake, support, &signature, now_ms)
    }

    pub fn cast_vote_signed(
        &mut self,
        proposal_id: u64,
        voter: Address,
        verifying: &VerifyingKey,
        stake: u128,
        support: bool,
        signature: &[u8; 64],
        now_ms: u64,
    ) -> Result<(), GovError> {
        let proposal = self
            .proposals
            .iter_mut()
            .find(|proposal| proposal.id == proposal_id)
            .ok_or(GovError::UnknownProposal)?;
        if proposal.status != ProposalStatus::Open || now_ms > proposal.deadline_ms {
            return Err(GovError::Closed);
        }
        if !proposal.voters.insert(voter) {
            return Err(GovError::AlreadyVoted);
        }
        if !keys::verify(verifying, &Self::vote_message(proposal_id, support), signature) {
            proposal.voters.remove(&voter);
            return Err(GovError::BadSignature);
        }
        if support {
            proposal.votes_for = proposal.votes_for.saturating_add(stake);
        } else {
            proposal.votes_against = proposal.votes_against.saturating_add(stake);
        }
        Ok(())
    }

    pub fn tally(&mut self, proposal_id: u64, total_stake: u128, now_ms: u64) -> Result<ProposalStatus, GovError> {
        let proposal = self
            .proposals
            .iter_mut()
            .find(|proposal| proposal.id == proposal_id)
            .ok_or(GovError::UnknownProposal)?;
        if now_ms < proposal.deadline_ms {
            return Err(GovError::Closed);
        }
        let passed = quorum_met(proposal.votes_for, total_stake).map_err(|e| GovError::Wasm(e.to_string()))?;
        proposal.status = if passed {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        };
        Ok(proposal.status.clone())
    }

    pub fn spend_message(spend: &TreasurySpend) -> Vec<u8> {
        let mut out = b"SCBPS-TREASURY-v1".to_vec();
        out.extend_from_slice(&spend.to.0);
        out.extend_from_slice(&spend.amount.to_be_bytes());
        out.extend_from_slice(spend.asset.as_bytes());
        out
    }

    pub fn authorize_spend(
        &mut self,
        spend: TreasurySpend,
        signatures: &[(Address, [u8; 64])],
        keys: &[(Address, VerifyingKey)],
    ) -> Result<(), GovError> {
        if self.executed_spends.contains(&spend.digest) {
            return Err(GovError::Threshold);
        }
        let message = Self::spend_message(&spend);
        let mut valid = BTreeSet::new();
        for (signer, signature) in signatures {
            if !self.treasury_signers.contains(signer) {
                continue;
            }
            if let Some((_, key)) = keys.iter().find(|(addr, _)| addr == signer) {
                if keys::verify(key, &message, signature) {
                    valid.insert(*signer);
                }
            }
        }
        if valid.len() < self.treasury_threshold {
            return Err(GovError::Threshold);
        }
        self.executed_spends.insert(spend.digest);
        Ok(())
    }

    pub fn open_dispute(&mut self, claimant: Address, respondent: Address, evidence: Hash) -> u64 {
        let id = self.next_dispute;
        self.next_dispute += 1;
        self.disputes.push(Dispute {
            id,
            claimant,
            respondent,
            evidence,
            stage: DisputeStage::Open,
            award_bps: None,
        });
        id
    }

    pub fn arbitrate(&mut self, id: u64, award_bps: u32) -> Result<(), GovError> {
        let dispute = self.disputes.iter_mut().find(|d| d.id == id).ok_or(GovError::UnknownProposal)?;
        if dispute.stage != DisputeStage::Open || award_bps > 10_000 {
            return Err(GovError::BadStage);
        }
        dispute.stage = DisputeStage::Arbitrated;
        dispute.award_bps = Some(award_bps);
        Ok(())
    }

    pub fn appeal(&mut self, id: u64) -> Result<(), GovError> {
        let dispute = self.disputes.iter_mut().find(|d| d.id == id).ok_or(GovError::UnknownProposal)?;
        if dispute.stage != DisputeStage::Arbitrated {
            return Err(GovError::BadStage);
        }
        dispute.stage = DisputeStage::Appealed;
        Ok(())
    }

    pub fn consortium_award(&mut self, id: u64, award_bps: u32) -> Result<(), GovError> {
        let dispute = self.disputes.iter_mut().find(|d| d.id == id).ok_or(GovError::UnknownProposal)?;
        if dispute.stage != DisputeStage::Appealed || award_bps > 10_000 {
            return Err(GovError::BadStage);
        }
        dispute.stage = DisputeStage::Closed;
        dispute.award_bps = Some(award_bps);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_of_four_passes_and_two_of_four_fails() {
        let voters: Vec<KeyPair> = (0..4u8).map(|i| KeyPair::from_seed(&[i])).collect();
        let mut gov = Governance::default();
        let id = gov.propose(voters[0].address, "base fee", "fee:base:2", 0);
        for voter in voters.iter().take(3) {
            gov.cast_vote(id, voter, 25, true, 10).unwrap();
        }
        assert_eq!(gov.tally(id, 100, 86_400_000).unwrap(), ProposalStatus::Passed);

        let id = gov.propose(voters[0].address, "no", "text:no", 0);
        for voter in voters.iter().take(2) {
            gov.cast_vote(id, voter, 25, true, 10).unwrap();
        }
        assert_eq!(gov.tally(id, 100, 86_400_000).unwrap(), ProposalStatus::Rejected);
    }

    #[test]
    fn treasury_and_appeal() {
        let signers: Vec<KeyPair> = (0..3u8).map(|i| KeyPair::from_seed(&[10 + i])).collect();
        let mut gov = Governance {
            treasury_signers: signers.iter().map(|k| k.address).collect(),
            treasury_threshold: 2,
            ..Governance::default()
        };
        let spend = TreasurySpend {
            digest: Hash::sha256(b"spend-1"),
            to: signers[0].address,
            amount: 10,
            asset: "cbdc:brl".into(),
        };
        let message = Governance::spend_message(&spend);
        let sigs = vec![
            (signers[0].address, signers[0].sign(&message)),
            (signers[1].address, signers[1].sign(&message)),
        ];
        let keys = signers
            .iter()
            .map(|k| (k.address, k.verifying))
            .collect::<Vec<_>>();
        gov.authorize_spend(spend, &sigs, &keys).unwrap();

        let id = gov.open_dispute(signers[0].address, signers[1].address, Hash::sha256(b"evidence"));
        gov.arbitrate(id, 2_500).unwrap();
        gov.appeal(id).unwrap();
        gov.consortium_award(id, 5_000).unwrap();
        assert_eq!(gov.disputes[0].stage, DisputeStage::Closed);
        assert_eq!(gov.disputes[0].award_bps, Some(5_000));
    }
}
