//! The promotion gate: every protocol adapter of the promoted environment stores the root this source
//! generates. Runs only on a pull request into `staging` or `main` (selected via `GITHUB_BASE_REF`, or
//! `PROMOTION_TARGET` for a local run); a tree change is deployed after merging into `next`, and the promotion
//! pull request turning green is the proof it happened.

use anoma_pa_evm_bindings::addresses::Environment;
use anoma_pa_evm_bindings::contract::protocol_adapter;
use anoma_pa_solana_client::addresses::Environment as SolanaEnvironment;
use anoma_pa_solana_client::error::BindingsError;
use anoma_risc0_kind_trees::{Caip2ChainId, Chain, tree};
use anoma_risc0_kind_trees_integration_test::{provider, solana_rpc};
use anyhow::{Context, Result, bail, ensure};
use risc0_zkvm::Digest;
use std::collections::BTreeMap;

fn promotion_target() -> Option<Environment> {
    let base = std::env::var("PROMOTION_TARGET")
        .or_else(|_| std::env::var("GITHUB_BASE_REF"))
        .ok()?;
    match base.as_str() {
        "staging" => Some(Environment::Staging),
        "main" => Some(Environment::Production),
        _ => None,
    }
}

fn solana_environment(environment: Environment) -> SolanaEnvironment {
    match environment {
        Environment::Staging => SolanaEnvironment::Staging,
        Environment::Production => SolanaEnvironment::Production,
    }
}

fn roots(environment: Environment) -> &'static BTreeMap<Caip2ChainId, Digest> {
    match environment {
        Environment::Staging => tree::staging::roots(),
        Environment::Production => tree::production::roots(),
    }
}

#[tokio::test]
async fn the_promoted_environment_stores_the_generated_roots() -> Result<()> {
    let Some(environment) = promotion_target() else {
        eprintln!("skipped: not a promotion pull request");
        return Ok(());
    };

    for (chain, expected) in roots(environment) {
        let stored = match Chain::try_from(chain)? {
            Chain::Evm(_) => {
                let provider = provider(chain)?;
                let adapter = protocol_adapter(&provider, environment)
                    .await
                    .with_context(|| {
                        format!("no {environment:?} protocol adapter recorded on {chain}")
                    })?;
                let stored = adapter
                    .getKindTreeRoot()
                    .call()
                    .await
                    .with_context(|| chain.to_string())?;
                Digest::try_from(stored.as_slice()).context("a root is 32 bytes")?
            }
            Chain::Solana(cluster) => {
                let rpc = solana_rpc(cluster);
                let adapter = anoma_pa_solana_client::contract::protocol_adapter(
                    &rpc,
                    solana_environment(environment),
                )
                .await
                .with_context(|| {
                    format!("no {environment:?} protocol adapter recorded on {chain}")
                })?;
                match adapter.get_kind_tree_root().await {
                    Err(BindingsError::AccountMissing { .. }) => {
                        bail!(
                            "{chain}: the adapter {} is not initialized",
                            adapter.address()
                        )
                    }
                    stored => Digest::from(stored.with_context(|| chain.to_string())?),
                }
            }
        };
        ensure!(
            stored == *expected,
            "{chain}: the protocol adapter stores {stored}, the source generates {expected}"
        );
    }
    Ok(())
}
