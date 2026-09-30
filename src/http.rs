use std::{io::Read, time::Duration};

use reqwest::{Url, blocking::Client, header::AUTHORIZATION};
use serde::de::DeserializeOwned;

use crate::{LegacySource, Manifest, Page, SourceError};

/// Blocking REST adapter. Use a bearer-token convenience constructor or provide the surrounding
/// application's authenticated client. Production callers must use HTTPS.
pub struct HttpLegacySource {
    base: Url,
    bearer_token: Option<String>,
    client: Client,
}

impl HttpLegacySource {
    /// Use the exporter's default `/v1` endpoint prefix.
    pub fn new(base_url: &str, bearer_token: impl Into<String>) -> Result<Self, SourceError> {
        Self::with_endpoint_prefix(base_url, "/v1", bearer_token)
    }

    /// Append a configurable exporter prefix (for example `/api/migration/export/v1`) to
    /// `base_url`. The surrounding application remains responsible for supplying its valid token.
    pub fn with_endpoint_prefix(
        base_url: &str,
        endpoint_prefix: &str,
        bearer_token: impl Into<String>,
    ) -> Result<Self, SourceError> {
        let base = endpoint_base(base_url, endpoint_prefix)?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| SourceError::Permanent(e.to_string()))?;
        Ok(Self {
            base,
            bearer_token: Some(bearer_token.into()),
            client,
        })
    }

    /// Use a client configured by the surrounding application with its authentication headers,
    /// cookies, timeout, and transport policy.
    pub fn with_authenticated_client(
        base_url: &str,
        endpoint_prefix: &str,
        client: Client,
    ) -> Result<Self, SourceError> {
        Ok(Self {
            base: endpoint_base(base_url, endpoint_prefix)?,
            bearer_token: None,
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
        if !query.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        }
        let mut request = self.client.get(url);
        if let Some(token) = &self.bearer_token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = request.send().map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(if status.as_u16() == 503 {
                SourceError::Transient(format!("HTTP {status}"))
            } else {
                SourceError::Permanent(format!("HTTP {status}"))
            });
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

fn endpoint_base(base_url: &str, endpoint_prefix: &str) -> Result<Url, SourceError> {
    let mut base = Url::parse(base_url).map_err(|e| SourceError::Permanent(e.to_string()))?;
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
    let prefix = endpoint_prefix
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if prefix.is_empty()
        || prefix
            .iter()
            .any(|segment| *segment == "." || *segment == "..")
        || endpoint_prefix.contains(['?', '#'])
    {
        return Err(SourceError::Permanent(
            "endpoint prefix must be a nonempty URL path".into(),
        ));
    }
    base.path_segments_mut()
        .map_err(|_| SourceError::Permanent("invalid base URL".into()))?
        .pop_if_empty()
        .extend(prefix);
    Ok(base)
}

fn transport_error(e: reqwest::Error) -> SourceError {
    if e.is_timeout() || e.is_connect() || e.is_body() {
        SourceError::Transient(e.to_string())
    } else {
        SourceError::Permanent(e.to_string())
    }
}

impl LegacySource for HttpLegacySource {
    fn manifest(&self) -> Result<Manifest, SourceError> {
        self.get(&["manifest"], &[])
    }

    fn page(
        &self,
        dataset: &str,
        after_id: Option<i32>,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let mut query = Vec::with_capacity(2);
        if let Some(after_id) = after_id {
            query.push(("after_id", after_id.to_string()));
        }
        query.push(("limit", limit.to_string()));
        self.get(&["datasets", dataset], &query)
    }
}
