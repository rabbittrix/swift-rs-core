# Settlement architecture

The gateway finalizes a payment only after three checks succeed:

1. KYC registry and sanctions screening (`swift-rs-privacy`)
2. Risk score (`swift-rs-ai`)
3. A HotStuff-style certificate covering at least 67% of validator stake (`swift-rs-blockchain`)

Screening is fail-closed. The ledger admission callback rejects a transaction that fails it, so a caller cannot finalize a block by skipping the HTTP handler. Sanctions changes themselves are governance proposals: they pass only when yes-votes cover 67% of stake. One central bank cannot rewrite history or empty the treasury.

## What this MVP runs

- Five CBDCs: BRL, CNY, INR, AED, and EUR, each pre-issued up to a declared backing amount
- Atomic cross-border lock and release for BRL/CNY using a 3-of-4 oracle
- Security-token whitelist and a collateralized stablecoin register
- Public explorer records that store a commitment, asset, and purpose. Counterparties and amounts stay inside an AES-256-GCM envelope opened with the regulator secret
- WASM spending-limit and quorum predicates executed with fuel metering
- In-process payment channels and a two-phase cross-shard swap that rolls back on failure

## What this MVP does not claim

- It is one process, not a geographically distributed validator network, and it does not target 10,000 TPS
- Selective disclosure is a commitment plus regulator escrow. It is not a zk-SNARK. A production proof must come from an audited library
- Post-quantum signatures (Dilithium, Falcon) are not implemented. Shipping a homemade scheme would be unsafe
- SWIFT, CIPS, SPFS, and TIPS adapters translate to ISO 20022. They do not open live sessions to those networks
- Demo seeds and the regulator secret `demo-regulator` are for local tests only

## Compliance boundary

Jurisdiction lists are enforced. A sanctioned party cannot pay or be paid, including through a bridge, a swap, or an imported ISO 20022 draft that is later submitted. The implementation does not contain a mode that exempts a country or an account from those lists.
