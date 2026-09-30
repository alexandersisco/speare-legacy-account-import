# Speare Account Export: importer integration contract

Implement the importer against the following **REST `/v1` contract**. These are the
approved exporter defaults; reconcile any existing importer assumptions with them.

## 1. Endpoints and authentication

The intended server mount is:

```text
GET /api/migration/export/v1/manifest
GET /api/migration/export/v1/datasets/:dataset
```

The exporter supports a configurable prefix; its default is `/v1`. Make the
importer's endpoint prefix configurable.

Every request must use the surrounding application's authentication mechanism.
The server resolves the authorized legacy account internally.

**Do not send `UserId`, `accountId`, `export_id`, or signed cursors.** Unknown query
parameters are rejected.

The exporter is stateless. There is no export establishment, session lookup,
invalidation endpoint, or `410 Gone` lifecycle.

## 2. Manifest

Fetch the manifest before requesting datasets.

Response shape:

```json
{
  "datasets": [
    {
      "name": "Card",
      "source_table": "andrew.Card",
      "ownership": "andrew.Card.UserId = authenticated account UserId",
      "fields": [
        { "name": "Id", "type": "INT", "nullable": false }
      ],
      "key": "Id",
      "order": "ascending",
      "row_count": 42,
      "max_id": 987
    }
  ],
  "pagination": {
    "default_limit": 100,
    "max_limit": 1000,
    "continuation": "exclusive after_id"
  },
  "consistency": "application-quiesced"
}
```

This example is abbreviated. The actual manifest contains every dataset and every
legacy column.

Rules:

- `row_count` is the exact account-scoped count, represented as a JSON integer.
- `max_id` is the largest account-scoped legacy integer `Id`.
- **Empty datasets use `row_count: 0` and `max_id: null`.**
- Every required dataset appears, including empty datasets.
- There is no additional protocol-version field or catalog revision.
- Save the initial manifest durably.

Expected dataset names are case-sensitive:

```text
Board
CardSettings
CardSnapshot
Card
DataTable
DeletedResource
DeletedUser
DocCardSnapshot
DocSettings
DocSnapshot
FavoritesList
Folder
Image
InternalLink
MissingResource
NavHistory
OmniSearch
PageLink
PinnedResources
PublishedDocLink
ShareLink
SpeareUser
Stack
TeamFeatureEarlyAccess
TeamMember
TeamResource
Team
UserSettings
UserToken
V4LegacyDocLink
V4LegacyDocLinkVisits
WorkspaceDetails
WorkspaceSettings
Workspace
SpeareDocs
Speare_Blocks
SpeareWorkspaces
Speare_WorkspaceTrees
SpeareUserSettings
```

Reject missing, duplicate, or unsupported datasets rather than silently ignoring
them.

## 3. Dataset requests

First page:

```text
GET .../datasets/Card?limit=100
```

Subsequent page:

```text
GET .../datasets/Card?after_id=123&limit=100
```

Rules:

- Omit `after_id` for the first page.
- `after_id` is an **exclusive** legacy integer `Id`, not an offset.
- Accept the full SQL INT range: `-2147483648` through `2147483647`.
- Do not use `0` as a first-page sentinel.
- `limit` defaults to `100`; valid range is `1`–`1000`.
- Query integers must use canonical decimal notation.
- Do not send blank values, repeated parameters, floats, exponent notation, or `null`.
- Page size may change on resume.

## 4. Page response

Nonterminal:

```json
{
  "dataset": "Card",
  "rows": [
    { "Id": 120 },
    { "Id": 123 }
  ],
  "next_after_id": 123,
  "complete": false
}
```

Terminal:

```json
{
  "dataset": "Card",
  "rows": [
    { "Id": 130 }
  ],
  "next_after_id": null,
  "complete": true
}
```

Actual rows contain all original columns.

Validate:

- Response `dataset` matches the request.
- Number of rows does not exceed the requested limit.
- IDs are SQL integers in strictly increasing order.
- Every returned ID is greater than the requested `after_id`, when supplied.
- Nonterminal pages are nonempty and `next_after_id` equals the last returned ID.
- Terminal pages have `next_after_id: null`.
- Terminal pages may be empty or contain exactly `limit` rows.
- **Do not infer completion from page length; use `complete`.**

## 5. SQLite persistence and retry

For each page, transactionally persist:

1. All rows.
2. The next continuation or terminal status.
3. Any associated progress counters.

Use legacy `Id` as the dataset row key. Do not assume resource GUIDs such as
`CardId` are unique.

On restart:

- A committed nonterminal page resumes from its saved `next_after_id`.
- An uncommitted page is requested again.
- A committed terminal page remains complete.

Repeating the same request against the continuously frozen source returns the same
logical result. The exporter does not remember prior requests.

Keep account, dataset, and local migration-run boundaries explicit so staging
cannot mix separate accounts or attempts.

## 6. Faithful staging

Preserve source values without transformation:

| Source family | JSON representation |
| --- | --- |
| SQL INT | Number, including integer deletion flags |
| SQL BIT | Boolean |
| UUID/text | String |
| SQL NULL | `null` |
| DATETIME/DATETIME2(3) | `YYYY-MM-DD HH:mm:ss.SSS` string, no timezone |

Preserve Unicode, trailing spaces, sentinel values, and unparsed content strings.
Do not convert dates through JavaScript `Date` or assign UTC semantics.

Soft-deleted rows are included. `Image` contains metadata and blob references,
**not binary image bytes**.

## 7. Completeness

Before marking the account's SQL download complete:

1. Require all 39 expected datasets.
2. Require a committed terminal response for every dataset.
3. Validate actual staged SQLite contents against each manifest's `row_count`
   and `max_id`.
4. Detect duplicate keys, ordering violations, malformed rows, and checkpoint
   inconsistencies.
5. Re-fetch the manifest and verify it matches the initial manifest.
6. Durably record completion only after these checks pass.

Do not treat an HTTP success or terminal page alone as complete export verification.

No page/dataset checksums are provided or required.

## 8. Consistency and errors

The application must block and drain all relevant source writes throughout the
migration. The exporter does not create a SQL snapshot or freeze the account itself.

If the write barrier lapses, do not continue combining old and new reads.
Coordinate with the application and restart into fresh staging.

Errors use:

```json
{
  "error": {
    "code": "DATABASE_ERROR",
    "message": "Legacy database read failed."
  }
}
```

| Status | Importer handling |
| --- | --- |
| 401 / 403 | Resolve authentication/authorization failure |
| 400 | Invalid request; correct it rather than retrying unchanged |
| 404 | Wrong route/dataset or contract mismatch |
| 405 | Use GET |
| 409 | Account barrier not active; pause and coordinate with application |
| 503 | Retry the same page with bounded backoff |
| 500 | Surface server/configuration/result failure for investigation |

Network failures can also retry the same uncommitted request. Never advance a
checkpoint after a failed request.

## 9. Ownership boundary

The exporter—not the importer—determines account membership.

Approved indirect rules include:

- `WorkspaceDetails` through owned `Workspace`.
- `V4LegacyDocLinkVisits` through owned `V4LegacyDocLink`.
- `TeamMember` and `TeamResource` through **team ownership**, not membership-only
  inclusion.
- `InternalLink` through its **source Card**, not its target.

Do not discard exported team members because their `MemberUserId` differs from
the migrating account.

Importer completeness checks prove faithful staging of the declared export; they
cannot prove that exporter ownership filters cover the correct legacy data. That
remains a live integration validation responsibility.
