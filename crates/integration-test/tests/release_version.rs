//! The promotion gate for the crate version: a promotion into `main` carries a release version, and a promotion into
//! `staging` a release or a release candidate. Every crate of the workspace takes the workspace version. Runs only on a
//! pull request into `staging` or `main`, selected as the freshness gate is; elsewhere it skips.

use anoma_pa_evm_bindings::addresses::Environment;
use anyhow::{Result, ensure};

fn promotion_target() -> Option<Environment> {
    let base = std::env::var("PROMOTION_TARGET")
        .or_else(|_| std::env::var("GITHUB_BASE_REF"))
        .ok()?;
    match base.as_str() {
        "staging" => Some(Environment::Staging),
        "main" => Some(Environment::Production),
        _ => None,
    }
}

#[test]
fn the_promoted_version_is_a_release_or_for_staging_a_release_candidate() -> Result<()> {
    let Some(environment) = promotion_target() else {
        eprintln!("skipped: not a promotion pull request");
        return Ok(());
    };

    let version = env!("CARGO_PKG_VERSION");
    match environment {
        Environment::Staging => ensure!(
            is_release(version) || is_release_candidate(version),
            "a promotion into staging needs a release or a release candidate version, not {version}"
        ),
        Environment::Production => ensure!(
            is_release(version),
            "a promotion into main needs a release version, not {version}"
        ),
    }
    Ok(())
}

/// Whether a version is a release, i.e. carries no prerelease suffix.
fn is_release(version: &str) -> bool {
    !version.contains('-')
}

/// Whether a version is a release candidate, i.e. carries an `-rc.<number>` prerelease suffix. Any other prerelease
/// (`-alpha.1`, `-rc`, `-rc.x`) is not one.
fn is_release_candidate(version: &str) -> bool {
    let Some((_, suffix)) = version.split_once('-') else {
        return false;
    };
    let Some(number) = suffix.strip_prefix("rc.") else {
        return false;
    };

    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

#[test]
fn is_release_accepts_a_version_without_a_suffix() {
    assert!(is_release("1.0.0"));
}

#[test]
fn is_release_rejects_a_prerelease() {
    assert!(!is_release("1.0.0-rc.10"));
    assert!(!is_release("1.0.0-alpha.6"));
}

#[test]
fn is_release_candidate_accepts_a_numbered_candidate() {
    assert!(is_release_candidate("1.0.0-rc.0"));
    assert!(is_release_candidate("1.0.0-rc.10"));
}

#[test]
fn is_release_candidate_rejects_another_prerelease() {
    assert!(!is_release_candidate("1.0.0-alpha.6"));
    assert!(!is_release_candidate("1.0.0-beta.1"));
}

#[test]
fn is_release_candidate_rejects_a_candidate_without_a_number() {
    assert!(!is_release_candidate("1.0.0-rc"));
    assert!(!is_release_candidate("1.0.0-rc."));
    assert!(!is_release_candidate("1.0.0-rc.x"));
}

#[test]
fn is_release_candidate_rejects_a_version_without_a_suffix() {
    assert!(!is_release_candidate("1.0.0"));
}
