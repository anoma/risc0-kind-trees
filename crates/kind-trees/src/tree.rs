//! Generated chain trees and their roots, one set per environment. A chain's tree lives in
//! `data/generated/<environment>/<namespace>_<reference>.json`, named for the chain's CAIP-2 chain ID; the trees and
//! roots are embedded and looked up per chain.

use crate::chain::{Caip2ChainId, Chain};
use crate::entry::{Entry, hex_digest};
use crate::error::{Error, Result};
use crate::root;
use k256::elliptic_curve::sec1::{FromEncodedPoint, Tag};
use k256::{AffinePoint, EncodedPoint};
use risc0_zkvm::Digest;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// One chain's kind tree: its entries in key order, each kind listed once with a kind base the resource machine
/// accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    entries: Vec<Entry>,
}

impl Tree {
    /// Builds a tree as arm-risc0 does: rejects a kind base that is not an uncompressed secp256k1 point and a kind
    /// listed twice, and sorts the entries by key.
    pub fn new(mut entries: Vec<Entry>) -> Result<Self> {
        if let Some(entry) = entries.iter().find(|entry| !is_kind_base(&entry.kind_base)) {
            return Err(Error::InvalidKindBase {
                logic_ref: entry.logic_ref,
                label_ref: entry.label_ref,
            });
        }
        entries.sort_by_key(Entry::key);
        if let Some(pair) = entries
            .windows(2)
            .find(|pair| pair[0].key() == pair[1].key())
        {
            return Err(Error::DuplicateKind {
                logic_ref: pair[0].logic_ref,
                label_ref: pair[0].label_ref,
            });
        }
        Ok(Self { entries })
    }

    /// Parses a tree from its JSON representation: an array of entries.
    pub fn from_json(json: &str) -> Result<Self> {
        Self::new(serde_json::from_str(json)?)
    }

    /// Reads a tree from a generated `<namespace>_<reference>.json` file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_json(&std::fs::read_to_string(path)?)
    }

    /// The entries, in key order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The root over the entries.
    pub fn root(&self) -> Digest {
        root::of(&self.entries)
    }

    /// The aliases: the entries assigned another kind as their kind base. Only the tree can express them, and they
    /// are the review surface.
    pub fn aliases(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_alias())
            .collect()
    }
}

/// Whether `bytes` is the uncompressed SEC1 encoding of a secp256k1 point, the only kind base the resource machine
/// accepts.
fn is_kind_base(bytes: &[u8]) -> bool {
    EncodedPoint::from_bytes(bytes).is_ok_and(|encoded| {
        encoded.tag() == Tag::Uncompressed
            && AffinePoint::from_encoded_point(&encoded).is_some().into()
    })
}

/// Reads roots in the format of `roots.json`. Rejects a chain that `Chain` does not know.
pub fn roots_from_json(json: &str) -> Result<BTreeMap<Caip2ChainId, Digest>> {
    let recorded: BTreeMap<Caip2ChainId, ChainRoot> =
        serde_json::from_str(json).map_err(Error::InvalidRootsJson)?;
    recorded
        .into_iter()
        .map(|(id, recorded)| {
            // A chain that does not resolve is an error: dropping it would drop its freshness check.
            Chain::try_from(&id)?;
            Ok((id, recorded.root))
        })
        .collect()
}

/// One chain's recorded root. The `_comment` naming the chain is review context and is not deserialized.
#[derive(Deserialize)]
struct ChainRoot {
    #[serde(with = "hex_digest")]
    root: Digest,
}

/// Embeds one environment's roots and trees. Each chain is listed as the namespace and reference of its CAIP-2
/// chain ID, from which the macro builds both the ID and the tree's file name.
macro_rules! environment_module {
    ($name:ident $(, ($namespace:literal, $reference:literal))*) => {
        pub mod $name {
            use super::*;
            use std::sync::LazyLock;

            static ROOTS: LazyLock<BTreeMap<Caip2ChainId, Digest>> = LazyLock::new(|| {
                roots_from_json(include_str!(concat!(
                    "../data/generated/",
                    stringify!($name),
                    "/roots.json"
                )))
                .unwrap_or_else(|error| panic!("{}/roots.json: {error}", stringify!($name)))
            });

            static TREES: LazyLock<BTreeMap<Caip2ChainId, Tree>> = LazyLock::new(|| {
                let trees: Vec<(Caip2ChainId, Tree)> = vec![$((
                    concat!($namespace, ":", $reference)
                        .parse()
                        .unwrap_or_else(|error| panic!("{error}")),
                    Tree::from_json(include_str!(concat!(
                        "../data/generated/",
                        stringify!($name),
                        "/",
                        $namespace,
                        "_",
                        $reference,
                        ".json"
                    )))
                    .unwrap_or_else(|error| {
                        panic!("invalid tree for {}: {error}", concat!($namespace, ":", $reference))
                    }),
                )),*];
                trees.into_iter().collect()
            });

            /// The chains this environment records a tree for.
            pub fn chains() -> Vec<Caip2ChainId> {
                ROOTS.keys().cloned().collect()
            }

            /// The roots of all recorded chains.
            pub fn roots() -> &'static BTreeMap<Caip2ChainId, Digest> {
                &ROOTS
            }

            /// The root recorded for the chain.
            pub fn root(chain: impl Into<Caip2ChainId>) -> Result<Digest> {
                let chain = chain.into();
                ROOTS
                    .get(&chain)
                    .copied()
                    .ok_or(Error::UnrecordedChain(chain))
            }

            /// The trees of all recorded chains.
            pub fn trees() -> &'static BTreeMap<Caip2ChainId, Tree> {
                &TREES
            }

            /// The tree recorded for the chain.
            pub fn tree(chain: impl Into<Caip2ChainId>) -> Result<&'static Tree> {
                let chain = chain.into();
                TREES.get(&chain).ok_or(Error::UnrecordedChain(chain))
            }
        }
    };
}

environment_module!(
    staging,
    ("eip155", "11155111"),
    ("eip155", "143"),
    ("eip155", "84532"),
    ("solana", "EtWTRABZaYq6iMfeYKouRu166VU2xqa1")
);
environment_module!(production);
