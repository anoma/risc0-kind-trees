//! The rules that make the entries of a kind tree. The caller reads the inputs: the supported tokens, the circuit
//! versions, the migrated chains and the deployment records.

use alloy::primitives::Address;
use anoma_risc0_kind_trees::{
    AliasOf, CircuitVersion, Entry, Metadata, SolanaAddress, SplToken, Status, Token, circuits,
    kind,
};
use anyhow::{Context, Result};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

/// The legacy ERC20 forwarder of a chain: the immutable forwarder of its legacy protocol adapter.
pub struct LegacyForwarder {
    pub address: Address,
    /// The logic ref it accepts, the only one its resources carry.
    pub logic_ref: Digest,
}

/// The generic call circuit: its logic ref and the version of the crate that ships it.
pub struct GenericCallCircuit {
    pub logic_ref: Digest,
    pub version: String,
}

/// The ERC20 entries of a chain: one fungibility domain for each token under the label of `forwarder`, the
/// forwarder of the environment. For a token that soft-migrates, the logic ref of `legacy` under the label of the
/// legacy forwarder is one more member; the caller passes `legacy` only for a migrated chain. Errors if `circuits`
/// fails `circuits::check`.
pub fn erc20_entries(
    tokens: &[Token],
    circuits: &[CircuitVersion],
    forwarder: Address,
    legacy: Option<&LegacyForwarder>,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for token in tokens {
        let metadata = |forwarder: Address| {
            move |version, status, alias_of| Metadata::Erc20 {
                version,
                name: token.symbol.clone(),
                token: token.address,
                forwarder,
                status,
                alias_of,
            }
        };
        let (point, active_kind) = domain(
            circuits,
            kind::erc20_label_ref(&forwarder, &token.address),
            token.precompute_kind_base,
            metadata(forwarder),
            &mut entries,
        )?;
        if token.soft_migrate_from_legacy_forwarder
            && let Some(legacy) = legacy
        {
            let circuit = circuits
                .iter()
                .find(|circuit| circuit.logic_ref == legacy.logic_ref)
                .with_context(|| {
                    format!(
                        "the legacy ERC20 forwarder {} accepts logic ref {}, which no listed ERC20 circuit version has",
                        legacy.address, legacy.logic_ref
                    )
                })?;
            entries.push(member(
                circuit,
                kind::erc20_label_ref(&legacy.address, &token.address),
                &point,
                Some(active_kind),
                metadata(legacy.address),
            ));
        }
    }
    Ok(entries)
}

/// The SPL token entries of a Solana cluster: one fungibility domain for each mint under the label of `forwarder`,
/// the SPL token forwarder program of the environment. Errors if `circuits` fails `circuits::check`.
pub fn spl_token_entries(
    tokens: &[SplToken],
    circuits: &[CircuitVersion],
    forwarder: SolanaAddress,
) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for token in tokens {
        domain(
            circuits,
            kind::spl_token_label_ref(&forwarder, &token.mint),
            token.precompute_kind_base,
            |version, status, alias_of| Metadata::SplToken {
                version,
                name: token.symbol.clone(),
                mint: token.mint,
                forwarder,
                status,
                alias_of,
            },
            &mut entries,
        )?;
    }
    Ok(entries)
}

/// The generic call entry of `forwarder`. It is assigned its own kind, so it makes no alias.
pub fn generic_call_entry(forwarder: Address, circuit: &GenericCallCircuit) -> Entry {
    let label_ref = *Impl::hash_bytes(forwarder.as_slice());
    Entry {
        metadata: Some(Metadata::GenericCall {
            version: circuit.version.clone(),
            forwarder,
        }),
        kind_base: kind::point(&circuit.logic_ref, &label_ref),
        logic_ref: circuit.logic_ref,
        label_ref,
    }
}

/// One token's fungibility domain under `label_ref`, the label of the environment's forwarder: every deprecated
/// circuit version as an alias of the active version under that label, and the active version itself, assigned its
/// own kind, only if `precompute` is set for the token. Returns the domain's kind base and the kind every alias
/// names, for members added under another label.
fn domain(
    circuits: &[CircuitVersion],
    label_ref: Digest,
    precompute: bool,
    metadata: impl Fn(String, Status, Option<AliasOf>) -> Metadata,
    entries: &mut Vec<Entry>,
) -> Result<(Vec<u8>, AliasOf)> {
    let active = circuits::check(circuits)?;
    let point = kind::point(&active.logic_ref, &label_ref);
    let active_kind = AliasOf {
        version: active.version.to_string(),
        logic_ref: active.logic_ref,
        label_ref,
    };
    for circuit in circuits {
        let alias_of = (circuit.status == Status::Deprecated).then(|| active_kind.clone());
        if alias_of.is_none() && !precompute {
            continue;
        }
        entries.push(member(circuit, label_ref, &point, alias_of, &metadata));
    }
    Ok((point, active_kind))
}

/// A member of a token's fungibility domain: one circuit version under one forwarder's label, assigned the
/// fungibility domain's kind base. `alias_of` names the kind that point is; only the active version under the
/// label of the environment's forwarder carries none, and only that member is active. `metadata` names the token
/// and forwarder the member belongs to, given the member's version, status and `alias_of`.
fn member(
    circuit: &CircuitVersion,
    label_ref: Digest,
    domain_kind_base: &[u8],
    alias_of: Option<AliasOf>,
    metadata: impl FnOnce(String, Status, Option<AliasOf>) -> Metadata,
) -> Entry {
    let status = if alias_of.is_none() {
        Status::Active
    } else {
        Status::Deprecated
    };
    Entry {
        metadata: Some(metadata(circuit.version.to_string(), status, alias_of)),
        logic_ref: circuit.logic_ref,
        label_ref,
        kind_base: domain_kind_base.to_vec(),
    }
}
