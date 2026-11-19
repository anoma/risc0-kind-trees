#![allow(dead_code)]

use anoma_risc0_kind_trees::{Caip2ChainId, Entry, kind};
use risc0_zkvm::Digest;
use serde_json::Value;

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

/// A JSON object keyed by chain, with the values in the given order. A chain may come twice, which `json!` cannot write.
pub fn by_chain(values: &[(&Caip2ChainId, &Value)]) -> String {
    let values: Vec<String> = values
        .iter()
        .map(|(chain, value)| format!("{:?}: {value}", chain.to_string()))
        .collect();
    format!("{{{}}}", values.join(", "))
}
