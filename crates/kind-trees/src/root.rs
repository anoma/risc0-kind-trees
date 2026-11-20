//! The kind tree root — the Merkle root that arm-risc0 builds over a tree. Its leaves are
//! `sha256(logic_ref ‖ label_ref ‖ kind_base)` in `(logic_ref, label_ref)` byte order, padded to a power of two with
//! `sha256("EMPTY")`, and each node hashes its two children. The value `setKindTreeRoot` installs and every
//! compliance proof that takes a kind from the tree reproduces.

use crate::entry::Entry;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

/// Computes the root over entries in key order.
pub(crate) fn of(entries: &[Entry]) -> Digest {
    let padding = *Impl::hash_bytes(b"EMPTY");
    let mut layer: Vec<Digest> = entries.iter().map(leaf).collect();
    if layer.is_empty() {
        return padding;
    }
    layer.resize(layer.len().next_power_of_two(), padding);
    while layer.len() > 1 {
        layer = layer
            .chunks(2)
            .map(|pair| node(&pair[0], &pair[1]))
            .collect();
    }
    layer[0]
}

fn leaf(entry: &Entry) -> Digest {
    *Impl::hash_bytes(
        &[
            entry.logic_ref.as_bytes(),
            entry.label_ref.as_bytes(),
            &entry.kind_base,
        ]
        .concat(),
    )
}

fn node(left: &Digest, right: &Digest) -> Digest {
    *Impl::hash_words(&[left.as_words(), right.as_words()].concat())
}
