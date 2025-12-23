//! TLS configuration for secure connections.

use std::path::PathBuf;

/// Configuration for TLS/SSL connections.
///
/// By default, the SDK uses system root certificates and validates
/// server certificates. This configuration allows customization for
/// enterprise environments.
///
/// ## Example: Custom CA
///
/// ```rust
/// use inferadb::TlsConfig;
///
/// let config = TlsConfig::new()
///     .with_ca_cert_file("/path/to/ca.crt");
/// ```
///
/// ## Example: Client Certificate (mTLS)
///
/// ```rust
/// use inferadb::TlsConfig;
///
/// let config = TlsConfig::new()
///     .with_client_cert_file("/path/to/client.crt")
///     .with_client_key_file("/path/to/client.key");
/// ```
#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    /// Custom CA certificate file path.
    pub ca_cert_file: Option<PathBuf>,

    /// Custom CA certificate PEM data.
    pub ca_cert_pem: Option<String>,

    /// Client certificate file path (for mTLS).
    pub client_cert_file: Option<PathBuf>,

    /// Client key file path (for mTLS).
    pub client_key_file: Option<PathBuf>,

    /// Whether to skip certificate verification.
    ///
    /// **WARNING**: This is insecure and should only be used for local development.
    pub skip_verification: bool,
}

impl TlsConfig {
    /// Creates a new TLS configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the CA certificate file path.
    #[must_use]
    pub fn with_ca_cert_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.ca_cert_file = Some(path.into());
        self
    }

    /// Sets the CA certificate PEM data directly.
    #[must_use]
    pub fn with_ca_cert_pem(mut self, pem: impl Into<String>) -> Self {
        self.ca_cert_pem = Some(pem.into());
        self
    }

    /// Sets the client certificate file path for mTLS.
    #[must_use]
    pub fn with_client_cert_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.client_cert_file = Some(path.into());
        self
    }

    /// Sets the client key file path for mTLS.
    #[must_use]
    pub fn with_client_key_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.client_key_file = Some(path.into());
        self
    }

    /// Returns `true` if mTLS is configured.
    pub fn is_mtls_configured(&self) -> bool {
        self.client_cert_file.is_some() && self.client_key_file.is_some()
    }

    /// Returns `true` if custom CA is configured.
    pub fn has_custom_ca(&self) -> bool {
        self.ca_cert_file.is_some() || self.ca_cert_pem.is_some()
    }

    /// Disables certificate verification (insecure, for development only).
    ///
    /// **WARNING**: This makes connections vulnerable to man-in-the-middle attacks.
    /// Only use this for local development with self-signed certificates.
    #[must_use]
    pub fn insecure(mut self) -> Self {
        self.skip_verification = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = TlsConfig::new();
        assert!(config.ca_cert_file.is_none());
        assert!(!config.is_mtls_configured());
        assert!(!config.has_custom_ca());
    }

    #[test]
    fn test_ca_cert_file() {
        let config = TlsConfig::new().with_ca_cert_file("/path/to/ca.crt");
        assert!(config.has_custom_ca());
        assert_eq!(config.ca_cert_file, Some(PathBuf::from("/path/to/ca.crt")));
    }

    #[test]
    fn test_ca_cert_pem() {
        let config = TlsConfig::new().with_ca_cert_pem("-----BEGIN CERTIFICATE-----");
        assert!(config.has_custom_ca());
    }

    #[test]
    fn test_mtls() {
        let config = TlsConfig::new()
            .with_client_cert_file("/path/to/client.crt")
            .with_client_key_file("/path/to/client.key");
        assert!(config.is_mtls_configured());
    }

    #[test]
    fn test_partial_mtls() {
        // Only cert, no key
        let config = TlsConfig::new().with_client_cert_file("/path/to/client.crt");
        assert!(!config.is_mtls_configured());
    }

    #[test]
    fn test_insecure() {
        let config = TlsConfig::new().insecure();
        assert!(config.skip_verification);
    }
}
