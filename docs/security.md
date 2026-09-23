# Security practices

- Keep validator keys in an HSM. `MemoryKeyStore` is an in-process stand-in for tests.
- Rotate the regulator key outside the repository. The demo secret is `demo-regulator` and must not be used on a shared network.
- Do not disable the admission callback. A node without it will finalize any well-formed transfer.
- Treat sanctions updates as governance proposals with a 67% stake threshold. Do not add an admin key that can list or delist accounts by itself.
- WASM policies run with fuel. Reject modules that request host imports this runtime does not provide.
- Replay `Node::audit` after restore. It rechecks parent links, certificates, Merkle roots, and the state root.
- Fee base amounts are burned. Tips go to the proposer. Slashing for a double-sign is 5% of stake and jails the validator.
- Imported ISO 20022 messages are drafts. Settlement still requires screening and a certificate.
