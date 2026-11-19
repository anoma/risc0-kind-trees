use anoma_risc0_kind_trees::Error;
use anoma_risc0_kind_trees::circuits::*;
use risc0_zkvm::Digest;
use serde_json::{Value, json};

/// A circuit version whose logic ref differs from that of each version with another `seed`.
fn circuit(version: &str, seed: u8, status: Status) -> CircuitVersion {
    CircuitVersion {
        version: version.into(),
        logic_ref: Digest::from_bytes([seed; 32]),
        status,
    }
}

#[test]
fn the_embedded_list_passes_its_own_checks() {
    check(erc20()).unwrap();
    assert_eq!(erc20_active().status, Status::Active);
}

#[test]
fn the_embedded_spl_token_list_passes_its_own_checks() {
    check(spl_token()).unwrap();
    let active = spl_token_active();
    assert_eq!(active.status, Status::Active);
    assert_eq!(spl_token_version(&active.logic_ref), Some(active));
}

/// The generator looks up the legacy forwarder's logic ref this way and fails on `None`.
#[test]
fn erc20_version_finds_each_listed_logic_ref_and_none_of_the_other_list() {
    for circuit in erc20() {
        assert_eq!(erc20_version(&circuit.logic_ref), Some(circuit));
    }
    let spl_token = spl_token_active().logic_ref;
    assert!(
        erc20().iter().all(|circuit| circuit.logic_ref != spl_token),
        "premise: the lists share no logic ref"
    );
    assert_eq!(erc20_version(&spl_token), None);
}

#[test]
fn check_returns_the_active_version() {
    let list = [
        circuit("1.0.0", 1, Status::Deprecated),
        circuit("2.0.0", 2, Status::Active),
    ];
    assert_eq!(check(&list).unwrap(), &list[1]);
}

/// The SPL token list holds both versions.
#[test]
fn check_orders_the_versions_by_semantic_version_precedence() {
    let (deprecated, active) = ("2.0.0-rc.9", "2.0.0-rc.12");
    assert!(
        deprecated > active,
        "premise: as text, the deprecated version is higher"
    );
    let list = [
        circuit(deprecated, 1, Status::Deprecated),
        circuit(active, 2, Status::Active),
    ];
    assert_eq!(check(&list).unwrap(), &list[1]);
}

#[test]
fn check_rejects_a_version_listed_twice() {
    let list = [
        circuit("1.0.0", 1, Status::Deprecated),
        circuit("1.0.0", 2, Status::Deprecated),
        circuit("2.0.0", 3, Status::Active),
    ];
    assert!(matches!(
        check(&list),
        Err(Error::DuplicateCircuitVersion(version)) if version == "1.0.0"
    ));
}

#[test]
fn check_rejects_a_logic_ref_listed_twice() {
    let list = [
        circuit("1.0.0", 1, Status::Deprecated),
        circuit("2.0.0", 1, Status::Active),
    ];
    assert!(matches!(
        check(&list),
        Err(Error::DuplicateCircuitLogicRef { logic_ref, first, second })
            if logic_ref == list[0].logic_ref && first == "1.0.0" && second == "2.0.0"
    ));
}

#[test]
fn check_rejects_a_version_that_is_not_a_semantic_version() {
    let list = [circuit("v2.0.0", 1, Status::Active)];
    assert!(matches!(
        check(&list),
        Err(Error::InvalidCircuitVersion { version, .. }) if version == "v2.0.0"
    ));
}

#[test]
fn check_rejects_a_list_without_an_active_version() {
    let list = [circuit("1.0.0", 1, Status::Deprecated)];
    assert!(matches!(check(&list), Err(Error::NoActiveCircuitVersion)));
}

#[test]
fn check_rejects_a_second_active_version() {
    let list = [
        circuit("1.0.0", 1, Status::Active),
        circuit("2.0.0", 2, Status::Active),
    ];
    assert!(matches!(
        check(&list),
        Err(Error::TwoActiveCircuitVersions { first, second }) if first == "1.0.0" && second == "2.0.0"
    ));
}

#[test]
fn check_rejects_an_active_version_below_a_listed_version() {
    let list = [
        circuit("1.0.0", 1, Status::Active),
        circuit("2.0.0", 2, Status::Deprecated),
    ];
    assert!(matches!(
        check(&list),
        Err(Error::ActiveCircuitVersionNotHighest { active, highest })
            if active == "1.0.0" && highest == "2.0.0"
    ));
}

/// Lists in the format of `circuit-versions.json`, with a `_comment` at each place the file may carry one.
fn file() -> Value {
    json!({
        "_comment": "the circuit versions of each resource type",
        "ERC20Resource": [{
            "_comment": "the circuit that the legacy forwarders accept",
            "version": "1.0.0",
            "logic_ref": Digest::from_bytes([1; 32]).to_string(),
            "status": "active",
        }],
        "SPLTokenResource": [],
    })
}

#[test]
fn circuit_versions_from_json_reads_the_lists_and_skips_the_comments() {
    let versions = CircuitVersions::from_json(&file().to_string()).unwrap();
    assert_eq!(versions.erc20, [circuit("1.0.0", 1, Status::Active)]);
    assert_eq!(versions.spl_token, []);
}

#[test]
fn circuit_versions_from_json_rejects_an_unknown_field_of_a_version() {
    let mut file = file();
    file["ERC20Resource"][0]["denied"] = true.into();
    assert!(matches!(
        CircuitVersions::from_json(&file.to_string()),
        Err(Error::InvalidCircuitVersionsJson(error)) if error.to_string().contains("unknown field `denied`")
    ));
}

/// A list that the crate does not read would give no entries.
#[test]
fn circuit_versions_from_json_rejects_an_unknown_list() {
    let mut file = file();
    file["GenericCallResource"] = json!([]);
    assert!(matches!(
        CircuitVersions::from_json(&file.to_string()),
        Err(Error::InvalidCircuitVersionsJson(error))
            if error.to_string().contains("unknown field `GenericCallResource`")
    ));
}
