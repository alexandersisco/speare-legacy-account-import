use std::{io::Read, time::Duration};

use reqwest::{Url, blocking::Client, header::AUTHORIZATION};
use serde::de::DeserializeOwned;

use crate::{LegacySource, Manifest, Page, SourceError};

/// Blocking REST adapter. Instantiate with a server base URL and an account-scoped bearer token.
/// Production callers must use HTTPS. Tokens are never persisted in the staging database.
pub struct HttpLegacySource {
    base: Url,
    bearer_token: String,
    client: Client,
}

impl HttpLegacySource {
    pub fn new(base_url: &str, bearer_token: impl Into<String>) -> Result<Self, SourceError> {
        let base = Url::parse(base_url).map_err(|e| SourceError::Permanent(e.to_string()))?;
        if !matches!(base.scheme(), "http" | "https")
            || base.cannot_be_a_base()
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.username().is_empty()
            || base.password().is_some()
        {
            return Err(SourceError::Permanent(
                "expected an HTTP(S) base URL without credentials, query or fragment".into(),
            ));
        }
        if base.scheme() == "http"
            && !matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        {
            return Err(SourceError::Permanent(
                "HTTPS is required outside loopback development".into(),
            ));
        }
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| SourceError::Permanent(e.to_string()))?;
        Ok(Self {
            base,
            bearer_token: bearer_token.into(),
            client,
        })
    }

    fn get<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<T, SourceError> {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .map_err(|_| SourceError::Permanent("invalid base URL".into()))?
            .pop_if_empty()
            .extend(segments);
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.bearer_token))
            .send()
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(
                if status.is_server_error() || status.as_u16() == 429 || status.as_u16() == 408 {
                    SourceError::Transient(format!("HTTP {status}"))
                } else {
                    SourceError::Permanent(format!("HTTP {status}"))
                },
            );
        }
        // A single row may contain large legacy content; still enforce a hard network bound.
        const MAX_RESPONSE: u64 = 32 * 1024 * 1024;
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| SourceError::Transient(e.to_string()))?;
        if bytes.len() as u64 > MAX_RESPONSE {
            return Err(SourceError::Permanent(
                "response exceeds 32 MiB; reduce page size or provide a smaller-row export".into(),
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|e| SourceError::Permanent(format!("invalid JSON response: {e}")))
    }
}

fn transport_error(e: reqwest::Error) -> SourceError {
    if e.is_timeout() || e.is_connect() || e.is_body() {
        SourceError::Transient(e.to_string())
    } else {
        SourceError::Permanent(e.to_string())
    }
}

impl LegacySource for HttpLegacySource {
    fn manifest(&self, account_id: &str) -> Result<Manifest, SourceError> {
        self.get(&["v1", "legacy-exports", account_id], &[])
    }

    fn page(
        &self,
        account_id: &str,
        export_id: &str,
        dataset: &str,
        after_id: i64,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.get(
            &[
                "v1",
                "legacy-exports",
                account_id,
                export_id,
                "datasets",
                dataset,
                "rows",
            ],
            &[
                ("after_id", after_id.to_string()),
                ("limit", limit.to_string()),
            ],
        )
    }
}
