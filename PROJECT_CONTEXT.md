# speare-legacy-account-import

## Purpose

`speare-legacy-account-import` is a Rust library responsible for acquiring the data needed to migrate an existing Speare account from the legacy Speare system.

This project is part of a temporary migration system whose ultimate purpose is to migrate existing Speare accounts away from the legacy SQL Server infrastructure so that SQL Server can be retired.

## Where This Project Fits

The overall relationship is:

```text
speare-desktop-app
        │
        ▼
speare-account-import
        │
        ▼
speare-legacy-account-import
        │
        ▼
Speare server
        │
        ▼
Legacy SQL Server
```

### speare-desktop-app

The desktop application owns the user-facing migration experience.

### speare-account-import

`speare-account-import` owns and orchestrates the complete account migration.

It uses `speare-legacy-account-import` to acquire a local representation of the legacy account and then transforms that legacy data into the current Speare data model.

### speare-legacy-account-import

This repository owns the legacy **acquisition** side of the migration.

Its responsibility is to obtain the required legacy account data from the Speare server and persist a complete, durable, validated local representation of that data.

It does not transform legacy data into the current Speare data model.

### Speare Server

The Speare server has access to the legacy SQL Server database and will expose an API through which `speare-legacy-account-import` can obtain legacy account data.

`speare-legacy-account-import` must never connect directly to SQL Server.

## Server API

The migration API on the Speare server does not exist yet.

Part of designing this library is determining what capabilities and contract that API needs to provide.

Do not assume a pre-existing endpoint structure, pagination scheme, cursor format, response envelope, or other REST protocol design.

The resulting server-side API should be reasonably simple to implement as a conventional REST API. Avoid designs that require substantial migration orchestration or migration state management on the server.

The migration machinery should primarily live in `speare-legacy-account-import`.

## Legacy Account Source Abstraction

The acquisition engine should not be tightly coupled to HTTP.

There should be a clear abstraction representing the source from which legacy account data is acquired.

The real implementation will communicate with the Speare server's migration API.

A fake implementation must also be possible so that the acquisition system can be developed and thoroughly tested without the real server API.

Conceptually:

```text
                         Fake implementation
                        /
Acquisition engine ----
                        \
                         Server API implementation
                                  │
                                  ▼
                            Speare server
```

The same acquisition, staging, checkpointing, retry, completion, and validation logic should work with either source.

The exact Rust abstraction and API design are implementation decisions for this project.

## SQLite Staging Database

`speare-legacy-account-import` owns a local SQLite staging database.

This database is the durable boundary between acquisition and transformation:

```text
remote legacy account
        │
        │ acquisition
        ▼
local SQLite legacy account
        │
        │ transformation
        ▼
current Speare account
```

The first operation belongs to this repository.

The second belongs to `speare-account-import`.

The staging representation should preserve the legacy data faithfully rather than prematurely transforming it into the current Speare model.

Once acquisition has completed successfully, downstream transformation should be able to run or rerun without downloading the legacy account again.

## Legacy Schema Reference

The project contains a reference copy of the legacy SQL Server schema.

Use it to understand the structure and semantics of the legacy data that may need to be acquired.

The schema is a **current reference, not a guarantee that the migration dataset is complete**.

Additional legacy tables may be discovered during development. The architecture should make adding another legacy dataset/table routine rather than requiring substantial architectural changes.

Likewise, the presence of a table in the reference schema does not necessarily mean that the table must be migrated. The actual migration dataset should be determined from migration requirements.

## Large Accounts and Memory Usage

A Speare account may contain a large amount of data.

Assume migration may need to run on a machine with approximately 8 GB of RAM.

The library must therefore use bounded memory and should process data incrementally.

Do not design around loading an entire account into memory or downloading an entire account as one large JSON document.

The acquisition protocol, persistence model, and public API should support processing large accounts in reasonably small units.

## Resumability

Resumability is a core requirement.

Acquisition may be interrupted by circumstances such as:

- network failures;
- temporary server failures;
- application termination;
- application restart;
- machine restart;
- partially completed downloads;
- uncertain request completion.

The library should persist enough progress information to resume safely.

A non-technical user should not need to determine where the migration stopped or how to restart it correctly.

## Idempotency and Retries

Acquisition must be safe to retry.

A useful governing principle is:

> If the client cannot determine whether an operation completed, repeating that operation should be safe.

Retries and resumed downloads must not duplicate staged data or corrupt acquisition state.

## Validation and Completion

Successful acquisition must have an explicit meaning.

The absence of an error is not sufficient evidence that an account has been downloaded completely.

The library and eventual server protocol must provide enough information to establish that all required legacy account data has been acquired and durably stored.

The exact validation and completion protocol is part of the design work for this project.

## Testing

The fake legacy account source should make it possible to exercise the acquisition system without the real server.

Testing should be capable of covering scenarios including:

- ordinary account downloads;
- multiple datasets;
- datasets requiring multiple chunks/pages;
- empty datasets;
- large accounts;
- deterministic failures during acquisition;
- interruption after partial progress;
- restart and resume;
- retries after uncertain completion;
- repeated data or requests;
- malformed or inconsistent source responses;
- incomplete acquisition;
- validation failures.

Resumability and idempotency should be demonstrated through tests rather than merely assumed from the implementation.

## Public Library Boundary

`speare-account-import` will consume this library.

Provide a suitable public Rust API for initiating/resuming acquisition and accessing the resulting staged legacy account.

Downstream code must be able to process staged data incrementally without loading an entire account into memory.

The SQLite staging implementation, indexes, checkpoints, progress metadata, and related acquisition details remain owned by this library unless there is a good reason to expose them.

## Design Freedom

This document intentionally specifies responsibilities and constraints rather than an implementation.

Determine appropriate designs for:

- the Rust library API;
- the legacy account source abstraction;
- the fake source;
- the future server REST API contract;
- incremental/chunked acquisition;
- the SQLite staging representation;
- checkpoints and resumability;
- retry and idempotency behavior;
- validation and completion semantics;
- error handling;
- testing strategy.

Prefer the simplest design that satisfies the migration requirements.

Do not introduce complexity merely to make the migration infrastructure more general or permanent.

## Lifecycle

This is intentionally temporary infrastructure.

The expected lifecycle is:

```text
legacy SQL Server
        ↓
migration infrastructure
        ↓
existing accounts migrated
        ↓
migrations validated
        ↓
legacy SQL Server retired
        ↓
migration infrastructure eventually retired
```

This project still handles customer data and therefore needs strong correctness, durability, recovery behavior, validation, diagnostics, and testing.

At the same time, avoid turning temporary legacy compatibility into a permanent part of the modern Speare architecture.

## Guiding Principles

When making design decisions, favor:

1. Correctness over cleverness.
2. Durability over in-memory convenience.
3. Bounded memory usage.
4. Resumability by design.
5. Idempotency and safe retries by design.
6. Small, recoverable units of work.
7. Faithful preservation of legacy data before transformation.
8. Clear separation between acquisition and transformation.
9. A simple server-side API where practical.
10. Testability without the real server.
11. Useful diagnostics for failures.
12. Minimal demands on non-technical users.
13. Easy addition of newly discovered legacy datasets.
14. No permanent legacy compatibility burden.
15. Eventual deletion of the migration infrastructure.
