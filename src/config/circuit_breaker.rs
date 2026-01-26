//! Circuit breaker configuration for preventing cascade failures.
//!
//! Circuit breakers prevent cascade failures by temporarily stopping requests
//! to a failing service. Unlike retry, which handles transient failures,
//! circuit breakers protect against sustained outages.
//!
//! ## States
//!
//! - **Closed**: Normal operation, requests flow through
//! - **Open**: Requests fail immediately (circuit tripped)
//! - **HalfOpen**: Testing if service has recovered
//!
//! ## Example
//!
//! ```rust
//! use inferadb::CircuitBreakerConfig;
//! use std::time::Duration;
//!
//! let config = CircuitBreakerConfig::builder()
//!     .failure_threshold(5)           // Open after 5 consecutive failures
//!     .success_threshold(2)           // Close after 2 successes in half-open
//!     .timeout(Duration::from_secs(30))  // Try half-open after 30s
//!     .build();
//! ```

use std::time::Duration;

use crate::ErrorKind;

/// Circuit breaker configuration.
///
/// Controls when the circuit breaker opens (stops requests) and closes
/// (resumes requests) based on failure patterns.
///
/// ## Example
///
/// ```rust
/// use inferadb::CircuitBreakerConfig;
/// use std::time::Duration;
///
/// let config = CircuitBreakerConfig::builder()
///     .failure_threshold(5)
///     .success_threshold(2)
///     .timeout(Duration::from_secs(30))
///     .failure_rate_threshold(0.5)
///     .minimum_requests(10)
///     .build();
/// ```
#[derive(Debug, Clone, bon::Builder)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to open the circuit.
    #[builder(default = 5)]
    failure_threshold: u32,

    /// Number of successes in half-open state to close the circuit.
    #[builder(default = 2)]
    success_threshold: u32,

    /// Duration to wait before transitioning from open to half-open.
    #[builder(default = Duration::from_secs(30))]
    timeout: Duration,

    /// Alternative: Open circuit when failure rate exceeds this threshold.
    #[builder(default = 0.5)]
    failure_rate_threshold: f64,

    /// Minimum number of requests before failure rate is considered.
    #[builder(default = 10)]
    minimum_requests: u32,

    /// Which errors count as failures.
    #[builder(default)]
    failure_predicate: FailurePredicate,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl CircuitBreakerConfig {
    /// Returns the failure threshold.
    pub fn get_failure_threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// Returns the success threshold.
    pub fn get_success_threshold(&self) -> u32 {
        self.success_threshold
    }

    /// Returns the timeout.
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the failure rate threshold.
    pub fn get_failure_rate_threshold(&self) -> f64 {
        self.failure_rate_threshold
    }

    /// Returns the minimum requests.
    pub fn get_minimum_requests(&self) -> u32 {
        self.minimum_requests
    }

    /// Returns the failure predicate.
    pub fn get_failure_predicate(&self) -> &FailurePredicate {
        &self.failure_predicate
    }

    /// Returns whether the given error kind counts as a failure.
    pub fn is_failure(&self, kind: ErrorKind) -> bool {
        self.failure_predicate.is_failure(kind)
    }
}

/// Determines which errors count toward circuit breaker failure threshold.
///
/// ## Example
///
/// ```rust
/// use inferadb::{FailurePredicate, ErrorKind};
///
/// // Only count timeouts and connection failures
/// let predicate = FailurePredicate::only([
///     ErrorKind::Timeout,
///     ErrorKind::Connection,
/// ]);
///
/// // Default plus exclude rate limiting
/// let predicate = FailurePredicate::default()
///     .exclude(ErrorKind::RateLimited);
/// ```
#[derive(Debug, Clone)]
pub struct FailurePredicate {
    /// Count these error kinds as failures.
    include: Vec<ErrorKind>,
    /// Exclude these error kinds from failure count.
    exclude: Vec<ErrorKind>,
}

impl Default for FailurePredicate {
    /// Default: Timeout, Connection, Unavailable, Internal are failures.
    fn default() -> Self {
        Self {
            include: vec![
                ErrorKind::Timeout,
                ErrorKind::Connection,
                ErrorKind::Unavailable,
                ErrorKind::Internal,
            ],
            exclude: vec![],
        }
    }
}

impl FailurePredicate {
    /// Creates a predicate that only counts specific error kinds as failures.
    pub fn only(kinds: impl IntoIterator<Item = ErrorKind>) -> Self {
        Self { include: kinds.into_iter().collect(), exclude: vec![] }
    }

    /// Adds an error kind to exclude from failure count.
    #[must_use]
    pub fn exclude(mut self, kind: ErrorKind) -> Self {
        self.exclude.push(kind);
        self
    }

    /// Adds an error kind to include in failure count.
    #[must_use]
    pub fn include(mut self, kind: ErrorKind) -> Self {
        self.include.push(kind);
        self
    }

    /// Returns whether the given error kind counts as a failure.
    pub fn is_failure(&self, kind: ErrorKind) -> bool {
        self.include.contains(&kind) && !self.exclude.contains(&kind)
    }
}

/// Current state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation, requests flow through.
    Closed,
    /// Requests fail immediately (circuit tripped).
    Open,
    /// Testing if service has recovered.
    HalfOpen,
}

impl CircuitState {
    /// Returns `true` if the circuit is closed (normal operation).
    pub fn is_closed(&self) -> bool {
        matches!(self, CircuitState::Closed)
    }

    /// Returns `true` if the circuit is open (blocking requests).
    pub fn is_open(&self) -> bool {
        matches!(self, CircuitState::Open)
    }

    /// Returns `true` if the circuit is half-open (testing recovery).
    pub fn is_half_open(&self) -> bool {
        matches!(self, CircuitState::HalfOpen)
    }
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Detailed circuit breaker statistics.
///
/// Use this for monitoring and alerting on circuit breaker state.
#[derive(Debug, Clone)]
pub struct CircuitStats {
    /// Current state of the circuit.
    pub state: CircuitState,
    /// Number of consecutive failures.
    pub failure_count: u32,
    /// Number of consecutive successes (in half-open state).
    pub success_count: u32,
    /// Total requests since last state change.
    pub total_requests: u64,
    /// Failed requests since last state change.
    pub failed_requests: u64,
    /// Time when circuit last transitioned to open.
    pub last_open_time: Option<std::time::Instant>,
    /// Time when circuit last transitioned to closed.
    pub last_close_time: Option<std::time::Instant>,
}

impl CircuitStats {
    /// Creates new stats in the closed state.
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            total_requests: 0,
            failed_requests: 0,
            last_open_time: None,
            last_close_time: None,
        }
    }

    /// Returns the current state.
    pub fn current_state(&self) -> CircuitState {
        self.state
    }

    /// Returns the failure count.
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Returns the success count.
    pub fn success_count(&self) -> u32 {
        self.success_count
    }

    /// Returns the current failure rate.
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.failed_requests as f64 / self.total_requests as f64
        }
    }
}

impl Default for CircuitStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the circuit breaker.
#[derive(Debug, Clone)]
pub enum CircuitEvent {
    /// Circuit transitioned to open state.
    Opened {
        /// Number of failures that triggered the open.
        failure_count: u32,
        /// Description of the last error.
        last_error: String,
    },
    /// Circuit transitioned to half-open state.
    HalfOpened,
    /// Circuit transitioned to closed state.
    Closed {
        /// Number of successes that triggered the close.
        success_count: u32,
    },
}

impl std::fmt::Display for CircuitEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitEvent::Opened { failure_count, last_error } => {
                write!(f, "circuit opened after {} failures: {}", failure_count, last_error)
            },
            CircuitEvent::HalfOpened => write!(f, "circuit half-opened (testing recovery)"),
            CircuitEvent::Closed { success_count } => {
                write!(f, "circuit closed after {} successes", success_count)
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.failure_rate_threshold, 0.5);
        assert_eq!(config.minimum_requests, 10);
    }

    #[test]
    fn test_config_builder() {
        let config = CircuitBreakerConfig::builder()
            .failure_threshold(10)
            .success_threshold(3)
            .timeout(Duration::from_secs(60))
            .failure_rate_threshold(0.8)
            .minimum_requests(20)
            .build();

        assert_eq!(config.get_failure_threshold(), 10);
        assert_eq!(config.get_success_threshold(), 3);
        assert_eq!(config.get_timeout(), Duration::from_secs(60));
        assert_eq!(config.get_failure_rate_threshold(), 0.8);
        assert_eq!(config.get_minimum_requests(), 20);
    }

    #[test]
    fn test_default_failure_predicate() {
        let predicate = FailurePredicate::default();
        assert!(predicate.is_failure(ErrorKind::Timeout));
        assert!(predicate.is_failure(ErrorKind::Connection));
        assert!(predicate.is_failure(ErrorKind::Unavailable));
        assert!(predicate.is_failure(ErrorKind::Internal));
        assert!(!predicate.is_failure(ErrorKind::Forbidden));
        assert!(!predicate.is_failure(ErrorKind::NotFound));
    }

    #[test]
    fn test_failure_predicate_only() {
        let predicate = FailurePredicate::only([ErrorKind::Timeout]);
        assert!(predicate.is_failure(ErrorKind::Timeout));
        assert!(!predicate.is_failure(ErrorKind::Connection));
    }

    #[test]
    fn test_failure_predicate_exclude() {
        let predicate = FailurePredicate::default().exclude(ErrorKind::Timeout);
        assert!(!predicate.is_failure(ErrorKind::Timeout));
        assert!(predicate.is_failure(ErrorKind::Connection));
    }

    #[test]
    fn test_circuit_state() {
        assert!(CircuitState::Closed.is_closed());
        assert!(!CircuitState::Closed.is_open());
        assert!(!CircuitState::Closed.is_half_open());

        assert!(!CircuitState::Open.is_closed());
        assert!(CircuitState::Open.is_open());
        assert!(!CircuitState::Open.is_half_open());

        assert!(!CircuitState::HalfOpen.is_closed());
        assert!(!CircuitState::HalfOpen.is_open());
        assert!(CircuitState::HalfOpen.is_half_open());
    }

    #[test]
    fn test_circuit_stats() {
        let mut stats = CircuitStats::new();
        assert_eq!(stats.current_state(), CircuitState::Closed);
        assert_eq!(stats.failure_count(), 0);
        assert_eq!(stats.success_count(), 0);
        assert_eq!(stats.failure_rate(), 0.0);

        stats.total_requests = 10;
        stats.failed_requests = 3;
        assert!((stats.failure_rate() - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_circuit_event_display() {
        let event =
            CircuitEvent::Opened { failure_count: 5, last_error: "connection refused".to_string() };
        let display = event.to_string();
        assert!(display.contains("5 failures"));
        assert!(display.contains("connection refused"));

        let event = CircuitEvent::HalfOpened;
        assert!(event.to_string().contains("half-opened"));

        let event = CircuitEvent::Closed { success_count: 2 };
        assert!(event.to_string().contains("2 successes"));
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "closed");
        assert_eq!(format!("{}", CircuitState::Open), "open");
        assert_eq!(format!("{}", CircuitState::HalfOpen), "half-open");
    }

    #[test]
    fn test_circuit_stats_default() {
        let stats = CircuitStats::default();
        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 0);
    }

    #[test]
    fn test_failure_predicate_include() {
        let predicate = FailurePredicate::only([ErrorKind::Timeout]).include(ErrorKind::Connection);
        assert!(predicate.is_failure(ErrorKind::Timeout));
        assert!(predicate.is_failure(ErrorKind::Connection));
        assert!(!predicate.is_failure(ErrorKind::NotFound));
    }

    #[test]
    fn test_config_is_failure() {
        let config = CircuitBreakerConfig::default();
        assert!(config.is_failure(ErrorKind::Timeout));
        assert!(!config.is_failure(ErrorKind::NotFound));
    }

    #[test]
    fn test_config_custom_predicate() {
        let predicate = FailurePredicate::only([ErrorKind::NotFound]);
        let config = CircuitBreakerConfig::builder().failure_predicate(predicate).build();
        assert!(config.is_failure(ErrorKind::NotFound));
        assert!(!config.is_failure(ErrorKind::Timeout));
    }

    #[test]
    fn test_config_get_failure_predicate() {
        let config = CircuitBreakerConfig::default();
        let predicate = config.get_failure_predicate();
        assert!(predicate.is_failure(ErrorKind::Timeout));
    }

    #[test]
    fn test_circuit_event_clone() {
        let event = CircuitEvent::Opened { failure_count: 5, last_error: "error".to_string() };
        let cloned = event.clone();
        match cloned {
            CircuitEvent::Opened { failure_count, .. } => assert_eq!(failure_count, 5),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_circuit_state_copy() {
        let state = CircuitState::Open;
        let copied: CircuitState = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn test_circuit_stats_with_times() {
        let mut stats = CircuitStats::new();
        stats.last_open_time = Some(std::time::Instant::now());
        stats.last_close_time = Some(std::time::Instant::now());
        assert!(stats.last_open_time.is_some());
        assert!(stats.last_close_time.is_some());
    }
}
