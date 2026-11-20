//! The supported tokens — the authored identity list the token entries of every chain tree are built from.
//! Being supported is a standing commitment: a chain may list tokens before anything is deployed to it. An EVM
//! chain lists ERC20 tokens by address; a Solana cluster lists SPL token mints.

use crate::chain::{self, Caip2ChainId, Chain, SolanaCluster};
use crate::error::{Error, Result};
use crate::solana::SolanaAddress;
use alloy::primitives::Address;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// The identity of a supported ERC20 token: what the contract itself reports, and whether its legacy-forwarder
/// resources convert.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub address: Address,
    /// Whether, on a migrated chain, the legacy ERC20 forwarder's label joins this token's fungibility domain, which
    /// makes its legacy-forwarder resources fungible with those of the environment's forwarder, so they convert and
    /// leave through the environment's forwarder. Every token states it; a token set to `false` keeps its
    /// legacy-forwarder resources where they are.
    pub soft_migrate_from_legacy_forwarder: bool,
    /// Whether the tree lists the token's active kind with its own kind base, so the compliance circuit reads the
    /// point instead of computing it by hash to curve. Every token states it; its aliases are listed either way.
    pub precompute_kind_base: bool,
}

/// The identity of a supported SPL token: its mint, and the name and symbol review knows it by. A mint reports
/// only its decimals on chain; symbol and name are review context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplToken {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub mint: SolanaAddress,
    /// Whether the tree lists the mint's active kind with its own kind base, so the compliance circuit reads the
    /// point instead of computing it by hash to curve. Every mint states it; its aliases are listed either way.
    pub precompute_kind_base: bool,
}

/// The supported tokens, split by chain family: the key of a section decides which token type it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub erc20: BTreeMap<Caip2ChainId, Vec<Token>>,
    pub spl: BTreeMap<SolanaCluster, Vec<SplToken>>,
}

impl Tokens {
    /// Reads a list in the format of `tokens.json`. Rejects a chain listed twice, a chain that `Chain` does not know, a
    /// token listed twice on a chain, and a field that the format does not have.
    pub fn from_json(json: &str) -> Result<Self> {
        let sections: BTreeMap<Caip2ChainId, serde_json::Value> = chain::by_chain_from_json(json)?;
        let (mut erc20, mut spl) = (BTreeMap::new(), BTreeMap::new());
        for (id, section) in sections {
            // A chain that does not resolve is an error: dropping it would drop its kinds.
            match Chain::try_from(&id)? {
                Chain::Evm(_) => {
                    let listed =
                        section_tokens(&id, section, |token: &Token| token.address.to_string())?;
                    erc20.insert(id, listed);
                }
                Chain::Solana(cluster) => {
                    let listed =
                        section_tokens(&id, section, |token: &SplToken| token.mint.to_string())?;
                    spl.insert(cluster, listed);
                }
            }
        }
        Ok(Self { erc20, spl })
    }
}

/// All supported ERC20 tokens, per EVM chain.
pub fn all() -> &'static BTreeMap<Caip2ChainId, Vec<Token>> {
    &TOKENS.erc20
}

/// The supported ERC20 tokens on the chain; none on a Solana cluster.
pub fn on(chain: impl Into<Caip2ChainId>) -> &'static [Token] {
    TOKENS.erc20.get(&chain.into()).map_or(&[], Vec::as_slice)
}

/// All supported SPL token mints, per Solana cluster.
pub fn spl_all() -> &'static BTreeMap<SolanaCluster, Vec<SplToken>> {
    &TOKENS.spl
}

/// The supported SPL token mints on the cluster.
pub fn spl_on(cluster: SolanaCluster) -> &'static [SplToken] {
    TOKENS.spl.get(&cluster).map_or(&[], Vec::as_slice)
}

static TOKENS: LazyLock<Tokens> = LazyLock::new(|| {
    Tokens::from_json(include_str!("../data/tokens.json"))
        .unwrap_or_else(|error| panic!("tokens.json: {error}"))
});

/// One chain's section. The `_comment` that names the chain is review context.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section<T> {
    _comment: Option<String>,
    tokens: Vec<T>,
}

/// The tokens of a chain's section, as the token type that the chain family lists. `identity` names a token; a token
/// has one fungibility domain on a chain, so no two tokens have the same identity.
fn section_tokens<T: DeserializeOwned>(
    chain: &Caip2ChainId,
    section: serde_json::Value,
    identity: impl Fn(&T) -> String,
) -> Result<Vec<T>> {
    let Section { tokens, .. } =
        serde_json::from_value::<Section<T>>(section).map_err(|reason| {
            Error::InvalidTokenSection {
                chain: chain.clone(),
                reason,
            }
        })?;
    let mut seen = BTreeSet::new();
    if let Some(token) = tokens.iter().find(|token| !seen.insert(identity(token))) {
        return Err(Error::DuplicateToken {
            chain: chain.clone(),
            token: identity(token),
        });
    }
    Ok(tokens)
}
