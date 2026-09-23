//! Application service that screens, prices, and finalizes payments.
//!
//! Every finalized transaction has already passed KYC and sanctions screening.
//! There is no switch that skips those checks.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use swift_rs_ai::fraud::RiskLevel;
use swift_rs_ai::{score_transfer, TransferFeatures};
use swift_rs_blockchain::{
    Address, AssetId, Consensus, GenesisAllocation, KeyPair, Node, SignedTransaction, TxBody,
    Validator,
};
use swift_rs_bridge::{to_xml, PriceOracle, Rail};
use swift_rs_cbdc::{lock_and_release, redeem, CurrencyReserve, LockReceipt};
use swift_rs_economics::{next_base_fee, slash_amount, SlashReason};
use swift_rs_governance::{Governance, ProposalStatus};
use swift_rs_privacy::{
    ComplianceEngine, Disclosure, PartyPolicy, PrivacyError, PrivacyVault, RevealedTransfer,
};
use swift_rs_tokenization::{SecurityToken, Stablecoin, TokenEngine};
use thiserror::Error;

const DAY_MS: u64 = 86_400_000;
const BANK_LIMIT: u128 = 20_000_000;
const ISSUANCE: u128 = 1_000_000_000;
const BANK_FLOAT: u128 = 50_000_000;
const ESCROW_FLOAT: u128 = 200_000_000;

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("{0}")]
    Message(String),
}

impl SystemError {
    fn msg(value: impl ToString) -> Self {
        Self::Message(value.to_string())
    }
}

struct Party {
    key: KeyPair,
    unlimited: bool,
}

struct StoredDisclosure {
    height: u64,
    disclosure: Disclosure,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransferReceipt {
    pub tx_id: String,
    pub height: u64,
    pub asset: String,
    pub amount: u128,
    pub purpose: String,
    pub risk_score: f64,
    pub finalized: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SwapReceipt {
    pub receipt_id: String,
    pub source_amount: u128,
    pub dest_amount: u128,
    pub rate_e8: u128,
    pub height: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReserveView {
    pub asset: String,
    pub backing: u128,
    pub circulating: u128,
    pub escrow: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChainStatus {
    pub system_version: &'static str,
    pub height: u64,
    pub base_fee: u128,
    pub finalized_transactions: usize,
    pub cbdc_count: usize,
    pub oracle_brl_cny_e8: u128,
    pub reserves: Vec<ReserveView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicRecord {
    pub height: u64,
    pub tx_id: String,
    pub asset: String,
    pub purpose: String,
    pub commitment: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProposalView {
    pub id: u64,
    pub title: String,
    pub payload: String,
    pub votes_for: u128,
    pub votes_against: u128,
    pub status: String,
}

pub struct PaymentSystem {
    node: Node,
    parties: BTreeMap<String, Party>,
    compliance: Arc<Mutex<ComplianceEngine>>,
    vault: PrivacyVault,
    reserves: BTreeMap<String, CurrencyReserve>,
    receipts: Vec<LockReceipt>,
    tokens: TokenEngine,
    oracle: PriceOracle,
    governance: Governance,
    disclosures: BTreeMap<String, StoredDisclosure>,
    target_tx_count: u64,
    now_ms: u64,
    frozen_clock: bool,
    seen_pairs: BTreeSet<(Address, Address)>,
    recent_amounts: BTreeMap<Address, Vec<u128>>,
}

impl PaymentSystem {
    pub fn bootstrap() -> Result<Self, SystemError> {
        let now_ms = wall_clock_ms();
        let specs = [
            ("br-cb", "scbps-br-cb", true, "cbdc:brl"),
            ("cn-cb", "scbps-cn-cb", true, "cbdc:cny"),
            ("in-cb", "scbps-in-cb", true, "cbdc:inr"),
            ("ae-cb", "scbps-ae-cb", true, "cbdc:aed"),
            ("eu-cb", "scbps-eu-cb", false, "cbdc:eur"),
        ];
        let mut parties = BTreeMap::new();
        let mut validators = Vec::new();
        let mut issuers = Vec::new();
        for (alias, seed, is_validator, asset) in specs {
            let key = KeyPair::from_seed(seed.as_bytes());
            if is_validator {
                validators.push(Validator {
                    address: key.address,
                    stake: 100,
                    verifying_key: key.verifying,
                    signing_key: Some(key.clone()),
                    jailed: false,
                });
            }
            issuers.push((alias.to_string(), key.address, asset.to_string()));
            parties.insert(
                alias.to_string(),
                Party {
                    key,
                    unlimited: true,
                },
            );
        }
        let escrow = KeyPair::from_seed(b"scbps-escrow");
        parties.insert(
            "escrow".into(),
            Party {
                key: escrow,
                unlimited: true,
            },
        );
        for (alias, seed) in [
            ("br-bank", "scbps-br-bank"),
            ("cn-bank", "scbps-cn-bank"),
            ("in-bank", "scbps-in-bank"),
            ("ae-bank", "scbps-ae-bank"),
            ("eu-bank", "scbps-eu-bank"),
            ("restricted", "scbps-restricted"),
            ("timelock", "scbps-timelock"),
        ] {
            let key = KeyPair::from_seed(seed.as_bytes());
            parties.insert(
                alias.to_string(),
                Party {
                    key,
                    unlimited: false,
                },
            );
        }

        let mut compliance = ComplianceEngine::new(PartyPolicy {
            daily_limit: BANK_LIMIT,
            monthly_limit: BANK_LIMIT.saturating_mul(10),
            not_before_ms: None,
            purposes: None,
        });
        let unlimited = PartyPolicy {
            daily_limit: u128::MAX / 4,
            monthly_limit: u128::MAX / 4,
            not_before_ms: None,
            purposes: None,
        };
        for party in parties.values() {
            compliance.kyc.insert(party.key.address);
            if party.unlimited {
                compliance.set_policy(party.key.address, unlimited.clone());
            }
        }
        let restricted = parties.get("restricted").expect("restricted").key.address;
        compliance.set_policy(
            restricted,
            PartyPolicy {
                daily_limit: BANK_LIMIT,
                monthly_limit: BANK_LIMIT.saturating_mul(10),
                not_before_ms: None,
                purposes: Some(BTreeSet::from(["oil-settlement".into()])),
            },
        );
        let timelock = parties.get("timelock").expect("timelock").key.address;
        compliance.set_policy(
            timelock,
            PartyPolicy {
                daily_limit: BANK_LIMIT,
                monthly_limit: BANK_LIMIT.saturating_mul(10),
                not_before_ms: Some(now_ms.saturating_add(10 * DAY_MS)),
                purposes: None,
            },
        );
        let compliance = Arc::new(Mutex::new(compliance));
        let mut node = Node::new(Consensus::new(validators), 1);
        for party in parties.values() {
            node.register(&party.key.verifying).map_err(SystemError::msg)?;
        }
        let hooked = Arc::clone(&compliance);
        node.admission = Some(Arc::new(move |tx: &TxBody| {
            hooked
                .lock()
                .expect("compliance lock")
                .check(tx)
                .map(|_| ())
                .map_err(|err| err.to_string())
        }));

        let mut allocations = Vec::new();
        let mut reserves = BTreeMap::new();
        for (alias, address, asset_name) in &issuers {
            let asset = AssetId::new(asset_name).map_err(SystemError::msg)?;
            allocations.push(GenesisAllocation {
                to: *address,
                asset: asset.clone(),
                amount: ISSUANCE,
            });
            reserves.insert(
                asset_name.clone(),
                CurrencyReserve {
                    asset,
                    issuer: *address,
                    backing: ISSUANCE,
                    minimum_escrow: 100_000_000,
                },
            );
            let _ = alias;
        }
        let gold = AssetId::new("rwa:gold").map_err(SystemError::msg)?;
        let stable = AssetId::new("stable:usd").map_err(SystemError::msg)?;
        let fiat = AssetId::new("fiat:usd").map_err(SystemError::msg)?;
        let br_cb = parties.get("br-cb").expect("br").key.address;
        let br_bank = parties.get("br-bank").expect("bank").key.address;
        allocations.push(GenesisAllocation {
            to: br_cb,
            asset: gold.clone(),
            amount: 1_000_000,
        });
        allocations.push(GenesisAllocation {
            to: br_cb,
            asset: stable.clone(),
            amount: ISSUANCE,
        });
        allocations.push(GenesisAllocation {
            to: br_bank,
            asset: fiat,
            amount: 500_000,
        });
        node.seal_genesis(now_ms, allocations).map_err(SystemError::msg)?;

        let mut tokens = TokenEngine::default();
        let mut whitelist = BTreeSet::new();
        whitelist.insert(br_cb);
        whitelist.insert(br_bank);
        tokens
            .issue_security(
                SecurityToken {
                    asset: gold,
                    name: "Vaulted gold".into(),
                    issuer: br_cb,
                    decimals: 6,
                    partition: "series-a".into(),
                    whitelist,
                    total_supply: 1_000_000,
                },
                swift_rs_blockchain::Hash::sha256(b"gold-kyc"),
            )
            .map_err(SystemError::msg)?;
        tokens
            .register_stablecoin(
                Stablecoin {
                    asset: stable,
                    collateral_asset: AssetId::new("fiat:usd").map_err(SystemError::msg)?,
                    ratio_bps: 10_000,
                    supply: 0,
                    collateral: 0,
                },
                br_cb,
                swift_rs_blockchain::Hash::sha256(b"stable-kyc"),
            )
            .map_err(SystemError::msg)?;

        let oracle_keys = ["br-cb", "cn-cb", "in-cb", "ae-cb"]
            .into_iter()
            .map(|alias| parties.get(alias).expect(alias).key.verifying)
            .collect();
        let oracle = PriceOracle {
            pair: "brl/cny".into(),
            value_e8: 120_000_000,
            epoch: 1,
            threshold: 3,
            signers: oracle_keys,
        };
        let governance = Governance::with_treasury(
            ["br-cb", "cn-cb", "in-cb", "ae-cb"]
                .into_iter()
                .map(|alias| parties.get(alias).expect(alias).key.address)
                .collect(),
            3,
        );

        let mut system = Self {
            node,
            parties,
            compliance,
            vault: PrivacyVault::from_regulator_secret(b"scbps-demo-regulator"),
            reserves,
            receipts: Vec::new(),
            tokens,
            oracle,
            governance,
            disclosures: BTreeMap::new(),
            target_tx_count: 4,
            now_ms,
            frozen_clock: false,
            seen_pairs: BTreeSet::new(),
            recent_amounts: BTreeMap::new(),
        };
        for (issuer, _asset) in [
            ("br-cb", "cbdc:brl"),
            ("cn-cb", "cbdc:cny"),
            ("in-cb", "cbdc:inr"),
            ("ae-cb", "cbdc:aed"),
            ("eu-cb", "cbdc:eur"),
        ] {
            let bank = issuer.replace("-cb", "-bank");
            system.transfer(issuer, &bank, _asset, BANK_FLOAT, "genesis-distribution")?;
            system.transfer(issuer, "escrow", _asset, ESCROW_FLOAT, "genesis-distribution")?;
        }
        system.transfer("br-cb", "restricted", "cbdc:brl", 1_000_000, "genesis-distribution")?;
        system.transfer("br-cb", "timelock", "cbdc:brl", 1_000_000, "genesis-distribution")?;
        Ok(system)
    }

    pub fn freeze_clock(&mut self, now_ms: u64) {
        self.frozen_clock = true;
        self.now_ms = now_ms;
    }

    pub fn advance_clock(&mut self, delta_ms: u64) {
        self.frozen_clock = true;
        self.now_ms = self.now_ms.saturating_add(delta_ms);
    }

    pub fn height(&self) -> u64 {
        self.node.height()
    }

    pub fn balance(&self, alias: &str, asset: &str) -> Result<u128, SystemError> {
        let party = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?;
        let asset = AssetId::new(asset).map_err(SystemError::msg)?;
        Ok(self.node.ledger.balance(&party.key.address, &asset))
    }

    pub fn status(&self) -> ChainStatus {
        let escrow = self.parties.get("escrow").expect("escrow").key.address;
        let reserves = self
            .reserves
            .values()
            .map(|reserve| {
                let issuer_balance = self.node.ledger.balance(&reserve.issuer, &reserve.asset);
                let supply = self
                    .node
                    .ledger
                    .supply
                    .get(&reserve.asset.0)
                    .copied()
                    .unwrap_or(0);
                ReserveView {
                    asset: reserve.asset.0.clone(),
                    backing: reserve.backing,
                    circulating: supply.saturating_sub(issuer_balance),
                    escrow: self.node.ledger.balance(&escrow, &reserve.asset),
                }
            })
            .collect::<Vec<_>>();
        ChainStatus {
            system_version: env!("CARGO_PKG_VERSION"),
            height: self.node.height(),
            base_fee: self.node.base_fee,
            finalized_transactions: self.disclosures.len(),
            cbdc_count: self.reserves.len(),
            oracle_brl_cny_e8: self.oracle.value_e8,
            reserves,
        }
    }

    pub fn public_records(&self) -> Vec<PublicRecord> {
        self.disclosures
            .iter()
            .map(|(tx_id, stored)| PublicRecord {
                height: stored.height,
                tx_id: tx_id.clone(),
                asset: stored.disclosure.asset.clone(),
                purpose: stored.disclosure.purpose.clone(),
                commitment: stored.disclosure.commitment.to_hex(),
            })
            .collect()
    }

    pub fn open_for_regulator(&self, tx_id: &str, secret: &str) -> Result<RevealedTransfer, SystemError> {
        if secret != "demo-regulator" {
            return Err(SystemError::msg("regulator key rejected"));
        }
        let stored = self
            .disclosures
            .get(tx_id)
            .ok_or_else(|| SystemError::msg("unknown transaction"))?;
        self.vault.open(&stored.disclosure).map_err(SystemError::msg)
    }

    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        asset: &str,
        amount: u128,
        purpose: &str,
    ) -> Result<TransferReceipt, SystemError> {
        self.touch_clock();
        let asset = AssetId::new(asset).map_err(SystemError::msg)?;
        let (from_key, to_address) = self.endpoints(from, to)?;
        if asset.as_str() == "rwa:gold" {
            self.tokens
                .authorize_security_transfer(&asset, &from_key.address, &to_address)
                .map_err(SystemError::msg)?;
        }
        let body = self.body(&from_key, to_address, asset, amount, purpose)?;
        let score = self.score(&body)?;
        self.screen(&body)?;
        let signed = SignedTransaction::sign(body.clone(), &from_key).map_err(SystemError::msg)?;
        let tx_id = body.id().map_err(SystemError::msg)?.to_hex();
        let block = self
            .node
            .finalize_batch(vec![signed], self.now_ms)
            .map_err(SystemError::msg)?;
        self.finish(vec![body], block.header.height, block.header.tx_count as u64)?;
        Ok(TransferReceipt {
            tx_id,
            height: block.header.height,
            asset: asset_name(&block),
            amount,
            purpose: purpose.to_string(),
            risk_score: score,
            finalized: true,
        })
    }

    pub fn swap(
        &mut self,
        from: &str,
        to: &str,
        source: &str,
        dest: &str,
        amount: u128,
        purpose: &str,
    ) -> Result<SwapReceipt, SystemError> {
        self.touch_clock();
        let source_asset = AssetId::new(source).map_err(SystemError::msg)?;
        let dest_asset = AssetId::new(dest).map_err(SystemError::msg)?;
        let rate = self.rate_for(source, dest)?;
        let (sender, recipient) = self.endpoints(from, to)?;
        let escrow = self.parties.get("escrow").expect("escrow").key.clone();
        let escrow_balance = self.node.ledger.balance(&escrow.address, &dest_asset);
        let (txs, receipt) = lock_and_release(
            &escrow,
            &sender,
            &recipient,
            &source_asset,
            &dest_asset,
            amount,
            rate,
            self.node.ledger.nonce(&sender.address),
            self.node.ledger.nonce(&escrow.address),
            self.node.base_fee,
            self.now_ms,
            escrow_balance,
            purpose,
            self.compliance.lock().expect("compliance").daily_limit_of(&sender.address),
        )
        .map_err(SystemError::msg)?;
        for tx in &txs {
            self.score(&tx.body)?;
            self.screen(&tx.body)?;
        }
        let block = self
            .node
            .finalize_batch(txs.clone(), self.now_ms)
            .map_err(SystemError::msg)?;
        let bodies = txs.into_iter().map(|tx| tx.body).collect::<Vec<_>>();
        let dest_amount = receipt.dest_amount;
        let receipt_id = receipt.id.to_hex();
        self.receipts.push(receipt);
        self.finish(bodies, block.header.height, block.header.tx_count as u64)?;
        Ok(SwapReceipt {
            receipt_id,
            source_amount: amount,
            dest_amount,
            rate_e8: rate,
            height: block.header.height,
        })
    }

    pub fn redeem(&mut self, receipt_id: &str, holder: &str) -> Result<u64, SystemError> {
        self.touch_clock();
        let index = self
            .receipts
            .iter()
            .position(|receipt| receipt.id.to_hex() == receipt_id)
            .ok_or_else(|| SystemError::msg("unknown receipt"))?;
        let receipt = self.receipts[index].clone();
        let holder_key = self.parties.get(holder).ok_or_else(|| SystemError::msg("unknown party"))?.key.clone();
        let escrow = self.parties.get("escrow").expect("escrow").key.clone();
        let balance = self.node.ledger.balance(&escrow.address, &receipt.source_asset);
        let txs = redeem(
            &escrow,
            &holder_key,
            &receipt,
            self.node.ledger.nonce(&holder_key.address),
            self.node.ledger.nonce(&escrow.address),
            self.node.base_fee,
            self.now_ms,
            balance,
        )
        .map_err(SystemError::msg)?;
        for tx in &txs {
            self.screen(&tx.body)?;
        }
        let block = self.node.finalize_batch(txs.clone(), self.now_ms).map_err(SystemError::msg)?;
        self.receipts[index].redeemed = true;
        let bodies = txs.into_iter().map(|tx| tx.body).collect();
        self.finish(bodies, block.header.height, block.header.tx_count as u64)?;
        Ok(block.header.height)
    }

    pub fn mint_stablecoin(&mut self, holder: &str, amount: u128) -> Result<TransferReceipt, SystemError> {
        let collateral = self
            .tokens
            .required_collateral(&AssetId::new("stable:usd").map_err(SystemError::msg)?, amount)
            .map_err(SystemError::msg)?;
        self.transfer(holder, "escrow", "fiat:usd", collateral, "stable-mint")?;
        let receipt = self.transfer("br-cb", holder, "stable:usd", amount, "stable-mint")?;
        self.tokens
            .record_mint(&AssetId::new("stable:usd").map_err(SystemError::msg)?, amount, collateral)
            .map_err(SystemError::msg)?;
        Ok(receipt)
    }

    pub fn propose(&mut self, alias: &str, title: &str, payload: &str) -> Result<u64, SystemError> {
        self.touch_clock();
        let proposer = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?.key.address;
        Ok(self.governance.propose(proposer, title, payload, self.now_ms))
    }

    pub fn vote(&mut self, alias: &str, proposal_id: u64, support: bool) -> Result<(), SystemError> {
        self.touch_clock();
        let voter = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?.key.clone();
        let stake = self
            .node
            .consensus
            .validators
            .iter()
            .find(|validator| validator.address == voter.address)
            .map(|validator| validator.stake)
            .unwrap_or(0);
        self.governance
            .cast_vote(proposal_id, &voter, stake, support, self.now_ms)
            .map_err(SystemError::msg)
    }

    pub fn tally(&mut self, proposal_id: u64) -> Result<String, SystemError> {
        self.touch_clock();
        let total = self.node.consensus.total_stake();
        let status = self
            .governance
            .tally(proposal_id, total, self.now_ms)
            .map_err(SystemError::msg)?;
        if status == ProposalStatus::Passed {
            let payload = self
                .governance
                .proposals
                .iter()
                .find(|proposal| proposal.id == proposal_id)
                .expect("proposal")
                .payload
                .clone();
            self.execute(&payload)?;
        }
        Ok(format!("{status:?}"))
    }

    pub fn proposals(&self) -> Vec<ProposalView> {
        self.governance
            .proposals
            .iter()
            .map(|proposal| ProposalView {
                id: proposal.id,
                title: proposal.title.clone(),
                payload: proposal.payload.clone(),
                votes_for: proposal.votes_for,
                votes_against: proposal.votes_against,
                status: format!("{:?}", proposal.status),
            })
            .collect()
    }

    pub fn export_message(&self, rail: &str, tx_id: &str) -> Result<String, SystemError> {
        let rail = match rail {
            "swift" => Rail::Swift,
            "cips" => Rail::Cips,
            "spfs" => Rail::Spfs,
            "tips" => Rail::Tips,
            _ => return Err(SystemError::msg("unknown rail")),
        };
        let stored = self.disclosures.get(tx_id).ok_or_else(|| SystemError::msg("unknown transaction"))?;
        let revealed = self.vault.open(&stored.disclosure).map_err(SystemError::msg)?;
        let body = TxBody {
            from: revealed.from,
            to: revealed.to,
            asset: AssetId::new(&revealed.asset).map_err(SystemError::msg)?,
            amount: revealed.amount,
            fee: self.node.base_fee,
            nonce: 0,
            purpose: revealed.purpose,
            timestamp_ms: self.now_ms,
        };
        to_xml(rail, &body, tx_id).map_err(SystemError::msg)
    }

    pub fn metrics(&self) -> String {
        let status = self.status();
        format!(
            "# HELP scbps_chain_height Finalized block height\n# TYPE scbps_chain_height gauge\nscbps_chain_height {}\n# HELP scbps_base_fee Current base fee\n# TYPE scbps_base_fee gauge\nscbps_base_fee {}\n# HELP scbps_finalized_txs Finalized payments\n# TYPE scbps_finalized_txs counter\nscbps_finalized_txs {}\n# HELP scbps_cbdc_count Issued CBDC assets\n# TYPE scbps_cbdc_count gauge\nscbps_cbdc_count {}\n",
            status.height, status.base_fee, status.finalized_transactions, status.cbdc_count
        )
    }

    fn execute(&mut self, payload: &str) -> Result<(), SystemError> {
        if let Some(alias) = payload.strip_prefix("sanction:add:") {
            let address = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?.key.address;
            self.compliance.lock().expect("compliance").sanctions.insert(address);
        } else if let Some(alias) = payload.strip_prefix("sanction:remove:") {
            let address = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?.key.address;
            self.compliance.lock().expect("compliance").sanctions.remove(&address);
        } else if let Some(value) = payload.strip_prefix("fee:base:") {
            let fee = value.parse::<u128>().map_err(SystemError::msg)?;
            self.node.set_base_fee(fee.max(1));
        } else if payload.starts_with("text:") {
            return Ok(());
        } else {
            return Err(SystemError::msg("unknown proposal payload"));
        }
        Ok(())
    }

    pub fn slash(&mut self, alias: &str, reason: SlashReason) -> Result<u128, SystemError> {
        let address = self.parties.get(alias).ok_or_else(|| SystemError::msg("unknown party"))?.key.address;
        let stake = self
            .node
            .consensus
            .validators
            .iter()
            .find(|validator| validator.address == address)
            .map(|validator| validator.stake)
            .ok_or_else(|| SystemError::msg("not a validator"))?;
        let penalty = slash_amount(stake, reason);
        self.node.consensus.set_stake(&address, stake.saturating_sub(penalty));
        if reason == SlashReason::DoubleSign {
            self.node.consensus.jail(&address, true);
        }
        Ok(penalty)
    }

    fn endpoints(&self, from: &str, to: &str) -> Result<(KeyPair, Address), SystemError> {
        let from_key = self.parties.get(from).ok_or_else(|| SystemError::msg("unknown sender"))?.key.clone();
        let to_address = self.parties.get(to).ok_or_else(|| SystemError::msg("unknown recipient"))?.key.address;
        Ok((from_key, to_address))
    }

    fn body(
        &self,
        from: &KeyPair,
        to: Address,
        asset: AssetId,
        amount: u128,
        purpose: &str,
    ) -> Result<TxBody, SystemError> {
        Ok(TxBody {
            from: from.address,
            to,
            asset,
            amount,
            fee: self.node.base_fee,
            nonce: self.node.ledger.nonce(&from.address),
            purpose: purpose.to_string(),
            timestamp_ms: self.now_ms,
        })
    }

    fn screen(&self, body: &TxBody) -> Result<(), SystemError> {
        self.compliance
            .lock()
            .expect("compliance")
            .check(body)
            .map(|_| ())
            .map_err(|err: PrivacyError| SystemError::msg(err.to_string()))
    }

    fn score(&self, body: &TxBody) -> Result<f64, SystemError> {
        let compliance = self.compliance.lock().expect("compliance");
        let repeated = self
            .recent_amounts
            .get(&body.from)
            .map(|amounts| amounts.iter().filter(|amount| **amount == body.amount).count())
            .unwrap_or(0) as u32;
        let features = TransferFeatures {
            amount: body.amount,
            daily_spent_before: compliance.daily_spent(&body.from, body.timestamp_ms),
            daily_limit: compliance.daily_limit_of(&body.from),
            purpose_allowed: compliance.purpose_allowed(&body.from, &body.purpose),
            sanctioned: compliance.sanctions.contains(&body.from) || compliance.sanctions.contains(&body.to),
            new_counterparty: !self.seen_pairs.contains(&(body.from, body.to)),
            equal_split_count: repeated,
        };
        let score = score_transfer(&features);
        if score.risk_level == RiskLevel::Critical {
            return Err(SystemError::msg(format!("risk {}", score.flags.join(","))));
        }
        Ok(score.score)
    }

    fn finish(&mut self, bodies: Vec<TxBody>, height: u64, tx_count: u64) -> Result<(), SystemError> {
        for body in bodies {
            self.compliance.lock().expect("compliance").commit(&body);
            let disclosure = self.vault.seal(&body).map_err(SystemError::msg)?;
            let tx_id = body.id().map_err(SystemError::msg)?.to_hex();
            self.seen_pairs.insert((body.from, body.to));
            self.recent_amounts.entry(body.from).or_default().push(body.amount);
            self.disclosures.insert(tx_id, StoredDisclosure { height, disclosure });
        }
        let next = next_base_fee(self.node.base_fee, tx_count, self.target_tx_count);
        self.node.set_base_fee(next);
        Ok(())
    }

    fn rate_for(&self, source: &str, dest: &str) -> Result<u128, SystemError> {
        if source == "cbdc:brl" && dest == "cbdc:cny" {
            Ok(self.oracle.value_e8)
        } else if source == "cbdc:cny" && dest == "cbdc:brl" {
            Ok(10_000_000_000_000_000 / self.oracle.value_e8)
        } else {
            Err(SystemError::msg("no oracle for this pair"))
        }
    }

    fn touch_clock(&mut self) {
        if !self.frozen_clock {
            self.now_ms = wall_clock_ms();
        }
    }
}

fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn asset_name(block: &swift_rs_blockchain::Block) -> String {
    block
        .transactions
        .first()
        .map(|tx| tx.body.asset.0.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settles_a_screened_payment_and_hides_parties_from_the_public_record() {
        let mut system = PaymentSystem::bootstrap().unwrap();
        system.freeze_clock(system.now_ms);
        let before = system.balance("br-bank", "cbdc:brl").unwrap();
        let receipt = system
            .transfer("br-bank", "cn-bank", "cbdc:brl", 1_000, "trade")
            .unwrap();
        assert!(receipt.finalized);
        assert_eq!(system.balance("br-bank", "cbdc:brl").unwrap(), before - 1_000 - 1);
        assert_eq!(system.balance("cn-bank", "cbdc:brl").unwrap(), 1_000);
        system.node.audit().unwrap();
        let public_record = system.public_records().into_iter().find(|row| row.tx_id == receipt.tx_id).unwrap();
        let sender = system.parties.get("br-bank").unwrap().key.address.to_hex();
        assert!(!public_record.commitment.is_empty());
        assert!(!format!("{public_record:?}").contains(&sender));
        let revealed = system.open_for_regulator(&receipt.tx_id, "demo-regulator").unwrap();
        assert_eq!(revealed.amount, 1_000);
        assert!(system.open_for_regulator(&receipt.tx_id, "nope").is_err());
        assert!(system.export_message("cips", &receipt.tx_id).unwrap().contains("BRL"));
    }

    #[test]
    fn sanctions_purpose_and_timelock_cannot_be_bypassed() {
        let mut system = PaymentSystem::bootstrap().unwrap();
        system.freeze_clock(system.now_ms);
        let id = system.propose("br-cb", "list", "sanction:add:restricted").unwrap();
        for voter in ["br-cb", "cn-cb", "in-cb"] {
            system.vote(voter, id, true).unwrap();
        }
        system.advance_clock(DAY_MS);
        assert_eq!(system.tally(id).unwrap(), "Passed");
        let err = system
            .transfer("restricted", "br-bank", "cbdc:brl", 10, "oil-settlement")
            .unwrap_err();
        assert!(err.to_string().contains("sanctions"));
        system.advance_clock(1);
        let err = system.transfer("timelock", "br-bank", "cbdc:brl", 10, "trade").unwrap_err();
        assert!(err.to_string().contains("timelock"));
        system.advance_clock(11 * DAY_MS);
        let err = system
            .transfer("br-bank", "restricted", "cbdc:brl", 10, "trade")
            .unwrap_err();
        assert!(err.to_string().contains("sanctions"));
        let err = system
            .transfer("cn-bank", "br-bank", "cbdc:cny", 10, "trade")
            .unwrap();
        let _ = err;
        let blocked = system.transfer("eu-bank", "br-bank", "cbdc:eur", 10, "payroll").unwrap();
        assert!(blocked.finalized);
        let purpose = PaymentSystem::bootstrap().unwrap();
        let mut purpose = purpose;
        purpose.freeze_clock(purpose.now_ms);
        let err = purpose
            .transfer("restricted", "br-bank", "cbdc:brl", 10, "trade")
            .unwrap_err();
        assert!(err.to_string().contains("purpose"));
    }

    #[test]
    fn atomic_cbdc_swap_keeps_backing_and_redeems() {
        let mut system = PaymentSystem::bootstrap().unwrap();
        system.freeze_clock(system.now_ms);
        let swap = system
            .swap("br-bank", "cn-bank", "cbdc:brl", "cbdc:cny", 10_000, "trade")
            .unwrap();
        assert_eq!(swap.dest_amount, 12_000);
        assert_eq!(system.balance("cn-bank", "cbdc:cny").unwrap(), 50_000_000 + 12_000);
        system.node.audit().unwrap();
        assert!(system.status().reserves.iter().all(|reserve| reserve.circulating <= reserve.backing));
        assert_eq!(system.status().cbdc_count, 5);
        system.redeem(&swap.receipt_id, "cn-bank").unwrap();
        assert_eq!(system.balance("cn-bank", "cbdc:cny").unwrap(), 50_000_000 - 1);
        system.node.audit().unwrap();
    }

    #[test]
    fn two_validators_cannot_pass_governance_and_slash_reduces_stake() {
        let mut system = PaymentSystem::bootstrap().unwrap();
        system.freeze_clock(system.now_ms);
        let id = system.propose("br-cb", "note", "text:observe").unwrap();
        system.vote("br-cb", id, true).unwrap();
        system.vote("cn-cb", id, true).unwrap();
        system.advance_clock(DAY_MS);
        assert_eq!(system.tally(id).unwrap(), "Rejected");
        let penalty = system.slash("ae-cb", SlashReason::DoubleSign).unwrap();
        assert_eq!(penalty, 5);
        let err = system
            .transfer("br-bank", "cn-bank", "cbdc:brl", BANK_LIMIT + 1, "trade")
            .unwrap_err();
        assert!(err.to_string().contains("limit") || err.to_string().contains("daily"));
    }
}
