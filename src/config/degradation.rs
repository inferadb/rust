//! Degradation configuration for graceful failure handling.

use std::time::Duration;

/// Behavior when authorization service is unavailable.
///
/// This determines what happens when the SDK cannot reach the
/// authorization service (network issues, service down, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailureMode {
    /// Deny all requests when the service is unavailable.
    ///
    /// This is the safest option for security-sensitive applications.
    /// All authorization checks fail closed.
    #[default]
    FailClosed,

    /// Allow all requests when the service is unavailable.
    ///
    /// Use this for applications where availability is more important
    /// than strict authorization enforcement. Consider logging all
    /// decisions made during degraded mode.
    FailOpen,

    /// Use cached decisions when the service is unavailable.
    ///
    /// If a cached decision exists, use it regardless of TTL.
    /// If no cached decision exists, fall back to the specified default.
    UseCached {
        /// Default decision when no cached value exists.
        default_allow: bool,
    },
}

impl FailureMode {
    /// Returns `true` if this mode allows requests when unavailable.
    pub fn allows_on_failure(&self) -> bool {
        match self {
            FailureMode::FailClosed => false,
            FailureMode::FailOpen => true,
            FailureMode::UseCached { default_allow } => *default_allow,
        }
    }
}

/// Configuration for graceful degradation under failure conditions.
///
/// This allows the SDK to handle various failure scenarios gracefully,
/// trading off between availability and strict authorization enforcement.
///
/// ## Example: Fail Closed (Default)
///
/// ```rust
/// use inferadb::{DegradationConfig, FailureMode};
///
/// let config = DegradationConfig::builder()
///     .failure_mode(FailureMode::FailClosed)
///     .build();
/// ```
///
/// ## Example: Circuit Breaker
///
/// ```rust
/// use inferadb::DegradationConfig;
/// use std::time::Duration;
///
/// let config = DegradationConfig::builder()
///     .circuit_breaker_enabled(true)
///     .circuit_breaker_threshold(5)
///     .circuit_breaker_reset_timeout(Duration::from_secs(30))
///     .build();
/// ```
#[derive(Debug, Clone, bon::Builder)]
pub struct DegradationConfig {
    /// How to handle requests when the service is unavailable.
    #[builder(default)]
    pub failure_mode: FailureMode,

    /// Whether to enable the circuit breaker.
    #[builder(default = true)]
    pub circuit_breaker_enabled: bool,

    /// Number of failures before the circuit breaker opens.
    #[builder(default = 5)]
    pub circuit_breaker_threshold: u32,

    /// Time to wait before attempting to close the circuit.
    #[builder(default = Duration::from_secs(30))]
    pub circuit_breaker_reset_timeout: Duration,

    /// Timeout for individual requests.
    #[builder(default = Duration::from_secs(5))]
    pub request_timeout: Duration,

    /// Whether to log decisions made during degraded mode.
    #[builder(default = true)]
    pub log_degraded_decisions: bool,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl DegradationConfig {
    /// Creates a fail-open configuration.
    ///
    /// All requests are allowed when the service is unavailable.
    pub fn fail_open() -> Self {
        Self::builder().failure_mode(FailureMode::FailOpen).build()
    }

    /// Creates a fail-closed configuration.
    ///
    /// All requests are denied when the service is unavailable.
    pub fn fail_closed() -> Self {
        Self::builder().failure_mode(FailureMode::FailClosed).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fail_closed() {
        let config = DegradationConfig::default();
        assert_eq!(config.failure_mode, FailureMode::FailClosed);
        assert!(!config.failure_mode.allows_on_failure());
    }

    #[test]
    fn test_fail_open() {
        let config = DegradationConfig::fail_open();
        assert_eq!(config.failure_mode, FailureMode::FailOpen);
        assert!(config.failure_mode.allows_on_failure());
    }

    #[test]
    fn test_use_cached() {
        let mode = FailureMode::UseCached { default_allow: true };
        assert!(mode.allows_on_failure());

        let mode = FailureMode::UseCached { default_allow: false };
        assert!(!mode.allows_on_failure());
    }

    #[test]
    fn test_circuit_breaker_config() {
        let config = DegradationConfig::builder()
            .circuit_breaker_enabled(true)
            .circuit_breaker_threshold(10)
            .circuit_breaker_reset_timeout(Duration::from_secs(60))
            .build();

        assert!(config.circuit_breaker_enabled);
        assert_eq!(config.circuit_breaker_threshold, 10);
        assert_eq!(config.circuit_breaker_reset_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_request_timeout() {
        let config = DegradationConfig::builder()
            .request_timeout(Duration::from_secs(10))
            .build();
        assert_eq!(config.request_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_with_failure_mode() {
        let config = DegradationConfig::builder()
            .failure_mode(FailureMode::FailOpen)
            .build();
        assert_eq!(config.failure_mode, FailureMode::FailOpen);
    }

    #[test]
    fn test_log_degraded_decisions() {
        let config = DegradationConfig::builder()
            .log_degraded_decisions(false)
            .build();
        assert!(!config.log_degraded_decisions);

        let config = DegradationConfig::builder()
            .log_degraded_decisions(true)
            .build();
        assert!(config.log_degraded_decisions);
    }

    #[test]
    fn test_fail_closed() {
        let config = DegradationConfig::fail_closed();
        assert_eq!(config.failure_mode, FailureMode::FailClosed);
    }
}
