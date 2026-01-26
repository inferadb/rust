//! Credentials provider trait for dynamic credential management.

use std::{future::Future, pin::Pin, sync::Arc};

use crate::Error;

/// A type alias for the boxed future returned by credentials providers.
pub type CredentialsFuture<'a> = Pin<Box<dyn Future<Output = Result<String, Error>> + Send + 'a>>;

/// Trait for providing authentication credentials dynamically.
///
/// This trait allows custom credential management strategies, such as:
/// - Fetching tokens from external services (Vault, AWS Secrets Manager)
/// - Implementing custom refresh logic
/// - Rotating credentials periodically
///
/// The SDK uses this trait internally for the built-in credentials types,
/// but you can implement it for custom scenarios.
///
/// ## Object Safety
///
/// This trait is object-safe and can be used as `Arc<dyn CredentialsProvider>`.
///
/// ## Example: Environment Variable Provider
///
/// ```rust
/// use inferadb::CredentialsProvider;
/// use std::sync::Arc;
///
/// struct EnvCredentialsProvider {
///     env_var: String,
/// }
///
/// impl EnvCredentialsProvider {
///     fn new(env_var: &str) -> Self {
///         Self { env_var: env_var.to_string() }
///     }
/// }
///
/// impl CredentialsProvider for EnvCredentialsProvider {
///     fn get_token(&self) -> inferadb::auth::CredentialsFuture<'_> {
///         let env_var = self.env_var.clone();
///         Box::pin(async move {
///             std::env::var(&env_var)
///                 .map_err(|_| inferadb::Error::configuration(
///                     format!("environment variable {} not set", env_var)
///                 ))
///         })
///     }
/// }
///
/// // Use with client
/// // let provider = Arc::new(EnvCredentialsProvider::new("INFERADB_TOKEN"));
/// // let client = Client::builder()
/// //     .credentials_provider(provider)
/// //     .build()
/// //     .await?;
/// ```
///
/// ## Example: External Secret Manager
///
/// ```rust,ignore
/// use inferadb::CredentialsProvider;
///
/// struct VaultCredentialsProvider {
///     vault_client: VaultClient,
///     secret_path: String,
/// }
///
/// impl CredentialsProvider for VaultCredentialsProvider {
///     fn get_token(&self) -> inferadb::auth::CredentialsFuture<'_> {
///         Box::pin(async move {
///             let secret = self.vault_client
///                 .read_secret(&self.secret_path)
///                 .await
///                 .map_err(|e| inferadb::Error::configuration(e.to_string()))?;
///
///             secret.get("token")
///                 .ok_or_else(|| inferadb::Error::configuration("token not found in secret"))
///                 .map(|s| s.to_string())
///         })
///     }
/// }
/// ```
pub trait CredentialsProvider: Send + Sync {
    /// Returns a future that resolves to a bearer token.
    ///
    /// The returned token should be valid for immediate use in API requests.
    /// The SDK will call this method when:
    /// - Making the first request
    /// - The current token has expired or is about to expire
    /// - A request returns an authentication error
    ///
    /// # Errors
    ///
    /// Return an error if the token cannot be obtained. The SDK will
    /// propagate this error to the caller.
    fn get_token(&self) -> CredentialsFuture<'_>;

    /// Returns a hint about when the token should be refreshed.
    ///
    /// The default implementation returns `None`, meaning the SDK will
    /// use its default refresh strategy (typically refresh on 401 or
    /// when the token is close to expiration based on JWT claims).
    ///
    /// Override this to provide explicit refresh hints if your token
    /// source provides expiration information.
    fn refresh_hint(&self) -> Option<std::time::Duration> {
        None
    }

    /// Returns `true` if the provider supports proactive token refresh.
    ///
    /// When `true`, the SDK may call `get_token()` before the current
    /// token expires to ensure uninterrupted service.
    ///
    /// Default: `false`
    fn supports_refresh(&self) -> bool {
        false
    }
}

// Allow using Arc<dyn CredentialsProvider> as CredentialsProvider
impl<T: CredentialsProvider + ?Sized> CredentialsProvider for Arc<T> {
    fn get_token(&self) -> CredentialsFuture<'_> {
        (**self).get_token()
    }

    fn refresh_hint(&self) -> Option<std::time::Duration> {
        (**self).refresh_hint()
    }

    fn supports_refresh(&self) -> bool {
        (**self).supports_refresh()
    }
}

// Allow using Box<dyn CredentialsProvider> as CredentialsProvider
impl<T: CredentialsProvider + ?Sized> CredentialsProvider for Box<T> {
    fn get_token(&self) -> CredentialsFuture<'_> {
        (**self).get_token()
    }

    fn refresh_hint(&self) -> Option<std::time::Duration> {
        (**self).refresh_hint()
    }

    fn supports_refresh(&self) -> bool {
        (**self).supports_refresh()
    }
}

/// A simple static token provider.
///
/// This provider always returns the same token and does not support refresh.
/// Useful for testing or when you have a long-lived token.
#[derive(Debug, Clone)]
pub struct StaticTokenProvider {
    token: Arc<str>,
}

impl StaticTokenProvider {
    /// Creates a new static token provider.
    pub fn new(token: impl Into<String>) -> Self {
        Self { token: Arc::from(token.into()) }
    }
}

impl CredentialsProvider for StaticTokenProvider {
    fn get_token(&self) -> CredentialsFuture<'_> {
        let token = self.token.clone();
        Box::pin(async move { Ok(token.to_string()) })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_token_provider() {
        let provider = StaticTokenProvider::new("test_token");
        let token = provider.get_token().await.unwrap();
        assert_eq!(token, "test_token");
    }

    #[tokio::test]
    async fn test_static_token_provider_multiple_calls() {
        let provider = StaticTokenProvider::new("consistent");
        let token1 = provider.get_token().await.unwrap();
        let token2 = provider.get_token().await.unwrap();
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_static_token_provider_defaults() {
        let provider = StaticTokenProvider::new("token");
        assert!(provider.refresh_hint().is_none());
        assert!(!provider.supports_refresh());
    }

    #[tokio::test]
    async fn test_arc_provider() {
        let provider: Arc<dyn CredentialsProvider> =
            Arc::new(StaticTokenProvider::new("arc_token"));
        let token = provider.get_token().await.unwrap();
        assert_eq!(token, "arc_token");
    }

    #[tokio::test]
    async fn test_box_provider() {
        let provider: Box<dyn CredentialsProvider> =
            Box::new(StaticTokenProvider::new("box_token"));
        let token = provider.get_token().await.unwrap();
        assert_eq!(token, "box_token");
    }

    // Custom provider for testing
    struct CustomProvider {
        counter: std::sync::atomic::AtomicU32,
    }

    impl CustomProvider {
        fn new() -> Self {
            Self { counter: std::sync::atomic::AtomicU32::new(0) }
        }
    }

    impl CredentialsProvider for CustomProvider {
        fn get_token(&self) -> CredentialsFuture<'_> {
            let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move { Ok(format!("token_{}", count)) })
        }

        fn supports_refresh(&self) -> bool {
            true
        }

        fn refresh_hint(&self) -> Option<std::time::Duration> {
            Some(std::time::Duration::from_secs(300))
        }
    }

    #[tokio::test]
    async fn test_custom_provider() {
        let provider = CustomProvider::new();
        assert!(provider.supports_refresh());
        assert_eq!(provider.refresh_hint(), Some(std::time::Duration::from_secs(300)));

        let token1 = provider.get_token().await.unwrap();
        let token2 = provider.get_token().await.unwrap();
        assert_eq!(token1, "token_0");
        assert_eq!(token2, "token_1");
    }

    #[tokio::test]
    async fn test_arc_provider_delegations() {
        let provider: Arc<dyn CredentialsProvider> = Arc::new(CustomProvider::new());
        // Test that Arc properly delegates all methods
        assert!(provider.supports_refresh());
        assert_eq!(provider.refresh_hint(), Some(std::time::Duration::from_secs(300)));
        let token = provider.get_token().await.unwrap();
        assert_eq!(token, "token_0");
    }

    #[tokio::test]
    async fn test_box_provider_delegations() {
        let provider: Box<dyn CredentialsProvider> = Box::new(CustomProvider::new());
        // Test that Box properly delegates all methods
        assert!(provider.supports_refresh());
        assert_eq!(provider.refresh_hint(), Some(std::time::Duration::from_secs(300)));
        let token = provider.get_token().await.unwrap();
        assert_eq!(token, "token_0");
    }

    #[test]
    fn test_static_token_provider_debug() {
        let provider = StaticTokenProvider::new("test");
        let debug = format!("{:?}", provider);
        assert!(debug.contains("StaticTokenProvider"));
    }

    #[test]
    fn test_static_token_provider_clone() {
        let provider = StaticTokenProvider::new("clone_test");
        let cloned = provider.clone();
        // Both should have the same token
        assert_eq!(format!("{:?}", provider), format!("{:?}", cloned));
    }
}
