mod common;

use alloy::primitives::Address;
use anoma_risc0_kind_trees::{
    AliasOf, CircuitVersion, Entry, Error, Metadata, Status, Token, kind,
};
use anoma_risc0_kind_trees_generator::{LegacyForwarder, erc20_entries};
use common::{circuits, deprecated_and_active};
use risc0_zkvm::Digest;
use std::collections::BTreeSet;

fn forwarder() -> Address {
    Address::with_last_byte(0xf0)
}

fn token(soft_migrate: bool, precompute: bool) -> Token {
    Token {
        symbol: "USDC".into(),
        name: "USD Coin".into(),
        decimals: 6,
        address: Address::with_last_byte(1),
        soft_migrate_from_legacy_forwarder: soft_migrate,
        precompute_kind_base: precompute,
    }
}

/// A legacy forwarder accepts the logic ref of a deprecated version.
fn legacy() -> LegacyForwarder {
    LegacyForwarder {
        address: Address::with_last_byte(0xe0),
        logic_ref: deprecated_and_active(&circuits()).0.logic_ref,
    }
}

#[test]
fn erc20_entries_makes_each_deprecated_version_an_alias_of_the_active_version() {
    let circuits = circuits();
    let (deprecated, active) = deprecated_and_active(&circuits);
    let token = token(false, false);
    let label_ref = kind::erc20_label_ref(&forwarder(), &token.address);

    assert_eq!(
        erc20_entries(std::slice::from_ref(&token), &circuits, forwarder(), None).unwrap(),
        [Entry {
            metadata: Some(Metadata::Erc20 {
                version: deprecated.version.clone(),
                name: token.symbol,
                token: token.address,
                forwarder: forwarder(),
                status: Status::Deprecated,
                alias_of: Some(AliasOf {
                    version: active.version.clone(),
                    logic_ref: active.logic_ref,
                    label_ref,
                }),
            }),
            logic_ref: deprecated.logic_ref,
            label_ref,
            kind_base: kind::point(&active.logic_ref, &label_ref).unwrap(),
        }]
    );
}

#[test]
fn erc20_entries_lists_the_active_version_if_the_token_asks_for_a_precomputed_kind_base() {
    let circuits = circuits();
    let (_, active) = deprecated_and_active(&circuits);
    let entries = erc20_entries(&[token(false, true)], &circuits, forwarder(), None).unwrap();

    assert_eq!(entries.len(), circuits.len(), "one entry for each version");
    let entry = entries
        .iter()
        .find(|entry| entry.logic_ref == active.logic_ref)
        .expect("the active version has an entry");
    assert!(!entry.is_alias(), "the active version keeps its own kind");
    assert!(matches!(
        entry.metadata,
        Some(Metadata::Erc20 {
            status: Status::Active,
            alias_of: None,
            ..
        })
    ));
}

#[test]
fn erc20_entries_makes_one_fungibility_domain_for_each_token() {
    let weth = Token {
        symbol: "WETH".into(),
        address: Address::with_last_byte(2),
        ..token(false, false)
    };
    let entries =
        erc20_entries(&[token(false, false), weth], &circuits(), forwarder(), None).unwrap();

    let kind_bases: BTreeSet<&Vec<u8>> = entries.iter().map(|entry| &entry.kind_base).collect();
    assert_eq!(entries.len(), 2, "one alias for each token");
    assert_eq!(kind_bases.len(), 2, "each token has its own kind base");
}

/// The member makes the legacy-forwarder resources of the token fungible with the resources of the environment's
/// forwarder.
#[test]
fn erc20_entries_adds_the_legacy_member_only_for_a_token_that_soft_migrates() {
    let circuits = circuits();
    let (_, active) = deprecated_and_active(&circuits);
    let legacy = legacy();
    let address = token(false, false).address;
    let legacy_label = kind::erc20_label_ref(&legacy.address, &address);
    let domain_kind_base = kind::point(
        &active.logic_ref,
        &kind::erc20_label_ref(&forwarder(), &address),
    )
    .unwrap();

    let staying = erc20_entries(
        &[token(false, false)],
        &circuits,
        forwarder(),
        Some(&legacy),
    )
    .unwrap();
    assert!(
        staying.iter().all(|entry| entry.label_ref != legacy_label),
        "no member under the legacy label"
    );

    let migrating =
        erc20_entries(&[token(true, false)], &circuits, forwarder(), Some(&legacy)).unwrap();
    let member = migrating
        .iter()
        .find(|entry| entry.label_ref == legacy_label)
        .expect("a member under the legacy label");
    assert_eq!(member.logic_ref, legacy.logic_ref);
    assert_eq!(
        member.kind_base, domain_kind_base,
        "an alias of the token's domain"
    );
    assert!(matches!(
        &member.metadata,
        Some(Metadata::Erc20 { forwarder, status: Status::Deprecated, .. }) if *forwarder == legacy.address
    ));
}

#[test]
fn erc20_entries_adds_no_legacy_member_on_a_chain_without_a_legacy_forwarder() {
    let circuits = circuits();
    assert_eq!(
        erc20_entries(&[token(true, false)], &circuits, forwarder(), None).unwrap(),
        erc20_entries(&[token(false, false)], &circuits, forwarder(), None).unwrap()
    );
}

#[test]
fn erc20_entries_errors_on_a_legacy_logic_ref_that_no_listed_version_has() {
    let circuits = circuits();
    let unlisted = LegacyForwarder {
        logic_ref: Digest::from_bytes([3; 32]),
        ..legacy()
    };
    assert!(
        circuits
            .iter()
            .all(|circuit| circuit.logic_ref != unlisted.logic_ref),
        "premise: no listed version has the logic ref"
    );

    let error = erc20_entries(
        &[token(true, false)],
        &circuits,
        forwarder(),
        Some(&unlisted),
    )
    .unwrap_err();
    assert!(error.to_string().contains(&unlisted.logic_ref.to_string()));
}

/// Two active versions is a list that still has an active version to build a fungibility domain from.
#[test]
fn erc20_entries_errors_on_a_list_that_fails_the_circuit_checks() {
    let two_active: Vec<_> = circuits()
        .into_iter()
        .map(|circuit| CircuitVersion {
            status: Status::Active,
            ..circuit
        })
        .collect();
    let error = erc20_entries(&[token(false, false)], &two_active, forwarder(), None).unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(Error::TwoActiveCircuitVersions { .. })
    ));
}
