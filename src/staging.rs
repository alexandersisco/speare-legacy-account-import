use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value as SqlValue};

use crate::legacy_schema::{self, quoted};
use crate::{Dataset, Error, Manifest, Page, Result};

/// The staging file is owned by this crate. A completed file can be read without a server.
pub struct StagedAccount {
    pub(crate) db: Connection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedDataset {
    pub name: String,
    pub expected_rows: u64,
    pub acquired_rows: u64,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StagedRow {
    pub id: i64,
    pub data: serde_json::Value,
}

impl StagedAccount {
    /// Open an existing staging file (does not create an empty one).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        configure(&db)?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 1 {
            return Err(Error::Conflict(format!(
                "unsupported staging schema version {version}"
            )));
        }
        Ok(Self { db })
    }

    pub(crate) fn create(path: impl AsRef<Path>) -> Result<Self> {
        let db = Connection::open(path)?;
        configure(&db)?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 0 && version != 1 {
            return Err(Error::Conflict(format!(
                "unsupported staging schema version {version}"
            )));
        }
        if version == 0 {
            db.execute_batch("BEGIN IMMEDIATE;")?;
            let creation = (|| -> Result<()> {
                db.execute_batch(
                "
                 CREATE TABLE export (id INTEGER PRIMARY KEY CHECK (id = 1), account_id TEXT NOT NULL,
                     manifest_json TEXT NOT NULL, complete INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE datasets (name TEXT PRIMARY KEY, expected_rows INTEGER NOT NULL,
                     max_id INTEGER NOT NULL, acquired_rows INTEGER NOT NULL DEFAULT 0,
                     last_id INTEGER NOT NULL DEFAULT 0, complete INTEGER NOT NULL DEFAULT 0);",
                )?;
                legacy_schema::create_tables(&db)?;
                db.execute_batch("PRAGMA user_version = 1;")?;
                Ok(())
            })();
            if let Err(e) = creation {
                db.execute_batch("ROLLBACK;")?;
                return Err(e);
            }
            db.execute_batch("COMMIT;")?;
        }
        Ok(Self { db })
    }

    pub fn account_id(&self) -> Result<Option<String>> {
        Ok(self
            .db
            .query_row("SELECT account_id FROM export WHERE id=1", [], |r| r.get(0))
            .optional()?)
    }

    pub fn is_complete(&self) -> Result<bool> {
        Ok(self
            .db
            .query_row("SELECT complete FROM export WHERE id=1", [], |r| {
                r.get::<_, bool>(0)
            })
            .optional()?
            .unwrap_or(false))
    }

    pub fn datasets(&self) -> Result<Vec<StagedDataset>> {
        let mut stmt = self.db.prepare(
            "SELECT name, expected_rows, acquired_rows, complete FROM datasets ORDER BY name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(StagedDataset {
                name: r.get(0)?,
                expected_rows: r.get::<_, i64>(1)? as u64,
                acquired_rows: r.get::<_, i64>(2)? as u64,
                complete: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// A bounded page for downstream transformation. Only complete exports may be read.
    /// Repeated calls with the last returned Id allow incremental processing.
    pub fn read_rows(&self, dataset: &str, after_id: i64, limit: usize) -> Result<Vec<StagedRow>> {
        if !self.is_complete()? {
            return Err(Error::Incomplete);
        }
        if !(1..=1000).contains(&limit) || after_id < 0 {
            return Err(Error::Invalid(
                "read limit must be 1..=1000 and after_id nonnegative".into(),
            ));
        }
        if self
            .db
            .query_row(
                "SELECT 1 FROM datasets WHERE name=?1",
                [dataset],
                |_| Ok(()),
            )
            .optional()?
            .is_none()
        {
            return Err(Error::Invalid(format!("unknown dataset {dataset}")));
        }
        let table = legacy_schema::table(dataset)?;
        let mut stmt = self.db.prepare(&format!(
            "SELECT * FROM {} WHERE \"Id\">?1 ORDER BY \"Id\" LIMIT ?2",
            quoted(table.name)
        ))?;
        let mut rows = stmt.query(params![after_id, limit as i64])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            let values = (0..table.columns.len())
                .map(|i| row.get::<_, SqlValue>(i))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let data = legacy_schema::json_row(table, values)?;
            let id = data["Id"]
                .as_i64()
                .ok_or_else(|| Error::Conflict("staged Id is invalid".into()))?;
            result.push(StagedRow { id, data });
        }
        Ok(result)
    }

    pub(crate) fn manifest(&self) -> Result<Option<Manifest>> {
        let raw: Option<String> = self
            .db
            .query_row("SELECT manifest_json FROM export WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()?;
        raw.map(|s| {
            serde_json::from_str(&s)
                .map_err(|e| Error::Conflict(format!("invalid staged manifest: {e}")))
        })
        .transpose()
    }

    pub(crate) fn initialize(&mut self, manifest: &Manifest) -> Result<()> {
        if let Some(existing) = self.manifest()? {
            if existing != *manifest {
                return Err(Error::Conflict("source manifest changed; keep the existing file for diagnosis and start a new export in a new file".into()));
            }
            return self.validate_checkpoint();
        }
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO export (id, account_id, manifest_json) VALUES (1, ?1, ?2)",
            params![
                manifest.account_id,
                serde_json::to_string(manifest).map_err(|e| Error::Invalid(e.to_string()))?
            ],
        )?;
        for d in &manifest.datasets {
            tx.execute(
                "INSERT INTO datasets (name, expected_rows, max_id) VALUES (?1, ?2, ?3)",
                params![d.name, d.row_count as i64, d.max_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn checkpoint(&self, name: &str) -> Result<(i64, u64, bool)> {
        let (id, count, complete): (i64, i64, bool) = self.db.query_row(
            "SELECT last_id, acquired_rows, complete FROM datasets WHERE name=?1",
            [name],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok((id, count as u64, complete))
    }

    pub(crate) fn persist_page(
        &mut self,
        dataset: &Dataset,
        old_id: i64,
        page: &Page,
    ) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let (actual_id, count, complete): (i64, i64, bool) = tx.query_row(
            "SELECT last_id, acquired_rows, complete FROM datasets WHERE name=?1",
            [&dataset.name],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if complete || actual_id != old_id {
            return Err(Error::Conflict(
                "checkpoint changed while acquiring page".into(),
            ));
        }
        let mut last = old_id;
        let table = legacy_schema::table(&dataset.name)?;
        let columns = table
            .columns
            .iter()
            .map(|c| quoted(c.name))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=table.columns.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO {} ({columns}) VALUES ({placeholders})",
            quoted(table.name)
        );
        let mut stmt = tx.prepare(&sql)?;
        for row in &page.rows {
            let values = legacy_schema::sql_values(table, &row.data)?;
            stmt.execute(params_from_iter(values))?;
            last = row.id;
        }
        drop(stmt);
        let new_count = count + page.rows.len() as i64;
        tx.execute(
            "UPDATE datasets SET last_id=?2, acquired_rows=?3, complete=?4 WHERE name=?1",
            params![dataset.name, last, new_count, page.complete],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.validate_checkpoint()?;
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let remaining: i64 = tx.query_row(
            "SELECT count(*) FROM datasets WHERE complete=0 OR acquired_rows!=expected_rows",
            [],
            |r| r.get(0),
        )?;
        if remaining != 0 {
            return Err(Error::Incomplete);
        }
        tx.execute("UPDATE export SET complete=1 WHERE id=1", [])?;
        tx.commit()?;
        Ok(())
    }

    fn validate_checkpoint(&self) -> Result<()> {
        let mut stmt = self
            .db
            .prepare("SELECT name, acquired_rows, last_id FROM datasets")?;
        let mut result = stmt.query([])?;
        while let Some(row) = result.next()? {
            let name: String = row.get(0)?;
            let expected: i64 = row.get(1)?;
            let last: i64 = row.get(2)?;
            let table = legacy_schema::table(&name)?;
            let (actual, max): (i64, i64) = self.db.query_row(
                &format!(
                    "SELECT count(*), coalesce(max(\"Id\"), 0) FROM {}",
                    quoted(table.name)
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if expected != actual || last != max {
                return Err(Error::Conflict(format!(
                    "staged checkpoint differs from rows for {name}"
                )));
            }
        }
        Ok(())
    }
}

fn configure(db: &Connection) -> Result<()> {
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")?;
    Ok(())
}
