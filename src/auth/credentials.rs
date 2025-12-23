//! Credentials types for InferaDB authentication.

use std::fmt;
use std::sync::Arc;

use super::Ed25519PrivateKey;

/// OAuth 2.0 client credentials configuration.
///
/// This is the recommended authentication method for production use.
/// It uses Ed25519 keys to sign JWT assertions for token exchange.
///
/// ## Setup
///
/// 1. Generate an Ed25519 key pair
/// 2. Register the public key in the InferaDB dashboard
/// 3. Use the private key with your client ID
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::{Client, ClientCredentialsConfig, Ed25519PrivateKey};
///
/// let config = ClientCredentialsConfig {
///     client_id: "client_abc123".into(),
///     private_key: Ed25519PrivateKey::from_pem_file("private.pem")?,
///     certificate_id: Some("cert_xyz789".into()), // Optional
/// };
///
/// let client = Client::builder()
///     .url("https://api.inferadb.com")
///     .credentials(config)
///     .build()
///     .await?;
/// ```
///
/// ## Token Refresh
///
/// The SDK automatically refreshes tokens before expiration.
/// You don't need to handle token management manually.
pub struct ClientCredentialsConfig {
    /// The OAuth client ID.
    pub client_id: String,

    /// The Ed25519 private key for signing JWT assertions.
    pub private_key: Ed25519PrivateKey,

    /// Optional certificate ID for certificate binding.
    ///
    /// When set, the JWT will include certificate binding claims
    /// for enhanced security.
    pub certificate_id: Option<String>,
}

impl ClientCredentialsConfig {
    /// Creates a new client credentials configuration.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The OAuth client ID
    /// * `private_key` - The Ed25519 private key for signing
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::{ClientCredentialsConfig, Ed25519PrivateKey};
    ///
    /// let config = ClientCredentialsConfig::new(
    ///     "client_abc123",
    ///     Ed25519PrivateKey::generate(),
    /// );
    /// ```
    pub fn new(client_id: impl Into<String>, private_key: Ed25519PrivateKey) -> Self {
        Self {
            client_id: client_id.into(),
            private_key,
            certificate_id: None,
        }
    }

    /// Sets the certificate ID for certificate binding.
    #[must_use]
    pub fn with_certificate_id(mut self, certificate_id: impl Into<String>) -> Self {
        self.certificate_id = Some(certificate_id.into());
        self
    }
}

impl fmt::Debug for ClientCredentialsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientCredentialsConfig")
            .field("client_id", &self.client_id)
            .field("private_key", &"[REDACTED]")
            .field("certificate_id", &self.certificate_id)
            .finish()
    }
}

/// Bearer token credentials configuration.
///
/// This is a simpler authentication method using a pre-generated token.
/// Suitable for development, testing, or when token management is handled
/// externally.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::{Client, BearerCredentialsConfig};
///
/// let client = Client::builder()
///     .url("https://api.inferadb.com")
///     .credentials(BearerCredentialsConfig::new("your-api-token"))
///     .build()
///     .await?;
/// ```
///
/// ## Token Refresh
///
/// Unlike `ClientCredentialsConfig`, bearer tokens are not automatically
/// refreshed. You need to provide a new token when the current one expires.
///
/// For automatic token refresh, consider using a `CredentialsProvider`.
#[derive(Clone)]
pub struct BearerCredentialsConfig {
    /// The bearer token.
    token: Arc<str>,
}

impl BearerCredentialsConfig {
    /// Creates a new bearer credentials configuration.
    ///
    /// # Arguments
    ///
    /// * `token` - The bearer token (access token)
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::BearerCredentialsConfig;
    ///
    /// let config = BearerCredentialsConfig::new("eyJhbGciOiJFZERTQSI...");
    /// ```
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: Arc::from(token.into()),
        }
    }

    /// Returns the bearer token.
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for BearerCredentialsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BearerCredentialsConfig")
            .field("token", &"[REDACTED]")
            .finish()
    }
}

impl<S: Into<String>> From<S> for BearerCredentialsConfig {
    fn from(token: S) -> Self {
        Self::new(token)
    }
}

/// Authentication credentials for the InferaDB SDK.
///
/// This enum represents the different authentication methods supported:
///
/// - `ClientCredentials`: OAuth 2.0 client credentials with Ed25519 keys (recommended)
/// - `Bearer`: Direct bearer token (simpler, but no auto-refresh)
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::{Credentials, ClientCredentialsConfig, BearerCredentialsConfig, Ed25519PrivateKey};
///
/// // Client credentials (recommended for production)
/// let creds: Credentials = ClientCredentialsConfig::new(
///     "client_id",
///     Ed25519PrivateKey::from_pem_file("key.pem")?,
/// ).into();
///
/// // Bearer token (for development)
/// let creds: Credentials = BearerCredentialsConfig::new("token").into();
/// ```
pub enum Credentials {
    /// OAuth 2.0 client credentials with Ed25519 JWT signing.
    ClientCredentials(Box<ClientCredentialsConfig>),

    /// Direct bearer token.
    Bearer(BearerCredentialsConfig),
}

impl Credentials {
    /// Returns `true` if this is client credentials authentication.
    pub fn is_client_credentials(&self) -> bool {
        matches!(self, Credentials::ClientCredentials(_))
    }

    /// Returns `true` if this is bearer token authentication.
    pub fn is_bearer(&self) -> bool {
        matches!(self, Credentials::Bearer(_))
    }

    /// Returns the client credentials config if applicable.
    pub fn as_client_credentials(&self) -> Option<&ClientCredentialsConfig> {
        match self {
            Credentials::ClientCredentials(config) => Some(config),
            _ => None,
        }
    }

    /// Returns the bearer config if applicable.
    pub fn as_bearer(&self) -> Option<&BearerCredentialsConfig> {
        match self {
            Credentials::Bearer(config) => Some(config),
            _ => None,
        }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credentials::ClientCredentials(config) => {
                f.debug_tuple("ClientCredentials").field(config).finish()
            }
            Credentials::Bearer(config) => f.debug_tuple("Bearer").field(config).finish(),
        }
    }
}

impl From<ClientCredentialsConfig> for Credentials {
    fn from(config: ClientCredentialsConfig) -> Self {
        Credentials::ClientCredentials(Box::new(config))
    }
}

impl From<BearerCredentialsConfig> for Credentials {
    fn from(config: BearerCredentialsConfig) -> Self {
        Credentials::Bearer(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_credentials_new() {
        let key = Ed25519PrivateKey::generate();
        let config = ClientCredentialsConfig::new("test_client", key);
        assert_eq!(config.client_id, "test_client");
        assert!(config.certificate_id.is_none());
    }

    #[test]
    fn test_client_credentials_with_certificate() {
        let key = Ed25519PrivateKey::generate();
        let config =
            ClientCredentialsConfig::new("test_client", key).with_certificate_id("cert_123");
        assert_eq!(config.certificate_id, Some("cert_123".to_string()));
    }

    #[test]
    fn test_client_credentials_debug() {
        let key = Ed25519PrivateKey::generate();
        let config = ClientCredentialsConfig::new("test_client", key);
        let debug = format!("{:?}", config);
        assert!(debug.contains("test_client"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn test_bearer_credentials_new() {
        let config = BearerCredentialsConfig::new("test_token");
        assert_eq!(config.token(), "test_token");
    }

    #[test]
    fn test_bearer_credentials_debug() {
        let config = BearerCredentialsConfig::new("secret_token");
        let debug = format!("{:?}", config);
        assert!(!debug.contains("secret_token"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn test_bearer_credentials_clone() {
        let config = BearerCredentialsConfig::new("test_token");
        let cloned = config.clone();
        assert_eq!(cloned.token(), "test_token");
    }

    #[test]
    fn test_credentials_from_client_credentials() {
        let key = Ed25519PrivateKey::generate();
        let config = ClientCredentialsConfig::new("client", key);
        let creds: Credentials = config.into();
        assert!(creds.is_client_credentials());
        assert!(!creds.is_bearer());
    }

    #[test]
    fn test_credentials_from_bearer() {
        let config = BearerCredentialsConfig::new("token");
        let creds: Credentials = config.into();
        assert!(creds.is_bearer());
        assert!(!creds.is_client_credentials());
    }

    #[test]
    fn test_credentials_as_methods() {
        let key = Ed25519PrivateKey::generate();
        let client_creds: Credentials = ClientCredentialsConfig::new("client", key).into();
        assert!(client_creds.as_client_credentials().is_some());
        assert!(client_creds.as_bearer().is_none());

        let bearer_creds: Credentials = BearerCredentialsConfig::new("token").into();
        assert!(bearer_creds.as_bearer().is_some());
        assert!(bearer_creds.as_client_credentials().is_none());
    }

    #[test]
    fn test_bearer_from_string() {
        let config: BearerCredentialsConfig = "test_token".into();
        assert_eq!(config.token(), "test_token");

        let config: BearerCredentialsConfig = String::from("owned_token").into();
        assert_eq!(config.token(), "owned_token");
    }

    #[test]
    fn test_credentials_debug_client_credentials() {
        let key = Ed25519PrivateKey::generate();
        let creds: Credentials = ClientCredentialsConfig::new("client_id", key).into();
        let debug = format!("{:?}", creds);
        assert!(debug.contains("ClientCredentials"));
        assert!(debug.contains("client_id"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn test_credentials_debug_bearer() {
        let creds: Credentials = BearerCredentialsConfig::new("secret").into();
        let debug = format!("{:?}", creds);
        assert!(debug.contains("Bearer"));
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("secret"));
    }
}
