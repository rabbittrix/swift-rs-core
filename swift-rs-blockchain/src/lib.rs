//! Permissioned ledger for cross-border settlement.
//!
//! Validators are identified central-bank keys. A block is final when signed
//! stake reaches 67%. Account admission is an explicit callback so a payment
//! cannot be finalized unless the application accepts it.

pub mod block;
pub mod channel;
pub mod consensus;
pub mod error;
pub mod hashutil;
pub mod keys;
pub mod ledger;
pub mod merkle;
pub mod node;
pub mod pilot;
pub mod pq;
pub mod rollup;
pub mod shard;
pub mod tx;
pub mod types;
pub mod wasm;

pub use block::{Block, BlockHeader, QuorumCertificate, Vote};
pub use consensus::{Consensus, SlashReason, Validator};
pub use error::ChainError;
pub use keys::KeyPair;
pub use ledger::Ledger;
pub use node::{GenesisAllocation, Node};
pub use tx::{SignedTransaction, TxBody};
pub use types::{Address, AssetId, Hash};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{close_txs, open_funding_txs, resolve_dispute};
    use crate::merkle::{merkle_proof, verify_proof};
    use crate::shard::ShardLayout;
    use crate::types::meets_quorum;
    use crate::wasm::{quorum_met, within_limit};
    use std::sync::Arc;

    fn validator(seed: &str, stake: u128) -> Validator {
        let key = KeyPair::from_seed(seed.as_bytes());
        Validator {
            address: key.address,
            stake,
            verifying_key: key.verifying,
            signing_key: Some(key),
            jailed: false,
        }
    }

    fn sealed_node(base_fee: u128) -> (Node, KeyPair, KeyPair) {
        let alice = KeyPair::from_seed(b"alice");
        let bob = KeyPair::from_seed(b"bob");
        let vault = KeyPair::from_seed(b"vault");
        let validators = vec![
            validator("v1", 25),
            validator("v2", 25),
            validator("v3", 25),
            validator("v4", 25),
        ];
        let mut node = Node::new(Consensus::new(validators), base_fee);
        node.register(&alice.verifying).unwrap();
        node.register(&bob.verifying).unwrap();
        node.register(&vault.verifying).unwrap();
        let validator_keys: Vec<_> = node
            .consensus
            .validators
            .iter()
            .map(|validator| validator.verifying_key)
            .collect();
        for key in validator_keys {
            node.register(&key).unwrap();
        }
        let asset = AssetId::new("cbdc:brl").unwrap();
        node.seal_genesis(
            1_700_000_000_000,
            vec![
                GenesisAllocation {
                    to: alice.address,
                    asset: asset.clone(),
                    amount: 10_000,
                },
                GenesisAllocation {
                    to: vault.address,
                    asset: asset.clone(),
                    amount: 5_000,
                },
            ],
        )
        .unwrap();
        (node, alice, bob)
    }

    #[test]
    fn quorum_threshold_matches_spec() {
        assert!(meets_quorum(67, 100));
        assert!(!meets_quorum(66, 100));
        assert!(meets_quorum(3, 4));
        assert!(!meets_quorum(2, 4));
    }

    #[test]
    fn three_of_four_finalizes_and_two_of_four_does_not() {
        let (mut node, alice, bob) = sealed_node(1);
        let asset = AssetId::new("cbdc:brl").unwrap();
        let tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: asset.clone(),
                amount: 100,
                fee: 1,
                nonce: 0,
                purpose: "trade".into(),
                timestamp_ms: 1_700_000_000_100,
            },
            &alice,
        )
        .unwrap();
        let block = node
            .finalize_batch(vec![tx], 1_700_000_000_100)
            .unwrap();
        assert_eq!(block.header.height, 1);
        assert!(block.certificate.is_some());
        assert_eq!(node.ledger.balance(&bob.address, &asset), 100);
        assert_eq!(node.ledger.balance(&alice.address, &asset), 9_899);
        assert!(node.ledger.conserved());
        node.audit().unwrap();

        let first = node.consensus.validators[0].address;
        let second = node.consensus.validators[1].address;
        node.consensus.jail(&first, true);
        node.consensus.jail(&second, true);
        let tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset,
                amount: 10,
                fee: 1,
                nonce: 1,
                purpose: "trade".into(),
                timestamp_ms: 1_700_000_000_200,
            },
            &alice,
        )
        .unwrap();
        let err = node
            .finalize_batch(vec![tx], 1_700_000_000_200)
            .unwrap_err();
        assert_eq!(err, ChainError::QuorumNotReached);
    }

    #[test]
    fn admission_callback_blocks_finality() {
        let (mut node, alice, bob) = sealed_node(1);
        node.admission = Some(Arc::new(|body: &TxBody| {
            if body.purpose == "blocked" {
                Err("sanctions".into())
            } else {
                Ok(())
            }
        }));
        let tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: AssetId::new("cbdc:brl").unwrap(),
                amount: 5,
                fee: 1,
                nonce: 0,
                purpose: "blocked".into(),
                timestamp_ms: 1_700_000_000_100,
            },
            &alice,
        )
        .unwrap();
        let err = node
            .finalize_batch(vec![tx], 1_700_000_000_100)
            .unwrap_err();
        assert!(matches!(err, ChainError::Rejected(_)));
        assert_eq!(node.height(), 0);
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let (mut node, alice, bob) = sealed_node(1);
        let mut tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: AssetId::new("cbdc:brl").unwrap(),
                amount: 5,
                fee: 1,
                nonce: 0,
                purpose: "trade".into(),
                timestamp_ms: 1_700_000_000_100,
            },
            &alice,
        )
        .unwrap();
        tx.signature[0] ^= 0xff;
        let err = node
            .finalize_batch(vec![tx], 1_700_000_000_100)
            .unwrap_err();
        assert_eq!(err, ChainError::BadSignature);
    }

    #[test]
    fn merkle_inclusion_and_fee_burn() {
        let (mut node, alice, bob) = sealed_node(2);
        let asset = AssetId::new("cbdc:brl").unwrap();
        let tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: asset.clone(),
                amount: 50,
                fee: 5,
                nonce: 0,
                purpose: "trade".into(),
                timestamp_ms: 1_700_000_000_100,
            },
            &alice,
        )
        .unwrap();
        let id = tx.body.id().unwrap();
        let block = node.finalize_batch(vec![tx], 1_700_000_000_100).unwrap();
        let proof = merkle_proof(&[id], 0).unwrap();
        assert!(verify_proof(&id, &proof, &block.header.merkle_root));
        assert_eq!(node.ledger.burned.get("cbdc:brl").copied(), Some(2));
        let proposer = block.header.proposer;
        assert_eq!(node.ledger.balance(&proposer, &asset), 3);
    }

    #[test]
    fn double_sign_is_detected() {
        let key = KeyPair::from_seed(b"equiv");
        let consensus = Consensus::new(vec![Validator {
            address: key.address,
            stake: 10,
            verifying_key: key.verifying,
            signing_key: Some(key.clone()),
            jailed: false,
        }]);
        let left = Hash::sha256(b"left");
        let right = Hash::sha256(b"right");
        let first = crate::block::Vote {
            voter: key.address,
            signature: key.sign(&crate::block::vote_message(1, &left)),
        };
        let second = crate::block::Vote {
            voter: key.address,
            signature: key.sign(&crate::block::vote_message(1, &right)),
        };
        assert_eq!(
            consensus
                .detect_double_sign(1, &first, &left, &second, &right)
                .unwrap(),
            key.address
        );
    }

    #[test]
    fn channel_dispute_picks_higher_nonce_and_settles() {
        let (mut node, alice, bob) = sealed_node(1);
        let vault = KeyPair::from_seed(b"vault");
        let asset = AssetId::new("cbdc:brl").unwrap();
        let (mut channel, funding) = open_funding_txs(
            &vault.address,
            &alice,
            &bob,
            &asset,
            40,
            0,
            0,
            0,
            1,
            1_700_000_000_100,
        )
        .unwrap();
        node.finalize_batch(funding, 1_700_000_000_100).unwrap();
        channel.bal_a = 15;
        channel.bal_b = 25;
        channel.nonce = 1;
        let older = {
            let mut stale = channel.clone();
            stale.bal_a = 40;
            stale.bal_b = 0;
            stale.nonce = 0;
            stale.sign_state(&alice, &bob).unwrap()
        };
        let latest = channel.sign_state(&alice, &bob).unwrap();
        let winner = resolve_dispute(&older, &latest, &alice, &bob).unwrap();
        assert_eq!(winner.nonce, 1);
        channel.bal_a = winner.bal_a;
        channel.bal_b = winner.bal_b;
        channel.open = true;
        let closing = close_txs(&channel, &vault, 0, 1, 1_700_000_000_200).unwrap();
        node.finalize_batch(closing, 1_700_000_000_200).unwrap();
        assert_eq!(node.ledger.balance(&alice.address, &asset), 15 + (10_000 - 40 - 1));
        node.audit().unwrap();
    }

    #[test]
    fn cross_shard_swap_aborts_without_partial_debit() {
        let mut layout = ShardLayout::new(8);
        let alice = KeyPair::from_seed(b"shard-alice");
        let bob = KeyPair::from_seed(b"shard-bob");
        let brl = AssetId::new("cbdc:brl").unwrap();
        let cny = AssetId::new("cbdc:cny").unwrap();
        let a = layout.shard_index(&brl);
        let b = layout.shard_index(&cny);
        layout.shards[a].credit(&alice.address, &brl, 30).unwrap();
        layout.shards[b].credit(&bob.address, &cny, 5).unwrap();
        let err = layout
            .atomic_swap(a, b, &alice.address, &bob.address, &brl, &cny, 10, 9)
            .unwrap_err();
        assert!(matches!(err, ChainError::ShardAbort(_)));
        assert_eq!(layout.shards[a].balance(&alice.address, &brl), 30);
        assert_eq!(layout.shards[b].balance(&bob.address, &cny), 5);
        layout
            .atomic_swap(a, b, &alice.address, &bob.address, &brl, &cny, 10, 4)
            .unwrap();
        assert_eq!(layout.shards[a].balance(&alice.address, &brl), 20);
        assert_eq!(layout.shards[b].balance(&bob.address, &brl), 10);
    }

    #[test]
    fn wasm_policy_agrees_with_integer_quorum() {
        assert_eq!(within_limit(1, 2).unwrap(), true);
        assert_eq!(quorum_met(3, 4).unwrap(), meets_quorum(3, 4));
    }

    #[test]
    fn future_timestamp_cannot_bypass_the_block_clock() {
        let (mut node, alice, bob) = sealed_node(1);
        let tx = SignedTransaction::sign(
            TxBody {
                from: alice.address,
                to: bob.address,
                asset: AssetId::new("cbdc:brl").unwrap(),
                amount: 5,
                fee: 1,
                nonce: 0,
                purpose: "trade".into(),
                timestamp_ms: 1_700_000_000_500,
            },
            &alice,
        )
        .unwrap();
        let err = node
            .finalize_batch(vec![tx], 1_700_000_000_100)
            .unwrap_err();
        assert!(matches!(err, ChainError::InvalidTransaction(_)));
    }

    #[test]
    fn batch_settlement_exceeds_one_thousand_tps() {
        let alice = KeyPair::from_seed(b"tps-alice");
        let bob = KeyPair::from_seed(b"tps-bob");
        let validators = vec![
            validator("tps-v1", 25),
            validator("tps-v2", 25),
            validator("tps-v3", 25),
            validator("tps-v4", 25),
        ];
        let mut node = Node::new(Consensus::new(validators), 1);
        node.register(&alice.verifying).unwrap();
        node.register(&bob.verifying).unwrap();
        let validator_keys: Vec<_> = node
            .consensus
            .validators
            .iter()
            .map(|item| item.verifying_key)
            .collect();
        for key in validator_keys {
            node.register(&key).unwrap();
        }
        let count = 2_000u64;
        let asset = AssetId::new("cbdc:brl").unwrap();
        node.seal_genesis(
            1_000,
            vec![GenesisAllocation {
                to: alice.address,
                asset: asset.clone(),
                amount: u128::from(count) * 2,
            }],
        )
        .unwrap();
        let txs = (0..count)
            .map(|nonce| {
                SignedTransaction::sign(
                    TxBody {
                        from: alice.address,
                        to: bob.address,
                        asset: asset.clone(),
                        amount: 1,
                        fee: 1,
                        nonce,
                        purpose: "trade".into(),
                        timestamp_ms: 2_000,
                    },
                    &alice,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let started = std::time::Instant::now();
        node.finalize_batch(txs, 2_000).unwrap();
        let elapsed = started.elapsed().as_secs_f64().max(0.000_001);
        let tps = count as f64 / elapsed;
        // Throughput is measured on optimized builds; debug is far slower (no inlining).
        if cfg!(debug_assertions) {
            assert!(
                tps >= 80.0,
                "debug batch throughput was {tps:.0} tx/s; run `cargo test -p swift-rs-blockchain batch_settlement --release` for the 1000+ gate"
            );
        } else {
            assert!(
                tps >= 1_000.0,
                "batch throughput was {tps:.0} tx/s, below 1000"
            );
        }
    }
}
