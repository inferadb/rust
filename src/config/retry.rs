//! Retry configuration for transient failures.

use std::time::Duration;

/// Configuration for retry behavior on transient failures.
///
/// The SDK uses exponential backoff with jitter for retries.
///
/// ## Default Values
///
/// - `max_retries`: 3
/// - `initial_delay`: 100ms
/// - `max_delay`: 10s
/// - `multiplier`: 2.0
/// - `jitter`: 0.1 (10%)
///
/// ## Example
///
/// ```rust
/// use inferadb::RetryConfig;
/// use std::time::Duration;
///
/// let config = RetryConfig::new()
///     .with_max_retries(5)
///     .with_initial_delay(Duration::from_millis(200))
///     .with_max_delay(Duration::from_secs(30));
/// ```
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,

    /// Initial delay before the first retry.
    pub initial_delay: Duration,

    /// Maximum delay between retries.
    pub max_delay: Duration,

    /// Multiplier for exponential backoff.
    pub multiplier: f64,

    /// Jitter factor (0.0 to 1.0) to add randomness to delays.
    pub jitter: f64,

    /// Whether to retry on timeout errors.
    pub retry_on_timeout: bool,

    /// Whether to retry on connection errors.
    pub retry_on_connection_error: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: 0.1,
            retry_on_timeout: true,
            retry_on_connection_error: true,
        }
    }
}

impl RetryConfig {
    /// Creates a new retry configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a configuration that disables retries.
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Sets the maximum number of retry attempts.
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the initial delay before the first retry.
    #[must_use]
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay between retries.
    #[must_use]
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Sets the exponential backoff multiplier.
    #[must_use]
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Sets the jitter factor.
    #[must_use]
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Sets whether to retry on timeout errors.
    #[must_use]
    pub fn with_retry_on_timeout(mut self, retry: bool) -> Self {
        self.retry_on_timeout = retry;
        self
    }

    /// Sets whether to retry on connection errors.
    #[must_use]
    pub fn with_retry_on_connection_error(mut self, retry: bool) -> Self {
        self.retry_on_connection_error = retry;
        self
    }

    /// Calculates the delay for a given retry attempt.
    ///
    /// Uses exponential backoff: `initial_delay * multiplier^attempt`
    /// capped at `max_delay`, with optional jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let base_delay =
            self.initial_delay.as_secs_f64() * self.multiplier.powi(attempt as i32 - 1);
        let capped_delay = base_delay.min(self.max_delay.as_secs_f64());

        // Add jitter
        let jittered = if self.jitter > 0.0 {
            let jitter_range = capped_delay * self.jitter;
            let jitter_offset = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
            (capped_delay + jitter_offset).max(0.0)
        } else {
            capped_delay
        };

        Duration::from_secs_f64(jittered)
    }

    /// Returns `true` if retries are enabled.
    pub fn is_enabled(&self) -> bool {
        self.max_retries > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert!(config.is_enabled());
    }

    #[test]
    fn test_disabled() {
        let config = RetryConfig::disabled();
        assert_eq!(config.max_retries, 0);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_builder() {
        let config = RetryConfig::new()
            .with_max_retries(5)
            .with_initial_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(30))
            .with_multiplier(3.0)
            .with_jitter(0.2);

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 3.0);
        assert_eq!(config.jitter, 0.2);
    }

    #[test]
    fn test_delay_for_attempt() {
        let config = RetryConfig::new()
            .with_jitter(0.0) // Disable jitter for predictable testing
            .with_initial_delay(Duration::from_millis(100))
            .with_multiplier(2.0);

        assert_eq!(config.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_delay_capped_at_max() {
        let config = RetryConfig::new()
            .with_jitter(0.0)
            .with_initial_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(5))
            .with_multiplier(10.0);

        // 1 * 10^9 would be huge, but should be capped at 5s
        assert!(config.delay_for_attempt(10) <= Duration::from_secs(5));
    }

    #[test]
    fn test_jitter_clamped() {
        let config = RetryConfig::new().with_jitter(2.0);
        assert_eq!(config.jitter, 1.0);

        let config = RetryConfig::new().with_jitter(-0.5);
        assert_eq!(config.jitter, 0.0);
    }

    #[test]
    fn test_with_retry_on_timeout() {
        let config = RetryConfig::new().with_retry_on_timeout(false);
        assert!(!config.retry_on_timeout);

        let config = RetryConfig::new().with_retry_on_timeout(true);
        assert!(config.retry_on_timeout);
    }

    #[test]
    fn test_with_retry_on_connection_error() {
        let config = RetryConfig::new().with_retry_on_connection_error(false);
        assert!(!config.retry_on_connection_error);

        let config = RetryConfig::new().with_retry_on_connection_error(true);
        assert!(config.retry_on_connection_error);
    }

    #[test]
    fn test_delay_with_jitter() {
        let config = RetryConfig::new()
            .with_jitter(0.5) // 50% jitter
            .with_initial_delay(Duration::from_millis(100));

        // With jitter, the delay should vary but still be reasonable
        let delay = config.delay_for_attempt(1);
        // With 50% jitter, delay should be between 50ms and 150ms
        assert!(delay >= Duration::from_millis(50));
        assert!(delay <= Duration::from_millis(150));
    }
}
