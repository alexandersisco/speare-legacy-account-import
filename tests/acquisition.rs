use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use serde_json::json;
use speare_legacy_account_import::{
    Acquirer, AcquisitionOptions, Dataset, Error, LegacySource, Manifest, Page, SourceError,
    SourceRow, StagedAccount, required_dataset_names,
};

#[derive(Clone)]
struct Fake {
    state: Arc<Mutex<State>>,
}

struct State {
    manifest: Manifest,
    rows: BTreeMap<String, Vec<SourceRow>>,
    calls: Vec<(String, i64)>,
    fail_at: Option<(String, i64, bool)>, // after_id, transient?
    malformed: Option<Page>,
}

impl Fake {
    fn new(sizes: &[(&str, usize)]) -> Self {
        let mut datasets = Vec::new();
        let mut rows = BTreeMap::new();
        for (name, n) in sizes {
            datasets.push(Dataset {
                name: (*name).into(),
                row_count: *n as u64,
                max_id: *n as i64 * 2,
            });
            rows.insert(
                (*name).into(),
                (1..=*n)
                    .map(|i| SourceRow {
                        id: i as i64 * 2,
                        data: sample_row(name, i as i64 * 2),
                    })
                    .collect(),
            );
        }
        for name in required_dataset_names() {
            if !rows.contains_key(name) {
                datasets.push(Dataset {
                    name: name.into(),
                    row_count: 0,
                    max_id: 0,
                });
                rows.insert(name.into(), Vec::new());
            }
        }
        let manifest = Manifest {
            account_id: "account-1".into(),
            export_id: "view-1".into(),
            datasets,
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

    fn fail(&self, dataset: &str, after: i64, transient: bool) {
        self.state.lock().unwrap().fail_at = Some((dataset.into(), after, transient));
    }

    fn calls(&self) -> Vec<(String, i64)> {
        self.state.lock().unwrap().calls.clone()
    }
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
    fn manifest(&self, _: &str) -> Result<Manifest, SourceError> {
        Ok(self.state.lock().unwrap().manifest.clone())
    }

    fn page(
        &self,
        _: &str,
        export: &str,
        dataset: &str,
        after: i64,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let mut s = self.state.lock().unwrap();
        if s.manifest.export_id != export {
            return Err(SourceError::Permanent("invalid export".into()));
        }
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
            .filter(|r| r.id > after)
            .take(limit)
            .cloned()
            .collect();
        let complete = rows
            .last()
            .is_none_or(|last| last.id == all.last().unwrap().id);
        Ok(Page { rows, complete })
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
    .acquire("account-1", path)
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
    assert!(staged.read_rows("DocSettings", 0, 2).unwrap().is_empty());
    let first = staged.read_rows("Card", 0, 2).unwrap();
    assert_eq!(first.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2, 4]);
    assert_eq!(first[0].data["Content"], "[]");
    assert_eq!(staged.read_rows("Card", 4, 2).unwrap()[0].id, 6);
    let before = fake.calls().len();
    run(fake.clone(), &path, 0, 2).unwrap();
    assert_eq!(fake.calls().len(), before); // offline success needs no source
}

#[test]
fn interruption_restarts_from_committed_checkpoint_and_does_not_duplicate() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("staging.db");
    let fake = Fake::new(&[("Card", 5)]);
    fake.fail("Card", 4, false);
    assert!(matches!(
        run(fake.clone(), &path, 0, 2),
        Err(Error::Source(_))
    ));
    let staged = StagedAccount::open(&path).unwrap();
    assert!(!staged.is_complete().unwrap());
    assert!(matches!(
        staged.read_rows("Card", 0, 2),
        Err(Error::Incomplete)
    ));
    assert_eq!(acquired_rows(&staged, "Card"), 2);
    drop(staged);
    fake.state.lock().unwrap().fail_at = None;
    run(fake.clone(), &path, 0, 2).unwrap();
    assert_eq!(
        &fake.calls()[..4],
        &[
            ("Card".into(), 0),
            ("Card".into(), 4),
            ("Card".into(), 4),
            ("Card".into(), 8)
        ]
    );
    assert_eq!(
        StagedAccount::open(&path)
            .unwrap()
            .read_rows("Card", 0, 100)
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn transient_retry_repeats_request_without_duplicate_rows() {
    let temp = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[("Card", 1)]);
    fake.fail("Card", 0, true);
    assert!(matches!(
        run(fake.clone(), &temp.path().join("db"), 1, 1),
        Err(Error::Source(_))
    ));
    assert_eq!(fake.calls().len(), 2);
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
fn malformed_order_duplicate_and_premature_completion_are_rejected() {
    for page in [
        Page {
            rows: vec![
                SourceRow {
                    id: 2,
                    data: json!({"Id":2}),
                },
                SourceRow {
                    id: 2,
                    data: json!({"Id":2}),
                },
            ],
            complete: false,
        },
        Page {
            rows: vec![SourceRow {
                id: 2,
                data: json!({"Id":999}),
            }],
            complete: false,
        },
        Page {
            rows: vec![SourceRow {
                id: 2,
                data: json!({"Id":2}),
            }],
            complete: true,
        },
        Page {
            rows: vec![],
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
    fake.fail("Card", 0, false);
    run(fake.clone(), &path, 0, 2).unwrap_err();
    fake.state.lock().unwrap().manifest.export_id = "view-2".into();
    assert!(matches!(
        run(fake.clone(), &path, 0, 2),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        Acquirer::new(fake).acquire("other", &path),
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
            if alter == "count" {
                state.manifest.datasets[0].row_count = 3;
            } else {
                state.manifest.datasets[0].max_id = 6;
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
    fake.fail("Card", 2, false);
    run(fake.clone(), &path, 0, 1).unwrap_err();
    fake.state.lock().unwrap().manifest.datasets[0].row_count = 3;
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
        staged.read_rows("TeamResource", 0, 1).unwrap()[0].data["ResourceType"],
        serde_json::Value::Null
    );
    assert_eq!(
        staged.read_rows("Card", 0, 1).unwrap()[0].data["Deleted"],
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
        let data = s.rows.get_mut("Card").unwrap()[0]
            .data
            .as_object_mut()
            .unwrap();
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
        staged.read_rows("Speare_WorkspaceTrees", 0, 1).unwrap()[0].data["NodeHidden"],
        true
    );
    assert_eq!(
        staged.read_rows("Speare_WorkspaceTrees", 0, 1).unwrap()[0].data["Deleted"],
        0
    );
}
