//! The migrated chains of each environment: the chains on which its protocol adapter copied the state of the legacy
//! protocol adapter. A legacy-forwarder resource exists only there, so only there does a tree list the members under
//! the label of the legacy ERC20 forwarder.

use crate::chain::{Caip2ChainId, Chain};
use crate::error::Result;
use serde::Deserialize;
use serde_with::{SetPreventDuplicates, serde_as};
use std::collections::BTreeSet;
use std::sync::LazyLock;

/// The migrated chains of each environment. A chain may be listed before its protocol adapter is deployed. The
/// `_comment` is review context.
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigratedChains {
    _comment: Option<String>,
    #[serde_as(as = "SetPreventDuplicates<_>")]
    pub staging: BTreeSet<Caip2ChainId>,
    #[serde_as(as = "SetPreventDuplicates<_>")]
    pub production: BTreeSet<Caip2ChainId>,
}

impl MigratedChains {
    /// Reads lists in the format of `migrated-chains.json`. Rejects a chain listed twice in one environment, a chain
    /// that `Chain` does not know, and a field that the format does not have.
    pub fn from_json(json: &str) -> Result<Self> {
        let migrated: Self = serde_json::from_str(json)?;
        for chain in migrated.staging.iter().chain(&migrated.production) {
            Chain::try_from(chain)?;
        }
        Ok(migrated)
    }
}

/// The migrated chains of each environment, from `migrated-chains.json`.
pub fn chains() -> &'static MigratedChains {
    &MIGRATED_CHAINS
}

static MIGRATED_CHAINS: LazyLock<MigratedChains> = LazyLock::new(|| {
    MigratedChains::from_json(include_str!("../data/migrated-chains.json"))
        .unwrap_or_else(|error| panic!("migrated-chains.json: {error}"))
});
