mod common;

use anoma_risc0_kind_trees::{Entry, Tree};
use common::own_kind;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

fn root(entries: Vec<Entry>) -> Digest {
    Tree::new(entries).expect("a valid tree").root()
}

fn leaf(entry: &Entry) -> Digest {
    *Impl::hash_bytes(
        &[
            entry.logic_ref.as_bytes(),
            entry.label_ref.as_bytes(),
            entry.kind_base.as_slice(),
        ]
        .concat(),
    )
}

#[test]
fn root_is_independent_of_the_entry_order() {
    let (a, b) = (own_kind(1), own_kind(2));
    assert_eq!(root(vec![a.clone(), b.clone()]), root(vec![b, a]));
}

#[test]
fn root_of_a_one_entry_tree_is_its_leaf() {
    let a = own_kind(1);
    assert_eq!(root(vec![a.clone()]), leaf(&a));
}

#[test]
fn root_of_the_empty_tree_is_the_padding_leaf() {
    assert_eq!(root(Vec::new()), *Impl::hash_bytes(b"EMPTY"));
}

/// Three leaves are the smallest count that needs a padding leaf at a level below the root.
#[test]
fn root_pads_the_leaves_to_a_power_of_two_with_the_padding_leaf() {
    let entries = [own_kind(1), own_kind(2), own_kind(3)];
    assert!(
        entries.windows(2).all(|pair| pair[0].key() < pair[1].key()),
        "premise: the entries are in key order"
    );
    let node = |left: Digest, right: Digest| {
        *Impl::hash_bytes(&[left.as_bytes(), right.as_bytes()].concat())
    };
    let padding = *Impl::hash_bytes(b"EMPTY");

    assert_eq!(
        root(entries.to_vec()),
        node(
            node(leaf(&entries[0]), leaf(&entries[1])),
            node(leaf(&entries[2]), padding)
        )
    );
}
