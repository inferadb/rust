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
/// let config = TlsConfig::builder()
///     .ca_cert_file("/path/to/ca.crt")
///     .build();
/// ```
///
/// ## Example: Client Certificate (mTLS)
///
/// ```rust
/// use inferadb::TlsConfig;
///
/// let config = TlsConfig::builder()
///     .client_cert_file("/path/to/client.crt")
///     .client_key_file("/path/to/client.key")
///     .build();
/// ```
#[derive(Debug, Clone, Default, bon::Builder)]
pub struct TlsConfig {
    /// Custom CA certificate file path.
    #[builder(into)]
    pub ca_cert_file: Option<PathBuf>,

    /// Custom CA certificate PEM data.
    #[builder(into)]
    pub ca_cert_pem: Option<String>,

    /// Client certificate file path (for mTLS).
    #[builder(into)]
    pub client_cert_file: Option<PathBuf>,

    /// Client key file path (for mTLS).
    #[builder(into)]
    pub client_key_file: Option<PathBuf>,

    /// Whether to skip certificate verification.
    ///
    /// **WARNING**: This is insecure and should only be used for local development.
    #[builder(default = false)]
    pub skip_verification: bool,
}

impl TlsConfig {
    /// Creates an insecure TLS config that skips verification.
    ///
    /// **WARNING**: This makes connections vulnerable to man-in-the-middle attacks.
    /// Only use this for local development with self-signed certificates.
    pub fn insecure() -> Self {
        Self::builder().skip_verification(true).build()
    }

    /// Returns `true` if mTLS is configured.
    pub fn is_mtls_configured(&self) -> bool {
        self.client_cert_file.is_some() && self.client_key_file.is_some()
    }

    /// Returns `true` if custom CA is configured.
    pub fn has_custom_ca(&self) -> bool {
        self.ca_cert_file.is_some() || self.ca_cert_pem.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = TlsConfig::default();
        assert!(config.ca_cert_file.is_none());
        assert!(!config.is_mtls_configured());
        assert!(!config.has_custom_ca());
    }

    #[test]
    fn test_ca_cert_file() {
        let config = TlsConfig::builder().ca_cert_file("/path/to/ca.crt").build();
        assert!(config.has_custom_ca());
        assert_eq!(config.ca_cert_file, Some(PathBuf::from("/path/to/ca.crt")));
    }

    #[test]
    fn test_ca_cert_pem() {
        let config = TlsConfig::builder()
            .ca_cert_pem("-----BEGIN CERTIFICATE-----")
            .build();
        assert!(config.has_custom_ca());
    }

    #[test]
    fn test_mtls() {
        let config = TlsConfig::builder()
            .client_cert_file("/path/to/client.crt")
            .client_key_file("/path/to/client.key")
            .build();
        assert!(config.is_mtls_configured());
    }

    #[test]
    fn test_partial_mtls() {
        // Only cert, no key
        let config = TlsConfig::builder()
            .client_cert_file("/path/to/client.crt")
            .build();
        assert!(!config.is_mtls_configured());
    }

    #[test]
    fn test_insecure() {
        let config = TlsConfig::insecure();
        assert!(config.skip_verification);
    }
}
