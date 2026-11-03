use anoma_pa_solana_client::addresses::{
    Environment, protocol_adapter_address, protocol_adapter_deployments_map,
};
use anoma_risc0_kind_trees::chain::{Caip2ChainId, SolanaCluster};
use anoma_risc0_kind_trees::tree::{production, staging};
use anomapay_spl_token_forwarder_client::addresses::{
    spl_token_forwarder_address, spl_token_forwarder_deployments_map,
};

/// The Solana programs the trees are built for, as the clients' deployment records hold them: staging records the
/// devnet adapter and SPL token forwarder, production records nothing on mainnet-beta.
#[test]
fn staging_records_the_devnet_programs_and_production_none() {
    let devnet = SolanaCluster::Devnet.into();
    let adapter = protocol_adapter_address(Environment::Staging, &devnet);
    let forwarder = spl_token_forwarder_address(Environment::Staging, &devnet);
    println!("staging devnet: adapter {adapter:?}, forwarder {forwarder:?}");
    assert_eq!(
        adapter.map(|address| address.to_string()).as_deref(),
        Some("2gH5DnTwDLLFk3oYxnbrceSFdTMtvaxQz29x5W8Etmpx")
    );
    assert_eq!(
        forwarder.map(|address| address.to_string()).as_deref(),
        Some("34kd782wMwzkdwGFS5jfGGdTAz8zbaxVczQHs3E8oKF4")
    );

    let mainnet_beta = SolanaCluster::MainnetBeta.into();
    assert_eq!(
        protocol_adapter_address(Environment::Production, &mainnet_beta),
        None
    );
    assert_eq!(
        spl_token_forwarder_address(Environment::Production, &mainnet_beta),
        None
    );
    assert!(protocol_adapter_deployments_map(Environment::Production).is_empty());
    assert!(spl_token_forwarder_deployments_map(Environment::Production).is_empty());
}

/// Each environment's Solana trees are those of the clusters its adapter record holds: no tree for a cluster
/// without a recorded adapter, and no recorded adapter without a tree.
#[test]
fn the_solana_trees_are_the_recorded_adapters_clusters() {
    for (environment, chains) in [
        (Environment::Staging, staging::chains()),
        (Environment::Production, production::chains()),
    ] {
        let trees: Vec<Caip2ChainId> = chains
            .into_iter()
            .filter(|chain| chain.namespace() == Caip2ChainId::SOLANA)
            .collect();
        let mut recorded: Vec<Caip2ChainId> = protocol_adapter_deployments_map(environment)
            .into_iter()
            .map(|(cluster, _)| {
                Caip2ChainId::from(
                    SolanaCluster::try_from(cluster)
                        .unwrap_or_else(|error| panic!("{environment:?}: {error}")),
                )
            })
            .collect();
        recorded.sort();
        println!("{environment:?}: trees {trees:?}, recorded adapters {recorded:?}");
        assert_eq!(trees, recorded, "{environment:?}");
    }
}
