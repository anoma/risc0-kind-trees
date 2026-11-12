# Entries and Rules

This document specifies how the generator makes the entries and the root of each kind tree, and the rules that each tree follows. The [overview](./README.md) lists the other documents, and the glossary [`CONTEXT.md`](../CONTEXT.md#language) defines the terms.

## 1. How the entries are made

- **One fungibility domain for each supported token.** Its kind base is the kind of the active version under the label that names the token and the forwarder of the environment. Each deprecated version gets an entry under this label with this kind base: an alias of the active version.
- **The active version** keeps its own kind base. It gets an entry only if the token sets `precompute_kind_base`.
- **The legacy-forwarder resources** of a token get an alias in its fungibility domain, on a chain with a legacy ERC20 forwarder, if `tokens.json` marks the token with `soft_migrate_from_legacy_forwarder` ([`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md)).
- **The generic call resources** get an entry with their own kind base. This entry makes no alias.
- **The root** is the Merkle root over the hashes of the entries, sorted by logic ref and label ref, with `sha256("EMPTY")` as the padding leaf. This is the tree that arm-risc0 builds, and a test compares the two calculations ([ADR-0004](./adr/0004-the-hash-is-owned-here-and-cross-checked.md)).

## 2. Rules

Each kind tree follows three rules:

- **Only equivalent kinds share a kind base.** Two kinds are equivalent if equal quantities of both kinds stand for equal amounts of one asset. Otherwise, a conversion can give a user resources that stand for more than the resources that the user consumed. So a reviewer reads each alias as a permission to create resources.
  - For ERC20 and SPL token resources, two kinds are equivalent if the same forwarder holds the same token for the resources of both kinds. Anyone can deploy a forwarder, so the generator makes entries only for the forwarders that the deployment records name.
- **Keep the conversions that unspent resources need.** For each kind with unspent resources, its application accepts the kind, or the kind can convert into a kind that its application accepts.
  - For ERC20 and SPL token resources, the application accepts a kind if its label names the forwarder of the environment and this forwarder accepts its logic ref. The rule covers each supported token, under the labels of the forwarder of the environment and of the legacy ERC20 forwarder. So a version stays listed while unspent resources of it exist, also when a protocol adapter denies its logic ref.
- **Only the highest version is active.** In each list of `circuit-versions.json`, exactly one version is active: the highest listed.

[ADR-0008](./adr/0008-kinds-follow-the-forwarder.md) records the decision behind these rules.

## 3. Trust assumptions

- **The owner of each protocol adapter sets only roots that this repository published after a review.** No out-of-circuit check covers the entries: the protocol adapter checks only that the kind tree root of a transaction is its stored root or the root of the empty tree.
- **A reviewer checks the inputs.** The tests check each entry against its fungibility domain and each root against arm-risc0. But only a reviewer finds a missing token or a wrong flag in `tokens.json`, or a version that leaves `circuit-versions.json` while it has unspent resources.
- **Nobody knows a linear relation between the kind bases.** No in-circuit check compares a kind base with its kind. The generator derives each kind base from a kind with the same hash to curve as arm-risc0, which a test checks. So each kind base in a tree is the hash to curve of a kind, and such points meet this condition.
