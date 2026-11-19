use anoma_risc0_kind_trees::circuits::*;

#[test]
fn the_embedded_list_passes_its_own_checks() {
    check_erc20().unwrap();
    assert_eq!(erc20_active().status, Status::Active);
}

#[test]
fn the_embedded_spl_token_list_passes_its_own_checks() {
    check_spl_token().unwrap();
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
