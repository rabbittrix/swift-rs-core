# ADR 0001: Permissioned BFT with fail-closed screening

## Status

Accepted

## Context

Cross-border CBDC settlement needs finality without a single operator, and it needs KYC/AML controls that still let a regulator open a specific payment.

## Decision

- Validator identity is an ed25519 key. A block is final when signed stake is at least 67% of total stake, including jailed stake in the denominator so a minority cannot finalize alone.
- Blocks are hash-linked with SHA-256. SHA3-256 is available for commitments that opt into it. Transaction inclusion uses a Merkle root.
- Every finalized payment passes the compliance engine first. Sanctions, missing KYC, limits, purpose locks, and timelocks are hard rejects.
- Public data is a commitment. The plaintext is encrypted to the regulator key. This is selective disclosure by escrow, not a zero-knowledge proof.
- Programmable limits run as WASM under a fuel cap. The host rejects modules that do not terminate within that cap.

## Consequences

- Adding a bank is a genesis registration in this MVP. Later onboarding has to be its own finalized transaction so replay stays deterministic.
- Hand-rolled zk-SNARKs and post-quantum signatures are out of scope until an audited implementation is chosen.
