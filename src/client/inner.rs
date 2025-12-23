//! Internal client implementation.

use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "rest")]
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
#[cfg(feature = "rest")]
use serde::{de::DeserializeOwned, Serialize};

use crate::auth::Credentials;
use crate::config::{CacheConfig, DegradationConfig, RetryConfig, TlsConfig};
#[cfg(feature = "rest")]
use crate::error::{Error, ErrorKind};
use crate::transport::TransportClient;

use super::health::ShutdownGuard;

pub(crate) struct ClientInner {
    /// The InferaDB API URL.
    pub url: String,

    /// Authentication credentials.
    pub credentials: Credentials,

    /// Retry configuration.
    pub retry_config: RetryConfig,

    /// Cache configuration.
    pub cache_config: CacheConfig,

    /// TLS configuration.
    pub tls_config: TlsConfig,

    /// Degradation configuration.
    pub degradation_config: DegradationConfig,

    /// Request timeout.
    pub timeout: Duration,

    /// Transport client for Engine API calls.
    #[cfg(any(feature = "grpc", feature = "rest"))]
    pub transport: Option<Arc<dyn TransportClient + Send + Sync>>,

    /// HTTP client for Control API calls.
    #[cfg(feature = "rest")]
    pub http_client: Option<reqwest::Client>,

    /// Current auth token for Control API (cached).
    #[cfg(feature = "rest")]
    pub auth_token: parking_lot::RwLock<Option<String>>,

    /// Shutdown guard for graceful shutdown tracking.
    pub shutdown_guard: Option<ShutdownGuard>,
}

#[cfg(feature = "rest")]
impl ClientInner {
    /// Builds the URL for a Control API endpoint.
    fn build_url(&self, path: &str) -> Result<url::Url, Error> {
        let base = url::Url::parse(&self.url).map_err(|e| {
            Error::new(ErrorKind::Configuration, format!("Invalid base URL: {}", e))
        })?;
        base.join(path)
            .map_err(|e| Error::new(ErrorKind::Configuration, format!("Invalid URL path: {}", e)))
    }

    /// Builds headers for Control API requests.
    fn build_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(ref token) = *self.auth_token.read() {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_value).map_err(|_| {
                    Error::new(ErrorKind::Authentication, "Invalid auth token format")
                })?,
            );
        }

        Ok(headers)
    }

    /// Returns the HTTP client, or an error if not available.
    fn http_client(&self) -> Result<&reqwest::Client, Error> {
        self.http_client
            .as_ref()
            .ok_or_else(|| Error::new(ErrorKind::Configuration, "HTTP client not available"))
    }

    /// Makes a GET request to the Control API.
    pub(crate) async fn control_get<R>(&self, path: &str) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client()?
            .get(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Transport, format!("Request failed: {}", e)))?;

        self.handle_response(response).await
    }

    /// Makes a POST request to the Control API.
    pub(crate) async fn control_post<T, R>(&self, path: &str, body: &T) -> Result<R, Error>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client()?
            .post(url)
            .headers(headers)
            .json(body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Transport, format!("Request failed: {}", e)))?;

        self.handle_response(response).await
    }

    /// Makes a POST request to the Control API without a body.
    pub(crate) async fn control_post_empty<R>(&self, path: &str) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client()?
            .post(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Transport, format!("Request failed: {}", e)))?;

        self.handle_response(response).await
    }

    /// Makes a PATCH request to the Control API.
    pub(crate) async fn control_patch<T, R>(&self, path: &str, body: &T) -> Result<R, Error>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client()?
            .patch(url)
            .headers(headers)
            .json(body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Transport, format!("Request failed: {}", e)))?;

        self.handle_response(response).await
    }

    /// Makes a DELETE request to the Control API.
    pub(crate) async fn control_delete(&self, path: &str) -> Result<(), Error> {
        let url = self.build_url(path)?;
        let headers = self.build_headers()?;

        let response = self
            .http_client()?
            .delete(url)
            .headers(headers)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Transport, format!("Request failed: {}", e)))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(self.map_status_error(status, &body))
        }
    }

    /// Handles an HTTP response.
    async fn handle_response<R>(&self, response: reqwest::Response) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let status = response.status();
        if status.is_success() {
            response.json().await.map_err(|e| {
                Error::new(
                    ErrorKind::InvalidResponse,
                    format!("Failed to parse response: {}", e),
                )
            })
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(self.map_status_error(status, &body))
        }
    }

    /// Maps HTTP status codes to errors.
    fn map_status_error(&self, status: reqwest::StatusCode, body: &str) -> Error {
        match status.as_u16() {
            400 => Error::new(ErrorKind::InvalidArgument, format!("Bad request: {}", body)),
            401 => Error::new(ErrorKind::Authentication, "Authentication required"),
            403 => Error::new(ErrorKind::PermissionDenied, "Permission denied"),
            404 => Error::new(ErrorKind::NotFound, format!("Not found: {}", body)),
            409 => Error::new(ErrorKind::Conflict, format!("Conflict: {}", body)),
            429 => Error::new(ErrorKind::RateLimited, "Rate limit exceeded"),
            500..=599 => Error::new(ErrorKind::Internal, format!("Server error: {}", body)),
            _ => Error::new(ErrorKind::Transport, format!("HTTP {}: {}", status, body)),
        }
    }
}
