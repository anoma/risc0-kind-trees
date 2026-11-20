//! The kind derivation — RFC 9380 hash-to-curve over `logic_ref ‖ label_ref`, exactly as the compliance circuit
//! computes it. A fungibility domain's kind base is the kind of the active version under the label of the
//! environment's forwarder, so this is the only derivation.
use crate::solana::SolanaAddress;
use alloy::primitives::Address;
use k256::Secp256k1;
use k256::elliptic_curve::hash2curve::{ExpandMsgXmd, GroupDigest};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use risc0_zkvm::Digest;
use risc0_zkvm::sha::rust_crypto::Sha256;
use risc0_zkvm::sha::{Impl, Sha256 as _};

/// The RFC 9380 domain separation tag the compliance circuit uses.
const DST: &[u8] = b"QUUX-V01-CS02-with-secp256k1_XMD:SHA-256_SSWU_RO_";

/// Derives a kind: the uncompressed SEC1 encoding (65 bytes).
pub fn point(logic_ref: &Digest, label_ref: &Digest) -> Vec<u8> {
    let message: [&[u8]; 2] = [logic_ref.as_bytes(), label_ref.as_bytes()];
    Secp256k1::hash_from_bytes::<ExpandMsgXmd<Sha256>>(&message, &[DST])
        .expect("hash to curve fails only for an empty domain separation tag")
        .to_encoded_point(false)
        .as_bytes()
        .to_vec()
}

/// The label of an ERC20 resource: `sha256(forwarder ‖ token)`, as the transfer circuit computes it.
pub fn erc20_label_ref(forwarder: &Address, token: &Address) -> Digest {
    label_ref(forwarder.as_slice(), token.as_slice())
}

/// The label of an SPL token resource: `sha256(forwarder program id ‖ mint)`, as the Solana transfer circuit
/// computes it (`transfer_witness::calculate_label_ref` in anomapay-solana-resource), the ERC20 shape over 32-byte
/// inputs.
pub fn spl_token_label_ref(forwarder: &SolanaAddress, mint: &SolanaAddress) -> Digest {
    label_ref(forwarder.as_bytes(), mint.as_bytes())
}

/// A token resource's label: the hash of the forwarder holding it and the token it holds.
fn label_ref(forwarder: &[u8], token: &[u8]) -> Digest {
    *Impl::hash_bytes(&[forwarder, token].concat())
}
