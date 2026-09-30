use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde_json::json;
use speare_legacy_account_import::{
    Acquirer, AcquisitionOptions, Dataset, Error, Field, LegacySource, Manifest, Page, Pagination,
    SourceError, SourceRow, StagedAccount, required_dataset_names,
};

#[derive(Clone)]
struct Fake {
    state: Arc<Mutex<State>>,
}

struct State {
    manifest: Manifest,
    rows: BTreeMap<String, Vec<SourceRow>>,
    calls: Vec<(String, Option<i32>)>,
    fail_at: Option<(String, Option<i32>, bool)>, // after_id, transient?
    malformed: Option<Page>,
}

impl Fake {
    fn new(sizes: &[(&str, usize)]) -> Self {
        let specs = schema_specs();
        let requested = sizes.iter().copied().collect::<BTreeMap<_, _>>();
        let mut rows = BTreeMap::new();
        let mut datasets = Vec::new();
        for name in required_dataset_names() {
            let n = requested.get(name).copied().unwrap_or(0);
            rows.insert(
                name.into(),
                (1..=n).map(|i| sample_row(name, i as i64 * 2)).collect(),
            );
            let (source_table, fields) = specs.get(name).unwrap().clone();
            datasets.push(Dataset {
                name: name.into(),
                source_table,
                ownership: "authenticated account ownership rule".into(),
                fields,
                key: "Id".into(),
                order: "ascending".into(),
                row_count: n as u64,
                max_id: (n > 0).then_some(n as i32 * 2),
            });
        }
        let manifest = Manifest {
            datasets,
            pagination: Pagination {
                default_limit: 100,
                max_limit: 1000,
                continuation: "exclusive after_id".into(),
            },
            consistency: "application-quiesced".into(),
        };
        Self {
            state: Arc::new(Mutex::new(State {
                manifest,
                rows,
                calls: vec![],
                fail_at: None,
                malformed: None,
            })),
        }
    }

    fn fail(&self, dataset: &str, after: Option<i32>, transient: bool) {
        self.state.lock().unwrap().fail_at = Some((dataset.into(), after, transient));
    }

    fn calls(&self) -> Vec<(String, Option<i32>)> {
        self.state.lock().unwrap().calls.clone()
    }

    fn calls_for(&self, dataset: &str) -> Vec<(String, Option<i32>)> {
        self.calls()
            .into_iter()
            .filter(|call| call.0 == dataset)
            .collect()
    }
}

fn schema_specs() -> BTreeMap<String, (String, Vec<Field>)> {
    let mut result = BTreeMap::new();
    let mut current: Option<(String, String, Vec<Field>)> = None;
    for raw in include_str!("../legacy-sql-server-schema.sql").lines() {
        let line = raw.trim();
        if let Some(header) = line.strip_prefix("CREATE TABLE [") {
            let (schema, rest) = header.split_once("].[").unwrap();
            let name = rest.split_once(']').unwrap().0;
            current = Some((schema.into(), name.into(), Vec::new()));
        } else if line == ");" {
            if let Some((schema, name, fields)) = current.take() {
                result.insert(name.clone(), (format!("{schema}.{name}"), fields));
            }
        } else if let Some((_, _, fields)) = current.as_mut()
            && let Some(column) = line.strip_prefix('[')
        {
            let (name, rest) = column.split_once(']').unwrap();
            let rest = rest.trim_start();
            fields.push(Field {
                name: name.into(),
                source_type: rest.split_whitespace().next().unwrap().into(),
                nullable: !rest.contains("NOT NULL"),
            });
        }
    }
    result
}

fn sample_row(name: &str, id: i64) -> serde_json::Value {
    match name {
        "Card" => json!({"Id":id, "CardId":"11111111-1111-1111-1111-111111111111",
            "Title":"Card title", "Content":"[]", "Created":"2024-02-03T04:05:06.000",
            "Modified":"2024-02-04T04:05:06.000", "Deleted":false, "UserId":"account-1",
            "DocId":"00000000-0000-0000-0000-000000000000", "CardVersion":"1.0", "WordCount":-1}),
        "Workspace" => json!({"Id":id, "WorkspaceId":"11111111-1111-1111-1111-111111111111",
            "Title":"Workspace", "Content":"[]", "Created":"2024-02-03T04:05:06.000",
            "Modified":"2024-02-04T04:05:06.000", "Deleted":false, "UserId":"account-1",
            "WorkspaceVersion":"1.0"}),
        "Image" => json!({"Id":id, "UserId":"account-1", "Container":"images",
            "BlobName":"example.png", "ImageType":"png", "ImageSize":123,
            "Created":"2024-02-03T04:05:06.000", "Deleted":false}),
        "TeamResource" => json!({"Id":id, "ResourceId":"11111111-1111-1111-1111-111111111111",
            "ResourceType":null, "OwnerMemberId":"22222222-2222-2222-2222-222222222222",
            "TeamId":"33333333-3333-3333-3333-333333333333", "Created":"2024-02-03T04:05:06.000"}),
        "SpeareDocs" => json!({"Id":id, "DocId":"11111111-1111-1111-1111-111111111111",
            "UserId":"account-1", "Title":"Legacy doc", "Subtitle":"Subtitle", "Document":"{\"text\":\"hello\"}",
            "DocType":1, "Access":1, "Created":"2024-02-03T04:05:06.000",
            "Modified":"2024-02-04T04:05:06.000", "Published":0, "Search":"",
            "Updated":"2016-01-01T00:00:00.000", "Deleted":0, "RegCode":"ABCDEFGH",
            "OrderList":null, "Revisions":null, "Pinned":null, "ConnectionsCount":2}),
        "Speare_Blocks" => json!({"Id":id, "UserId":"account-1",
            "DocId":"11111111-1111-1111-1111-111111111111", "BlockId":"block-1",
            "Content":"<p>text</p>", "Tags":"[]", "NumOrder":1,
            "Created":"2024-02-03T04:05:06.000", "Modified":"2024-02-04T04:05:06.000", "Deleted":0}),
        "SpeareWorkspaces" => json!({"Id":id, "WorkspaceId":"11111111-1111-1111-1111-111111111111",
            "UserId":"account-1", "Title":"Legacy workspace", "Workstate":"{}", "Pinned":0,
            "Created":"2024-02-03T04:05:06.000", "Modified":"2024-02-04T04:05:06.000",
            "Deleted":0, "RegCode":"ABCDEFGH"}),
        "Speare_WorkspaceTrees" => json!({"Id":id, "UserId":"account-1",
            "NodeId":"11111111-1111-1111-1111-111111111111", "SpaceId":"22222222-2222-2222-2222-222222222222",
            "ParentNodeId":"00000000-0000-0000-0000-000000000000", "NodeTitle":"Root",
            "NodeOrder":1, "NodeCollapsed":false, "NodeHidden":true, "NodeType":"board",
            "BoardId":"00000000-0000-0000-0000-000000000000", "DocumentId":"00000000-0000-0000-0000-000000000000",
            "Deleted":0, "Created":"2024-02-03T04:05:06.000", "Modified":"2024-02-04T04:05:06.000",
            "SecretKey":"test-secret"}),
        "SpeareUserSettings" => json!({"Id":id, "UserId":"account-1", "Settings":null,
            "Created":"2024-02-03T04:05:06.000", "Modified":"2024-02-04T04:05:06.000"}),
        _ => panic!("no test fixture for {name}"),
    }
}

impl LegacySource for Fake {
    fn manifest(&self) -> Result<Manifest, SourceError> {
        Ok(self.state.lock().unwrap().manifest.clone())
    }

    fn page(&self, dataset: &str, after: Option<i32>, limit: usize) -> Result<Page, SourceError> {
        let mut s = self.state.lock().unwrap();
        s.calls.push((dataset.into(), after));
        if let Some((name, at, transient)) = &s.fail_at
            && name == dataset
            && *at == after
        {
            return Err(if *transient {
                SourceError::Transient("network failed after request".into())
            } else {
                SourceError::Permanent("test interruption".into())
            });
        }
        if let Some(page) = &s.malformed {
            return Ok(page.clone());
        }
        let all = s
            .rows
            .get(dataset)
            .ok_or_else(|| SourceError::Permanent("unknown dataset".into()))?;
        let rows: Vec<_> = all
            .iter()
            .filter(|row| after.is_none_or(|after| row["Id"].as_i64().unwrap() > i64::from(after)))
            .take(limit)
            .cloned()
            .collect();
        let complete = rows.last().is_none_or(|last| {
            all.last()
                .is_none_or(|final_row| last["Id"] == final_row["Id"])
        });
        let next_after_id =
            (!complete).then(|| rows.last().unwrap()["Id"].as_i64().unwrap() as i32);
        Ok(Page {
            dataset: dataset.into(),
            rows,
            next_after_id,
            complete,
        })
    }
}

fn run(fake: Fake, path: &std::path::Path, retries: usize, page_size: usize) -> Result<(), Error> {
    Acquirer::with_options(
        fake,
        AcquisitionOptions {
            transient_retries: retries,
            page_size,
        },
    )?
    .acquire("account-1", "run-1", path)
    .map(|_| ())
}

fn acquired_rows(staged: &StagedAccount, dataset: &str) -> u64 {
    staged
        .datasets()
        .unwrap()
        .into_iter()
        .find(|item| item.name == dataset)
        .unwrap()
        .acquired_rows
}

#[test]
fn ordinary_multiple_empty_and_incremental_read() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("staging.db");
    let fake = Fake::new(&[("Card", 5), ("DocSettings", 0), ("Workspace", 2)]);
    run(fake.clone(), &path, 0, 2).unwrap();
    let staged = StagedAccount::open(&path).unwrap();
    assert!(staged.is_complete().unwrap());
    let datasets = staged.datasets().unwrap();
    assert_eq!(datasets.len(), required_dataset_names().len());
    assert!(
        datasets
            .iter()
            .all(|d| d.complete && d.expected_rows == d.acquired_rows)
    );
    assert!(staged.read_rows("DocSettings", None, 2).unwrap().is_empty());
    let first = staged.read_rows("Card", None, 2).unwrap();
    assert_eq!(first.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2, 4]);
    assert_eq!(first[0].data["Content"], "[]");
    assert_eq!(staged.read_rows("Card", Some(4), 2).unwrap()[0].id, 6);
    let before = fake.calls().len();
    run(fake.clone(), &path, 0, 2).unwrap();
    assert_eq!(fake.calls().len(), before); // offline success needs no source
}

#[test]
fn interruption_restarts_from_committed_checkpoint_and_does_not_duplicate() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("staging.db");
    let fake = Fake::new(&[("Card", 5)]);
    fake.fail("Card", Some(4), false);
    assert!(matches!(
        run(fake.clone(), &path, 0, 2),
        Err(Error::Source(_))
    ));
    let staged = StagedAccount::open(&path).unwrap();
    assert!(!staged.is_complete().unwrap());
    assert!(matches!(
        staged.read_rows("Card", None, 2),
        Err(Error::Incomplete)
    ));
    assert_eq!(acquired_rows(&staged, "Card"), 2);
    drop(staged);
    fake.state.lock().unwrap().fail_at = None;
    run(fake.clone(), &path, 0, 2).unwrap();
    assert_eq!(
        &fake.calls_for("Card")[..4],
        &[
            ("Card".into(), None),
            ("Card".into(), Some(4)),
            ("Card".into(), Some(4)),
            ("Card".into(), Some(8))
        ]
    );
    assert_eq!(
        StagedAccount::open(&path)
            .unwrap()
            .read_rows("Card", None, 100)
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn transient_retry_repeats_request_without_duplicate_rows() {
    let temp = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[("Card", 1)]);
    fake.fail("Card", None, true);
    assert!(matches!(
        run(fake.clone(), &temp.path().join("db"), 1, 1),
        Err(Error::Source(_))
    ));
    assert_eq!(fake.calls_for("Card").len(), 2);
    fake.state.lock().unwrap().fail_at = None;
    run(fake.clone(), &temp.path().join("db"), 0, 1).unwrap();
    assert_eq!(
        acquired_rows(
            &StagedAccount::open(temp.path().join("db")).unwrap(),
            "Card"
        ),
        1
    );
}

#[test]
fn first_page_has_no_sentinel_and_negative_sql_int_ids_resume() {
    let temp = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[("Card", 3)]);
    {
        let mut state = fake.state.lock().unwrap();
        state.rows.insert(
            "Card".into(),
            [-3, -2, -1]
                .into_iter()
                .map(|id| sample_row("Card", id))
                .collect(),
        );
        let card = state
            .manifest
            .datasets
            .iter_mut()
            .find(|dataset| dataset.name == "Card")
            .unwrap();
        card.max_id = Some(-1);
    }
    run(fake.clone(), &temp.path().join("db"), 0, 2).unwrap();
    assert_eq!(
        fake.calls_for("Card"),
        vec![("Card".into(), None), ("Card".into(), Some(-2))]
    );
    let rows = StagedAccount::open(temp.path().join("db"))
        .unwrap()
        .read_rows("Card", None, 10)
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [-3, -2, -1]
    );
}

#[test]
fn malformed_order_duplicate_and_premature_completion_are_rejected() {
    for page in [
        Page {
            dataset: "Card".into(),
            rows: vec![json!({"Id":2}), json!({"Id":2})],
            next_after_id: Some(2),
            complete: false,
        },
        Page {
            dataset: "Wrong".into(),
            rows: vec![json!({"Id":2})],
            next_after_id: Some(2),
            complete: false,
        },
        Page {
            dataset: "Card".into(),
            rows: vec![json!({"Id":2})],
            next_after_id: None,
            complete: true,
        },
        Page {
            dataset: "Card".into(),
            rows: vec![],
            next_after_id: Some(0),
            complete: false,
        },
    ] {
        let temp = tempfile::tempdir().unwrap();
        let fake = Fake::new(&[("Card", 3)]);
        fake.state.lock().unwrap().malformed = Some(page);
        assert!(matches!(
            run(fake, &temp.path().join("db"), 0, 2),
            Err(Error::Invalid(_))
        ));
        assert!(
            !StagedAccount::open(temp.path().join("db"))
                .unwrap()
                .is_complete()
                .unwrap()
        );
    }
}

#[test]
fn manifest_changes_and_wrong_account_cannot_reuse_a_partial_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let fake = Fake::new(&[("Card", 3)]);
    fake.fail("Card", None, false);
    run(fake.clone(), &path, 0, 2).unwrap_err();
    fake.state.lock().unwrap().manifest.datasets[0].ownership = "changed rule".into();
    assert!(matches!(
        run(fake.clone(), &path, 0, 2),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        Acquirer::new(fake.clone()).acquire("other", "run-1", &path),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        Acquirer::new(fake).acquire("account-1", "run-2", &path),
        Err(Error::Conflict(_))
    ));
}

#[test]
fn large_account_stays_in_small_pages() {
    let temp = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[("Card", 10_000)]);
    run(fake.clone(), &temp.path().join("db"), 0, 64).unwrap();
    assert!(fake.calls().len() >= 157);
    assert_eq!(
        acquired_rows(
            &StagedAccount::open(temp.path().join("db")).unwrap(),
            "Card"
        ),
        10_000
    );
}

#[test]
fn inflated_manifest_count_or_max_id_cannot_complete() {
    for alter in ["count", "max"] {
        let temp = tempfile::tempdir().unwrap();
        let fake = Fake::new(&[("Card", 2)]);
        {
            let mut state = fake.state.lock().unwrap();
            let dataset = state
                .manifest
                .datasets
                .iter_mut()
                .find(|dataset| dataset.name == "Card")
                .unwrap();
            if alter == "count" {
                dataset.row_count = 3;
            } else {
                dataset.max_id = Some(6);
            }
        }
        assert!(matches!(
            run(fake, &temp.path().join("db"), 0, 2),
            Err(Error::Invalid(_))
        ));
        assert!(
            !StagedAccount::open(temp.path().join("db"))
                .unwrap()
                .is_complete()
                .unwrap()
        );
    }
}

#[test]
fn changed_manifest_after_partially_staged_page_is_not_discarded() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let fake = Fake::new(&[("Card", 2)]);
    fake.fail("Card", Some(2), false);
    run(fake.clone(), &path, 0, 1).unwrap_err();
    fake.state
        .lock()
        .unwrap()
        .manifest
        .datasets
        .iter_mut()
        .find(|dataset| dataset.name == "Card")
        .unwrap()
        .row_count = 3;
    assert!(matches!(run(fake, &path, 0, 1), Err(Error::Conflict(_))));
    assert_eq!(
        acquired_rows(&StagedAccount::open(path).unwrap(), "Card"),
        1
    );
}

#[test]
fn rows_are_staged_in_legacy_named_typed_tables() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let fake = Fake::new(&[("Card", 1), ("Image", 1), ("TeamResource", 1)]);
    run(fake, &path, 0, 10).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let card: (String, i64, i64) = db
        .query_row(
            "SELECT \"Content\", \"Deleted\", \"WordCount\" FROM \"Card\" WHERE \"Id\"=2",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(card, ("[]".into(), 0, -1));
    let columns: Vec<(String, String, i64)> = db
        .prepare("PRAGMA table_info('Card')")
        .unwrap()
        .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(columns.len(), 11);
    assert!(columns.contains(&("Title".into(), "TEXT".into(), 1)));
    assert!(columns.contains(&("WordCount".into(), "INTEGER".into(), 1)));
    let resource_type: Option<String> = db
        .query_row("SELECT \"ResourceType\" FROM \"TeamResource\"", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(resource_type, None);
    let image_size: i64 = db
        .query_row("SELECT \"ImageSize\" FROM \"Image\"", [], |r| r.get(0))
        .unwrap();
    assert_eq!(image_size, 123);
    let staged = StagedAccount::open(path).unwrap();
    assert_eq!(
        staged.read_rows("TeamResource", None, 1).unwrap()[0].data["ResourceType"],
        serde_json::Value::Null
    );
    assert_eq!(
        staged.read_rows("Card", None, 1).unwrap()[0].data["Deleted"],
        false
    );
}

#[test]
fn missing_extra_and_wrong_typed_columns_do_not_commit_page() {
    for corrupt in ["missing", "extra", "type", "null"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let fake = Fake::new(&[("Card", 1)]);
        let mut s = fake.state.lock().unwrap();
        let data = s.rows.get_mut("Card").unwrap()[0].as_object_mut().unwrap();
        match corrupt {
            "missing" => {
                data.remove("WordCount");
            }
            "extra" => {
                data.insert("Unknown".into(), json!("x"));
            }
            "type" => {
                data.insert("Deleted".into(), json!(0));
            }
            _ => {
                data.insert("Title".into(), serde_json::Value::Null);
            }
        }
        drop(s);
        assert!(matches!(run(fake, &path, 0, 10), Err(Error::Invalid(_))));
        assert_eq!(
            acquired_rows(&StagedAccount::open(path).unwrap(), "Card"),
            0
        );
    }
}

#[test]
fn unknown_legacy_table_is_not_silently_staged_as_json() {
    let temp = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[("Card", 0)]);
    fake.state.lock().unwrap().manifest.datasets[0].name = "NotInReference".into();
    assert!(matches!(
        run(fake, &temp.path().join("db"), 0, 10),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn missing_v4_or_v5_dataset_blocks_completion() {
    for missing in ["SpeareDocs", "SpeareUserSettings", "Card", "Workspace"] {
        let temp = tempfile::tempdir().unwrap();
        let fake = Fake::new(&[("Card", 0)]);
        fake.state
            .lock()
            .unwrap()
            .manifest
            .datasets
            .retain(|d| d.name != missing);
        assert!(matches!(
            run(fake, &temp.path().join("db"), 0, 10),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn all_five_dbo_tables_stage_real_columns_and_distinct_bit_and_int_flags() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db");
    let fake = Fake::new(&[
        ("SpeareDocs", 1),
        ("Speare_Blocks", 1),
        ("SpeareWorkspaces", 1),
        ("Speare_WorkspaceTrees", 1),
        ("SpeareUserSettings", 1),
    ]);
    run(fake, &path, 0, 2).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let (document, pinned): (String, Option<i64>) = db
        .query_row(
            "SELECT \"Document\", \"Pinned\" FROM \"SpeareDocs\"",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(document, "{\"text\":\"hello\"}");
    assert_eq!(pinned, None);
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM \"Speare_Blocks\" WHERE \"Deleted\"=0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    let title: String = db
        .query_row("SELECT \"Title\" FROM \"SpeareWorkspaces\"", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(title, "Legacy workspace");
    let tree: (i64, i64, String) = db
        .query_row(
            "SELECT \"NodeHidden\", \"Deleted\", \"SecretKey\" FROM \"Speare_WorkspaceTrees\"",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(tree, (1, 0, "test-secret".into()));
    let settings: Option<String> = db
        .query_row("SELECT \"Settings\" FROM \"SpeareUserSettings\"", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(settings, None);
    let staged = StagedAccount::open(path).unwrap();
    assert_eq!(
        staged.read_rows("Speare_WorkspaceTrees", None, 1).unwrap()[0].data["NodeHidden"],
        true
    );
    assert_eq!(
        staged.read_rows("Speare_WorkspaceTrees", None, 1).unwrap()[0].data["Deleted"],
        0
    );
}
