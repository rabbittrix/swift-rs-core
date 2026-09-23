//! Mock settlement commands for the desktop demonstration.
//!
//! Screening is fail-closed: a name marked restricted is refused and no proof
//! binding is returned. These functions are the seam for later `swift-rs-core`
//! calls. They do not talk to a live central bank.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

static TICK: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
pub struct PaymentRequest {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub currency_from: String,
    pub currency_to: String,
    pub route: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentStatus {
    pub id: String,
    pub route: String,
    pub status: String,
    pub fee_usd: f64,
    pub settlement_label: String,
    pub tx_hash: Option<String>,
    pub detail: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ValidatorNode {
    pub id: String,
    pub name: String,
    pub city: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct NetworkStats {
    pub tps: u32,
    pub total_volume_24h: String,
    pub active_nodes: u32,
    pub avg_settlement_ms: u32,
    pub validators: Vec<ValidatorNode>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CurrencyBalance {
    pub currency: String,
    pub name: String,
    pub amount: String,
    pub issuer: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LedgerRow {
    pub id: String,
    pub date: String,
    pub r#type: String,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub tx_hash: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LiveTransaction {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub amount: String,
    pub currency: String,
    pub settled_ms: u32,
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
    format!("{acc:016x}{acc:016x}")
}

fn now_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{prefix}-{millis}")
}

fn validators() -> Vec<ValidatorNode> {
    vec![
        node("br", "Banco Central do Brasil", "Brasília", 30.0, 68.0),
        node("cn", "PBOC", "Beijing", 78.0, 38.0),
        node("ae", "Central Bank of the UAE", "Abu Dhabi", 62.0, 48.0),
        node("in", "Reserve Bank of India", "Mumbai", 70.0, 54.0),
        node("eu", "ECB", "Frankfurt", 48.0, 36.0),
    ]
}

fn node(id: &str, name: &str, city: &str, x: f64, y: f64) -> ValidatorNode {
    ValidatorNode {
        id: id.into(),
        name: name.into(),
        city: city.into(),
        x,
        y,
    }
}

fn next_live() -> LiveTransaction {
    const PAIRS: &[(&str, &str, &str)] = &[
        ("Banco Central do Brasil", "PBOC", "BRL-CBDC"),
        ("PBOC", "Central Bank of the UAE", "CNY-CBDC"),
        ("Reserve Bank of India", "ECB", "INR-CBDC"),
        ("ECB", "Banco Central do Brasil", "EUR-CBDC"),
        ("Central Bank of the UAE", "Reserve Bank of India", "AED-CBDC"),
    ];
    let tick = TICK.fetch_add(1, Ordering::Relaxed);
    let (from_node, to_node, currency) = PAIRS[(tick as usize) % PAIRS.len()];
    let amount = 1_250_000 + (tick % 7) * 175_000;
    LiveTransaction {
        id: format!("live-{tick:x}"),
        from_node: from_node.into(),
        to_node: to_node.into(),
        amount: format_amount(amount),
        currency: currency.into(),
        settled_ms: 640 + ((tick % 5) as u32) * 40,
    }
}

fn format_amount(amount: u64) -> String {
    let digits = amount.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[tauri::command]
pub fn initiate_payment(payload: PaymentRequest) -> Result<PaymentStatus, String> {
    if payload.amount <= 0.0 {
        return Err("amount must be positive".into());
    }
    let sovereign = payload.route.eq_ignore_ascii_case("SOVEREIGN");
    let id = now_id("pay");
    if restricted(&payload.from) || restricted(&payload.to) {
        return Ok(PaymentStatus {
            id,
            route: payload.route,
            status: "blocked".into(),
            fee_usd: if sovereign { 0.0 } else { 45.0 },
            settlement_label: "not settled".into(),
            tx_hash: None,
            detail: "counterparty failed the sanctions screen".into(),
        });
    }
    if sovereign {
        return Ok(PaymentStatus {
            id: id.clone(),
            route: "SOVEREIGN".into(),
            status: "settled".into(),
            fee_usd: 0.01,
            settlement_label: "< 2 seconds".into(),
            tx_hash: Some(demo_hash(&id)),
            detail: format!(
                "atomic swap {} -> {} finalized by the local consortium",
                payload.currency_from, payload.currency_to
            ),
        });
    }
    Ok(PaymentStatus {
        id,
        route: "LEGACY".into(),
        status: "settled".into(),
        fee_usd: 45.0,
        settlement_label: "2–3 days".into(),
        tx_hash: None,
        detail: "correspondent chain and USD clearing".into(),
    })
}

#[tauri::command]
pub fn get_network_stats() -> NetworkStats {
    NetworkStats {
        tps: 4500,
        total_volume_24h: "1.2B USD".into(),
        active_nodes: 42,
        avg_settlement_ms: 840,
        validators: validators(),
    }
}

#[tauri::command]
pub fn get_wallet_balances() -> Vec<CurrencyBalance> {
    vec![
        balance("BRL-CBDC", "Digital Real", "48,250,000.00", "Banco Central do Brasil"),
        balance("CNY-CBDC", "Digital Yuan", "36,800,000.00", "PBOC"),
        balance("AED-CBDC", "Digital Dirham", "12,400,000.00", "Central Bank of the UAE"),
        balance("INR-CBDC", "Digital Rupee", "9,100,000.00", "Reserve Bank of India"),
        balance("EUR-CBDC", "Digital Euro", "6,750,000.00", "ECB"),
    ]
}

fn balance(currency: &str, name: &str, amount: &str, issuer: &str) -> CurrencyBalance {
    CurrencyBalance {
        currency: currency.into(),
        name: name.into(),
        amount: amount.into(),
        issuer: issuer.into(),
    }
}

#[tauri::command]
pub fn get_transactions() -> Vec<LedgerRow> {
    vec![
        row("hx-9f21", "23 Sep 2026 17:41", "Swap", "5,000,000.00", "BRL-CBDC", "Settled"),
        row("hx-9f18", "23 Sep 2026 16:05", "Receive", "1,200,000.00", "CNY-CBDC", "Settled"),
        row("hx-9f11", "23 Sep 2026 11:22", "Send", "800,000.00", "AED-CBDC", "Settled"),
        row("hx-9e90", "22 Sep 2026 19:14", "Swap", "250,000.00", "INR-CBDC", "Blocked"),
        row("hx-9e44", "22 Sep 2026 09:03", "Receive", "2,400,000.00", "EUR-CBDC", "Settled"),
    ]
}

fn row(id: &str, date: &str, kind: &str, amount: &str, currency: &str, status: &str) -> LedgerRow {
    LedgerRow {
        id: id.into(),
        date: date.into(),
        r#type: kind.into(),
        amount: amount.into(),
        currency: currency.into(),
        status: status.into(),
        tx_hash: demo_hash(id),
    }
}

/// Emits one corridor settlement. The background feed calls the same generator.
#[tauri::command]
pub fn simulate_live_transaction(app: AppHandle) {
    let _ = app.emit("live-transaction", next_live());
}

pub fn start_live_feed(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(3));
        let _ = app.emit("live-transaction", next_live());
    });
}
