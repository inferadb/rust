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
/// let config = CacheConfig::builder()
///     .enabled(true)
///     .ttl(Duration::from_secs(30))
///     .max_entries(10_000)
///     .build();
///
/// // Longer TTL with negative caching disabled
/// let config = CacheConfig::builder()
///     .enabled(true)
///     .ttl(Duration::from_secs(300))
///     .negative_caching(false)
///     .build();
/// ```
#[derive(Debug, Clone, bon::Builder)]
pub struct CacheConfig {
    /// Whether caching is enabled.
    #[builder(default = false)]
    pub enabled: bool,

    /// Time-to-live for cached entries.
    #[builder(default = Duration::from_secs(60))]
    pub ttl: Duration,

    /// Maximum number of entries in the cache.
    #[builder(default = 10_000)]
    pub max_entries: usize,

    /// Whether to cache negative (denied) results.
    ///
    /// Set to `false` if you want denial decisions to be re-evaluated
    /// more frequently (e.g., for time-sensitive permissions).
    #[builder(default = true)]
    pub negative_caching: bool,

    /// TTL for negative cache entries (if different from positive).
    pub negative_ttl: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl CacheConfig {
    /// Creates a configuration with caching enabled.
    pub fn enabled_config() -> Self {
        Self::builder().enabled(true).build()
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
        let config = CacheConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_enabled_config() {
        let config = CacheConfig::enabled_config();
        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_builder() {
        let config = CacheConfig::builder()
            .enabled(true)
            .ttl(Duration::from_secs(120))
            .max_entries(5000)
            .negative_caching(false)
            .build();

        assert!(config.enabled);
        assert_eq!(config.ttl, Duration::from_secs(120));
        assert_eq!(config.max_entries, 5000);
        assert!(!config.negative_caching);
    }

    #[test]
    fn test_negative_ttl() {
        let config = CacheConfig::builder()
            .enabled(true)
            .ttl(Duration::from_secs(300))
            .negative_ttl(Duration::from_secs(30))
            .build();

        assert_eq!(config.effective_negative_ttl(), Duration::from_secs(30));
    }

    #[test]
    fn test_effective_negative_ttl_fallback() {
        let config = CacheConfig::builder()
            .enabled(true)
            .ttl(Duration::from_secs(60))
            .build();
        assert_eq!(config.effective_negative_ttl(), Duration::from_secs(60));
    }
}
