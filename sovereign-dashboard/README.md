# Sovereign banking desk

Desktop demonstration that places a legacy correspondent payment beside Swift-RS CBDC settlement. Sanctions screening is fail-closed: a restricted counterparty is refused before a proof is built.

The Rust commands are mocks with the shapes a later gateway integration can replace. They do not call the workspace crates, and this folder is not a Cargo workspace member. `cargo test --workspace` does not compile it.

The operator console for the live gateway is [`../dashboard`](../dashboard/README.md). The TPS chart on the Network screen is a simulated corridor (the mock feed starts near 4,500). The measured ledger figure, above 1,000 TPS on a release `finalize_batch`, is documented in the [root README](../README.md).

## Browser

```bash
cd sovereign-dashboard
npm install
npm run dev
```

Open [http://127.0.0.1:1420](http://127.0.0.1:1420). Outside the Tauri shell the page plays the same mock feed in the browser.

## Desktop

```bash
npm run tauri dev
```

The window is 1440×900, sized for a 1080p laptop. `src-tauri/src/main.rs` starts the process. `src-tauri/src/lib.rs` registers the commands and a three-second `live-transaction` feed.

## Screens

- **Simulator** sends one instruction down both rails. Legacy shows correspondent delivery, New York dollar clearing, and the OFAC wait (a time-lapse of 2–3 days at $45). Sovereign locks BRL-CBDC, swaps atomically, mints CNY-CBDC, and settles in under two seconds at $0.01, with a demonstration proof binding. **Restricted Desk** stops both rails.
- **Network** shows the five local pilot validators (Brazil, China, UAE, India, ECB), the TPS chart, and the corridor feed.
- **Wallet** lists BRL, CNY, AED, INR, and EUR balances and a history. **View** opens the demonstration binding.
- **Compliance** shows the public inputs (limit, sanctions-clear flag, binding). Amount and identity stay hidden. A listed party produces no proof. The AML queue can clear, hold, or block, and a list match has no override.

## Commands

Defined in `src-tauri/src/commands.rs`:

- `initiate_payment(payload)`
- `get_network_stats()`
- `get_wallet_balances()`
- `get_transactions()`
- `simulate_live_transaction()` emits `live-transaction`

## UI stack

Vite, React, TypeScript, Tailwind CSS, and the shadcn-style controls in `src/components/ui` (`components.json`). Charts use Recharts. Step motion uses Framer Motion.

```bash
npx shadcn@latest add dialog
```

## License

Apache-2.0
