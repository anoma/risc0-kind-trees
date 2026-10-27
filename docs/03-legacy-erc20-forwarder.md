# Legacy ERC20 Forwarder

This document specifies the entries that let legacy-forwarder resources convert into resources of the active version. The glossary [`CONTEXT.md`](../CONTEXT.md#language) defines the terms of the legacy stack.

## 1. Legacy-forwarder entries

On a migrated chain, users convert their legacy-forwarder resources into resources of the active version. For this, on a chain with a legacy ERC20 forwarder, the fungibility domain of a token also holds an entry for the logic ref of the legacy ERC20 forwarder under the label ref of the legacy ERC20 forwarder, if `tokens.json` marks the token with `soft_migrate_from_legacy_forwarder`. Like each entry of the domain, it gets the kind point of the active version under the label ref of the ERC20 forwarder of the environment ([`01-entries-and-rules.md`](./01-entries-and-rules.md)).

## 2. Rules and checks

- **Legacy-forwarder entries only after the token move.** A token whose tokens did not move from the legacy ERC20 forwarder must not be marked for soft migration when the protocol adapter starts to execute transactions.
- **A promotion checks the conversion.** On a copy of Sepolia, the promotion checks that a legacy-forwarder resource converts and unwraps ([ADR-0007](./adr/0007-freshness-gates-promotions-token-validation-runs-always.md)).
