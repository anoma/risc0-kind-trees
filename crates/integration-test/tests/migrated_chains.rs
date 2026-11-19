//! On-chain check of `migrated-chains.json`. The protocol adapter of a migrated chain denies the logic ref of the
//! legacy ERC20 forwarder for created resources, so that no conversion creates a legacy-forwarder resource; a fresh
//! one does not.

use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_pa_evm_bindings::contract::protocol_adapter;
use anoma_risc0_kind_trees::{Caip2ChainId, migrated};
use anoma_risc0_kind_trees_integration_test::provider;
use anomapay_erc20_forwarder_bindings::addresses::legacy_erc20_forwarder;
use anyhow::{Context, Result, ensure};

#[tokio::test]
async fn every_protocol_adapter_deprecates_the_legacy_logic_ref_if_and_only_if_its_chain_is_migrated()
-> Result<()> {
    for (environment, migrated) in [
        (Environment::Staging, &migrated::chains().staging),
        (Environment::Production, &migrated::chains().production),
    ] {
        for chain in protocol_adapter_deployments_map(environment).into_keys() {
            let Some(legacy) = legacy_erc20_forwarder(&chain) else {
                continue;
            };
            let id = Caip2ChainId::from(chain);
            let context = || format!("{environment:?} {chain}");
            let provider = provider(&id)?;
            let adapter = protocol_adapter(&provider, environment)
                .await
                .with_context(context)?;
            let deprecated = adapter
                .isLogicRefDenied(legacy.logic_ref, false)
                .call()
                .await
                .with_context(context)?;
            ensure!(
                deprecated == migrated.contains(&id),
                "{}: migrated-chains.json lists the chain: {}, the protocol adapter deprecates {}: {deprecated}",
                context(),
                migrated.contains(&id),
                legacy.logic_ref
            );
        }
    }
    Ok(())
}
