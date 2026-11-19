use alloy::primitives::Address;
use anoma_risc0_kind_trees::{Entry, Metadata, kind};
use anoma_risc0_kind_trees_generator::{GenericCallCircuit, generic_call_entry};
use risc0_zkvm::Digest;
use risc0_zkvm::sha::{Impl, Sha256};

#[test]
fn generic_call_entry_is_assigned_its_own_kind_under_the_label_sha256_of_the_forwarder() {
    let forwarder = Address::with_last_byte(0xc0);
    let circuit = GenericCallCircuit {
        logic_ref: Digest::from_bytes([4; 32]),
        version: "2.0.0".into(),
    };
    let label_ref = *Impl::hash_bytes(forwarder.as_slice());

    let entry = generic_call_entry(forwarder, &circuit);
    assert_eq!(
        entry,
        Entry {
            metadata: Some(Metadata::GenericCall {
                version: circuit.version,
                forwarder,
            }),
            logic_ref: circuit.logic_ref,
            label_ref,
            kind_base: kind::point(&circuit.logic_ref, &label_ref),
        }
    );
}
