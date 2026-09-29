# speare-legacy-account-import

Rust library for acquiring a durable local copy of a legacy Speare account. It does **not**
connect to SQL Server or convert the rows into the current Speare model. See
[`PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md) for boundaries and [`MIGRATION_API.md`](MIGRATION_API.md)
for the proposed, not-yet-implemented server contract and its consistency precondition.
The staging SQLite file has tables named for the legacy tables, with the columns and compatible
SQLite types generated from `legacy-sql-server-schema.sql`. Every V4 (`dbo`) and V5 (`andrew`)
table in that reference is required in the server manifest, including empty tables. Acquisition
metadata and checkpoints live in separate tables.

```rust,no_run
use speare_legacy_account_import::{Acquirer, HttpLegacySource, StagedAccount};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let source = HttpLegacySource::new("https://speare.example/", "account-scoped-token")?;
let progress = Acquirer::new(source).acquire("legacy-user-id", "account.stage.sqlite")?;
assert!(progress.complete);

let staged = StagedAccount::open("account.stage.sqlite")?;
let mut after_id = 0;
loop {
    let rows = staged.read_rows("Card", after_id, 100)?;
    if rows.is_empty() { break; }
    for row in rows {
        after_id = row.id;
        // Transform this legacy row in speare-account-import, not this library.
    }
}
# Ok(())
# }
```

Reinvoke `acquire` with the same path after interruption; committed pages are not fetched again.
Open a completed staging file without the network for downstream reruns. Store it in an
access-controlled location: it contains customer data. Implement `LegacySource` to test against
an in-memory/fake export; integration tests in `tests/acquisition.rs` demonstrate this.
