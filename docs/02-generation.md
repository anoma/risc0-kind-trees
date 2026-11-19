# Generation

This document specifies the files of the repository and the checks of the generated trees. The [overview](./README.md) lists the other documents, and the glossary [`CONTEXT.md`](../CONTEXT.md#language) defines the terms. The [repository README](../README.md) gives the layout and the format of the entries.

## 1. Files

- [`crates/kind-trees/data/tokens.json`](../crates/kind-trees/data/tokens.json): the supported tokens of each chain, keyed by the CAIP-2 chain ID. Each token states whether its active kind gets an entry (`precompute_kind_base`). Each ERC20 token also has the flag `soft_migrate_from_legacy_forwarder` ([`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md)).
- [`crates/kind-trees/data/circuit-versions.json`](../crates/kind-trees/data/circuit-versions.json): the versions of the ERC20 and SPL token circuits, each with its logic ref and its status, active or deprecated.
- [`crates/kind-trees/data/migrated-chains.json`](../crates/kind-trees/data/migrated-chains.json): the migrated chains of each environment, on which its protocol adapter copied the legacy state ([`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md)).
- `crates/kind-trees/data/generated/<environment>/`: one kind tree for each chain, and `roots.json` with the root of each tree.
- The generator, the crate [`crates/generator`](../crates/generator), which is not published. It derives each entry from these inputs and from the deployment records of the protocol adapter and the forwarders. Nobody writes a kind base by hand.
- The crate `anoma-risc0-kind-trees`, from which a prover loads a tree, and [`crates/integration-test`](../crates/integration-test), which checks the trees against the chains and against arm-risc0.

## 2. Checks

- **Each list of circuit versions follows its rules.** `CircuitVersions::from_json` rejects a version that is not a semantic version. `circuits::check` rejects a version or a logic ref listed twice, and a list without exactly one active version or whose active version is not the highest listed. The generator checks each list before it makes entries from it.
- **Each loader reads only the fields of its format.** The crate reads each data file with one function: `Tokens::from_json`, `CircuitVersions::from_json`, `MigratedChains::from_json`, `tree::roots_from_json` or `Tree::from_json`. Each rejects a field that the format does not have, so a misspelled field fails and does not read as absent. The readers of `tokens.json`, `migrated-chains.json` and `roots.json` also reject a chain listed twice, and the reader of `tokens.json` rejects a token listed twice on a chain. A `_comment` is review context, and each format accepts it where it has one.
- **Each migrated chain is migrated on chain.** The generator rejects a migrated chain that records no legacy ERC20 forwarder. An integration test checks each recorded protocol adapter on a chain with a legacy ERC20 forwarder: it denies the legacy logic ref for created resources if and only if `migrated-chains.json` lists its chain for the environment. A migrated protocol adapter has this entry, so that no conversion creates a legacy-forwarder resource, and a fresh one does not.
- **Each tree lists each kind once, with a valid kind base.** `Tree::new` rejects a kind listed twice and a kind base that is not an uncompressed secp256k1 point, as arm-risc0 does. The generator builds each tree with it, and the crate loads each committed tree with it.
- **The committed trees are the generated trees.** The CI check `generate-check` fails if the committed trees differ from the trees that the generator makes from the committed inputs ([ADR-0002](./adr/0002-trees-are-generated-and-committed.md)).
- **Each protocol adapter stores the generated root.** A promotion of a branch passes only if each protocol adapter of the environment stores the generated root ([ADR-0007](./adr/0007-freshness-gates-promotions-token-validation-runs-always.md)).
- **Each forwarder accepts a listed version.** A promotion also checks that the forwarder of the environment on each chain accepts the logic ref of a listed version. Each listed version is a member of each fungibility domain, so the resources of each listed version can then convert into resources that the forwarder accepts ([`01-entries-and-rules.md`](./01-entries-and-rules.md#2-rules)).

A proof against a new tree is valid only after the protocol adapter stores its root. Until then, provers use the old tree or the empty tree.
