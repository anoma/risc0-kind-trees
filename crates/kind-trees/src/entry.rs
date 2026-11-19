//! One entry of a kind tree. The JSON representation is `anoma-rm-risc0`'s kind tree schema plus a `_metadata`
//! object, so the upstream loader reads a generated tree unchanged.

use crate::circuits::Status;
use crate::kind;
use crate::solana::SolanaAddress;
use alloy::primitives::Address;
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};
use serde_with::hex::Hex;
use serde_with::{DisplayFromStr, serde_as};

/// A kind, written as its `(logic_ref, label_ref)`, and the kind base it is assigned.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Review context for the entry; not covered by the root.
    #[serde(rename = "_metadata", default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde_as(as = "Hex")]
    pub logic_ref: Digest,
    #[serde_as(as = "Hex")]
    pub label_ref: Digest,
    /// The assigned kind base, SEC1-encoded and uncompressed (65 bytes).
    #[serde_as(as = "Hex")]
    pub kind_base: Vec<u8>,
}

/// What a kind belongs to: the circuit version behind `logic_ref`, and the deployment behind `label_ref`.
/// The root covers none of it, so it stays free to carry whatever a reviewer needs. Addresses are written with the
/// EIP-55 checksum, as `Display` writes them, so a reviewer can paste one into a block explorer.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Metadata {
    /// One supported token behind an ERC20 forwarder. `forwarder` is the address inside this kind's label. On a
    /// member under the legacy forwarder's label it is the legacy ERC20 forwarder, whose tokens move to the
    /// environment's forwarder.
    #[serde(rename = "ERC20Resource")]
    Erc20 {
        version: String,
        name: String,
        #[serde_as(as = "DisplayFromStr")]
        token: Address,
        #[serde_as(as = "DisplayFromStr")]
        forwarder: Address,
        /// `active` if and only if `alias_of` is absent: the backend creates only that member's resources.
        status: Status,
        /// The kind this entry takes its kind base from. Absent only on the active version under the label of the
        /// environment's forwarder.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias_of: Option<AliasOf>,
    },
    /// The arbitrary-call kind behind a generic call forwarder.
    #[serde(rename = "GenericCallResource")]
    GenericCall {
        version: String,
        #[serde_as(as = "DisplayFromStr")]
        forwarder: Address,
    },
    /// One supported SPL token mint behind the Solana token forwarder program. The same fungibility-domain
    /// rules as `Erc20`: every listed SPL token circuit version under the forwarder's label, assigned the kind of
    /// the active version.
    #[serde(rename = "SPLTokenResource")]
    SplToken {
        version: String,
        name: String,
        mint: SolanaAddress,
        forwarder: SolanaAddress,
        status: Status,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias_of: Option<AliasOf>,
    },
}

impl Metadata {
    /// The kind an alias takes its kind base from; `None` for every other entry.
    pub fn alias_of(&self) -> Option<&AliasOf> {
        match self {
            Self::Erc20 { alias_of, .. } | Self::SplToken { alias_of, .. } => alias_of.as_ref(),
            Self::GenericCall { .. } => None,
        }
    }
}

/// The kind an alias takes its kind base from: the active circuit version under the label of the environment's
/// forwarder. Written into `_metadata`, so an alias says what it is a second name for.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasOf {
    pub version: String,
    #[serde_as(as = "Hex")]
    pub logic_ref: Digest,
    #[serde_as(as = "Hex")]
    pub label_ref: Digest,
}

impl Entry {
    /// The sort order: entries sort by `logic_ref ‖ label_ref`.
    pub fn key(&self) -> [u8; 64] {
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(self.logic_ref.as_ref());
        key[32..].copy_from_slice(self.label_ref.as_ref());
        key
    }

    /// Whether the entry is assigned another kind as its kind base, not its own, so that the tree is the only
    /// place it can come from. Generic call and the active version under the label of the environment's forwarder
    /// are not.
    pub fn is_alias(&self) -> bool {
        kind::point(&self.logic_ref, &self.label_ref) != self.kind_base
    }
}

/// The entry as the resource machine loads it, without the metadata the root does not cover.
#[cfg(feature = "arm")]
impl From<&Entry> for anoma_rm_risc0::compliance::KindTreeEntry {
    fn from(entry: &Entry) -> Self {
        Self {
            logic_ref: entry.logic_ref,
            label_ref: entry.label_ref,
            kind_base: entry.kind_base.clone(),
        }
    }
}
