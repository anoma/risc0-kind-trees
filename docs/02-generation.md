# Generation

This document specifies the files of the repository and the checks of the generated trees. The [overview](./README.md) lists the other documents, and the glossary [`CONTEXT.md`](../CONTEXT.md#language) defines the terms. The [repository README](../README.md) gives the layout and the format of the entries.

## 1. Files

- [`crates/kind-trees/data/tokens.json`](../crates/kind-trees/data/tokens.json): the supported tokens of each chain, keyed by the CAIP-2 chain ID. Each token states whether its active kind gets an entry (`precompute_kind_point`). Each ERC20 token also has the flag `soft_migrate_from_legacy_forwarder` ([`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md)).
- [`crates/kind-trees/data/circuit-versions.json`](../crates/kind-trees/data/circuit-versions.json): the versions of the ERC20 and SPL token circuits, each with its logic ref and its status, active or deprecated.
- `crates/kind-trees/data/generated/<environment>/`: one kind tree for each chain, and `roots.json` with the root of each tree.
- The generator, [`crates/kind-trees/src/bin/generate.rs`](../crates/kind-trees/src/bin/generate.rs). It derives each entry from these inputs and from the deployment records of the protocol adapter and the forwarders. Nobody writes a kind point by hand.
- The crate `anoma-risc0-kind-trees`, from which a prover loads a tree, and [`crates/integration-test`](../crates/integration-test), which checks the trees against the chains and against arm-risc0.

## 2. Checks

- **The committed trees are the generated trees.** The CI check `generate-check` fails if the committed trees differ from the trees that the generator makes from the committed inputs ([ADR-0002](./adr/0002-trees-are-generated-and-committed.md)).
- **Each protocol adapter stores the generated root.** A promotion of a branch passes only if each protocol adapter of the environment stores the generated root. It also checks R2 on the chains ([ADR-0007](./adr/0007-freshness-gates-promotions-token-validation-runs-always.md)).

A proof against a new tree is valid only after the protocol adapter stores its root. Until then, provers use the old tree or the empty tree.
