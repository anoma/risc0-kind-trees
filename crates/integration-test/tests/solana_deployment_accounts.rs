//! The recorded Solana deployments' accounts decode through the clients' instances: the adapter's state through
//! anoma-pa-solana-client and the SPL token forwarder's config through anomapay-spl-token-forwarder-client. The
//! promotion gates read both; this runs on every pull request and push, so a layout change on chain fails here first.

use anoma_pa_solana_client::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_pa_solana_client::contract::protocol_adapter;
use anoma_pa_solana_client::error::BindingsError as AdapterError;
use anoma_risc0_kind_trees::SolanaCluster;
use anoma_risc0_kind_trees_integration_test::solana_rpc;
use anomapay_spl_token_forwarder_client::contract::spl_token_forwarder;
use anomapay_spl_token_forwarder_client::error::BindingsError as ForwarderError;
use anyhow::{Context, Result, bail};
use risc0_zkvm::Digest;

#[tokio::test]
async fn the_recorded_solana_deployments_accounts_decode() -> Result<()> {
    let mut read = 0;
    for environment in [Environment::Staging, Environment::Production] {
        for (cluster, _) in protocol_adapter_deployments_map(environment) {
            let cluster = SolanaCluster::try_from(cluster)?;
            let context = format!("{environment:?} {}", cluster.name());
            let rpc = solana_rpc(cluster);

            let adapter = protocol_adapter(&rpc, environment)
                .await
                .with_context(|| context.clone())?;
            let root = match adapter.get_kind_tree_root().await {
                Err(AdapterError::AccountMissing { .. }) => {
                    bail!(
                        "{context}: the adapter {} is not initialized",
                        adapter.address()
                    )
                }
                root => Digest::from(root.with_context(|| context.clone())?),
            };

            let forwarder = spl_token_forwarder(&rpc, environment)
                .await
                .with_context(|| context.clone())?;
            let logic_ref = match forwarder.get_logic_ref().await {
                Err(ForwarderError::AccountMissing { .. }) => {
                    bail!(
                        "{context}: the forwarder {} is not initialized",
                        forwarder.address()
                    )
                }
                logic_ref => Digest::from(logic_ref.with_context(|| context.clone())?),
            };

            eprintln!(
                "{context}: adapter {} stores {root}, forwarder {} accepts {logic_ref}",
                adapter.address(),
                forwarder.address()
            );
            read += 1;
        }
    }
    assert!(read > 0, "the records hold a Solana deployment to read");
    Ok(())
}
