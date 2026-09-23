//! Step-by-step payment simulation with live `transaction_update` events.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum RouteType {
    LegacySwift,
    SovereignRs,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    pub payment_id: Option<String>,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub currency_from: String,
    pub currency_to: String,
    pub route: RouteType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStep {
    pub index: u32,
    pub total: u32,
    pub label: String,
    pub detail: String,
    pub tone: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionUpdate {
    pub payment_id: String,
    pub route: RouteType,
    pub kind: String,
    pub step: Option<TransactionStep>,
    pub elapsed_ms: u64,
    pub fee_usd: Option<f64>,
    pub tx_hash: Option<String>,
    pub total_elapsed_ms: Option<u64>,
    pub timestamp: String,
}

struct StepDef {
    label: &'static str,
    detail: &'static str,
    tone: &'static str,
    delay_ms: u64,
    block_here_if_restricted: bool,
}

fn restricted(name: &str) -> bool {
    let value = name.to_ascii_lowercase();
    value.contains("restricted") || value.contains("sanction")
}

fn demo_hash(seed: &str) -> String {
    let mut acc: u64 = 0xcbf29ce484222325;
    for byte in seed.bytes() {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(0x100000001b3);
    }
    format!("0x{acc:016x}{acc:016x}")
}

fn legacy_steps() -> Vec<StepDef> {
    vec![
        StepDef {
            label: "Routing via Correspondent Bank (NY)",
            detail: "Instruction queued at the nostro agent in New York",
            tone: "amber",
            delay_ms: 1_400,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "Waiting for USD Clearing",
            detail: "Dollar leg parked in the correspondent chain",
            tone: "amber",
            delay_ms: 1_500,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "OFAC / Sanctions Compliance Check",
            detail: "Name screen pending at the intermediary",
            tone: "amber",
            delay_ms: 1_600,
            block_here_if_restricted: true,
        },
        StepDef {
            label: "Intermediary Fees Applied",
            detail: "Correspondent and FX mark-ups deducted",
            tone: "amber",
            delay_ms: 1_300,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "Final Settlement",
            detail: "Beneficiary credit after the chain completes",
            tone: "amber",
            delay_ms: 1_200,
            block_here_if_restricted: false,
        },
    ]
}

fn sovereign_steps() -> Vec<StepDef> {
    vec![
        StepDef {
            label: "ZK-Proof Compliance Validation",
            detail: "Public limit and sanctions-clear flag verified; amount stays private",
            tone: "blue",
            delay_ms: 350,
            block_here_if_restricted: true,
        },
        StepDef {
            label: "Lock BRL-CBDC in Smart Contract",
            detail: "Source units locked by the issuing central bank",
            tone: "blue",
            delay_ms: 400,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "Execute Atomic Swap (Oracle Price)",
            detail: "Both legs settle together or the batch aborts",
            tone: "emerald",
            delay_ms: 450,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "BFT Consensus Reached (67% Nodes)",
            detail: "Stake-weighted certificate from the local validator set",
            tone: "emerald",
            delay_ms: 400,
            block_here_if_restricted: false,
        },
        StepDef {
            label: "Final Irreversible Settlement",
            detail: "Proof binding published to the explorer commitment",
            tone: "emerald",
            delay_ms: 350,
            block_here_if_restricted: false,
        },
    ]
}

fn emit(app: &AppHandle, update: &TransactionUpdate) -> Result<(), String> {
    app.emit("transaction_update", update)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn simulate_payment(request: PaymentRequest, app: AppHandle) -> Result<String, String> {
    if request.amount <= 0.0 {
        return Err("amount must be positive".into());
    }
    let payment_id = request
        .payment_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let blocked_party = restricted(&request.from) || restricted(&request.to);
    let steps = match request.route {
        RouteType::LegacySwift => legacy_steps(),
        RouteType::SovereignRs => sovereign_steps(),
    };
    let total = steps.len() as u32;
    let started = Instant::now();

    for (index, def) in steps.iter().enumerate() {
        let idx = index as u32;
        let active = TransactionStep {
            index: idx,
            total,
            label: def.label.into(),
            detail: def.detail.into(),
            tone: def.tone.into(),
            status: "active".into(),
        };
        emit(
            &app,
            &TransactionUpdate {
                payment_id: payment_id.clone(),
                route: request.route,
                kind: "step".into(),
                step: Some(active),
                elapsed_ms: started.elapsed().as_millis() as u64,
                fee_usd: None,
                tx_hash: None,
                total_elapsed_ms: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        )?;
        sleep(Duration::from_millis(def.delay_ms)).await;

        if blocked_party && def.block_here_if_restricted {
            let done = TransactionStep {
                index: idx,
                total,
                label: def.label.into(),
                detail: "Counterparty failed the sanctions screen. No proof is built.".into(),
                tone: "rose".into(),
                status: "blocked".into(),
            };
            emit(
                &app,
                &TransactionUpdate {
                    payment_id: payment_id.clone(),
                    route: request.route,
                    kind: "blocked".into(),
                    step: Some(done),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                    fee_usd: Some(if request.route == RouteType::SovereignRs {
                        0.0
                    } else {
                        45.0
                    }),
                    tx_hash: None,
                    total_elapsed_ms: Some(started.elapsed().as_millis() as u64),
                    timestamp: Utc::now().to_rfc3339(),
                },
            )?;
            return Err("counterparty failed the sanctions screen".into());
        }

        let done = TransactionStep {
            index: idx,
            total,
            label: def.label.into(),
            detail: def.detail.into(),
            tone: def.tone.into(),
            status: "done".into(),
        };
        emit(
            &app,
            &TransactionUpdate {
                payment_id: payment_id.clone(),
                route: request.route,
                kind: "step".into(),
                step: Some(done),
                elapsed_ms: started.elapsed().as_millis() as u64,
                fee_usd: None,
                tx_hash: None,
                total_elapsed_ms: None,
                timestamp: Utc::now().to_rfc3339(),
            },
        )?;
    }

    let (fee, hash) = match request.route {
        RouteType::LegacySwift => (45.0, None),
        RouteType::SovereignRs => (0.01, Some(demo_hash(&payment_id))),
    };
    emit(
        &app,
        &TransactionUpdate {
            payment_id: payment_id.clone(),
            route: request.route,
            kind: "complete".into(),
            step: None,
            elapsed_ms: started.elapsed().as_millis() as u64,
            fee_usd: Some(fee),
            tx_hash: hash,
            total_elapsed_ms: Some(started.elapsed().as_millis() as u64),
            timestamp: Utc::now().to_rfc3339(),
        },
    )?;
    Ok(payment_id)
}
