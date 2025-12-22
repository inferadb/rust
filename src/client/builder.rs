//! Client builder with typestate pattern.

use std::marker::PhantomData;
use std::time::Duration;

use crate::auth::Credentials;
use crate::config::{CacheConfig, DegradationConfig, RetryConfig, TlsConfig};
use crate::{Client, Error};

use super::inner::ClientInner;

/// Marker type: URL not yet provided.
pub struct NoUrl;

/// Marker type: URL has been provided.
pub struct HasUrl;

/// Marker type: Credentials not yet provided.
pub struct NoCredentials;

/// Marker type: Credentials have been provided.
pub struct HasCredentials;

/// Builder for creating [`Client`] instances.
///
/// Uses the typestate pattern to ensure required configuration
/// (URL and credentials) is provided at compile time.
///
/// ## Required Configuration
///
/// - `url()`: The InferaDB API endpoint
/// - `credentials()`: Authentication credentials
///
/// ## Optional Configuration
///
/// - `retry_config()`: Retry behavior for transient failures
/// - `cache_config()`: Local caching configuration
/// - `tls_config()`: Custom TLS settings
/// - `degradation_config()`: Graceful degradation behavior
/// - `timeout()`: Request timeout
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::{Client, ClientCredentialsConfig, Ed25519PrivateKey};
///
/// let client = Client::builder()
///     .url("https://api.inferadb.com")
///     .credentials(ClientCredentialsConfig {
///         client_id: "client_123".into(),
///         private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
///         certificate_id: None,
///     })
///     .retry_config(RetryConfig::new().with_max_retries(5))
///     .timeout(Duration::from_secs(10))
///     .build()
///     .await?;
/// ```
pub struct ClientBuilder<UrlState, CredentialsState> {
    url: Option<String>,
    credentials: Option<Credentials>,
    retry_config: RetryConfig,
    cache_config: CacheConfig,
    tls_config: TlsConfig,
    degradation_config: DegradationConfig,
    timeout: Option<Duration>,
    _url_state: PhantomData<UrlState>,
    _credentials_state: PhantomData<CredentialsState>,
}

impl ClientBuilder<NoUrl, NoCredentials> {
    /// Creates a new client builder.
    pub fn new() -> Self {
        Self {
            url: None,
            credentials: None,
            retry_config: RetryConfig::default(),
            cache_config: CacheConfig::default(),
            tls_config: TlsConfig::default(),
            degradation_config: DegradationConfig::default(),
            timeout: None,
            _url_state: PhantomData,
            _credentials_state: PhantomData,
        }
    }
}

impl Default for ClientBuilder<NoUrl, NoCredentials> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> ClientBuilder<NoUrl, C> {
    /// Sets the InferaDB API URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The API endpoint (e.g., `https://api.inferadb.com`)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = Client::builder()
    ///     .url("https://api.inferadb.com");
    /// ```
    pub fn url(self, url: impl Into<String>) -> ClientBuilder<HasUrl, C> {
        ClientBuilder {
            url: Some(url.into()),
            credentials: self.credentials,
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout: self.timeout,
            _url_state: PhantomData,
            _credentials_state: PhantomData,
        }
    }
}

impl<U> ClientBuilder<U, NoCredentials> {
    /// Sets the authentication credentials.
    ///
    /// Accepts any type that can be converted into [`Credentials`]:
    /// - [`ClientCredentialsConfig`](crate::ClientCredentialsConfig)
    /// - [`BearerCredentialsConfig`](crate::BearerCredentialsConfig)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::{ClientCredentialsConfig, Ed25519PrivateKey};
    ///
    /// let builder = Client::builder()
    ///     .credentials(ClientCredentialsConfig {
    ///         client_id: "client_123".into(),
    ///         private_key: Ed25519PrivateKey::generate(),
    ///         certificate_id: None,
    ///     });
    /// ```
    pub fn credentials(
        self,
        credentials: impl Into<Credentials>,
    ) -> ClientBuilder<U, HasCredentials> {
        ClientBuilder {
            url: self.url,
            credentials: Some(credentials.into()),
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout: self.timeout,
            _url_state: PhantomData,
            _credentials_state: PhantomData,
        }
    }
}

impl<U, C> ClientBuilder<U, C> {
    /// Sets the retry configuration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::RetryConfig;
    ///
    /// let builder = builder.retry_config(
    ///     RetryConfig::new().with_max_retries(5)
    /// );
    /// ```
    #[must_use]
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Sets the cache configuration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::CacheConfig;
    /// use std::time::Duration;
    ///
    /// let builder = builder.cache_config(
    ///     CacheConfig::enabled().with_ttl(Duration::from_secs(60))
    /// );
    /// ```
    #[must_use]
    pub fn cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// Sets the TLS configuration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::TlsConfig;
    ///
    /// let builder = builder.tls_config(
    ///     TlsConfig::new().with_ca_cert_file("/path/to/ca.crt")
    /// );
    /// ```
    #[must_use]
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.tls_config = config;
        self
    }

    /// Sets the degradation configuration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::{DegradationConfig, FailureMode};
    ///
    /// let builder = builder.degradation_config(
    ///     DegradationConfig::fail_closed()
    /// );
    /// ```
    #[must_use]
    pub fn degradation_config(mut self, config: DegradationConfig) -> Self {
        self.degradation_config = config;
        self
    }

    /// Sets the request timeout.
    ///
    /// This timeout applies to individual API requests, not including retries.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let builder = builder.timeout(Duration::from_secs(10));
    /// ```
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl ClientBuilder<HasUrl, HasCredentials> {
    /// Builds the client.
    ///
    /// This validates the configuration and establishes the initial
    /// connection to the InferaDB service.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URL is invalid
    /// - The credentials are invalid
    /// - The initial connection fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = Client::builder()
    ///     .url("https://api.inferadb.com")
    ///     .credentials(credentials)
    ///     .build()
    ///     .await?;
    /// ```
    pub async fn build(self) -> Result<Client, Error> {
        let url = self
            .url
            .ok_or_else(|| Error::configuration("URL is required"))?;

        let credentials = self
            .credentials
            .ok_or_else(|| Error::configuration("credentials are required"))?;

        // Validate URL
        let _parsed_url = url::Url::parse(&url)
            .map_err(|e| Error::configuration(format!("invalid URL: {}", e)))?;

        // Ensure HTTPS (unless insecure feature is enabled)
        #[cfg(not(feature = "insecure"))]
        if _parsed_url.scheme() != "https" {
            return Err(Error::configuration(
                "HTTPS is required. Use the 'insecure' feature for development with HTTP.",
            ));
        }

        let inner = ClientInner {
            url,
            credentials,
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
        };

        Ok(Client::from_inner(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{BearerCredentialsConfig, ClientCredentialsConfig, Ed25519PrivateKey};

    #[test]
    fn test_builder_typestate() {
        // This test verifies the typestate pattern at compile time.
        // The following should compile:
        let _builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"));

        // The following would NOT compile (missing url):
        // let client = ClientBuilder::new()
        //     .credentials(BearerCredentialsConfig::new("token"))
        //     .build();
    }

    #[tokio::test]
    async fn test_build_invalid_url() {
        let result = ClientBuilder::new()
            .url("not-a-valid-url")
            .credentials(BearerCredentialsConfig::new("token"))
            .build()
            .await;

        assert!(result.is_err());
    }

    #[cfg(not(feature = "insecure"))]
    #[tokio::test]
    async fn test_build_requires_https() {
        let result = ClientBuilder::new()
            .url("http://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .build()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("HTTPS"));
    }

    #[tokio::test]
    async fn test_build_with_client_credentials() {
        let key = Ed25519PrivateKey::generate();
        let result = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(ClientCredentialsConfig::new("client_id", key))
            .build()
            .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_configs() {
        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .retry_config(RetryConfig::disabled())
            .cache_config(CacheConfig::enabled())
            .timeout(Duration::from_secs(60));

        assert_eq!(builder.retry_config.max_retries, 0);
        assert!(builder.cache_config.enabled);
        assert_eq!(builder.timeout, Some(Duration::from_secs(60)));
    }
}
