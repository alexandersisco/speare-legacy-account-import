# Legacy account export contract

The approved exporter integration contract is
[`docs/importer-api-contract.md`](docs/importer-api-contract.md). It is the source of truth for the
REST paths, manifest and page envelopes, cursor behavior, consistency requirements, and errors.

`HttpLegacySource` implements that stateless GET-only contract. The importer does not send an
account ID, export ID, migration-run ID, or first-page cursor to the exporter. The account and
migration-run IDs accepted by `Acquirer::acquire` are local staging identities only, preventing
a staging file from being reused for another authenticated account or write-barrier window.

Rows and their continuation are committed in one SQLite transaction. The importer saves and
compares the initial and final manifests, validates every required dataset and field declaration,
checks page continuations and SQL `INT` ordering, and verifies SQLite counts and maximum IDs before
recording completion. Source consistency still depends on the surrounding application keeping the
account write barrier active throughout acquisition.
