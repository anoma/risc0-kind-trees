mod common;

use alloy_chains::NamedChain;
use anoma_generic_call_forwarder_bindings::addresses::{
    Environment as GenericCallEnvironment, generic_call_forwarder_address,
};
use anoma_risc0_kind_trees::chain::{Chain, SolanaCluster};
use anoma_risc0_kind_trees::circuits::{self, CircuitVersion, Status};
use anoma_risc0_kind_trees::entry::Metadata;
use anoma_risc0_kind_trees::tree::*;
use anoma_risc0_kind_trees::{Caip2ChainId, Entry, Error, SolanaAddress};
use anoma_risc0_kind_trees::{kind, migrated, tokens};
use anomapay_erc20_forwarder_bindings::addresses::{
    Environment, erc20_forwarder_address, legacy_erc20_forwarder,
};
use anomapay_spl_token_forwarder_client::addresses::{
    Environment as SolanaEnvironment, spl_token_forwarder_address,
};
use common::{by_chain, own_kind};
use k256::EncodedPoint;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// A kind, written as its `(logic_ref, label_ref)`.
type Kind = (Digest, Digest);

/// Every embedded tree, with the environment whose records it is built from.
fn embedded_trees() -> Vec<(Environment, &'static Caip2ChainId, &'static Tree)> {
    let staging = staging::trees()
        .iter()
        .map(|(chain, tree)| (Environment::Staging, chain, tree));
    let production = production::trees()
        .iter()
        .map(|(chain, tree)| (Environment::Production, chain, tree));
    staging.chain(production).collect()
}

/// The generic call forwarder the environment records on the chain.
fn generic_call_forwarder(
    environment: Environment,
    chain: NamedChain,
) -> Option<alloy::primitives::Address> {
    let environment = match environment {
        Environment::Staging => GenericCallEnvironment::Staging,
        Environment::Production => GenericCallEnvironment::Production,
    };
    generic_call_forwarder_address(environment, &chain)
}

/// The chains on which the environment's protocol adapter copied the legacy state.
fn migrated_chains(environment: Environment) -> &'static BTreeSet<Caip2ChainId> {
    match environment {
        Environment::Staging => &migrated::chains().staging,
        Environment::Production => &migrated::chains().production,
    }
}

/// Adds a token's members under one label: every deprecated version, and the active version only if the token asks
/// for a precomputed kind base.
fn insert_members(
    kinds: &mut BTreeSet<Kind>,
    listed: &[CircuitVersion],
    label_ref: Digest,
    precompute_kind_base: bool,
) {
    for circuit in listed {
        if circuit.status == Status::Deprecated || precompute_kind_base {
            kinds.insert((circuit.logic_ref, label_ref));
        }
    }
}

/// The token kinds the generation rules call for on the chain, from the token list and the deployment records.
fn expected_token_kinds(environment: Environment, chain: Chain) -> BTreeSet<Kind> {
    let mut kinds = BTreeSet::new();
    match chain {
        Chain::Evm(named) => {
            let Some(forwarder) = erc20_forwarder_address(environment, &named) else {
                return kinds;
            };
            let legacy = legacy_erc20_forwarder(&named)
                .filter(|_| migrated_chains(environment).contains(&Caip2ChainId::from(named)));
            for token in tokens::on(named) {
                let label_ref = kind::erc20_label_ref(&forwarder, &token.address);
                insert_members(
                    &mut kinds,
                    circuits::erc20(),
                    label_ref,
                    token.precompute_kind_base,
                );
                if let Some(legacy) = legacy.filter(|_| token.soft_migrate_from_legacy_forwarder) {
                    let logic_ref = Digest::try_from(legacy.logic_ref.as_slice())
                        .expect("a logic ref is 32 bytes");
                    kinds.insert((
                        logic_ref,
                        kind::erc20_label_ref(&legacy.address, &token.address),
                    ));
                }
            }
        }
        Chain::Solana(cluster) => {
            let Some(forwarder) = spl_forwarder(environment, cluster) else {
                return kinds;
            };
            for token in tokens::spl_on(cluster) {
                insert_members(
                    &mut kinds,
                    circuits::spl_token(),
                    kind::spl_token_label_ref(&forwarder, &token.mint),
                    token.precompute_kind_base,
                );
            }
        }
    }
    kinds
}

/// The SPL token forwarder the environment records on the cluster, from the forwarder client's deployment record.
fn spl_forwarder(environment: Environment, cluster: SolanaCluster) -> Option<SolanaAddress> {
    let environment = match environment {
        Environment::Staging => SolanaEnvironment::Staging,
        Environment::Production => SolanaEnvironment::Production,
    };
    spl_token_forwarder_address(environment, &cluster.into())
        .map(|forwarder| SolanaAddress::new(forwarder.to_bytes()))
}

/// The alias rules of a fungibility domain, checked on one member: it is assigned the domain's kind base
/// (the kind of the active version under the label of the environment's forwarder), its version is in `listed`, it
/// is active if and only if it has no `alias_of`, and an `alias_of` names the active version under that label.
fn check_member(
    context: &str,
    entry: &Entry,
    listed: &[CircuitVersion],
    active: &CircuitVersion,
    active_label: Digest,
    status: Status,
) {
    let domain = kind::point(&active.logic_ref, &active_label);
    assert_eq!(
        entry.kind_base, domain,
        "{context}: a member is not assigned its fungibility domain's kind base"
    );
    assert!(
        listed
            .iter()
            .any(|circuit| circuit.logic_ref == entry.logic_ref),
        "{context}: a member of an unlisted version"
    );
    let alias_of = entry.metadata.as_ref().and_then(Metadata::alias_of);
    assert_eq!(
        status == Status::Active,
        alias_of.is_none(),
        "{context}: a member is active if and only if it has no alias_of"
    );
    match alias_of {
        None => {
            assert!(
                !entry.is_alias(),
                "{context}: an entry without alias_of is an alias"
            );
            assert_eq!(
                (entry.logic_ref, entry.label_ref),
                (active.logic_ref, active_label),
                "{context}: only the active version under the environment's forwarder has no alias_of"
            );
        }
        Some(alias_of) => {
            assert!(
                entry.is_alias(),
                "{context}: an entry with alias_of is not an alias"
            );
            assert_eq!(
                (&alias_of.version, alias_of.logic_ref, alias_of.label_ref),
                (&active.version.to_string(), active.logic_ref, active_label),
                "{context}: alias_of does not name the active version under the environment's forwarder"
            );
        }
    }
}

/// No kind base is authored. A token entry is assigned its fungibility domain's kind base — the kind of the
/// active version under the label of the environment's forwarder on the chain — and every alias says so in
/// `alias_of`. Every other entry is assigned its own kind. This reads the embedded trees only; the environment's
/// forwarder comes from the deployment records (the ERC20 forwarder bindings and the SPL token forwarder client),
/// never from the entries themselves, so a tree built for the wrong forwarder fails here.
#[test]
fn token_entries_share_their_fungibility_domains_kind_base() {
    for (module, environment, trees) in [
        ("staging", Environment::Staging, staging::trees()),
        ("production", Environment::Production, production::trees()),
    ] {
        for (chain, tree) in trees {
            let context = format!("{module} {chain}");
            let known = Chain::try_from(chain).unwrap_or_else(|error| panic!("{context}: {error}"));
            let (erc20_forwarder, spl_forwarder) = match known {
                Chain::Evm(named) => (erc20_forwarder_address(environment, &named), None),
                Chain::Solana(cluster) => (None, spl_forwarder(environment, cluster)),
            };
            for entry in tree.entries() {
                let (listed, active, active_label, status) = match &entry.metadata {
                    Some(Metadata::Erc20 { token, status, .. }) => {
                        let forwarder = erc20_forwarder.unwrap_or_else(|| {
                            panic!("{context}: an ERC20 entry but no ERC20 forwarder recorded")
                        });
                        (
                            circuits::erc20(),
                            circuits::erc20_active(),
                            kind::erc20_label_ref(&forwarder, token),
                            *status,
                        )
                    }
                    Some(Metadata::SplToken { mint, status, .. }) => {
                        let forwarder = spl_forwarder.unwrap_or_else(|| {
                            panic!(
                                "{context}: an SPL token entry but no SPL token forwarder recorded"
                            )
                        });
                        (
                            circuits::spl_token(),
                            circuits::spl_token_active(),
                            kind::spl_token_label_ref(&forwarder, mint),
                            *status,
                        )
                    }
                    Some(Metadata::GenericCall { .. }) | None => {
                        assert!(
                            !entry.is_alias(),
                            "{context}: an entry outside every fungibility domain is not assigned its own kind"
                        );
                        continue;
                    }
                };
                check_member(&context, entry, listed, active, active_label, status);
            }
        }
    }
}

/// Every kind besides the generic call kind is a token member the rules call for: no unsupported token, no
/// unrecorded forwarder, no legacy member of a token that does not soft-migrate or on a chain that did not migrate, and
/// no missing alias.
#[test]
fn every_tree_lists_exactly_the_token_kinds_the_token_list_and_the_records_call_for() {
    let mut checked = 0;
    for (environment, chain, tree) in embedded_trees() {
        let context = format!("{environment:?} {chain}");
        let known = Chain::try_from(chain).unwrap_or_else(|error| panic!("{context}: {error}"));
        let listed: BTreeSet<Kind> = tree
            .entries()
            .iter()
            .filter(|entry| !matches!(entry.metadata, Some(Metadata::GenericCall { .. })))
            .map(|entry| (entry.logic_ref, entry.label_ref))
            .collect();
        let expected = expected_token_kinds(environment, known);
        let missing: Vec<&Kind> = expected.difference(&listed).collect();
        let unexpected: Vec<&Kind> = listed.difference(&expected).collect();
        assert!(
            missing.is_empty() && unexpected.is_empty(),
            "{context}: missing {missing:?}, not called for {unexpected:?}"
        );
        checked += expected.len();
    }
    assert!(checked > 0, "the trees carry a token kind to check");
}

/// The generic call kind sits under the label of the recorded generic call forwarder, and Solana has none.
#[test]
fn every_tree_has_a_generic_call_entry_if_and_only_if_a_generic_call_forwarder_is_recorded() {
    for (environment, chain, tree) in embedded_trees() {
        let context = format!("{environment:?} {chain}");
        let recorded = match Chain::try_from(chain) {
            Ok(Chain::Evm(named)) => generic_call_forwarder(environment, named),
            Ok(Chain::Solana(_)) => None,
            Err(error) => panic!("{context}: {error}"),
        };
        let generic_calls: Vec<&Entry> = tree
            .entries()
            .iter()
            .filter(|entry| matches!(entry.metadata, Some(Metadata::GenericCall { .. })))
            .collect();
        match (recorded, generic_calls.as_slice()) {
            (None, []) => {}
            (Some(forwarder), [entry]) => {
                assert!(
                    matches!(entry.metadata, Some(Metadata::GenericCall { forwarder: named, .. }) if named == forwarder),
                    "{context}: the generic call entry names another forwarder than {forwarder}"
                );
                assert_eq!(
                    entry.label_ref,
                    *Impl::hash_bytes(forwarder.as_slice()),
                    "{context}: the generic call label is not sha256 of the recorded forwarder"
                );
            }
            (recorded, entries) => panic!(
                "{context}: {} generic call entries for the recorded forwarder {recorded:?}",
                entries.len()
            ),
        }
    }
}

/// An EVM consumer passes the chain it already has, so the lookups accept a `NamedChain`.
#[test]
fn tree_accepts_an_evm_chain() {
    let sepolia = Caip2ChainId::eip155(11_155_111);

    assert_eq!(
        staging::tree(NamedChain::Sepolia).unwrap(),
        staging::tree(sepolia.clone()).unwrap()
    );
    assert_eq!(
        staging::root(NamedChain::Sepolia).unwrap(),
        staging::root(sepolia).unwrap()
    );
    assert!(matches!(
        staging::tree(NamedChain::Mainnet),
        Err(Error::UnrecordedChain(chain)) if chain == Caip2ChainId::eip155(1)
    ));
}

/// The macro invocation lists the tree files by hand, so pin it to `roots.json`.
#[test]
fn embedded_trees_match_the_recorded_roots() {
    for (module, roots, trees) in [
        ("staging", staging::roots(), staging::trees()),
        ("production", production::roots(), production::trees()),
    ] {
        let recorded: Vec<&Caip2ChainId> = roots.keys().collect();
        let embedded: Vec<&Caip2ChainId> = trees.keys().collect();
        assert_eq!(recorded, embedded, "{module}: chains out of sync");
        for (chain, root) in roots {
            assert_eq!(
                trees[chain].root(),
                *root,
                "{module}: stale tree for {chain}"
            );
        }
    }
}

#[test]
fn root_errors_on_a_chain_the_environment_does_not_record() {
    assert!(matches!(
        staging::root(NamedChain::Mainnet),
        Err(Error::UnrecordedChain(chain)) if chain == Caip2ChainId::eip155(1)
    ));
}

#[test]
fn new_sorts_the_entries_by_key() {
    let (first, second) = (own_kind(1), own_kind(2));
    assert!(
        first.key() < second.key(),
        "premise: the first entry sorts first"
    );
    let tree = Tree::new(vec![second.clone(), first.clone()]).unwrap();
    assert_eq!(tree.entries(), [first, second]);
}

/// A kind listed twice with two kind bases would balance against two fungibility domains. The check compares kinds
/// only, so this also covers a kind listed twice with one kind base.
#[test]
fn new_rejects_a_kind_listed_twice() {
    let entry = own_kind(1);
    let other_base = Entry {
        kind_base: own_kind(2).kind_base,
        ..entry.clone()
    };
    assert_eq!(entry.key(), other_base.key(), "premise: one kind");
    assert!(matches!(
        Tree::new(vec![entry.clone(), own_kind(3), other_base]),
        Err(Error::DuplicateKind { logic_ref, label_ref })
            if (logic_ref, label_ref) == (entry.logic_ref, entry.label_ref)
    ));
}

#[test]
fn new_rejects_a_kind_base_that_is_not_an_uncompressed_curve_point() {
    let entry = own_kind(1);
    assert!(
        Tree::new(vec![entry.clone()]).is_ok(),
        "premise: the derived kind base passes"
    );
    let compressed = EncodedPoint::from_bytes(&entry.kind_base)
        .unwrap()
        .compress()
        .as_bytes()
        .to_vec();
    let mut other_tag = entry.kind_base.clone();
    other_tag[0] ^= 0xff;
    let mut off_curve = entry.kind_base.clone();
    off_curve[64] ^= 0xff;
    for (malformation, kind_base) in [
        ("the compressed encoding of the point", compressed),
        ("another SEC1 tag", other_tag),
        ("a y coordinate off the curve", off_curve),
    ] {
        let malformed = Entry {
            kind_base,
            ..entry.clone()
        };
        assert!(
            matches!(
                Tree::new(vec![own_kind(2), malformed]),
                Err(Error::InvalidKindBase { logic_ref, label_ref })
                    if (logic_ref, label_ref) == (entry.logic_ref, entry.label_ref)
            ),
            "{malformation}"
        );
    }
}

/// A tree file goes through the checks of `new`.
#[test]
fn from_json_rejects_a_kind_listed_twice() {
    let json = serde_json::to_string(&[own_kind(1), own_kind(1)]).unwrap();
    assert!(matches!(
        Tree::from_json(&json),
        Err(Error::DuplicateKind { .. })
    ));
}

/// A root in the format of `roots.json`, with the `_comment` that names its chain.
fn chain_root() -> serde_json::Value {
    serde_json::json!({ "_comment": "the chain name", "root": Digest::from_bytes([1; 32]).to_string() })
}

#[test]
fn roots_from_json_reads_the_root_of_each_chain() {
    let sepolia = Caip2ChainId::from(NamedChain::Sepolia);
    assert_eq!(
        roots_from_json(&by_chain(&[(&sepolia, &chain_root())])).unwrap(),
        BTreeMap::from([(sepolia, Digest::from_bytes([1; 32]))])
    );
}

#[test]
fn roots_from_json_rejects_a_chain_listed_twice() {
    let sepolia = Caip2ChainId::from(NamedChain::Sepolia);
    assert!(matches!(
        roots_from_json(&by_chain(&[(&sepolia, &chain_root()), (&sepolia, &chain_root())])),
        Err(Error::InvalidJson(error)) if error.to_string().contains("duplicate key")
    ));
}

#[test]
fn roots_from_json_rejects_a_chain_that_chain_does_not_know() {
    let unknown = Caip2ChainId::eip155(u64::MAX);
    assert!(matches!(
        roots_from_json(&by_chain(&[(&unknown, &chain_root())])),
        Err(Error::UnknownChain(chain)) if chain == unknown
    ));
}

#[test]
fn roots_from_json_rejects_an_unknown_field() {
    let sepolia = Caip2ChainId::from(NamedChain::Sepolia);
    let mut root = chain_root();
    root["block"] = 1.into();
    assert!(matches!(
        roots_from_json(&by_chain(&[(&sepolia, &root)])),
        Err(Error::InvalidJson(error)) if error.to_string().contains("unknown field `block`")
    ));
}
