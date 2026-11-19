mod common;

use anoma_risc0_kind_trees::{AliasOf, Entry, Metadata, SolanaAddress, SplToken, Status, kind};
use anoma_risc0_kind_trees_generator::spl_token_entries;
use common::{circuits, deprecated_and_active};

/// The member has the shape of an ERC20 member, under the SPL token label and with the mint in its metadata.
#[test]
fn spl_token_entries_makes_each_deprecated_version_an_alias_under_the_mint_label() {
    let circuits = circuits();
    let (deprecated, active) = deprecated_and_active(&circuits);
    let forwarder = SolanaAddress::new([0xf0; 32]);
    let token = SplToken {
        symbol: "TEST".into(),
        name: "Test token".into(),
        decimals: 9,
        mint: SolanaAddress::new([1; 32]),
        precompute_kind_base: false,
    };
    let label_ref = kind::spl_token_label_ref(&forwarder, &token.mint);

    assert_eq!(
        spl_token_entries(std::slice::from_ref(&token), &circuits, forwarder).unwrap(),
        [Entry {
            metadata: Some(Metadata::SplToken {
                version: deprecated.version.clone(),
                name: token.symbol,
                mint: token.mint,
                forwarder,
                status: Status::Deprecated,
                alias_of: Some(AliasOf {
                    version: active.version.clone(),
                    logic_ref: active.logic_ref,
                    label_ref,
                }),
            }),
            logic_ref: deprecated.logic_ref,
            label_ref,
            kind_base: kind::point(&active.logic_ref, &label_ref),
        }]
    );
}
