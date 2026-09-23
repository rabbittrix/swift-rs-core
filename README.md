# Swift-RS

Permissioned cross-border settlement in Rust: ISO 20022 messaging, a BFT ledger, multi-CBDC swap, and fail-closed sanctions screening.

[![Rust](https://img.shields.io/badge/built_with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Architecture](https://img.shields.io/badge/pattern-CQRS%2FES-blue)](https://martinfowler.com/bliki/CQRS.html)
[![Standard](https://img.shields.io/badge/ISO-20022%20%7C%20MT%2FMX-compliant-green)](https://www.iso20022.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

## Quick start

### Gateway

```bash
cargo test --workspace
cargo run --bin swift-rs-gateway
```

The gateway listens on [http://localhost:8080](http://localhost:8080). `GET /health` returns the process status. After bootstrap, chain status is `GET /api/v1/chain/status`.

A release build of `finalize_batch` on 2,000 pre-signed transfers measured above 1,000 TPS. That figure is the settlement batch, not the desktop demonstration feed.

### Operator dashboard

```bash
cd dashboard
npm install
npm run dev
```

[http://localhost:3000](http://localhost:3000) proxies `/api/swift/*` to the gateway. Start the gateway first.

### SovereignPay (investor desk)

```bash
cd sovereign-dashboard
npm install
npm run dev          # browser
npm run tauri dev    # Tauri v2 desktop
```

[SovereignPay](sovereign-dashboard/README.md) streams dual-rail settlement steps from Rust via `transaction_update` events (`simulate_payment` in `src-tauri/src/simulate.rs`). The network TPS chart is a simulated corridor, not the ledger benchmark above.

### Docker Compose

```bash
docker-compose up -d
```

This starts the gateway, the Next.js dashboard, PostgreSQL, and Kafka.

## Workspace

| Crate | Role |
| --- | --- |
| `swift-rs-core` | Domain types and message validation |
| `swift-rs-iso20022` | ISO 20022 parse and serialize |
| `swift-rs-connector` | Connectivity adapters |
| `swift-rs-cqrs` | In-process event store |
| `swift-rs-gateway` | HTTP API for messages, transfers, swaps, explorer, and governance |
| `swift-rs-ai` | Risk score used before finality |
| `swift-rs-blockchain` | Stake-weighted BFT (67%), Merkle proofs, channels, shards, WASM predicates, optimistic rollups, hybrid ed25519 + ML-DSA-44 |
| `swift-rs-cbdc` | Five CBDCs (BRL, CNY, INR, AED, EUR), lock/release, reserves |
| `swift-rs-tokenization` | Security tokens and stablecoins |
| `swift-rs-privacy` | KYC and sanctions screening, regulator envelope, Groth16 limit proof |
| `swift-rs-bridge` | ISO 20022 rail translation and the FX oracle |
| `swift-rs-governance` | Stake-weighted votes, treasury, disputes |
| `swift-rs-economics` | Base fee, slashing, genesis amounts |

`sovereign-dashboard/` is a separate Tauri application and is not a workspace member.

Settlement checks are described in [docs/architecture.md](docs/architecture.md).

Release version is **0.2.0** (see root [`VERSION`](VERSION) and `[workspace.package]` in `Cargo.toml`). `GET /health` and `GET /api/v1/chain/status` report `system_version` from the gateway crate.

## What the ledger does

- A block is final at 67% of validator stake. Jailed stake stays in the quorum denominator.
- Screening is fail-closed. A sanctioned or unknown party cannot be included, including through a swap or an imported ISO draft. Sanctions-list changes are governance proposals.
- The public explorer stores a commitment, asset, and purpose. Parties and amounts are opened with the regulator envelope.
- A Groth16 proof shows a payment is under a public limit and sanctions-clear. The amount is a witness. A listed party is rejected before a proof is built. Setup is local, not a ceremony.
- Hybrid signatures (ed25519 and ML-DSA-44) are required together on the local five-bank pilot vote path. The hot path for ordinary certificates stays ed25519.
- Optimistic rollups post a state root and revert it if a challenge replay disagrees.
- The five-bank pilot (BR, CN, IN, AE, EU) is an in-process consortium.

## Limits

- Rail adapters translate ISO 20022. They do not open live sessions to SWIFT, CIPS, SPFS, or TIPS.
- The pilot is not a connection to a live central bank.
- An external security firm has not audited this tree.
- Demo seeds and the regulator secret `demo-regulator` are for local runs only.

## License

Apache-2.0

## Author

Roberto de Souza <rabbittrix@hotmail.com>
