# SovereignPay

Tauri v2 desktop desk that compares legacy SWIFT friction with Swift-RS CBDC settlement. Sanctions screening is fail-closed: a restricted counterparty is refused before a proof is built.

## Scaffolding

```bash
cd sovereign-dashboard
npm install
npm run dev          # Vite @ http://127.0.0.1:1420
npm run tauri dev    # Native window (SovereignPay)
```

Dependencies include Tailwind CSS, shadcn-style UI primitives, Framer Motion, Lucide, Recharts, and Zustand. Rust uses `serde`, `tokio`, `uuid`, and `chrono`.

This folder is not in the parent Cargo workspace.

## Architecture

| Layer | Role |
| --- | --- |
| `src-tauri/src/simulate.rs` | `simulate_payment` — async steps, `tokio::time::sleep`, `transaction_update` events |
| `src-tauri/src/commands.rs` | Stats, wallet, live corridor feed |
| `src/components/PaymentSimulator.tsx` | Dual-track UI, listens for `transaction_update` |
| `src/components/NetworkDashboard.tsx` | Validators, TPS chart, live feed |
| `src/components/WalletView.tsx` | CBDC balances and verified history |
| `src/lib/simulation.ts` | Browser fallback when not running inside Tauri |

## `simulate_payment`

```rust
#[tauri::command]
async fn simulate_payment(request: PaymentRequest, app: AppHandle) -> Result<String, String>
```

- `RouteType`: `LegacySwift` | `SovereignRs`
- Emits `transaction_update` with `TransactionStep` payloads (active → done, or blocked)
- Legacy: ~1.2–1.6s per step (amber). Sovereign: ~350–450ms per step (blue/emerald)
- Returns the `payment_id` on success

Frontend: `subscribeTransactionUpdates` + parallel invokes for both rails.

## Demo tips

- Default ticket: **10M BRL → CNY** at oracle **1.20**
- **Restricted Desk** blocks at OFAC (legacy) or ZK compliance (sovereign)
- Network TPS is a **simulated corridor**, not the ledger benchmark in the root README

## License

Apache-2.0
