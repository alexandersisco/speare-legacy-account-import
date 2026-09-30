use std::{collections::HashSet, path::Path, thread, time::Duration};

use crate::legacy_schema;
use crate::{
    Dataset, Error, LegacySource, Manifest, Page, Result, SourceError, StagedAccount, StagedDataset,
};

#[derive(Clone, Debug)]
pub struct AcquisitionOptions {
    /// Number of rows requested per page; hard capped to bound memory.
    pub page_size: usize,
    /// Additional attempts for transient failures (not including the original call).
    pub transient_retries: usize,
}

impl Default for AcquisitionOptions {
    fn default() -> Self {
        Self {
            page_size: 100,
            transient_retries: 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub complete: bool,
    pub datasets: Vec<StagedDataset>,
}

pub struct Acquirer<S> {
    source: S,
    options: AcquisitionOptions,
}

impl<S: LegacySource> Acquirer<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            options: AcquisitionOptions::default(),
        }
    }

    pub fn with_options(source: S, options: AcquisitionOptions) -> Result<Self> {
        if !(1..=1000).contains(&options.page_size) || options.transient_retries > 20 {
            return Err(Error::Invalid(
                "page_size must be 1..=1000 and transient_retries <= 20".into(),
            ));
        }
        Ok(Self { source, options })
    }

    /// Run or resume one local migration run at `path`. Reuse the same run ID after an
    /// interruption; use a new run ID and staging file after the source write barrier lapses.
    pub fn acquire(
        &self,
        account_id: &str,
        migration_run_id: &str,
        path: impl AsRef<Path>,
    ) -> Result<Progress> {
        if account_id.is_empty() || migration_run_id.is_empty() {
            return Err(Error::Invalid("empty account or migration-run ID".into()));
        }
        let mut staged = StagedAccount::create(path)?;
        if let Some(existing) = staged.account_id()? {
            if existing != account_id {
                return Err(Error::Conflict(
                    "staging file belongs to another account".into(),
                ));
            }
            if staged.migration_run_id()?.as_deref() != Some(migration_run_id) {
                return Err(Error::Conflict(
                    "staging file belongs to another migration run".into(),
                ));
            }
            if staged.is_complete()? {
                return Ok(Progress {
                    complete: true,
                    datasets: staged.datasets()?,
                });
            }
        }
        let manifest = self.retry(|| self.source.manifest())?;
        validate_manifest(&manifest)?;
        staged.initialize(account_id, migration_run_id, &manifest)?;
        for dataset in &manifest.datasets {
            let (mut after_id, mut count, complete) = staged.checkpoint(&dataset.name)?;
            if complete {
                continue;
            }
            loop {
                let page = self.retry(|| {
                    self.source
                        .page(&dataset.name, after_id, self.options.page_size)
                })?;
                validate_page(dataset, after_id, count, &page, self.options.page_size)?;
                staged.persist_page(dataset, after_id, &page)?;
                count += page.rows.len() as u64;
                after_id = page.next_after_id;
                if page.complete {
                    break;
                }
            }
        }
        let current = self.retry(|| self.source.manifest())?;
        if current != manifest {
            return Err(Error::Conflict(
                "source export changed during acquisition".into(),
            ));
        }
        staged.finish()?;
        Ok(Progress {
            complete: true,
            datasets: staged.datasets()?,
        })
    }

    fn retry<T>(&self, mut call: impl FnMut() -> std::result::Result<T, SourceError>) -> Result<T> {
        for attempt in 0..=self.options.transient_retries {
            match call() {
                Ok(value) => return Ok(value),
                Err(SourceError::Transient(_)) if attempt < self.options.transient_retries => {
                    thread::sleep(Duration::from_millis(100 * (1_u64 << attempt.min(5))));
                }
                Err(e) => return Err(e.into()),
            }
        }
        unreachable!()
    }
}

fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.datasets.is_empty()
        || m.pagination.default_limit != 100
        || m.pagination.max_limit != 1000
        || m.pagination.continuation != "exclusive after_id"
        || m.consistency != "application-quiesced"
    {
        return Err(Error::Invalid(
            "invalid manifest pagination or consistency contract".into(),
        ));
    }
    let mut names = HashSet::new();
    for d in &m.datasets {
        let table = legacy_schema::table(&d.name)?;
        if d.name.is_empty()
            || !names.insert(d.name.as_str())
            || d.source_table != table.source_table
            || d.ownership.is_empty()
            || d.key != "Id"
            || d.order != "ascending"
            || (d.row_count == 0) != d.max_id.is_none()
            || d.row_count > i64::MAX as u64
            || d.fields.len() != table.columns.len()
            || d.fields.iter().zip(table.columns).any(|(field, column)| {
                field.name != column.name
                    || !field.source_type.eq_ignore_ascii_case(column.source_type)
                    || field.nullable != column.nullable
            })
        {
            return Err(Error::Invalid(format!(
                "invalid dataset descriptor: {}",
                d.name
            )));
        }
    }
    for table in legacy_schema::TABLES {
        if !names.contains(table.name) {
            return Err(Error::Invalid(format!(
                "required legacy dataset {} is missing from manifest",
                table.name
            )));
        }
    }
    Ok(())
}

fn row_id(row: &serde_json::Value) -> Option<i32> {
    row.as_object()
        .and_then(|object| object.get("Id"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn validate_page(
    d: &Dataset,
    after: Option<i32>,
    count: u64,
    p: &Page,
    limit: usize,
) -> Result<()> {
    let new_count = count.saturating_add(p.rows.len() as u64);
    if p.rows.len() > limit
        || p.dataset != d.name
        || (!p.complete && p.rows.is_empty())
        || new_count > d.row_count
        || (!p.complete && new_count == d.row_count)
        || (p.complete && p.next_after_id.is_some())
    {
        return Err(Error::Invalid(format!(
            "invalid page size or count for {}",
            d.name
        )));
    }
    let mut last = after;
    for row in &p.rows {
        let Some(id) = row_id(row) else {
            return Err(Error::Invalid(format!("invalid row Id in {}", d.name)));
        };
        if last.is_some_and(|previous| id <= previous) || d.max_id.is_some_and(|max| id > max) {
            return Err(Error::Invalid(format!(
                "invalid row Id or order in {}",
                d.name
            )));
        }
        last = Some(id);
    }
    if !p.complete && p.next_after_id != last {
        return Err(Error::Invalid(format!(
            "invalid continuation in {}",
            d.name
        )));
    }
    if p.complete && (new_count != d.row_count || last.or(after) != d.max_id) {
        return Err(Error::Invalid(format!(
            "premature completion in {}",
            d.name
        )));
    }
    Ok(())
}
