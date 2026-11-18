mod common;

use anoma_risc0_kind_trees::Entry;
use anoma_risc0_kind_trees::root::*;
use common::own_kind;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

#[test]
fn of_is_independent_of_the_entry_order() {
    let (a, b) = (own_kind(1), own_kind(2));
    assert_eq!(of(&[a.clone(), b.clone()]), of(&[b, a]));
}

#[test]
fn of_gives_a_one_entry_tree_its_leaf() {
    let a = own_kind(1);
    let leaf = [
        a.logic_ref.as_bytes(),
        a.label_ref.as_bytes(),
        a.kind_base.as_slice(),
    ]
    .concat();
    assert_eq!(of(std::slice::from_ref(&a)), *Impl::hash_bytes(&leaf));
}

#[test]
fn of_gives_the_empty_tree_the_padding_leaf() {
    assert_eq!(of(&[]), *Impl::hash_bytes(b"EMPTY"));
}

/// Three leaves are the smallest count that needs a padding leaf at a level below the root.
#[test]
fn of_pads_the_leaves_to_a_power_of_two_with_the_padding_leaf() {
    let entries = [own_kind(1), own_kind(2), own_kind(3)];
    assert!(
        entries.windows(2).all(|pair| pair[0].key() < pair[1].key()),
        "premise: the entries are in key order"
    );
    let leaf = |entry: &Entry| {
        *Impl::hash_bytes(
            &[
                entry.logic_ref.as_bytes(),
                entry.label_ref.as_bytes(),
                entry.kind_base.as_slice(),
            ]
            .concat(),
        )
    };
    let node = |left: Digest, right: Digest| {
        *Impl::hash_bytes(&[left.as_bytes(), right.as_bytes()].concat())
    };
    let padding = *Impl::hash_bytes(b"EMPTY");

    assert_eq!(
        of(&entries),
        node(
            node(leaf(&entries[0]), leaf(&entries[1])),
            node(leaf(&entries[2]), padding)
        )
    );
}
