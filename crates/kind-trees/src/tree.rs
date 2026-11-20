//! Generated chain trees and their roots, one set per environment. A chain's tree lives in
//! `data/generated/<environment>/<namespace>_<reference>.json`, named for the chain's CAIP-2 chain ID; the trees and
//! roots are embedded and looked up per chain.

use crate::chain::{self, Caip2ChainId, Chain};
use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::root;
use k256::elliptic_curve::sec1::{FromEncodedPoint, Tag};
use k256::{AffinePoint, EncodedPoint};
use risc0_zkvm::Digest;
use serde::Deserialize;
use serde_with::hex::Hex;
use serde_with::serde_as;
use std::collections::BTreeMap;

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

    /// The entries, in key order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The root over the entries.
    pub fn root(&self) -> Digest {
        root::of(&self.entries)
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

/// Reads roots in the format of `roots.json`. Rejects a chain listed twice, a chain that `Chain` does not know, and a
/// field that the format does not have.
pub fn roots_from_json(json: &str) -> Result<BTreeMap<Caip2ChainId, Digest>> {
    let recorded: BTreeMap<Caip2ChainId, ChainRoot> = chain::by_chain_from_json(json)?;
    recorded
        .into_iter()
        .map(|(id, recorded)| {
            // A chain that does not resolve is an error: dropping it would drop its freshness check.
            Chain::try_from(&id)?;
            Ok((id, recorded.root))
        })
        .collect()
}

/// One chain's recorded root. The `_comment` that names the chain is review context.
#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChainRoot {
    _comment: Option<String>,
    #[serde_as(as = "Hex")]
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
    ("eip155", "56"),
    ("eip155", "84532"),
    ("solana", "EtWTRABZaYq6iMfeYKouRu166VU2xqa1")
);
environment_module!(
    production,
    ("eip155", "1"),
    ("eip155", "143"),
    ("eip155", "42161"),
    ("eip155", "56"),
    ("eip155", "8453")
);
