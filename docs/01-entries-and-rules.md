# Entries and Rules

This document specifies how the generator makes the entries and the root of each kind tree, and the rules that each tree follows. The [overview](./README.md) lists the other documents, and the glossary [`CONTEXT.md`](../CONTEXT.md#language) defines the terms.

## 1. How the entries are made

- **One fungibility domain for each supported token.** Its kind base is the kind of the active version under the label that names the token and the forwarder of the environment. Each deprecated version gets an entry under this label with this kind base: an alias of the active version.
- **The active version** keeps its own kind base. It gets an entry only if the token sets `precompute_kind_base`.
- **The legacy-forwarder resources** of a token get an alias in its fungibility domain, on a chain with a legacy ERC20 forwarder, if `tokens.json` marks the token with `soft_migrate_from_legacy_forwarder` ([`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md)).
- **The generic call resources** get an entry with their own kind base. This entry makes no alias.
- **The root** is the Merkle root over the hashes of the entries, sorted by logic ref and label ref, with `sha256("EMPTY")` as the padding leaf. This is the tree that arm-risc0 builds, and a test compares the two calculations ([ADR-0004](./adr/0004-the-hash-is-owned-here-and-cross-checked.md)).

## 2. Rules

[ADR-0008](./adr/0008-kinds-follow-the-forwarder.md) gives the rules for each kind tree:

- **R1, one forwarder.** Two kinds can share a kind base only if one forwarder holds the tokens that back the resources of both kinds. A conversion keeps the quantity, so a wrong alias lets users take tokens that back other resources. Reviewers read each alias as a permission to create tokens.
- **R2, a way to unwrap.** Each kind with unspent resources keeps a way to unwrap: the forwarder accepts its logic ref, or it is an alias of a kind whose logic ref the forwarder accepts.
- **One active version.** Exactly one version is active, and it is the highest listed. A deprecated version stays listed until no resource of it exists, also when the protocol adapter denies its logic ref.

## 3. Trust assumptions

- **The owner of each protocol adapter sets only roots that this repository published after a review.** No out-of-circuit check covers the entries: the protocol adapter checks only that the kind tree root of a transaction is its stored root or the root of the empty tree.
- **A reviewer checks the inputs.** The tests check each entry against its fungibility domain and each root against arm-risc0, but only a reviewer finds a missing token or a wrong flag in `tokens.json`.
- **Nobody knows a linear relation between the kind bases.** No in-circuit check compares a kind base with its kind. The generator derives each kind base from a kind with the same hash to curve as arm-risc0, which a test checks. So each kind base in a tree is the hash to curve of a kind, and such points meet this condition.
