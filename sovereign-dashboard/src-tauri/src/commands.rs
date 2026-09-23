//! Mock settlement commands for the desktop demonstration.
//!
//! Screening is fail-closed: a name marked restricted is refused and no proof
//! binding is returned. These functions are the seam for later `swift-rs-core`
//! calls. They do not talk to a live central bank.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

const LEDGER_CAP: usize = 500;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LedgerRow {
    pub id: String,
    pub date: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub tx_hash: String,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub corridor: Option<String>,
}

pub struct LedgerState(pub Mutex<Vec<LedgerRow>>);

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
    crate::countries::country_catalog()
        .into_iter()
        .map(|country| ValidatorNode {
            id: country.id,
            name: country.central_bank,
            city: country.name,
            x: country.map_x,
            y: country.map_y,
        })
        .collect()
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
pub fn get_transactions(state: State<'_, LedgerState>) -> Vec<LedgerRow> {
    state.0.lock().map(|rows| rows.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn append_ledger_entries(
    state: State<'_, LedgerState>,
    entries: Vec<LedgerRow>,
) -> Result<Vec<LedgerRow>, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "ledger lock poisoned".to_string())?;
    for entry in entries {
        if guard.iter().any(|row| row.id == entry.id) {
            continue;
        }
        guard.insert(0, entry);
    }
    guard.truncate(LEDGER_CAP);
    Ok(guard.clone())
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
