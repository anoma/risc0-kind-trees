#![allow(dead_code)]

use anoma_risc0_kind_trees::{Entry, kind};
use risc0_zkvm::Digest;

/// An entry assigned its own kind: a logic ref of `seed` bytes and the zero label ref.
pub fn own_kind(seed: u8) -> Entry {
    let logic_ref = Digest::from_bytes([seed; 32]);
    let label_ref = Digest::default();
    Entry {
        metadata: None,
        kind_base: kind::point(&logic_ref, &label_ref).expect("a kind derives"),
        logic_ref,
        label_ref,
    }
}
