use serde::{Deserialize, Serialize};

/// Manifest is the complete, authoritative list of required datasets for this export.
/// `export_id` names a stable source view; a changed view must use a different ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub account_id: String,
    pub export_id: String,
    pub datasets: Vec<Dataset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    pub name: String,
    pub row_count: u64,
    /// Inclusive upper bound on legacy SQL Server `Id`; 0 for an empty dataset.
    pub max_id: i64,
}

/// The payload contains all legacy column names and values, including `Id`. Large NVARCHAR
/// fields are opaque strings (not parsed into a modern model).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRow {
    pub id: i64,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub rows: Vec<SourceRow>,
    /// True iff there are no more rows in this dataset's export view after this page.
    pub complete: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("temporary source failure: {0}")]
    Transient(String),
    #[error("permanent source failure: {0}")]
    Permanent(String),
}

/// A fake can implement this trait without HTTP. Calls must be repeatable; after_id is exclusive.
/// An export must remain stable across process restarts or return an error, never silently change.
pub trait LegacySource {
    fn manifest(&self, account_id: &str) -> std::result::Result<Manifest, SourceError>;
    fn page(
        &self,
        account_id: &str,
        export_id: &str,
        dataset: &str,
        after_id: i64,
        limit: usize,
    ) -> std::result::Result<Page, SourceError>;
}
