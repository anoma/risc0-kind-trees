//! Regenerates `crates/kind-trees/data/generated/` from the authored inputs (`tokens.json`,
//! `circuit-versions.json`) and the pinned dependencies (the generic-call logic ref, the forwarder and protocol
//! adapter deployment records). CI reruns this and fails on any diff, so the committed artifacts always match the pins.

use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::Environment as GenericCallEnvironment;
use anoma_pa_evm_bindings::addresses::{Environment, protocol_adapter_deployments_map};
use anoma_pa_solana_client::addresses::Environment as SolanaEnvironment;
use anoma_risc0_kind_trees::{
    Caip2ChainId, Chain, Entry, SolanaAddress, SolanaCluster, circuits, tokens,
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
            logic_ref: digest(forwarder.logic_ref.as_slice()),
        }
    })
}

fn digest(bytes: &[u8]) -> Digest {
    Digest::try_from(bytes).expect("a logic ref is 32 bytes")
}

fn hex(digest: &Digest) -> String {
    hex::encode(digest.as_bytes())
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

/// The ERC20 circuit crate this generator pins, the active version's, with the logic ref it compiles to. A release
/// replaces the pin. Another listed version is checked only by a deployment record that names its logic ref, as the
/// legacy forwarders' records name the legacy version's.
fn pinned_erc20_circuits(versions: &Versions) -> Vec<(String, Digest)> {
    vec![(
        versions.anomapay_erc20.clone(),
        digest(anomapay_erc20_library::ANOMAPAY_ERC20_CIRCUIT_ID.as_bytes()),
    )]
}

/// Each list must pass `circuits::check`, and every pinned circuit crate must be listed with the logic ref it
/// compiles to. Raising a pin without listing the release stops here, and so does a mistyped logic ref.
fn check_circuit_versions(versions: &Versions) -> Result<()> {
    for (resource, listed, pinned) in [
        (
            "ERC20Resource",
            circuits::erc20(),
            pinned_erc20_circuits(versions),
        ),
        ("SPLTokenResource", circuits::spl_token(), Vec::new()),
    ] {
        circuits::check(listed).with_context(|| format!("circuit-versions.json: {resource}"))?;
        for (version, compiled) in pinned {
            ensure!(
                listed.iter().any(|circuit| circuit.logic_ref == compiled),
                "a {resource} circuit crate at {version} is pinned and compiles to logic ref {}, which circuit-versions.json does not list",
                hex(&compiled)
            );
        }
    }
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
    versions: &Versions,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    if let Some(forwarder) =
        anoma_generic_call_forwarder_bindings::addresses::generic_call_forwarder_address(
            generic_call_environment(environment),
            &chain,
        )
    {
        entries.push(generator::generic_call_entry(
            forwarder,
            &GenericCallCircuit {
                logic_ref: digest(anoma_generic_call_library::GENERIC_CALL_ID.as_bytes()),
                version: versions.generic_call.clone(),
            },
        )?);
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

    // Everything is generated before anything is written: a rejected input must not leave the tree without
    // the trees the crate embeds.
    let mut generated = Vec::new();
    for (environment, name) in [
        (Environment::Staging, "staging"),
        (Environment::Production, "production"),
    ] {
        let mut chains: Vec<_> = protocol_adapter_deployments_map(environment)
            .into_keys()
            .collect();
        chains.sort();

        let mut trees = Vec::new();
        for chain in chains {
            trees.push((
                Chain::Evm(chain),
                chain_entries(environment, chain, &versions)
                    .with_context(|| format!("{name} {chain}"))?,
            ));
        }
        let mut clusters = anoma_pa_solana_client::addresses::protocol_adapter_deployments_map(
            solana_environment(environment),
        )
        .into_iter()
        .map(|(cluster, _)| SolanaCluster::try_from(cluster))
        .collect::<anoma_risc0_kind_trees::Result<Vec<_>>>()?;
        clusters.sort();
        for cluster in clusters {
            trees.push((
                Chain::Solana(cluster),
                solana_entries(environment, cluster)
                    .with_context(|| format!("{name} {}", cluster.name()))?,
            ));
        }

        let mut files = Vec::new();
        let mut roots = BTreeMap::new();
        for (chain, entries) in trees {
            let tree = generator::tree(entries).with_context(|| format!("{name} {chain}"))?;
            let id = Caip2ChainId::from(chain);
            files.push((
                id.file_name(),
                serde_json::to_string_pretty(tree.entries())? + "\n",
            ));
            roots.insert(
                id,
                ChainRoot {
                    comment: chain.to_string(),
                    root: hex::encode(tree.root().as_bytes()),
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
