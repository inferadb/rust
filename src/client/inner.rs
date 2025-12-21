//! Internal client implementation.

use std::time::Duration;

use crate::auth::Credentials;
use crate::config::{CacheConfig, DegradationConfig, RetryConfig, TlsConfig};

/// Internal client state shared across all client handles.
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
}
