use anoma_risc0_kind_trees::{Caip2ChainId, Error, MigratedChains};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Lists in the format of `migrated-chains.json`, with one chain in both environments and the `_comment`.
fn file() -> Value {
    json!({
        "_comment": "the migrated chains of each environment",
        "staging": ["eip155:143", "eip155:11155111"],
        "production": ["eip155:1", "eip155:143"],
    })
}

fn chains(ids: &[u64]) -> BTreeSet<Caip2ChainId> {
    ids.iter().copied().map(Caip2ChainId::eip155).collect()
}

#[test]
fn migrated_chains_from_json_reads_the_lists_of_both_environments() {
    let migrated = MigratedChains::from_json(&file().to_string()).unwrap();
    assert_eq!(migrated.staging, chains(&[143, 11_155_111]));
    assert_eq!(migrated.production, chains(&[1, 143]));
}

/// A plain JSON reader would fold the second listing into the first.
#[test]
fn migrated_chains_from_json_rejects_a_chain_listed_twice_in_one_environment() {
    let mut file = file();
    file["staging"] = json!(["eip155:143", "eip155:143"]);
    let error = MigratedChains::from_json(&file.to_string()).unwrap_err();
    assert!(matches!(error, Error::InvalidJson(_)), "{error}");
    assert!(error.to_string().contains("duplicate"), "{error}");
}

#[test]
fn migrated_chains_from_json_rejects_an_unknown_chain() {
    let unknown = Caip2ChainId::eip155(u64::MAX);
    let mut file = file();
    file["production"] = json!([unknown.to_string()]);
    assert!(matches!(
        MigratedChains::from_json(&file.to_string()),
        Err(Error::UnknownChain(chain)) if chain == unknown
    ));
}

/// An environment that the crate does not read would migrate no chain.
#[test]
fn migrated_chains_from_json_rejects_an_unknown_environment() {
    let mut file = file();
    file["development"] = json!([]);
    assert!(matches!(
        MigratedChains::from_json(&file.to_string()),
        Err(Error::InvalidJson(error)) if error.to_string().contains("unknown field `development`")
    ));
}
