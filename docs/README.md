# Kind Trees

This folder states what this repository contains and why the protocol needs it. The glossary of this repository, [`CONTEXT.md`](../CONTEXT.md#language), defines the terms. The [repository README](../README.md) gives the layout, the format of the entries and the workflows. [`RELEASE_CHECKLIST.md`](../RELEASE_CHECKLIST.md) has the commands.

## 1. Summary

The compliance circuit takes the kind point of a resource from a kind tree, if the tree has an entry for its kind. Else it calculates the kind point from the kind. The protocol adapter stores only the root of one kind tree, and it accepts proofs against this root or against the empty kind tree. So for each protocol adapter, someone must decide the entries, calculate the root as the circuit does, and publish it. This repository does this, for each chain and environment.

The kind trees have two uses:

- **Shorter proofs.** An entry with a precalculated kind point lets the compliance circuit make an in-circuit check of a Merkle path instead of calculating the kind point. The trees have such entries for the tokens that many transactions use, and for the generic call resources.
- **Soft migration.** An entry that gives one kind the kind point of another kind makes them aliases. Users can then convert resources of a deprecated version into resources of the active version.

## 2. Documents

| Document                                                         | Content                                                                                      |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [`01-entries-and-rules.md`](./01-entries-and-rules.md)           | How the generator makes the entries and the root of each tree, and the rules of each tree    |
| [`02-generation.md`](./02-generation.md)                         | The files of the repository, and the checks of the generated trees                           |
| [`03-legacy-erc20-forwarder.md`](./03-legacy-erc20-forwarder.md) | The entries that let legacy-forwarder resources convert into resources of the active version |

## 3. Decisions

The [ADRs](./adr/) record the decisions behind the layout, the hash and the rules, for example that the trees are generated and committed ([ADR-0002](./adr/0002-trees-are-generated-and-committed.md)) and that kinds follow the forwarder ([ADR-0008](./adr/0008-kinds-follow-the-forwarder.md)).
