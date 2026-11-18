mod common;

use alloy::primitives::Address;
use anoma_risc0_kind_trees::{Error, Token};
use anoma_risc0_kind_trees_generator::{erc20_entries, tree};
use common::circuits;

/// A token listed twice in `tokens.json` makes each of its kinds twice.
#[test]
fn tree_names_the_token_of_a_kind_listed_twice() {
    let token = Token {
        symbol: "USDC".into(),
        name: "USD Coin".into(),
        decimals: 6,
        address: Address::with_last_byte(1),
        soft_migrate_from_legacy_forwarder: false,
        precompute_kind_base: false,
    };
    let entries = |tokens: &[Token]| {
        erc20_entries(tokens, &circuits(), Address::with_last_byte(0xf0), None).unwrap()
    };
    assert!(
        tree(entries(std::slice::from_ref(&token))).is_ok(),
        "premise: the token listed once makes a tree"
    );

    let error = tree(entries(&[token.clone(), token.clone()])).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<Error>(),
        Some(Error::DuplicateKind { .. })
    ));
    let message = format!("{error:#}");
    assert!(
        message.contains(&format!("{} {}", token.symbol, token.address)),
        "{message}"
    );
}
