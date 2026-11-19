use anoma_risc0_kind_trees::tokens::*;
use anoma_risc0_kind_trees::{Caip2ChainId, SolanaCluster};
use std::collections::BTreeSet;

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
