//! Cache configuration for authorization decisions.

use std::time::Duration;

/// Configuration for local caching of authorization decisions.
///
/// Caching can significantly reduce latency for repeated checks, but
/// must be configured carefully to avoid stale authorization results.
///
/// ## Important: Cache Invalidation
///
/// Authorization decisions can become stale when:
/// - Relationships change
/// - Schema changes
/// - Conditions evaluate differently over time
///
/// Always use an appropriate TTL for your use case.
///
/// ## Example
///
/// ```rust
/// use inferadb::CacheConfig;
/// use std::time::Duration;
///
/// // Short TTL for frequently changing permissions
/// let config = CacheConfig::new()
///     .with_ttl(Duration::from_secs(30))
///     .with_max_entries(10_000);
///
/// // Longer TTL with negative caching disabled
/// let config = CacheConfig::new()
///     .with_ttl(Duration::from_secs(300))
///     .with_negative_caching(false);
/// ```
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Whether caching is enabled.
    pub enabled: bool,

    /// Time-to-live for cached entries.
    pub ttl: Duration,

    /// Maximum number of entries in the cache.
    pub max_entries: usize,

    /// Whether to cache negative (denied) results.
    ///
    /// Set to `false` if you want denial decisions to be re-evaluated
    /// more frequently (e.g., for time-sensitive permissions).
    pub negative_caching: bool,

    /// TTL for negative cache entries (if different from positive).
    pub negative_ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl: Duration::from_secs(60),
            max_entries: 10_000,
            negative_caching: true,
            negative_ttl: None,
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with caching disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a configuration with caching enabled.
    pub fn enabled() -> Self {
        Self { enabled: true, ..Default::default() }
    }

    /// Enables or disables caching.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the TTL for cached entries.
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Sets the maximum number of cache entries.
    #[must_use]
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// Sets whether to cache negative (denied) results.
    #[must_use]
    pub fn with_negative_caching(mut self, enabled: bool) -> Self {
        self.negative_caching = enabled;
        self
    }

    /// Sets a separate TTL for negative cache entries.
    #[must_use]
    pub fn with_negative_ttl(mut self, ttl: Duration) -> Self {
        self.negative_ttl = Some(ttl);
        self
    }

    /// Returns the effective TTL for negative entries.
    pub fn effective_negative_ttl(&self) -> Duration {
        self.negative_ttl.unwrap_or(self.ttl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_disabled() {
        let config = CacheConfig::new();
        assert!(!config.enabled);
    }

    #[test]
    fn test_enabled() {
        let config = CacheConfig::enabled();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_builder() {
        let config = CacheConfig::enabled()
            .with_ttl(Duration::from_secs(120))
            .with_max_entries(5000)
            .with_negative_caching(false);

        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(120));
        assert_eq!(config.max_entries, 5000);
        assert!(!config.negative_caching);
    }

    #[test]
    fn test_negative_ttl() {
        let config = CacheConfig::enabled()
            .with_ttl(Duration::from_secs(300))
            .with_negative_ttl(Duration::from_secs(30));

        assert_eq!(config.effective_negative_ttl(), Duration::from_secs(30));
    }

    #[test]
    fn test_effective_negative_ttl_fallback() {
        let config = CacheConfig::enabled().with_ttl(Duration::from_secs(60));
        assert_eq!(config.effective_negative_ttl(), Duration::from_secs(60));
    }
}
