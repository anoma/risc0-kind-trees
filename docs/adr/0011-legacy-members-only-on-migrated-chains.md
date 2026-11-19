# Legacy members only on migrated chains

ADR-0008 gave each tree of a chain that records a legacy ERC20 forwarder the members under its label, one for each token that the supported-token list marks for soft migration. But one chain can be migrated in one environment and start fresh in the other: BSC production copies the legacy state, and BSC staging starts fresh. A fresh protocol adapter holds no legacy-forwarder resource, and it does not deny the legacy logic ref for created resources. There the members give no resource an exit. They only let a conversion create legacy-forwarder resources, and each alias is a permission to create resources. They also enter a tree where the legacy forwarder's tokens never move to the environment's forwarder.

So the generator makes these members only on the migrated chains of each environment. `data/migrated-chains.json` lists them, one list for each environment. `soft_migrate_from_legacy_forwarder` stays one flag for each chain and token: both environments copy the same legacy state, so whether the legacy-forwarder resources of a token convert does not depend on the environment.

Whether a protocol adapter copied the legacy state is a fact of its deployment, and the pa-evm record does not state it. The record names the `initialImplementation`, but only an address, and nothing offline tells that it is the migrational implementation. The list here is a second source of a pa-evm fact, so a test checks it against the chain: a migrated protocol adapter denies the legacy logic ref for created resources, because pa-evm's migration checklist adds this entry, and a fresh one does not.

One folder for each environment, each with its own `tokens.json` and `circuit-versions.json`, was rejected. It would list the identity of each token on a chain twice, which ADR-0005 rules out. A circuit version is a dependency pin, and the branch of each environment carries the pins (ADR-0001).

## Consequences

- A chain may be listed before its protocol adapter is deployed, as a token may be listed before anything is deployed.
- The generator rejects a listed chain that records no legacy ERC20 forwarder, also before its protocol adapter is recorded.
- The on-chain test runs on every pull request and push, like token validation, and needs `ALCHEMY_API_KEY`. It reads only the recorded protocol adapters, and a migrated one takes the deprecation entry before the pa-evm bindings release its record.
- A chain that an environment migrates later is added to the list in the same pull request that pins the bindings with its record.
