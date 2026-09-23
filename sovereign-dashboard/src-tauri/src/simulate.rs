//! Step-by-step payment simulation with live `transaction_update` events.

use crate::countries::{corridor_context, CorridorContext};
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
    pub origin_country_id: String,
    pub destination_country_id: String,
    pub amount: f64,
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
    label: String,
    detail: String,
    tone: String,
    delay_ms: u64,
    block_here: bool,
}

fn demo_hash(seed: &str) -> String {
    let mut acc: u64 = 0xcbf29ce484222325;
    for byte in seed.bytes() {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(0x100000001b3);
    }
    format!("0x{acc:016x}{acc:016x}")
}

fn legacy_steps(ctx: &CorridorContext) -> Vec<StepDef> {
    let mut steps = vec![
        StepDef {
            label: format!(
                "Routing via Correspondent Bank ({})",
                if ctx.usd_corridor { "NY" } else { "London" }
            ),
            detail: format!(
                "{} → {} instructed through a nostro agent",
                ctx.origin.fiat_currency, ctx.dest.fiat_currency
            ),
            tone: "amber".into(),
            delay_ms: if ctx.high_swift_friction { 1_800 } else { 1_400 },
            block_here: false,
        },
    ];
    if ctx.usd_corridor || ctx.high_swift_friction {
        steps.push(StepDef {
            label: "Waiting for USD Clearing (New York)".into(),
            detail: "Dollar leg parked while compliance queues the instruction".into(),
            tone: "amber".into(),
            delay_ms: 2_000,
            block_here: false,
        });
    }
    if ctx.sanctioned_touch {
        steps.push(StepDef {
            label: "OFAC Sanctions Screening Triggered".into(),
            detail: format!(
                "Hit on {} or {} — enhanced due diligence required",
                ctx.origin.name, ctx.dest.name
            ),
            tone: "rose".into(),
            delay_ms: 2_200,
            block_here: false,
        });
        steps.push(StepDef {
            label: "Correspondent Bank in NY Freezing Assets for Review".into(),
            detail: "Funds held pending legal review; beneficiary credit blocked".into(),
            tone: "rose".into(),
            delay_ms: 2_400,
            block_here: true,
        });
    } else {
        steps.push(StepDef {
            label: "OFAC / Sanctions Compliance Check".into(),
            detail: "Name screen pending at the intermediary".into(),
            tone: "amber".into(),
            delay_ms: 1_600,
            block_here: false,
        });
    }
    steps.push(StepDef {
        label: "Intermediary Fees Applied".into(),
        detail: "Correspondent, FX, and compliance mark-ups deducted".into(),
        tone: "amber".into(),
        delay_ms: 1_500,
        block_here: false,
    });
    if ctx.sanctioned_touch {
        steps.push(StepDef {
            label: "Transaction Rejected or Delayed (High Risk)".into(),
            detail: "SWIFT chain will not release without manual exception (often denied)".into(),
            tone: "rose".into(),
            delay_ms: 2_000,
            block_here: true,
        });
    } else {
        steps.push(StepDef {
            label: "Final Settlement".into(),
            detail: "Beneficiary credit after the correspondent chain completes (2–5 days)".into(),
            tone: "amber".into(),
            delay_ms: 1_800,
            block_here: false,
        });
    }
    steps
}

fn sovereign_steps(ctx: &CorridorContext) -> Vec<StepDef> {
    let mut steps = vec![StepDef {
        label: "ZK-Proof Compliance Validation".into(),
        detail: if ctx.sanctioned_touch {
            "Listed jurisdiction detected — fail-closed screen; no proof is issued".into()
        } else {
            "Public limit and sanctions-clear flag verified; amount stays private".into()
        },
        tone: if ctx.sanctioned_touch { "rose" } else { "blue" }.into(),
        delay_ms: 380,
        block_here: ctx.sanctioned_touch,
    }];
    if ctx.sanctioned_touch {
        return steps;
    }
    steps.extend([
        StepDef {
            label: "Direct CBDC Corridor (No USD Correspondent)".into(),
            detail: format!(
                "Routing {} ({}) → {} ({}) without a dollar nostro hop",
                ctx.origin.cbdc_name, ctx.origin.fiat_currency, ctx.dest.cbdc_name, ctx.dest.fiat_currency
            ),
            tone: "blue".into(),
            delay_ms: 320,
            block_here: false,
        },
        StepDef {
            label: "mBridge-Style Atomic Swap (Oracle Price)".into(),
            detail: "Both CBDC legs lock and release in one batch".into(),
            tone: "emerald".into(),
            delay_ms: 420,
            block_here: false,
        },
        StepDef {
            label: "BFT Consensus Reached (67% Validators)".into(),
            detail: "Regional central-bank validators co-sign the batch".into(),
            tone: "emerald".into(),
            delay_ms: 380,
            block_here: false,
        },
        StepDef {
            label: "Final Irreversible Settlement".into(),
            detail: "Proof binding published; sub-2-second demo finality".into(),
            tone: "emerald".into(),
            delay_ms: 300,
            block_here: false,
        },
    ]);
    steps
}

fn legacy_fee(ctx: &CorridorContext) -> f64 {
    let mut fee: f64 = 45.0;
    if ctx.usd_corridor {
        fee += 25.0;
    }
    if ctx.sanctioned_touch {
        fee += 80.0;
    }
    if !ctx.origin.swift_member || !ctx.dest.swift_member {
        fee += 30.0;
    }
    fee.min(150.0)
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
    let ctx = corridor_context(&request.origin_country_id, &request.destination_country_id)?;
    let payment_id = request
        .payment_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let steps = match request.route {
        RouteType::LegacySwift => legacy_steps(&ctx),
        RouteType::SovereignRs => sovereign_steps(&ctx),
    };
    let total = steps.len() as u32;
    let started = Instant::now();

    for (index, def) in steps.iter().enumerate() {
        let idx = index as u32;
        let active = TransactionStep {
            index: idx,
            total,
            label: def.label.clone(),
            detail: def.detail.clone(),
            tone: def.tone.clone(),
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

        if def.block_here {
            let done = TransactionStep {
                index: idx,
                total,
                label: def.label.clone(),
                detail: if request.route == RouteType::SovereignRs {
                    "Sanctions list match — payment cannot be included in a block.".into()
                } else {
                    "Corridor frozen by compliance — settlement not released.".into()
                },
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
                        legacy_fee(&ctx)
                    }),
                    tx_hash: None,
                    total_elapsed_ms: Some(started.elapsed().as_millis() as u64),
                    timestamp: Utc::now().to_rfc3339(),
                },
            )?;
            return Err("corridor blocked by compliance".into());
        }

        let done = TransactionStep {
            index: idx,
            total,
            label: def.label.clone(),
            detail: def.detail.clone(),
            tone: def.tone.clone(),
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
        RouteType::LegacySwift => (legacy_fee(&ctx), None),
        RouteType::SovereignRs => (
            0.04,
            Some(demo_hash(&format!(
                "{payment_id}:{}:{}",
                ctx.origin.id, ctx.dest.id
            ))),
        ),
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
