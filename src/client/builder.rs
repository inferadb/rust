//! Client builder with typestate pattern.

use std::{marker::PhantomData, sync::Arc, time::Duration};

use super::inner::ClientInner;
#[cfg(feature = "grpc")]
use crate::transport::GrpcTransport;
#[cfg(feature = "rest")]
use crate::transport::RestTransport;
use crate::{
    Client, Error,
    auth::Credentials,
    config::{CacheConfig, DegradationConfig, RetryConfig, TlsConfig},
    transport::{PoolConfig, TransportStrategy},
};

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
    transport_strategy: TransportStrategy,
    pool_config: PoolConfig,
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
            transport_strategy: TransportStrategy::default(),
            pool_config: PoolConfig::default(),
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
            transport_strategy: self.transport_strategy,
            pool_config: self.pool_config,
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
            transport_strategy: self.transport_strategy,
            pool_config: self.pool_config,
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

    /// Disables TLS certificate verification and allows HTTP connections.
    ///
    /// **WARNING**: This is insecure and should only be used for local development.
    /// Never use this in production.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = Client::builder()
    ///     .url("http://localhost:8080")
    ///     .insecure()
    ///     .credentials(credentials)
    ///     .build()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn insecure(mut self) -> Self {
        self.tls_config.skip_verification = true;
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

    /// Sets the transport strategy.
    ///
    /// Controls which transport protocol(s) the client uses:
    /// - `GrpcOnly`: Use gRPC only, fail if unavailable
    /// - `RestOnly`: Use REST only
    /// - `PreferGrpc`: Use gRPC with automatic REST fallback (default)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::TransportStrategy;
    ///
    /// // Use gRPC only
    /// let builder = builder.transport_strategy(TransportStrategy::GrpcOnly);
    ///
    /// // Use REST only
    /// let builder = builder.transport_strategy(TransportStrategy::RestOnly);
    /// ```
    #[must_use]
    pub fn transport_strategy(mut self, strategy: TransportStrategy) -> Self {
        self.transport_strategy = strategy;
        self
    }

    /// Sets the connection pool configuration.
    ///
    /// Controls connection pooling behavior for the transport.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::PoolConfig;
    /// use std::time::Duration;
    ///
    /// let builder = builder.pool_config(
    ///     PoolConfig::default()
    ///         .with_max_connections(20)
    ///         .with_pool_timeout(Duration::from_secs(30))
    /// );
    /// ```
    #[must_use]
    pub fn pool_config(mut self, config: PoolConfig) -> Self {
        self.pool_config = config;
        self
    }
}

impl<U, C> ClientBuilder<U, C> {
    /// Creates the transport based on the configured strategy.
    #[allow(unused_variables)]
    async fn create_transport(
        &self,
        url: &url::Url,
        timeout: Duration,
        initial_token: Option<&String>,
    ) -> Result<Option<Arc<dyn crate::transport::TransportClient + Send + Sync>>, Error> {
        match &self.transport_strategy {
            #[cfg(feature = "grpc")]
            TransportStrategy::GrpcOnly => {
                let grpc = GrpcTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )
                .await?;
                Ok(Some(Arc::new(grpc)))
            },
            #[cfg(not(feature = "grpc"))]
            TransportStrategy::GrpcOnly => Err(Error::configuration(
                "gRPC transport requested but 'grpc' feature is not enabled",
            )),
            #[cfg(feature = "rest")]
            TransportStrategy::RestOnly => {
                let rest = RestTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )?;
                if let Some(token) = initial_token {
                    rest.set_auth_token(token.clone());
                }
                Ok(Some(Arc::new(rest)))
            },
            #[cfg(not(feature = "rest"))]
            TransportStrategy::RestOnly => Err(Error::configuration(
                "REST transport requested but 'rest' feature is not enabled",
            )),
            #[cfg(all(feature = "grpc", feature = "rest"))]
            TransportStrategy::PreferGrpc { .. } => {
                // Try gRPC first, fall back to REST on connection error
                match GrpcTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )
                .await
                {
                    Ok(grpc) => Ok(Some(Arc::new(grpc))),
                    Err(_) => {
                        // Fall back to REST
                        let rest = RestTransport::new(
                            url.clone(),
                            &self.tls_config,
                            &self.pool_config,
                            self.retry_config.clone(),
                            timeout,
                        )?;
                        if let Some(token) = initial_token {
                            rest.set_auth_token(token.clone());
                        }
                        Ok(Some(Arc::new(rest)))
                    },
                }
            },
            #[cfg(all(feature = "grpc", not(feature = "rest")))]
            TransportStrategy::PreferGrpc { .. } => {
                let grpc = GrpcTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )
                .await?;
                Ok(Some(Arc::new(grpc)))
            },
            #[cfg(all(not(feature = "grpc"), feature = "rest"))]
            TransportStrategy::PreferGrpc { .. } => {
                // gRPC not available, use REST
                let rest = RestTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )?;
                if let Some(token) = initial_token {
                    rest.set_auth_token(token.clone());
                }
                Ok(Some(Arc::new(rest)))
            },
            #[cfg(all(feature = "grpc", feature = "rest"))]
            TransportStrategy::PreferRest { .. } => {
                // Try REST first, fall back to gRPC on connection error
                match RestTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                ) {
                    Ok(rest) => {
                        if let Some(token) = initial_token {
                            rest.set_auth_token(token.clone());
                        }
                        Ok(Some(Arc::new(rest)))
                    },
                    Err(_) => {
                        // Fall back to gRPC
                        let grpc = GrpcTransport::new(
                            url.clone(),
                            &self.tls_config,
                            &self.pool_config,
                            self.retry_config.clone(),
                            timeout,
                        )
                        .await?;
                        Ok(Some(Arc::new(grpc)))
                    },
                }
            },
            #[cfg(all(feature = "rest", not(feature = "grpc")))]
            TransportStrategy::PreferRest { .. } => {
                let rest = RestTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )?;
                if let Some(token) = initial_token {
                    rest.set_auth_token(token.clone());
                }
                Ok(Some(Arc::new(rest)))
            },
            #[cfg(all(not(feature = "rest"), feature = "grpc"))]
            TransportStrategy::PreferRest { .. } => {
                // REST not available, use gRPC
                let grpc = GrpcTransport::new(
                    url.clone(),
                    &self.tls_config,
                    &self.pool_config,
                    self.retry_config.clone(),
                    timeout,
                )
                .await?;
                Ok(Some(Arc::new(grpc)))
            },
            #[cfg(not(any(feature = "grpc", feature = "rest")))]
            _ => Ok(None),
        }
    }
}

impl ClientBuilder<HasUrl, HasCredentials> {
    /// Builds the client with a custom transport (for testing).
    ///
    /// This is useful for injecting mock transports in tests.
    #[cfg(test)]
    pub async fn build_with_transport(
        self,
        transport: Arc<dyn crate::transport::TransportClient + Send + Sync>,
    ) -> Result<Client, Error> {
        let url = self.url.ok_or_else(|| Error::configuration("URL is required"))?;

        let credentials =
            self.credentials.ok_or_else(|| Error::configuration("credentials are required"))?;

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));

        let inner = ClientInner {
            url,
            credentials,
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout,
            #[cfg(any(feature = "grpc", feature = "rest"))]
            transport: Some(transport),
            #[cfg(feature = "rest")]
            http_client: None,
            #[cfg(feature = "rest")]
            auth_token: parking_lot::RwLock::new(None),
            shutdown_guard: None,
        };

        Ok(Client::from_inner(inner))
    }

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
        let url = self.url.clone().ok_or_else(|| Error::configuration("URL is required"))?;

        // Validate URL first
        let parsed_url = url::Url::parse(&url)
            .map_err(|e| Error::configuration(format!("invalid URL: {}", e)))?;

        // Ensure HTTPS unless insecure mode is enabled
        if parsed_url.scheme() != "https" && !self.tls_config.skip_verification {
            return Err(Error::configuration(
                "HTTPS is required. Use .insecure() for development with HTTP.",
            ));
        }

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));

        // Extract bearer token if using Bearer credentials (before consuming credentials)
        #[cfg(feature = "rest")]
        let initial_token =
            self.credentials.as_ref().and_then(|c| c.as_bearer()).map(|b| b.token().to_string());
        #[cfg(not(feature = "rest"))]
        let initial_token: Option<String> = None;

        // Create transport based on strategy (before consuming self)
        let transport = self.create_transport(&parsed_url, timeout, initial_token.as_ref()).await?;

        // Now extract credentials (consuming self)
        let credentials =
            self.credentials.ok_or_else(|| Error::configuration("credentials are required"))?;

        // Create HTTP client for Control API
        #[cfg(feature = "rest")]
        let http_client = {
            let mut builder = reqwest::Client::builder().timeout(timeout).connect_timeout(timeout);

            // Configure TLS if needed
            #[cfg(feature = "rustls")]
            if parsed_url.scheme() == "https" {
                builder = builder.use_rustls_tls();
            }

            // Configure TLS certificate verification
            if self.tls_config.skip_verification {
                builder = builder.danger_accept_invalid_certs(true);
            }

            Some(builder.build().map_err(|e| {
                Error::configuration(format!("Failed to create HTTP client: {}", e))
            })?)
        };

        let inner = ClientInner {
            url,
            credentials,
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout,
            #[cfg(any(feature = "grpc", feature = "rest"))]
            transport,
            #[cfg(feature = "rest")]
            http_client,
            #[cfg(feature = "rest")]
            auth_token: parking_lot::RwLock::new(initial_token),
            shutdown_guard: None,
        };

        Ok(Client::from_inner(inner))
    }

    /// Builds the client with a shutdown handle.
    ///
    /// Returns both the client and a shutdown handle that can be used
    /// to initiate graceful shutdown.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tokio::signal;
    /// use std::time::Duration;
    ///
    /// let (client, shutdown_handle) = Client::builder()
    ///     .url("https://api.inferadb.com")
    ///     .credentials(credentials)
    ///     .build_with_shutdown()
    ///     .await?;
    ///
    /// tokio::select! {
    ///     _ = signal::ctrl_c() => {
    ///         shutdown_handle.shutdown_timeout(Duration::from_secs(30)).await;
    ///     }
    ///     _ = run_server(client) => {}
    /// }
    /// ```
    pub async fn build_with_shutdown(
        self,
    ) -> Result<(Client, super::health::ShutdownHandle), Error> {
        let url = self.url.clone().ok_or_else(|| Error::configuration("URL is required"))?;

        // Validate URL first
        let parsed_url = url::Url::parse(&url)
            .map_err(|e| Error::configuration(format!("invalid URL: {}", e)))?;

        // Ensure HTTPS unless insecure mode is enabled
        if parsed_url.scheme() != "https" && !self.tls_config.skip_verification {
            return Err(Error::configuration(
                "HTTPS is required. Use .insecure() for development with HTTP.",
            ));
        }

        let timeout = self.timeout.unwrap_or(Duration::from_secs(30));

        // Extract bearer token if using Bearer credentials (before consuming credentials)
        #[cfg(feature = "rest")]
        let initial_token =
            self.credentials.as_ref().and_then(|c| c.as_bearer()).map(|b| b.token().to_string());
        #[cfg(not(feature = "rest"))]
        let initial_token: Option<String> = None;

        // Create transport based on strategy (before consuming self)
        let transport = self.create_transport(&parsed_url, timeout, initial_token.as_ref()).await?;

        // Now extract credentials (consuming self)
        let credentials =
            self.credentials.ok_or_else(|| Error::configuration("credentials are required"))?;

        // Create HTTP client for Control API
        #[cfg(feature = "rest")]
        let http_client = {
            let mut builder = reqwest::Client::builder().timeout(timeout).connect_timeout(timeout);

            // Configure TLS if needed
            #[cfg(feature = "rustls")]
            if parsed_url.scheme() == "https" {
                builder = builder.use_rustls_tls();
            }

            // Configure TLS certificate verification
            if self.tls_config.skip_verification {
                builder = builder.danger_accept_invalid_certs(true);
            }

            Some(builder.build().map_err(|e| {
                Error::configuration(format!("Failed to create HTTP client: {}", e))
            })?)
        };

        // Create shutdown handle and guard
        let (shutdown_handle, shutdown_guard) = super::health::ShutdownHandle::new();

        let inner = ClientInner {
            url,
            credentials,
            retry_config: self.retry_config,
            cache_config: self.cache_config,
            tls_config: self.tls_config,
            degradation_config: self.degradation_config,
            timeout,
            #[cfg(any(feature = "grpc", feature = "rest"))]
            transport,
            #[cfg(feature = "rest")]
            http_client,
            #[cfg(feature = "rest")]
            auth_token: parking_lot::RwLock::new(initial_token),
            shutdown_guard: Some(shutdown_guard),
        };

        Ok((Client::from_inner(inner), shutdown_handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{BearerCredentialsConfig, ClientCredentialsConfig, Ed25519PrivateKey},
        transport::mock::MockTransport,
    };

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
    async fn test_build_allows_http_when_insecure() {
        let result = ClientBuilder::new()
            .url("http://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .insecure()
            .build()
            .await;

        // Should succeed (or fail for other reasons like connection, not HTTPS)
        assert!(result.is_ok() || !result.unwrap_err().to_string().contains("HTTPS"));
    }

    #[tokio::test]
    async fn test_build_with_client_credentials() {
        let key = Ed25519PrivateKey::generate();
        let mock_transport = Arc::new(MockTransport::new());
        let result = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(ClientCredentialsConfig::new("client_id", key))
            .build_with_transport(mock_transport)
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

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_build_with_shutdown() {
        let result = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .build_with_shutdown()
            .await;

        assert!(result.is_ok());
        let (client, shutdown_handle) = result.unwrap();

        // Initially not shutting down
        assert!(!client.is_shutting_down());
        assert!(!shutdown_handle.is_shutting_down());

        // Initiate shutdown
        shutdown_handle.shutdown().await;

        // Client should now report shutting down
        assert!(client.is_shutting_down());
    }

    #[test]
    fn test_builder_tls_config() {
        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .tls_config(TlsConfig::default());

        // Default TLS config has no custom certificates
        assert!(builder.tls_config.ca_cert_file.is_none());
        assert!(builder.tls_config.ca_cert_pem.is_none());
        assert!(builder.tls_config.client_cert_file.is_none());
        assert!(builder.tls_config.client_key_file.is_none());
    }

    #[test]
    fn test_builder_degradation_config() {
        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .degradation_config(DegradationConfig::fail_open());

        assert_eq!(builder.degradation_config.failure_mode, crate::config::FailureMode::FailOpen);
    }

    #[test]
    fn test_builder_transport_strategy() {
        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .transport_strategy(crate::transport::TransportStrategy::RestOnly);

        assert_eq!(
            builder.transport_strategy.preferred_transport(),
            crate::transport::Transport::Http
        );
    }

    #[test]
    fn test_builder_pool_config() {
        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .pool_config(PoolConfig::default());

        assert_eq!(builder.pool_config.max_connections, 100);
    }

    #[test]
    fn test_builder_default() {
        let builder: ClientBuilder<NoUrl, NoCredentials> = ClientBuilder::default();
        assert!(builder.url.is_none());
        assert!(builder.credentials.is_none());
    }

    #[test]
    fn test_builder_all_configs_combined() {
        let mut cache_config = CacheConfig::new();
        cache_config.enabled = false;

        let builder = ClientBuilder::new()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("token"))
            .retry_config(RetryConfig::new().with_max_retries(10))
            .cache_config(cache_config)
            .tls_config(TlsConfig::default())
            .degradation_config(DegradationConfig::fail_closed())
            .timeout(Duration::from_secs(30))
            .transport_strategy(crate::transport::TransportStrategy::RestOnly)
            .pool_config(PoolConfig::default());

        assert_eq!(builder.retry_config.max_retries, 10);
        assert!(!builder.cache_config.enabled);
        assert_eq!(builder.timeout, Some(Duration::from_secs(30)));
    }
}
