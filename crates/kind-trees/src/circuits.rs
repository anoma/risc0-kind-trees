//! The ERC20 circuit versions, read from `data/circuit-versions.json`. Every listed version is a member of every
//! ERC20 fungibility domain, so its resources are fungible with every other listed version's behind the same
//! forwarder. Exactly one version is active: under the label of the environment's forwarder it keeps its own kind,
//! which is the kind base of every ERC20 fungibility domain, and the backend creates its resources. Every other
//! version is deprecated: an alias of the active one, which the backend consumes and converts. A deprecated
//! version leaves the list only when no resource of it exists under the label of a recorded forwarder. Until then it
//! stays, even when the protocol adapter denies its logic ref, with entries that nothing can use.

use crate::entry::hex_digest;
use crate::{Error, Result};
use risc0_zkvm::Digest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// Whether a circuit version, or a member, is the one the backend creates resources of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The version that keeps its own kind under the label of the environment's forwarder. The backend creates its
    /// resources.
    Active,
    /// An alias of the active version. The backend consumes its resources and converts them.
    Deprecated,
}

/// One listed version of the ERC20 transfer circuit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(from = "CircuitVersionJson")]
pub struct CircuitVersion {
    /// The circuit's release version. The crate that ships it can carry another version.
    pub version: String,
    /// The circuit's verifying key, which every resource of this version carries as its logic ref.
    pub logic_ref: Digest,
    pub status: Status,
}

/// The listed circuit versions of each resource type, in file order. The `_comment` is review context.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CircuitVersions {
    _comment: Option<String>,
    #[serde(rename = "ERC20Resource")]
    pub erc20: Vec<CircuitVersion>,
    #[serde(rename = "SPLTokenResource")]
    pub spl_token: Vec<CircuitVersion>,
}

impl CircuitVersions {
    /// Reads lists in the format of `circuit-versions.json`. Rejects a field that the format does not have.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// A circuit version's form in the file. The `_comment` is review context.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircuitVersionJson {
    _comment: Option<String>,
    version: String,
    #[serde(with = "hex_digest")]
    logic_ref: Digest,
    status: Status,
}

impl From<CircuitVersionJson> for CircuitVersion {
    fn from(
        CircuitVersionJson {
            version,
            logic_ref,
            status,
            ..
        }: CircuitVersionJson,
    ) -> Self {
        Self {
            version,
            logic_ref,
            status,
        }
    }
}

static FILE: LazyLock<CircuitVersions> = LazyLock::new(|| {
    CircuitVersions::from_json(include_str!("../data/circuit-versions.json"))
        .unwrap_or_else(|error| panic!("circuit-versions.json: {error}"))
});

/// The listed ERC20 circuit versions, in file order.
pub fn erc20() -> &'static [CircuitVersion] {
    &FILE.erc20
}

/// The listed SPL token circuit versions (the AnomaPay Solana transfer circuit), in file order, under the same
/// rules as the ERC20 list.
pub fn spl_token() -> &'static [CircuitVersion] {
    &FILE.spl_token
}

/// The active ERC20 circuit version. Its kind under the label of the environment's forwarder is the kind base of
/// every ERC20 fungibility domain. `check` says why there is exactly one.
pub fn erc20_active() -> &'static CircuitVersion {
    embedded_active(erc20(), "ERC20Resource")
}

/// The active SPL token circuit version; `check` says why there is exactly one.
pub fn spl_token_active() -> &'static CircuitVersion {
    embedded_active(spl_token(), "SPLTokenResource")
}

/// The listed ERC20 version a logic ref belongs to, if any.
pub fn erc20_version(logic_ref: &Digest) -> Option<&'static CircuitVersion> {
    version(erc20(), logic_ref)
}

/// The listed SPL token version a logic ref belongs to, if any.
pub fn spl_token_version(logic_ref: &Digest) -> Option<&'static CircuitVersion> {
    version(spl_token(), logic_ref)
}

/// Checks a list of circuit versions and returns its active version. Each version and each logic ref is listed once,
/// exactly one version is active, and the active version is the highest listed. The last rule rules out making an old
/// version active again, which is never the plan: the protocol adapter denies the logic ref of a broken release, and a
/// higher release replaces it.
pub fn check(versions: &[CircuitVersion]) -> Result<&CircuitVersion> {
    let mut seen_versions = BTreeSet::new();
    let mut seen_refs = BTreeMap::new();
    let mut parsed = Vec::new();
    for circuit in versions {
        if !seen_versions.insert(&circuit.version) {
            return Err(Error::DuplicateCircuitVersion(circuit.version.clone()));
        }
        if let Some(first) = seen_refs.insert(circuit.logic_ref, &circuit.version) {
            return Err(Error::DuplicateCircuitLogicRef {
                logic_ref: circuit.logic_ref,
                first: first.clone(),
                second: circuit.version.clone(),
            });
        }
        let version = semver::Version::parse(&circuit.version).map_err(|reason| {
            Error::InvalidCircuitVersion {
                version: circuit.version.clone(),
                reason,
            }
        })?;
        parsed.push((version, circuit));
    }
    let mut actives = parsed
        .iter()
        .filter(|(_, circuit)| circuit.status == Status::Active);
    let (active_version, active) = actives.next().ok_or(Error::NoActiveCircuitVersion)?;
    if let Some((_, second)) = actives.next() {
        return Err(Error::TwoActiveCircuitVersions {
            first: active.version.clone(),
            second: second.version.clone(),
        });
    }
    if let Some((highest_version, highest)) = parsed.iter().max_by_key(|(version, _)| version)
        && highest_version > active_version
    {
        return Err(Error::ActiveCircuitVersionNotHighest {
            active: active.version.clone(),
            highest: highest.version.clone(),
        });
    }
    Ok(active)
}

fn embedded_active(versions: &'static [CircuitVersion], resource: &str) -> &'static CircuitVersion {
    check(versions).unwrap_or_else(|error| panic!("circuit-versions.json: {resource}: {error}"))
}

fn version(
    versions: &'static [CircuitVersion],
    logic_ref: &Digest,
) -> Option<&'static CircuitVersion> {
    versions
        .iter()
        .find(|circuit| circuit.logic_ref == *logic_ref)
}
