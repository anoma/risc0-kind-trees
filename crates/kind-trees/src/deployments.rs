//! The Solana deployment record: the protocol adapter each recorded cluster's tree is installed on, and the SPL
//! token forwarder program whose label the SPL token members carry. Devnet runs the V2 programs; mainnet-beta has
//! no V2 deployment.
use crate::chain::SolanaCluster;
use crate::solana::SolanaAddress;

/// A cluster's V2 deployment: the protocol adapter program and the SPL token forwarder program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolanaDeployment {
    pub adapter: SolanaAddress,
    pub forwarder: SolanaAddress,
}

/// The V2 deployment recorded for the cluster, if there is one.
pub fn solana_deployment(cluster: SolanaCluster) -> Option<SolanaDeployment> {
    match cluster {
        SolanaCluster::Devnet => Some(SolanaDeployment {
            adapter: address("2gH5DnTwDLLFk3oYxnbrceSFdTMtvaxQz29x5W8Etmpx"),
            forwarder: address("34kd782wMwzkdwGFS5jfGGdTAz8zbaxVczQHs3E8oKF4"),
        }),
        SolanaCluster::MainnetBeta => None,
    }
}

fn address(base58: &str) -> SolanaAddress {
    base58
        .parse()
        .unwrap_or_else(|error| panic!("a recorded program id: {error}"))
}
