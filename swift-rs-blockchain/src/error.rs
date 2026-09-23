use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChainError {
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("bad signature")]
    BadSignature,
    #[error("bad nonce")]
    BadNonce,
    #[error("unknown account")]
    UnknownAccount,
    #[error("duplicate transaction")]
    Duplicate,
    #[error("quorum not reached")]
    QuorumNotReached,
    #[error("empty batch")]
    EmptyBatch,
    #[error("no active proposer")]
    NoProposer,
    #[error("invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("invalid block: {0}")]
    InvalidBlock(String),
    #[error("admission rejected: {0}")]
    Rejected(String),
    #[error("wasm error: {0}")]
    Wasm(String),
    #[error("channel error: {0}")]
    Channel(String),
    #[error("shard aborted: {0}")]
    ShardAbort(String),
    #[error("rollup: {0}")]
    Rollup(String),
}
