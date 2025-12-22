//! Metrics collection for observability.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;

/// Configuration for metrics collection.
///
/// ## Example
///
/// ```rust
/// use inferadb::tracing_support::MetricsConfig;
///
/// let config = MetricsConfig::default()
///     .with_prefix("inferadb")
///     .with_histograms_enabled(true);
/// ```
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Prefix for all metric names.
    pub prefix: String,
    /// Whether to collect histogram metrics.
    pub histograms_enabled: bool,
    /// Histogram bucket boundaries for latency metrics (in seconds).
    pub latency_buckets: Vec<f64>,
    /// Labels to add to all metrics.
    pub global_labels: HashMap<String, String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            prefix: "inferadb".to_string(),
            histograms_enabled: true,
            latency_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
            global_labels: HashMap::new(),
        }
    }
}

impl MetricsConfig {
    /// Sets the metric name prefix.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Enables or disables histogram collection.
    #[must_use]
    pub fn with_histograms_enabled(mut self, enabled: bool) -> Self {
        self.histograms_enabled = enabled;
        self
    }

    /// Sets the latency histogram buckets.
    #[must_use]
    pub fn with_latency_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.latency_buckets = buckets;
        self
    }

    /// Adds a global label.
    #[must_use]
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_labels.insert(key.into(), value.into());
        self
    }
}

/// A metrics collector for InferaDB SDK operations.
///
/// ## Example
///
/// ```rust
/// use inferadb::tracing_support::{Metrics, MetricsConfig};
///
/// let metrics = Metrics::new(MetricsConfig::default());
///
/// // Record a check operation
/// metrics.record_check_latency(std::time::Duration::from_millis(15), true);
/// metrics.increment_check_count(true);
///
/// // Get a snapshot
/// let snapshot = metrics.snapshot();
/// println!("Total checks: {}", snapshot.check_total);
/// ```
#[derive(Debug, Clone)]
pub struct Metrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug)]
struct MetricsInner {
    config: MetricsConfig,
    // Counters
    check_total: AtomicU64,
    check_allowed: AtomicU64,
    check_denied: AtomicU64,
    check_errors: AtomicU64,
    relationship_writes: AtomicU64,
    relationship_deletes: AtomicU64,
    // Latency histograms (simplified - stores sum and count)
    check_latency_sum_ns: AtomicU64,
    check_latency_count: AtomicU64,
    write_latency_sum_ns: AtomicU64,
    write_latency_count: AtomicU64,
    // Connection metrics
    connection_pool_size: AtomicU64,
    connection_errors: AtomicU64,
    // Custom counters
    custom_counters: RwLock<HashMap<String, AtomicU64>>,
    custom_gauges: RwLock<HashMap<String, AtomicU64>>,
}

impl Metrics {
    /// Creates a new metrics collector with the given configuration.
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                config,
                check_total: AtomicU64::new(0),
                check_allowed: AtomicU64::new(0),
                check_denied: AtomicU64::new(0),
                check_errors: AtomicU64::new(0),
                relationship_writes: AtomicU64::new(0),
                relationship_deletes: AtomicU64::new(0),
                check_latency_sum_ns: AtomicU64::new(0),
                check_latency_count: AtomicU64::new(0),
                write_latency_sum_ns: AtomicU64::new(0),
                write_latency_count: AtomicU64::new(0),
                connection_pool_size: AtomicU64::new(0),
                connection_errors: AtomicU64::new(0),
                custom_counters: RwLock::new(HashMap::new()),
                custom_gauges: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Returns the metrics configuration.
    pub fn config(&self) -> &MetricsConfig {
        &self.inner.config
    }

    /// Increments the check counter.
    pub fn increment_check_count(&self, allowed: bool) {
        self.inner.check_total.fetch_add(1, Ordering::Relaxed);
        if allowed {
            self.inner.check_allowed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.inner.check_denied.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increments the check error counter.
    pub fn increment_check_errors(&self) {
        self.inner.check_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Records check latency.
    pub fn record_check_latency(&self, duration: Duration, _allowed: bool) {
        let nanos = duration.as_nanos() as u64;
        self.inner.check_latency_sum_ns.fetch_add(nanos, Ordering::Relaxed);
        self.inner.check_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the relationship write counter.
    pub fn increment_relationship_writes(&self, count: u64) {
        self.inner.relationship_writes.fetch_add(count, Ordering::Relaxed);
    }

    /// Increments the relationship delete counter.
    pub fn increment_relationship_deletes(&self, count: u64) {
        self.inner.relationship_deletes.fetch_add(count, Ordering::Relaxed);
    }

    /// Records write latency.
    pub fn record_write_latency(&self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        self.inner.write_latency_sum_ns.fetch_add(nanos, Ordering::Relaxed);
        self.inner.write_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Sets the connection pool size gauge.
    pub fn set_connection_pool_size(&self, size: u64) {
        self.inner.connection_pool_size.store(size, Ordering::Relaxed);
    }

    /// Increments the connection error counter.
    pub fn increment_connection_errors(&self) {
        self.inner.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns a custom counter, creating it if it doesn't exist.
    pub fn counter(&self, name: &str) -> Counter {
        let counters = self.inner.custom_counters.read();
        if counters.contains_key(name) {
            drop(counters);
            return Counter {
                name: name.to_string(),
                metrics: self.clone(),
            };
        }
        drop(counters);

        let mut counters = self.inner.custom_counters.write();
        counters.entry(name.to_string()).or_insert_with(|| AtomicU64::new(0));

        Counter {
            name: name.to_string(),
            metrics: self.clone(),
        }
    }

    /// Returns a custom gauge, creating it if it doesn't exist.
    pub fn gauge(&self, name: &str) -> Gauge {
        let gauges = self.inner.custom_gauges.read();
        if gauges.contains_key(name) {
            drop(gauges);
            return Gauge {
                name: name.to_string(),
                metrics: self.clone(),
            };
        }
        drop(gauges);

        let mut gauges = self.inner.custom_gauges.write();
        gauges.entry(name.to_string()).or_insert_with(|| AtomicU64::new(0));

        Gauge {
            name: name.to_string(),
            metrics: self.clone(),
        }
    }

    /// Returns a histogram (simplified implementation).
    pub fn histogram(&self, name: &str) -> Histogram {
        Histogram {
            name: name.to_string(),
            metrics: self.clone(),
        }
    }

    /// Returns a snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let check_count = self.inner.check_latency_count.load(Ordering::Relaxed);
        let check_sum_ns = self.inner.check_latency_sum_ns.load(Ordering::Relaxed);
        let write_count = self.inner.write_latency_count.load(Ordering::Relaxed);
        let write_sum_ns = self.inner.write_latency_sum_ns.load(Ordering::Relaxed);

        MetricsSnapshot {
            check_total: self.inner.check_total.load(Ordering::Relaxed),
            check_allowed: self.inner.check_allowed.load(Ordering::Relaxed),
            check_denied: self.inner.check_denied.load(Ordering::Relaxed),
            check_errors: self.inner.check_errors.load(Ordering::Relaxed),
            relationship_writes: self.inner.relationship_writes.load(Ordering::Relaxed),
            relationship_deletes: self.inner.relationship_deletes.load(Ordering::Relaxed),
            check_latency_avg_ns: if check_count > 0 {
                check_sum_ns / check_count
            } else {
                0
            },
            write_latency_avg_ns: if write_count > 0 {
                write_sum_ns / write_count
            } else {
                0
            },
            connection_pool_size: self.inner.connection_pool_size.load(Ordering::Relaxed),
            connection_errors: self.inner.connection_errors.load(Ordering::Relaxed),
        }
    }

    /// Resets all metrics to zero.
    pub fn reset(&self) {
        self.inner.check_total.store(0, Ordering::Relaxed);
        self.inner.check_allowed.store(0, Ordering::Relaxed);
        self.inner.check_denied.store(0, Ordering::Relaxed);
        self.inner.check_errors.store(0, Ordering::Relaxed);
        self.inner.relationship_writes.store(0, Ordering::Relaxed);
        self.inner.relationship_deletes.store(0, Ordering::Relaxed);
        self.inner.check_latency_sum_ns.store(0, Ordering::Relaxed);
        self.inner.check_latency_count.store(0, Ordering::Relaxed);
        self.inner.write_latency_sum_ns.store(0, Ordering::Relaxed);
        self.inner.write_latency_count.store(0, Ordering::Relaxed);
        self.inner.connection_errors.store(0, Ordering::Relaxed);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

/// A snapshot of metrics values.
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    /// Total number of authorization checks.
    pub check_total: u64,
    /// Number of checks that returned allowed.
    pub check_allowed: u64,
    /// Number of checks that returned denied.
    pub check_denied: u64,
    /// Number of check errors.
    pub check_errors: u64,
    /// Total relationship writes.
    pub relationship_writes: u64,
    /// Total relationship deletes.
    pub relationship_deletes: u64,
    /// Average check latency in nanoseconds.
    pub check_latency_avg_ns: u64,
    /// Average write latency in nanoseconds.
    pub write_latency_avg_ns: u64,
    /// Current connection pool size.
    pub connection_pool_size: u64,
    /// Total connection errors.
    pub connection_errors: u64,
}

impl MetricsSnapshot {
    /// Returns the average check latency as a Duration.
    pub fn check_latency_avg(&self) -> Duration {
        Duration::from_nanos(self.check_latency_avg_ns)
    }

    /// Returns the average write latency as a Duration.
    pub fn write_latency_avg(&self) -> Duration {
        Duration::from_nanos(self.write_latency_avg_ns)
    }

    /// Returns the check allow rate (0.0 - 1.0).
    pub fn check_allow_rate(&self) -> f64 {
        if self.check_total == 0 {
            return 0.0;
        }
        self.check_allowed as f64 / self.check_total as f64
    }

    /// Returns the check error rate (0.0 - 1.0).
    pub fn check_error_rate(&self) -> f64 {
        let total = self.check_total + self.check_errors;
        if total == 0 {
            return 0.0;
        }
        self.check_errors as f64 / total as f64
    }
}

/// A counter metric that can only be incremented.
#[derive(Debug, Clone)]
pub struct Counter {
    name: String,
    metrics: Metrics,
}

impl Counter {
    /// Returns the counter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Increments the counter by 1.
    pub fn increment(&self) {
        self.add(1);
    }

    /// Adds the given value to the counter.
    pub fn add(&self, value: u64) {
        let counters = self.metrics.inner.custom_counters.read();
        if let Some(counter) = counters.get(&self.name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Returns the current value.
    pub fn value(&self) -> u64 {
        let counters = self.metrics.inner.custom_counters.read();
        counters
            .get(&self.name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

/// A gauge metric that can be set to any value.
#[derive(Debug, Clone)]
pub struct Gauge {
    name: String,
    metrics: Metrics,
}

impl Gauge {
    /// Returns the gauge name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the gauge value.
    pub fn set(&self, value: u64) {
        let gauges = self.metrics.inner.custom_gauges.read();
        if let Some(gauge) = gauges.get(&self.name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }

    /// Increments the gauge by 1.
    pub fn increment(&self) {
        self.add(1);
    }

    /// Decrements the gauge by 1.
    pub fn decrement(&self) {
        self.sub(1);
    }

    /// Adds the given value to the gauge.
    pub fn add(&self, value: u64) {
        let gauges = self.metrics.inner.custom_gauges.read();
        if let Some(gauge) = gauges.get(&self.name) {
            gauge.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Subtracts the given value from the gauge.
    pub fn sub(&self, value: u64) {
        let gauges = self.metrics.inner.custom_gauges.read();
        if let Some(gauge) = gauges.get(&self.name) {
            gauge.fetch_sub(value, Ordering::Relaxed);
        }
    }

    /// Returns the current value.
    pub fn value(&self) -> u64 {
        let gauges = self.metrics.inner.custom_gauges.read();
        gauges
            .get(&self.name)
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

/// A histogram metric for recording distributions.
///
/// Note: This is a simplified implementation that records sum and count.
/// For production use, consider integrating with OpenTelemetry or Prometheus.
#[derive(Debug, Clone)]
pub struct Histogram {
    name: String,
    metrics: Metrics,
}

impl Histogram {
    /// Returns the histogram name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Records a value in the histogram.
    pub fn record(&self, _value: f64) {
        // Simplified: just count
        let counters = self.metrics.inner.custom_counters.read();
        if let Some(counter) = counters.get(&self.name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records a duration in the histogram.
    pub fn record_duration(&self, duration: Duration) {
        self.record(duration.as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert_eq!(config.prefix, "inferadb");
        assert!(config.histograms_enabled);
        assert!(!config.latency_buckets.is_empty());
    }

    #[test]
    fn test_metrics_config_builder() {
        let config = MetricsConfig::default()
            .with_prefix("custom")
            .with_histograms_enabled(false)
            .with_label("env", "test");

        assert_eq!(config.prefix, "custom");
        assert!(!config.histograms_enabled);
        assert_eq!(config.global_labels.get("env"), Some(&"test".to_string()));
    }

    #[test]
    fn test_metrics_check_counters() {
        let metrics = Metrics::default();

        metrics.increment_check_count(true);
        metrics.increment_check_count(true);
        metrics.increment_check_count(false);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.check_total, 3);
        assert_eq!(snapshot.check_allowed, 2);
        assert_eq!(snapshot.check_denied, 1);
    }

    #[test]
    fn test_metrics_latency() {
        let metrics = Metrics::default();

        metrics.record_check_latency(Duration::from_millis(10), true);
        metrics.record_check_latency(Duration::from_millis(20), true);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.check_latency_avg_ns, 15_000_000); // 15ms
    }

    #[test]
    fn test_metrics_snapshot_rates() {
        let metrics = Metrics::default();

        for _ in 0..8 {
            metrics.increment_check_count(true);
        }
        for _ in 0..2 {
            metrics.increment_check_count(false);
        }

        let snapshot = metrics.snapshot();
        assert!((snapshot.check_allow_rate() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = Metrics::default();
        metrics.increment_check_count(true);
        metrics.increment_relationship_writes(5);

        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.check_total, 0);
        assert_eq!(snapshot.relationship_writes, 0);
    }

    #[test]
    fn test_custom_counter() {
        let metrics = Metrics::default();
        let counter = metrics.counter("my_counter");

        counter.increment();
        counter.add(5);

        assert_eq!(counter.value(), 6);
    }

    #[test]
    fn test_custom_gauge() {
        let metrics = Metrics::default();
        let gauge = metrics.gauge("my_gauge");

        gauge.set(100);
        assert_eq!(gauge.value(), 100);

        gauge.increment();
        assert_eq!(gauge.value(), 101);

        gauge.decrement();
        assert_eq!(gauge.value(), 100);
    }
}
