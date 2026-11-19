use crate::chain::Caip2ChainId;
use risc0_zkvm::Digest;
use solana_cluster_type::ClusterType;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid tokens JSON for {chain}: {reason}")]
    InvalidTokenSection {
        chain: Caip2ChainId,
        reason: serde_json::Error,
    },
    #[error("the token {token} is listed twice on {chain}")]
    DuplicateToken { chain: Caip2ChainId, token: String },
    #[error("the kind ({logic_ref}, {label_ref}) is listed twice")]
    DuplicateKind {
        logic_ref: Digest,
        label_ref: Digest,
    },
    #[error("the kind base of ({logic_ref}, {label_ref}) is not an uncompressed secp256k1 point")]
    InvalidKindBase {
        logic_ref: Digest,
        label_ref: Digest,
    },
    #[error("the circuit version {0} is listed twice")]
    DuplicateCircuitVersion(semver::Version),
    #[error("the circuit versions {first} and {second} have the same logic ref {logic_ref}")]
    DuplicateCircuitLogicRef {
        logic_ref: Digest,
        first: semver::Version,
        second: semver::Version,
    },
    #[error("no circuit version is active")]
    NoActiveCircuitVersion,
    #[error(
        "the circuit versions {first} and {second} are both active, but exactly one version may be"
    )]
    TwoActiveCircuitVersions {
        first: semver::Version,
        second: semver::Version,
    },
    #[error(
        "the circuit version {active} is active, but the listed version {highest} is higher; an old version never becomes active again"
    )]
    ActiveCircuitVersionNotHighest {
        active: semver::Version,
        highest: semver::Version,
    },
    #[error("invalid CAIP-2 chain ID {value:?}: {reason}")]
    InvalidCaip2ChainId { value: String, reason: &'static str },
    #[error("no chain known for {0}")]
    UnknownChain(Caip2ChainId),
    #[error("no chain known for the Solana cluster {0:?}")]
    UnknownSolanaCluster(ClusterType),
    #[error("no tree recorded for {0}")]
    UnrecordedChain(Caip2ChainId),
}
