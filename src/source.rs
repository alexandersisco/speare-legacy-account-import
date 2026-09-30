use serde::{Deserialize, Serialize};

/// Complete metadata for the authenticated account's continuously frozen source view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub datasets: Vec<Dataset>,
    pub pagination: Pagination,
    pub consistency: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pagination {
    pub default_limit: usize,
    pub max_limit: usize,
    pub continuation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dataset {
    pub name: String,
    pub source_table: String,
    pub ownership: String,
    pub fields: Vec<Field>,
    pub key: String,
    pub order: String,
    pub row_count: u64,
    /// Largest SQL `INT` Id, or `None` for an empty dataset.
    pub max_id: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub nullable: bool,
}

/// Rows are unwrapped JSON objects containing every original legacy column.
pub type SourceRow = serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub dataset: String,
    pub rows: Vec<SourceRow>,
    pub next_after_id: Option<i32>,
    pub complete: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("temporary source failure: {0}")]
    Transient(String),
    #[error("permanent source failure: {0}")]
    Permanent(String),
}

/// A fake can implement this trait without HTTP. The authenticated account is resolved by the
/// source; account and run identifiers are deliberately not sent through this interface.
pub trait LegacySource {
    fn manifest(&self) -> std::result::Result<Manifest, SourceError>;
    fn page(
        &self,
        dataset: &str,
        after_id: Option<i32>,
        limit: usize,
    ) -> std::result::Result<Page, SourceError>;
}
