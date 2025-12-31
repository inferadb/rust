//! Internal client implementation.

#[cfg(any(feature = "grpc", feature = "rest"))]
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "rest")]
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
#[cfg(feature = "rest")]
use serde::{Serialize, de::DeserializeOwned};

use super::health::ShutdownGuard;
#[cfg(feature = "rest")]
use crate::error::{Error, ErrorKind};
#[cfg(any(feature = "grpc", feature = "rest"))]
use crate::transport::TransportClient;
use crate::{
    auth::Credentials,
    config::{CacheConfig, DegradationConfig, RetryConfig, TlsConfig},
};

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
                    Error::new(ErrorKind::Unauthorized, "Invalid auth token format")
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
                Error::new(ErrorKind::InvalidResponse, format!("Failed to parse response: {}", e))
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
            401 => Error::new(ErrorKind::Unauthorized, "Authentication required"),
            403 => Error::new(ErrorKind::Forbidden, "Permission denied"),
            404 => Error::new(ErrorKind::NotFound, format!("Not found: {}", body)),
            409 => Error::new(ErrorKind::Conflict, format!("Conflict: {}", body)),
            429 => Error::new(ErrorKind::RateLimited, "Rate limit exceeded"),
            500..=599 => Error::new(ErrorKind::Internal, format!("Server error: {}", body)),
            _ => Error::new(ErrorKind::Transport, format!("HTTP {}: {}", status, body)),
        }
    }
}

#[cfg(all(test, feature = "rest"))]
mod tests {
    use reqwest::StatusCode;

    use super::*;
    use crate::auth::BearerCredentialsConfig;

    fn create_test_inner() -> ClientInner {
        let token = "test_token";
        ClientInner {
            url: "https://api.example.com".to_string(),
            credentials: BearerCredentialsConfig::new(token).into(),
            retry_config: RetryConfig::default(),
            cache_config: CacheConfig::default(),
            tls_config: TlsConfig::default(),
            degradation_config: DegradationConfig::default(),
            timeout: Duration::from_secs(30),
            transport: None,
            http_client: Some(reqwest::Client::new()),
            auth_token: parking_lot::RwLock::new(Some(token.to_string())),
            shutdown_guard: None,
        }
    }

    fn create_test_inner_no_token() -> ClientInner {
        ClientInner {
            url: "https://api.example.com".to_string(),
            credentials: BearerCredentialsConfig::new("test").into(),
            retry_config: RetryConfig::default(),
            cache_config: CacheConfig::default(),
            tls_config: TlsConfig::default(),
            degradation_config: DegradationConfig::default(),
            timeout: Duration::from_secs(30),
            transport: None,
            http_client: Some(reqwest::Client::new()),
            auth_token: parking_lot::RwLock::new(None),
            shutdown_guard: None,
        }
    }

    fn create_test_inner_no_http_client() -> ClientInner {
        ClientInner {
            url: "https://api.example.com".to_string(),
            credentials: BearerCredentialsConfig::new("test").into(),
            retry_config: RetryConfig::default(),
            cache_config: CacheConfig::default(),
            tls_config: TlsConfig::default(),
            degradation_config: DegradationConfig::default(),
            timeout: Duration::from_secs(30),
            transport: None,
            http_client: None,
            auth_token: parking_lot::RwLock::new(None),
            shutdown_guard: None,
        }
    }

    #[test]
    fn test_build_url_valid() {
        let inner = create_test_inner();
        let url = inner.build_url("/api/v1/test").unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/api/v1/test");
    }

    #[test]
    fn test_build_headers_with_token() {
        let inner = create_test_inner();
        let headers = inner.build_headers().unwrap();
        assert!(headers.contains_key("authorization"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("accept"));
    }

    #[test]
    fn test_build_headers_no_token() {
        let inner = create_test_inner_no_token();
        let headers = inner.build_headers().unwrap();
        assert!(!headers.contains_key("authorization"));
        assert!(headers.contains_key("content-type"));
    }

    #[test]
    fn test_http_client_available() {
        let inner = create_test_inner();
        assert!(inner.http_client().is_ok());
    }

    #[test]
    fn test_http_client_not_available() {
        let inner = create_test_inner_no_http_client();
        let result = inner.http_client();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Configuration));
    }

    #[test]
    fn test_map_status_error_400() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::BAD_REQUEST, "Invalid input");
        assert!(matches!(error.kind(), ErrorKind::InvalidArgument));
        assert!(error.to_string().contains("Invalid input"));
    }

    #[test]
    fn test_map_status_error_401() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::UNAUTHORIZED, "");
        assert!(matches!(error.kind(), ErrorKind::Unauthorized));
    }

    #[test]
    fn test_map_status_error_403() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::FORBIDDEN, "");
        assert!(matches!(error.kind(), ErrorKind::Forbidden));
    }

    #[test]
    fn test_map_status_error_404() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::NOT_FOUND, "Resource xyz");
        assert!(matches!(error.kind(), ErrorKind::NotFound));
        assert!(error.to_string().contains("xyz"));
    }

    #[test]
    fn test_map_status_error_409() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::CONFLICT, "Already exists");
        assert!(matches!(error.kind(), ErrorKind::Conflict));
    }

    #[test]
    fn test_map_status_error_429() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::TOO_MANY_REQUESTS, "");
        assert!(matches!(error.kind(), ErrorKind::RateLimited));
    }

    #[test]
    fn test_map_status_error_500() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::INTERNAL_SERVER_ERROR, "Oops");
        assert!(matches!(error.kind(), ErrorKind::Internal));
    }

    #[test]
    fn test_map_status_error_503() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::SERVICE_UNAVAILABLE, "Down");
        assert!(matches!(error.kind(), ErrorKind::Internal));
    }

    #[test]
    fn test_map_status_error_unknown() {
        let inner = create_test_inner();
        let error = inner.map_status_error(StatusCode::IM_A_TEAPOT, "I'm a teapot");
        assert!(matches!(error.kind(), ErrorKind::Transport));
        assert!(error.to_string().contains("418"));
    }
}
