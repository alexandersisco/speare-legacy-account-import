# Legacy account export contract (proposed v1)

This is the REST contract implemented by `HttpLegacySource`; the server endpoints do **not**
exist yet. `Acquirer` works with any `LegacySource`, including a fake with no server. This is a
read-only export API, not a server-side migration job.

## Scope and authorization

All calls require the desktop application's account-scoped bearer credential over HTTPS.
The server must authorize the requested account ID against that credential on **every** call,
including page requests; do not trust a client-supplied `UserId`. Do not log tokens or account
contents. Errors: 401/403 unauthorized, 404 unknown account/dataset, 409 or 410 invalidated or
expired export, 400 invalid cursor/limit, 429 or 5xx transient failures. Responses are JSON.
Do not cache sensitive responses in a shared cache (`Cache-Control: no-store`).

### `GET /v1/legacy-exports/{account_id}`

Returns the *complete* dataset catalog for this account and one stable export view:

```json
{
  "account_id": "legacy-user-guid",
  "export_id": "opaque-stable-view-id",
  "datasets": [
    {"name":"SpeareUser", "row_count":1, "max_id":47},
    {"name":"Card", "row_count":2, "max_id":203},
    {"name":"CardSnapshot", "row_count":0, "max_id":0}
  ]
}
```

Dataset names are opaque case-sensitive strings, not raw client-provided SQL identifiers. The
server has an allowlisted dataset catalog and the server determines all account-related rows,
including rows in tables whose ownership requires joining other legacy tables. **The final
required catalog remains a migration-requirements decision.** Start by reviewing user-owned
content (`SpeareUser`, `Card`, `Workspace`, `Folder`, `Stack`, `Board`, settings, snapshots,
links, etc.); do not blindly export administrative, derived/search, authentication/token, or
payment records just because they occur in the schema. `Image` contains blob references, not
necessarily the image bytes: if actual binaries must migrate, they need a separate bounded
binary export contract, not an assertion that the SQL row alone is sufficient.

The manifest must include empty required datasets. `row_count` is the exact number of visible
rows (including `Deleted` tombstones); `max_id` is the greatest SQL Server identity `Id`, or
zero for empty datasets. All IDs are positive signed 64-bit integers. Adding a dataset changes
the manifest/export ID, so an old staging file cannot silently claim completeness.

### `GET /v1/legacy-exports/{account_id}/{export_id}/datasets/{name}/rows?after_id=0&limit=100`

`after_id` is an exclusive numeric cursor, initially 0. `limit` is 1–1000; the server returns
at most that many rows ordered by strictly increasing SQL Server identity `Id`. It must never
filter out deleted rows. A response:

```json
{
  "rows": [
    {"id":201, "data":{"Id":201,"CardId":"guid","UserId":"legacy-user-guid",
                       "Title":"Example","Content":"[]","Deleted":false}}
  ],
  "complete": false
}
```

`complete` means *no more rows after this response* (including the empty page). An empty
non-final page is invalid. Repeating the same GET must yield the same logical rows and values.
The server must return each row with **all legacy columns** under the SQL column names, including
`Id`, version, timestamps and `Deleted`, and preserve JSON stored inside `NVARCHAR` as a string,
not interpret it. Use documented lossless JSON encodings: GUIDs as strings, dates as ISO 8601
with original precision (SQL `DATETIME` / `DATETIME2`), SQL `BIT` as boolean, integer as JSON
integer, nullable columns as null; non-integral decimals, if introduced, as strings to avoid
floating-point loss. For very large values, lower the page size; the HTTP adapter caps a single
response at 32 MiB (a single larger row needs a future streaming/blob extension).

## Consistency precondition (server work still needed)

`export_id` must identify an immutable view of *both* the dataset catalog and row values for
the entire duration of an export and through restart/retry. A repeat manifest GET must return
the same descriptor; an invalidated view must return 409/410, never silently serve newer rows
under the same ID. Keyset paging alone, `MAX(Id)`, and final row counts **cannot detect updates
to existing rows** or guarantee a cross-table coherent snapshot. The server team must choose a
practical way to meet this precondition (e.g. an account migration read/quiescence window or a
durable snapshot/export mechanism). A transaction-local SQL snapshot cannot survive arbitrary
client restarts. This contract does not require the server to track client checkpoints or run
the client migration state machine, but it does require stable source data. Do not claim an
account is safely migrated until that guarantee has been implemented and verified.

## Client durability and completion

The client saves each page's rows into SQLite tables with the legacy table and column names, and
its checkpoint in the same SQLite transaction
(`synchronous=FULL`). Restart resumes after the last committed `Id`, not the last *requested* ID.
It validates row ordering, IDs, page bounds, exact declared row counts and final `max_id` for
every dataset; it rechecks the manifest before marking the export complete. Duplicate replies,
uncertain network completion and app termination are safe to retry. A changed manifest leaves
the old file intact and reports a conflict: create a *new* staging file only after diagnosing
the change. Once complete, downstream reads require no network; staged rows are available via
`StagedAccount::open`, `datasets`, and bounded `read_rows` calls. The SQLite tables are generated
from the checked-in SQL Server schema reference at build time: `INT`/`BIT` become `INTEGER`,
GUID/date/text become `TEXT`, with required columns, primary IDs, and bit checks. SQLite cannot
reproduce SQL Server `NVARCHAR` length, `DATETIME` storage, defaults, or database-level behavior;
the API's lossless encoding rules remain necessary. Unknown tables and missing/extra/ill-typed
columns fail closed. To add a discovered dataset, add its definition to the reference schema,
review ownership/export rules, and update the server catalog. The staging file contains
customer data and should be stored in an access-controlled directory and removed according to
the migration application's retention policy. A malicious or buggy source lying consistently
about its catalog/counts is outside this validation guarantee; acceptance testing must compare
the server's dataset selection and exports with the migration requirements.
