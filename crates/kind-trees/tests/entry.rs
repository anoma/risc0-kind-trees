use alloy::primitives::Address;
use anoma_risc0_kind_trees::circuits::Status;
use anoma_risc0_kind_trees::entry::*;
use anoma_risc0_kind_trees::{Tree, kind};
use hex::FromHex;
use risc0_zkvm::Digest;
use std::collections::BTreeSet;

fn entry() -> Entry {
    let logic_ref =
        Digest::from_hex("898f3d23ccad1ec7f07051100973815ce3687870416bc96e823a7aeaa347c367")
            .unwrap();
    let label_ref = Digest::default();
    let kind_base = kind::point(&logic_ref, &label_ref).unwrap();
    Entry {
        metadata: None,
        logic_ref,
        label_ref,
        kind_base,
    }
}

fn erc20_metadata() -> Metadata {
    Metadata::Erc20 {
        version: "2.0.0".into(),
        name: "WETH".into(),
        token: Address::with_last_byte(6),
        forwarder: Address::with_last_byte(7),
        status: Status::Active,
        alias_of: None,
    }
}

/// The active kind of `entry()`, as the target of an alias.
fn alias_of() -> AliasOf {
    let active = entry();
    AliasOf {
        version: "2.0.0".into(),
        logic_ref: active.logic_ref,
        label_ref: active.label_ref,
    }
}

#[test]
fn is_alias_rejects_a_derived_point() {
    assert!(!entry().is_alias());
}

#[test]
fn is_alias_accepts_a_point_of_another_key() {
    let mut alias = entry();
    alias.label_ref = Digest::from([1u32; 8]);
    assert!(alias.is_alias());
}

/// A kind base that is not the kind's own point is an alias, so the review list shows a malformed one.
#[test]
fn is_alias_holds_for_a_truncated_kind_base() {
    let mut malformed = entry();
    malformed.kind_base.pop();
    assert!(malformed.is_alias());
}

/// The resource machine sorts its leaves in this order.
#[test]
fn key_orders_by_the_logic_ref_then_by_the_label_ref() {
    let (low, high) = (Digest::from_bytes([1; 32]), Digest::from_bytes([2; 32]));
    let kind = |logic_ref, label_ref| Entry {
        logic_ref,
        label_ref,
        ..entry()
    };
    assert!(kind(low, high).key() < kind(high, low).key());
    assert!(kind(low, low).key() < kind(low, high).key());
}

#[test]
fn the_json_representation_round_trips() {
    let mut entry = entry();
    entry.metadata = Some(erc20_metadata());
    let json = serde_json::to_string(&entry).unwrap();
    assert_eq!(serde_json::from_str::<Entry>(&json).unwrap(), entry);
}

/// The upstream loader reads the three fields and nothing else.
#[test]
fn an_entry_without_metadata_is_written_as_its_kind_and_kind_base_only() {
    let json = serde_json::to_value(entry()).unwrap();
    let fields: BTreeSet<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from(["logic_ref", "label_ref", "kind_base"])
    );
    assert_eq!(serde_json::from_value::<Entry>(json).unwrap(), entry());
}

#[test]
fn an_entry_with_a_logic_ref_that_is_not_32_bytes_does_not_parse() {
    let mut json = serde_json::to_value(entry()).unwrap();
    json["logic_ref"] = hex::encode(&entry().logic_ref.as_bytes()[1..]).into();
    assert!(serde_json::from_value::<Entry>(json).is_err());
}

#[test]
fn metadata_of_an_unknown_type_does_not_parse() {
    let mut json = serde_json::to_value(erc20_metadata()).unwrap();
    json["type"] = "ERC721Resource".into();
    assert!(serde_json::from_value::<Metadata>(json).is_err());
}

/// Addresses parse in any letter case and are written with the EIP-55 checksum.
#[test]
fn erc20_metadata_round_trips_with_checksummed_addresses() {
    let usdc = "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238";
    let metadata = Metadata::Erc20 {
        version: "1.0.0".into(),
        name: "USDC".into(),
        token: usdc.to_lowercase().parse().unwrap(),
        forwarder: Address::with_last_byte(7),
        status: Status::Deprecated,
        alias_of: Some(alias_of()),
    };
    let json = serde_json::to_value(&metadata).unwrap();
    assert_eq!(json["type"], "ERC20Resource");
    assert_eq!(json["token"], usdc);
    assert_eq!(json["status"], "deprecated");
    assert_eq!(json["alias_of"]["logic_ref"], entry().logic_ref.to_string());
    assert_eq!(serde_json::from_value::<Metadata>(json).unwrap(), metadata);
}

#[test]
fn the_metadata_stays_out_of_the_root_input() {
    let bare = entry();
    let mut annotated = entry();
    annotated.metadata = Some(erc20_metadata());
    let root = |entry| Tree::new(vec![entry]).unwrap().root();
    assert_eq!(root(bare), root(annotated));
}

#[test]
fn spl_token_metadata_round_trips_with_base58_addresses() {
    let metadata = Metadata::SplToken {
        version: "2.0.0-rc.1".to_string(),
        name: "AnomaPay devnet test token".to_string(),
        mint: "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
            .parse()
            .unwrap(),
        forwarder: "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
            .parse()
            .unwrap(),
        status: Status::Active,
        alias_of: None,
    };
    let json = serde_json::to_value(&metadata).unwrap();
    assert_eq!(json["type"], "SPLTokenResource");
    assert_eq!(json["mint"], "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr");
    assert_eq!(
        json["forwarder"],
        "5CrHbBeHjg53UyL3Htn9dCYYTy68fMcrbDoeAdo4yQrx"
    );
    assert_eq!(json.get("alias_of"), None);
    assert_eq!(serde_json::from_value::<Metadata>(json).unwrap(), metadata);
}

/// Without the rule, the metadata of a misspelled `_metadata` would be dropped.
#[test]
fn an_entry_with_an_unknown_field_does_not_parse() {
    let mut json = serde_json::to_value(entry()).unwrap();
    json["metadata"] = serde_json::to_value(erc20_metadata()).unwrap();
    let error = serde_json::from_value::<Entry>(json).unwrap_err();
    assert!(error.to_string().contains("unknown field `metadata`"));
}

/// Without the rule, an alias with a misspelled `alias_of` would read as the active kind.
#[test]
fn metadata_with_an_unknown_field_does_not_parse() {
    let mut json = serde_json::to_value(erc20_metadata()).unwrap();
    json["aliasof"] = serde_json::to_value(alias_of()).unwrap();
    let error = serde_json::from_value::<Metadata>(json).unwrap_err();
    assert!(error.to_string().contains("unknown field `aliasof`"));
}

#[test]
fn an_alias_of_with_an_unknown_field_does_not_parse() {
    let mut json = serde_json::to_value(alias_of()).unwrap();
    json["status"] = "active".into();
    let error = serde_json::from_value::<AliasOf>(json).unwrap_err();
    assert!(error.to_string().contains("unknown field `status`"));
}
