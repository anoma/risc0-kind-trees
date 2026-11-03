use anoma_risc0_kind_trees::SolanaCluster;
use anoma_risc0_kind_trees::deployments::*;

#[test]
fn devnet_records_the_v2_programs_and_mainnet_beta_none() {
    let devnet = solana_deployment(SolanaCluster::Devnet).unwrap();
    assert_eq!(
        devnet.adapter.to_string(),
        "2gH5DnTwDLLFk3oYxnbrceSFdTMtvaxQz29x5W8Etmpx"
    );
    assert_eq!(
        devnet.forwarder.to_string(),
        "34kd782wMwzkdwGFS5jfGGdTAz8zbaxVczQHs3E8oKF4"
    );
    assert_eq!(solana_deployment(SolanaCluster::MainnetBeta), None);
}
