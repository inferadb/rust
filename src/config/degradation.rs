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
/// let config = DegradationConfig::new()
///     .with_failure_mode(FailureMode::FailClosed);
/// ```
///
/// ## Example: Circuit Breaker
///
/// ```rust
/// use inferadb::DegradationConfig;
/// use std::time::Duration;
///
/// let config = DegradationConfig::new()
///     .with_circuit_breaker_enabled(true)
///     .with_circuit_breaker_threshold(5)
///     .with_circuit_breaker_reset_timeout(Duration::from_secs(30));
/// ```
#[derive(Debug, Clone)]
pub struct DegradationConfig {
    /// How to handle requests when the service is unavailable.
    pub failure_mode: FailureMode,

    /// Whether to enable the circuit breaker.
    pub circuit_breaker_enabled: bool,

    /// Number of failures before the circuit breaker opens.
    pub circuit_breaker_threshold: u32,

    /// Time to wait before attempting to close the circuit.
    pub circuit_breaker_reset_timeout: Duration,

    /// Timeout for individual requests.
    pub request_timeout: Duration,

    /// Whether to log decisions made during degraded mode.
    pub log_degraded_decisions: bool,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self {
            failure_mode: FailureMode::default(),
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_reset_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(5),
            log_degraded_decisions: true,
        }
    }
}

impl DegradationConfig {
    /// Creates a new degradation configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the failure mode.
    #[must_use]
    pub fn with_failure_mode(mut self, mode: FailureMode) -> Self {
        self.failure_mode = mode;
        self
    }

    /// Sets whether to enable the circuit breaker.
    #[must_use]
    pub fn with_circuit_breaker_enabled(mut self, enabled: bool) -> Self {
        self.circuit_breaker_enabled = enabled;
        self
    }

    /// Sets the circuit breaker failure threshold.
    #[must_use]
    pub fn with_circuit_breaker_threshold(mut self, threshold: u32) -> Self {
        self.circuit_breaker_threshold = threshold;
        self
    }

    /// Sets the circuit breaker reset timeout.
    #[must_use]
    pub fn with_circuit_breaker_reset_timeout(mut self, timeout: Duration) -> Self {
        self.circuit_breaker_reset_timeout = timeout;
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets whether to log degraded decisions.
    #[must_use]
    pub fn with_log_degraded_decisions(mut self, log: bool) -> Self {
        self.log_degraded_decisions = log;
        self
    }

    /// Creates a fail-open configuration.
    ///
    /// All requests are allowed when the service is unavailable.
    pub fn fail_open() -> Self {
        Self {
            failure_mode: FailureMode::FailOpen,
            ..Default::default()
        }
    }

    /// Creates a fail-closed configuration.
    ///
    /// All requests are denied when the service is unavailable.
    pub fn fail_closed() -> Self {
        Self {
            failure_mode: FailureMode::FailClosed,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fail_closed() {
        let config = DegradationConfig::new();
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
        let config = DegradationConfig::new()
            .with_circuit_breaker_enabled(true)
            .with_circuit_breaker_threshold(10)
            .with_circuit_breaker_reset_timeout(Duration::from_secs(60));

        assert!(config.circuit_breaker_enabled);
        assert_eq!(config.circuit_breaker_threshold, 10);
        assert_eq!(config.circuit_breaker_reset_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_request_timeout() {
        let config = DegradationConfig::new().with_request_timeout(Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(10));
    }
}
