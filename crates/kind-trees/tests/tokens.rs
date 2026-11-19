use alloy::primitives::Address;
use anoma_risc0_kind_trees::tokens::*;
use anoma_risc0_kind_trees::{Caip2ChainId, Error, SolanaCluster};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn sepolia() -> Caip2ChainId {
    "eip155:11155111".parse().unwrap()
}

fn devnet() -> Caip2ChainId {
    SolanaCluster::Devnet.into()
}

fn erc20_token(address: &str) -> Value {
    json!({
        "symbol": "TEST",
        "name": "Test token",
        "decimals": 6,
        "address": address,
        "precompute_kind_base": true,
        "soft_migrate_from_legacy_forwarder": true,
    })
}

fn spl_token(mint: &str) -> Value {
    json!({
        "symbol": "TEST",
        "name": "Test token",
        "decimals": 6,
        "mint": mint,
        "precompute_kind_base": true,
    })
}

/// A section with the `_comment` that names its chain.
fn section(tokens: Vec<Value>) -> Value {
    json!({ "_comment": "the chain name", "tokens": tokens })
}

/// One section for each chain, in the given order. A chain may come twice, which `json!` cannot write.
fn list(sections: &[(&Caip2ChainId, &Value)]) -> String {
    let sections: Vec<String> = sections
        .iter()
        .map(|(chain, section)| format!("{:?}: {section}", chain.to_string()))
        .collect();
    format!("{{{}}}", sections.join(", "))
}

#[test]
fn the_embedded_list_carries_the_devnet_test_mint_under_its_caip2_key() {
    let [token] = spl_on(SolanaCluster::Devnet) else {
        panic!("expected exactly one supported mint on solana-devnet");
    };
    assert_eq!(
        token.mint.to_string(),
        "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr"
    );
    assert_eq!(token.decimals, 6);
    assert!(
        on(SolanaCluster::Devnet).is_empty(),
        "a Solana chain has no ERC20 tokens"
    );
}

/// A token has one fungibility domain on a chain; addresses compare as bytes, so letter case does not hide a repeat.
#[test]
fn no_chain_lists_a_token_twice() {
    for (chain, supported) in all() {
        let mut seen = BTreeSet::new();
        for token in supported {
            assert!(
                seen.insert(token.address),
                "{chain}: {} ({}) is listed twice",
                token.symbol,
                token.address
            );
        }
    }
    for (cluster, supported) in spl_all() {
        let mut seen = BTreeSet::new();
        for token in supported {
            assert!(
                seen.insert(token.mint),
                "{}: {} ({}) is listed twice",
                cluster.name(),
                token.symbol,
                token.mint
            );
        }
    }
}

#[test]
fn on_lists_no_token_for_a_chain_without_a_section() {
    let unnamed = Caip2ChainId::eip155(u64::MAX);
    assert!(!all().contains_key(&unnamed), "premise: no section");
    assert!(on(unnamed).is_empty());
}

#[test]
fn from_json_reads_each_section_as_the_token_type_of_its_chain_family() {
    let address = Address::repeat_byte(0xab).to_string();
    let mint = "9EHEFzyuY7sZEzTVm7C3uMkNZFMgm5ZeWjGjirZ3MVfr";
    let tokens = Tokens::from_json(&list(&[
        (&sepolia(), &section(vec![erc20_token(&address)])),
        (&devnet(), &section(vec![spl_token(mint)])),
    ]))
    .unwrap();
    assert_eq!(tokens.erc20[&sepolia()][0].address.to_string(), address);
    assert_eq!(tokens.spl[&SolanaCluster::Devnet][0].mint.to_string(), mint);
}

#[test]
fn from_json_rejects_a_chain_listed_twice() {
    let empty = section(Vec::new());
    assert!(Tokens::from_json(&list(&[(&sepolia(), &empty)])).is_ok());
    assert!(matches!(
        Tokens::from_json(&list(&[(&sepolia(), &empty), (&sepolia(), &empty)])),
        Err(Error::InvalidTokensJson(error)) if error.to_string().contains("duplicate key")
    ));
}

#[test]
fn from_json_rejects_a_chain_that_chain_does_not_know() {
    let unknown = Caip2ChainId::eip155(u64::MAX);
    assert!(matches!(
        Tokens::from_json(&list(&[(&unknown, &section(Vec::new()))])),
        Err(Error::UnknownChain(chain)) if chain == unknown
    ));
}
