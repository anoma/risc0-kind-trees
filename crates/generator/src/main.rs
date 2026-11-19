//! Regenerates `crates/kind-trees/data/generated/` from the authored inputs (`tokens.json`,
//! `circuit-versions.json`) and the pinned dependencies (the generic-call logic ref, the forwarder and protocol
//! adapter deployment records). CI reruns this and fails on any diff, so the committed artifacts always match the pins.

use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_pa_solana_client::addresses::Environment as SolanaEnvironment;
use anoma_risc0_kind_trees::{
    Caip2ChainId, Chain, Entry, SolanaAddress, SolanaCluster, Tree, circuits, tokens,
};
use anoma_risc0_kind_trees_generator::{self as generator, GenericCallCircuit, LegacyForwarder};
use anomapay_erc20_forwarder_bindings::addresses::Environment as Erc20Environment;
use anyhow::{Context, Result, ensure};
use cargo_metadata::{CargoOpt, MetadataCommand};
use risc0_zkvm::Digest;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One entry of `roots.json`: the chain it belongs to, named for review, and the tree root.
#[derive(Serialize)]
struct ChainRoot {
    #[serde(rename = "_comment")]
    comment: String,
    root: String,
}

/// The legacy ERC20 forwarder a chain records, from the forwarder repository's deployment record, where its fork
/// tests verify it against the chain.
fn legacy_erc20_forwarder(chain: &NamedChain) -> Option<LegacyForwarder> {
    anomapay_erc20_forwarder_bindings::addresses::legacy_erc20_forwarder(chain).map(|forwarder| {
        LegacyForwarder {
            address: forwarder.address,
            logic_ref: Digest::from_bytes(forwarder.logic_ref.0),
        }
    })
}

/// The protocol adapter's environment is the generator's; each forwarder crate declares its own.
fn erc20_environment(environment: Environment) -> Erc20Environment {
    match environment {
        Environment::Staging => Erc20Environment::Staging,
        Environment::Production => Erc20Environment::Production,
    }
}

fn generic_call_environment(environment: Environment) -> GenericCallEnvironment {
    match environment {
        Environment::Staging => GenericCallEnvironment::Staging,
        Environment::Production => GenericCallEnvironment::Production,
    }
}

/// The Solana programs' clients share one environment, the adapter client's.
fn solana_environment(environment: Environment) -> SolanaEnvironment {
    match environment {
        Environment::Staging => SolanaEnvironment::Staging,
        Environment::Production => SolanaEnvironment::Production,
    }
}

/// The circuit version behind the generic-call logic ref, and that of the pinned ERC20 crate.
struct Versions {
    anomapay_erc20: String,
    generic_call: String,
}

/// Reads the circuit versions from the resolved dependency graph, so bumping a pin cannot leave a stale
/// version in the metadata.
fn versions() -> Result<Versions> {
    let metadata = MetadataCommand::new()
        .manifest_path(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .features(CargoOpt::AllFeatures)
        .other_options(vec!["--locked".to_string()])
        .exec()
        .context("cargo metadata")?;
    let resolve = metadata
        .resolve
        .as_ref()
        .context("cargo metadata reported no resolve graph")?;
    let root = resolve
        .root
        .as_ref()
        .context("cargo metadata names no root package")?;
    let version_of = |lib: &str| {
        resolve[root]
            .deps
            .iter()
            .find(|dep| dep.name == lib)
            .map(|dep| metadata[&dep.pkg].version.to_string())
            .with_context(|| format!("{lib} is not a resolved dependency"))
    };
    Ok(Versions {
        anomapay_erc20: version_of("anomapay_erc20_library")?,
        generic_call: version_of("anoma_generic_call_library")?,
    })
}

/// Each list must pass `circuits::check`, and the pinned ERC20 circuit crate must be listed with the logic ref it
/// compiles to. Raising the pin without listing the release stops here, and so does a mistyped logic ref.
fn check_circuit_versions(versions: &Versions) -> Result<()> {
    circuits::check(circuits::erc20()).context("circuit-versions.json: ERC20Resource")?;
    circuits::check(circuits::spl_token()).context("circuit-versions.json: SPLTokenResource")?;
    let compiled = *anomapay_erc20_library::ANOMAPAY_ERC20_CIRCUIT_ID;
    ensure!(
        circuits::erc20_version(&compiled).is_some(),
        "the pinned ERC20Resource circuit crate {} compiles to logic ref {compiled}, which circuit-versions.json does not list",
        versions.anomapay_erc20
    );
    Ok(())
}

/// A Solana cluster's entries. As on EVM chains, the padding kind is not listed. Solana has no generic call forwarder,
/// so no generic call entry.
fn solana_entries(environment: Environment, cluster: SolanaCluster) -> Result<Vec<Entry>> {
    let supported = tokens::spl_on(cluster);
    match anomapay_spl_token_forwarder_client::addresses::spl_token_forwarder_address(
        solana_environment(environment),
        &cluster.into(),
    ) {
        Some(forwarder) => generator::spl_token_entries(
            supported,
            circuits::spl_token(),
            SolanaAddress::new(forwarder.to_bytes()),
        ),
        None if supported.is_empty() => Ok(Vec::new()),
        None => {
            eprintln!(
                "{}: {} supported mints but no SPL token forwarder recorded - no token kinds emitted",
                cluster.name(),
                supported.len()
            );
            Ok(Vec::new())
        }
    }
}

/// An EVM chain's entries. The padding kind is not listed: its resources never convert and need no precomputed kind
/// base, and its logic ref would tie every tree to the arm-risc0 release.
fn chain_entries(
    environment: Environment,
    chain: NamedChain,
    generic_call: &GenericCallCircuit,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(
            generic_call_environment(environment),
            &chain,
        )
    {
        entries.push(generator::generic_call_entry(forwarder, generic_call));
    }

    let supported = tokens::on(chain);
    match anomapay_erc20_forwarder_bindings::addresses::erc20_forwarder_address(
        erc20_environment(environment),
        &chain,
    ) {
        Some(forwarder) => entries.extend(generator::erc20_entries(
            supported,
            circuits::erc20(),
            forwarder,
            legacy_erc20_forwarder(&chain).as_ref(),
        )?),
        None if supported.is_empty() => {}
        None => eprintln!(
            "{chain}: {} supported tokens but no ERC20 forwarder recorded - no token kinds emitted",
            supported.len()
        ),
    }
    Ok(entries)
}

fn main() -> Result<()> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kind-trees/data");

    let versions = versions()?;
    check_circuit_versions(&versions)?;
    let generic_call = GenericCallCircuit {
        logic_ref: *anoma_generic_call_library::GENERIC_CALL_ID,
        version: versions.generic_call,
    };

    // Everything is generated before anything is written: a rejected input must not leave the tree without
    // the trees the crate embeds.
    let mut generated = Vec::new();
    for (environment, name) in [
        (Environment::Staging, "staging"),
        (Environment::Production, "production"),
    ] {
        let mut chains: Vec<Chain> = protocol_adapter_deployments_map(environment)
            .into_keys()
            .map(Chain::Evm)
            .collect();
        for (cluster, _) in anoma_pa_solana_client::addresses::protocol_adapter_deployments_map(
            solana_environment(environment),
        ) {
            chains.push(Chain::Solana(cluster.try_into()?));
        }
        chains.sort();

        let mut files = Vec::new();
        let mut roots = BTreeMap::new();
        for chain in chains {
            let entries = match chain {
                Chain::Evm(chain) => chain_entries(environment, chain, &generic_call),
                Chain::Solana(cluster) => solana_entries(environment, cluster),
            };
            let tree = entries
                .and_then(|entries| Ok(Tree::new(entries)?))
                .with_context(|| format!("{name} {chain}"))?;
            let id = Caip2ChainId::from(chain);
            files.push((
                id.file_name(),
                serde_json::to_string_pretty(tree.entries())? + "\n",
            ));
            roots.insert(
                id,
                ChainRoot {
                    comment: chain.to_string(),
                    root: tree.root().to_string(),
                },
            );
        }
        files.push((
            "roots.json".to_string(),
            serde_json::to_string_pretty(&roots)? + "\n",
        ));
        generated.push((name, files, roots.len()));
    }

    for (name, files, chains) in generated {
        let out = data.join("generated").join(name);
        if out.exists() {
            fs::remove_dir_all(&out)?;
        }
        fs::create_dir_all(&out)?;
        for (file, contents) in files {
            fs::write(out.join(file), contents)?;
        }
        println!("{name}: {chains} chains");
    }
    Ok(())
}
