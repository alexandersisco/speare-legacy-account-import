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

    /// Run or resume an export at `path`. An error leaves all committed pages available for
    /// another call, including after a process restart. Keep one staging file per account.
    pub fn acquire(&self, account_id: &str, path: impl AsRef<Path>) -> Result<Progress> {
        if account_id.is_empty() {
            return Err(Error::Invalid("empty account ID".into()));
        }
        let mut staged = StagedAccount::create(path)?;
        if let Some(existing) = staged.account_id()? {
            if existing != account_id {
                return Err(Error::Conflict(
                    "staging file belongs to another account".into(),
                ));
            }
            if staged.is_complete()? {
                return Ok(Progress {
                    complete: true,
                    datasets: staged.datasets()?,
                });
            }
        }
        let manifest = self.retry(|| self.source.manifest(account_id))?;
        validate_manifest(&manifest, account_id)?;
        staged.initialize(&manifest)?;
        for dataset in &manifest.datasets {
            let (mut after_id, mut count, complete) = staged.checkpoint(&dataset.name)?;
            if complete {
                continue;
            }
            loop {
                let page = self.retry(|| {
                    self.source.page(
                        account_id,
                        &manifest.export_id,
                        &dataset.name,
                        after_id,
                        self.options.page_size,
                    )
                })?;
                validate_page(dataset, after_id, count, &page, self.options.page_size)?;
                staged.persist_page(dataset, after_id, &page)?;
                count += page.rows.len() as u64;
                if let Some(last) = page.rows.last() {
                    after_id = last.id;
                }
                if page.complete {
                    break;
                }
            }
        }
        // Detect an invalidated view before declaring success. The server must also honor its
        // export_id contract: rechecking counts alone cannot detect in-place row edits.
        let current = self.retry(|| self.source.manifest(account_id))?;
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

fn validate_manifest(m: &Manifest, requested: &str) -> Result<()> {
    if m.account_id != requested || m.export_id.is_empty() || m.datasets.is_empty() {
        return Err(Error::Invalid(
            "manifest has wrong account, empty export ID, or no datasets".into(),
        ));
    }
    let mut names = HashSet::new();
    for d in &m.datasets {
        legacy_schema::table(&d.name)?;
        if d.name.is_empty()
            || !names.insert(&d.name)
            || d.row_count > i64::MAX as u64
            || (d.row_count == 0 && d.max_id != 0)
            || (d.row_count > 0 && d.max_id <= 0)
            || (d.max_id > 0 && d.row_count > d.max_id as u64)
        {
            return Err(Error::Invalid(format!(
                "invalid dataset descriptor: {}",
                d.name
            )));
        }
    }
    Ok(())
}

fn validate_page(d: &Dataset, after: i64, count: u64, p: &Page, limit: usize) -> Result<()> {
    if p.rows.len() > limit
        || (!p.complete && p.rows.is_empty())
        || count.saturating_add(p.rows.len() as u64) > d.row_count
    {
        return Err(Error::Invalid(format!(
            "invalid page size or count for {}",
            d.name
        )));
    }
    let mut last = after;
    for row in &p.rows {
        if row.id <= last
            || row.id > d.max_id
            || row
                .data
                .as_object()
                .and_then(|o| o.get("Id"))
                .and_then(|v| v.as_i64())
                != Some(row.id)
        {
            return Err(Error::Invalid(format!(
                "invalid row Id or order in {}",
                d.name
            )));
        }
        last = row.id;
    }
    if p.complete && (count + p.rows.len() as u64 != d.row_count || last != d.max_id) {
        return Err(Error::Invalid(format!(
            "premature completion in {}",
            d.name
        )));
    }
    Ok(())
}
