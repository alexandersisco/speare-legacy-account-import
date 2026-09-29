//! Durable, resumable acquisition of a legacy account. No legacy-to-current-model conversion
//! occurs here. See `MIGRATION_API.md` for the source protocol and its consistency requirements.

mod acquisition;
mod http;
mod legacy_schema;
mod source;
mod staging;

pub use acquisition::{Acquirer, AcquisitionOptions, Progress};
pub use http::HttpLegacySource;
pub use source::{Dataset, LegacySource, Manifest, Page, SourceError, SourceRow};
pub use staging::{StagedAccount, StagedDataset, StagedRow};

/// Errors are deliberately distinguishable: an interrupted acquisition remains resumable;
/// an invalid response or changed manifest requires investigation rather than a blind retry.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("staging database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("source: {0}")]
    Source(#[from] SourceError),
    #[error("invalid source data: {0}")]
    Invalid(String),
    #[error("staged account conflict: {0}")]
    Conflict(String),
    #[error("account acquisition is incomplete")]
    Incomplete,
}

pub type Result<T> = std::result::Result<T, Error>;
