#![allow(dead_code)]

use anoma_risc0_kind_trees::{CircuitVersion, Status};
use risc0_zkvm::Digest;

/// A deprecated version and the active version: the shortest list that makes an alias.
pub fn circuits() -> Vec<CircuitVersion> {
    vec![
        CircuitVersion {
            version: "1.0.0".parse().unwrap(),
            logic_ref: Digest::from_bytes([1; 32]),
            status: Status::Deprecated,
            legacy_forwarders_only: false,
        },
        CircuitVersion {
            version: "2.0.0".parse().unwrap(),
            logic_ref: Digest::from_bytes([2; 32]),
            status: Status::Active,
            legacy_forwarders_only: false,
        },
    ]
}

/// The deprecated and the active version of `circuits()`.
pub fn deprecated_and_active(circuits: &[CircuitVersion]) -> (&CircuitVersion, &CircuitVersion) {
    let [deprecated, active] = circuits else {
        panic!("premise: two versions");
    };
    assert_eq!(
        (deprecated.status, active.status),
        (Status::Deprecated, Status::Active),
        "premise: the first version is deprecated, the second active"
    );
    (deprecated, active)
}
