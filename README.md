# speare-legacy-account-import

Rust library for acquiring a durable local copy of a legacy Speare account. It does **not**
connect to SQL Server or convert legacy rows into the current Speare model. See
[`PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md) for project boundaries and
[`docs/importer-api-contract.md`](docs/importer-api-contract.md) for the approved exporter API.

The normal workflow has two phases:

1. `Acquirer` downloads and validates all 39 legacy datasets into a local SQLite staging file.
2. `StagedAccount` exposes a completed file in bounded, ordered batches for transformation by
   `speare-account-import` or another downstream consumer.

This crate owns download, validation, retries, checkpoints, and faithful staging. The SQLite file
has legacy-named tables with columns generated from `legacy-sql-server-schema.sql`; acquisition
metadata and checkpoints are stored separately.

## Required source consistency

The exporter is stateless and resolves the account from the request's authentication context. The
surrounding application must activate and maintain the account write barrier throughout
acquisition. If that barrier lapses, do not resume the old staging file: begin a new local run with
a new run ID and a fresh file.

## Add the library

From another local Rust project:

```toml
[dependencies]
speare-legacy-account-import = { path = "../speare-legacy-account-import" }
```

The Rust import name uses underscores: `speare_legacy_account_import`.

## Download an account

```rust,no_run
use speare_legacy_account_import::{Acquirer, AcquisitionOptions, HttpLegacySource};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let token = std::env::var("SPEARE_MIGRATION_BEARER_TOKEN")?;
let source = HttpLegacySource::with_endpoint_prefix(
    "https://speare.example/",
    "/api/migration/export/v1",
    token,
)?;
let acquirer = Acquirer::with_options(
    source,
    AcquisitionOptions {
        page_size: 100,
        transient_retries: 3,
    },
)?;

let progress = acquirer.acquire(
    "local-account-key",
    "local-run-id",
    "account.stage.sqlite",
)?;
assert!(progress.complete);

for dataset in progress.datasets {
    println!(
        "{}: {}/{} rows",
        dataset.name, dataset.acquired_rows, dataset.expected_rows,
    );
}
# Ok(())
# }
```

The `account_id` and `migration_run_id` arguments are local safety boundaries. They are saved in
SQLite but are never sent to the exporter:

- Reuse the same account ID, run ID, and file path to resume an interrupted acquisition.
- A file cannot be accidentally reused for another account or local run.
- After a write-barrier lapse, use a new run ID **and** a new staging file.

Each page's rows, continuation, terminal state, and counters are committed in one SQLite
transaction. If acquisition returns an error, committed pages remain resumable. A successful
return means that every required dataset reached a committed terminal response, staged counts and
maximum IDs match the manifest, and the final manifest matches the initial one.

`HttpLegacySource::new(base_url, token)` uses the exporter's default `/v1` prefix. Use
`HttpLegacySource::with_endpoint_prefix` for mounts such as `/api/migration/export/v1`.
Applications that configure authentication on their own `reqwest::blocking::Client` can use:

```rust,no_run
# use speare_legacy_account_import::HttpLegacySource;
# fn example(authenticated_client: reqwest::blocking::Client) -> Result<(), Box<dyn std::error::Error>> {
let source = HttpLegacySource::with_authenticated_client(
    "https://speare.example/",
    "/api/migration/export/v1",
    authenticated_client,
)?;
# Ok(())
# }
```

## Access downloaded data for further processing

Open a completed staging file without contacting the exporter. `read_rows` requires a completed
account download and returns rows ordered by legacy SQL `Id`:

```rust,no_run
use speare_legacy_account_import::StagedAccount;

# fn process() -> Result<(), Box<dyn std::error::Error>> {
let staged = StagedAccount::open("account.stage.sqlite")?;
assert!(staged.is_complete()?);

for dataset in staged.datasets()? {
    println!(
        "{}: {}/{} rows; complete={}",
        dataset.name, dataset.acquired_rows, dataset.expected_rows, dataset.complete,
    );
}

let mut after_id = None;
loop {
    let rows = staged.read_rows("Card", after_id, 100)?;
    if rows.is_empty() {
        break;
    }

    for row in rows {
        after_id = Some(row.id);

        // `data` contains every original Card column. Values remain faithful
        // to the exporter: BIT is boolean, INT is a number, text/date values
        // are strings, and SQL NULL is JSON null.
        transform_card(row.id, row.data)?;
    }
}
# Ok(())
# }
# fn transform_card(_: i32, _: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
```

Use `None` for the first local read and then pass the last returned `row.id` as `Some(id)`. IDs use
the full signed SQL `INT` range, so zero is not a first-page sentinel. Page sizes must be between 1
and 1000. Call `required_dataset_names()` when downstream code needs the installed crate's
complete case-sensitive dataset catalog.

The legacy-named SQLite tables can also be inspected for diagnostics, but application code should
prefer `StagedAccount`: it enforces completion and reconstructs expected JSON value types. Staging
schema version 2 is required; older staging files are intentionally rejected.

The staging file can contain customer data, blob references, and secrets. Keep it in an
access-controlled location and remove it according to the migration application's retention
policy.

## Errors and recovery

- `Error::Source` reports exporter, authentication, contract, or network failures. Only transient
  network failures and HTTP 503 responses are retried automatically.
- `Error::Invalid` means the exporter response violated the approved contract.
- `Error::Conflict` means the staging identity, saved manifest, or persisted checkpoint does not
  match the attempted operation. Preserve the file for diagnosis; do not blindly overwrite it.
- `Error::Incomplete` prevents downstream reads before acquisition has completed.
- `Error::Database` reports SQLite failures.

Implement `LegacySource` to test acquisition without HTTP. The fake source in
`tests/acquisition.rs` demonstrates multi-page, retry, interruption, empty-dataset, and malformed
response scenarios.

## Exporter test CLI

The included CLI runs a complete acquisition against an exporter and reports the validated count
for every dataset. It reads the bearer token from the environment rather than a command-line
argument:

```sh
export SPEARE_MIGRATION_BEARER_TOKEN='...'
cargo run --bin speare-import-test -- \
  --base-url https://speare.example \
  --endpoint-prefix /api/migration/export/v1 \
  --account-id local-account-key \
  --run-id test-run-1 \
  --staging ./speare-test.sqlite
```

The account and run IDs are local safety boundaries and are never sent to the exporter. Reuse all
three local values to test restart/resume; choose a new run ID and staging file for a fresh test.
Optional flags include `--page-size`, `--retries`, and `--token-env`. Run
`cargo run --bin speare-import-test -- --help` for the complete usage text.
