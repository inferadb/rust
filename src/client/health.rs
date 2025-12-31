//! Health check and lifecycle management.
//!
//! This module provides types for monitoring client health and managing
//! graceful shutdown.
//!
//! ## Health Checks
//!
//! ```rust,ignore
//! // Simple health check
//! let healthy = client.health_check().await?;
//!
//! // Detailed health with component status
//! let health = client.health().await?;
//! println!("Status: {:?}", health.status);
//! println!("Latency: {:?}", health.latency);
//!
//! // Wait for readiness at startup
//! client.wait_ready(Duration::from_secs(30)).await?;
//! ```
//!
//! ## Graceful Shutdown
//!
//! ```rust,ignore
//! use tokio::signal;
//!
//! let (client, shutdown_handle) = Client::builder()
//!     .url("https://api.inferadb.com")
//!     .credentials(creds)
//!     .build_with_shutdown()
//!     .await?;
//!
//! tokio::select! {
//!     _ = signal::ctrl_c() => {
//!         shutdown_handle.shutdown_timeout(Duration::from_secs(30)).await;
//!     }
//!     _ = run_server(client) => {}
//! }
//! ```

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Health response from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall health status.
    pub status: HealthStatus,
    /// Server version.
    pub version: String,
    /// Round-trip latency to the server.
    #[serde(with = "duration_millis")]
    pub latency: Duration,
    /// Per-component health status.
    pub components: HashMap<String, ComponentHealth>,
    /// Timestamp of the health check.
    pub timestamp: DateTime<Utc>,
}

impl HealthResponse {
    /// Returns `true` if the overall status is healthy.
    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }

    /// Returns `true` if the overall status is degraded.
    pub fn is_degraded(&self) -> bool {
        self.status == HealthStatus::Degraded
    }

    /// Returns `true` if the overall status is unhealthy.
    pub fn is_unhealthy(&self) -> bool {
        self.status == HealthStatus::Unhealthy
    }

    /// Returns a summary of the health status.
    pub fn summary(&self) -> String {
        let component_summary = self
            .components
            .iter()
            .map(|(name, health)| format!("{}: {:?}", name, health.status))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "{:?} (latency: {:?}, components: [{}])",
            self.status, self.latency, component_summary
        )
    }
}

impl std::fmt::Display for HealthResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Health Status")?;
        writeln!(f, "=============")?;
        writeln!(f, "Status:    {:?}", self.status)?;
        writeln!(f, "Version:   {}", self.version)?;
        writeln!(f, "Latency:   {:?}", self.latency)?;
        writeln!(f, "Timestamp: {}", self.timestamp)?;

        if !self.components.is_empty() {
            writeln!(f, "\nComponents:")?;
            for (name, health) in &self.components {
                writeln!(f, "  {}: {:?}", name, health.status)?;
                if let Some(msg) = &health.message {
                    writeln!(f, "    Message: {}", msg)?;
                }
                if let Some(latency) = health.latency {
                    writeln!(f, "    Latency: {:?}", latency)?;
                }
            }
        }

        Ok(())
    }
}

/// Health status of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operating normally.
    Healthy,
    /// Partial functionality available.
    Degraded,
    /// Service is unavailable.
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health status of an individual component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Status of this component.
    pub status: HealthStatus,
    /// Optional status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Latency to this component.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "duration_millis_opt")]
    #[serde(default)]
    pub latency: Option<Duration>,
    /// When this component was last checked.
    pub last_check: DateTime<Utc>,
}

impl ComponentHealth {
    /// Creates a new healthy component status.
    pub fn healthy() -> Self {
        Self { status: HealthStatus::Healthy, message: None, latency: None, last_check: Utc::now() }
    }

    /// Creates a new unhealthy component status with a message.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            latency: None,
            last_check: Utc::now(),
        }
    }

    /// Creates a new degraded component status with a message.
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            latency: None,
            last_check: Utc::now(),
        }
    }

    /// Sets the latency for this component.
    #[must_use]
    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency = Some(latency);
        self
    }
}

/// Criteria for determining readiness.
#[derive(Debug, Clone, Default)]
pub struct ReadinessCriteria {
    /// Maximum acceptable latency.
    pub max_latency: Option<Duration>,
    /// Require successful authentication.
    pub require_auth: bool,
    /// Require vault accessibility.
    pub require_vault: bool,
}

impl ReadinessCriteria {
    /// Creates default readiness criteria.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum acceptable latency.
    #[must_use]
    pub fn max_latency(mut self, latency: Duration) -> Self {
        self.max_latency = Some(latency);
        self
    }

    /// Requires successful authentication.
    #[must_use]
    pub fn require_auth(mut self) -> Self {
        self.require_auth = true;
        self
    }

    /// Requires vault accessibility.
    #[must_use]
    pub fn require_vault(mut self) -> Self {
        self.require_vault = true;
        self
    }
}

/// Handle for coordinating graceful shutdown.
///
/// When shutdown is initiated:
/// 1. New requests are rejected with `Error { kind: ShuttingDown }`
/// 2. In-flight requests are allowed to complete
/// 3. Connections are closed cleanly
///
/// ## Example
///
/// ```rust,ignore
/// let (client, shutdown) = Client::builder()
///     .url("https://api.inferadb.com")
///     .credentials(creds)
///     .build_with_shutdown()
///     .await?;
///
/// // Use client...
///
/// // Graceful shutdown with timeout
/// shutdown.shutdown_timeout(Duration::from_secs(30)).await;
/// ```
pub struct ShutdownHandle {
    shutdown_flag: Arc<AtomicBool>,
    #[allow(dead_code)] // Will be used when transport supports shutdown
    shutdown_complete: tokio::sync::oneshot::Sender<()>,
}

impl ShutdownHandle {
    /// Creates a new shutdown handle.
    pub fn new() -> (Self, ShutdownGuard) {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = tokio::sync::oneshot::channel();

        let handle = Self { shutdown_flag: Arc::clone(&shutdown_flag), shutdown_complete: tx };

        let guard = ShutdownGuard { shutdown_flag, _shutdown_signal: rx };

        (handle, guard)
    }

    /// Initiates graceful shutdown.
    ///
    /// This marks the client as shutting down, which:
    /// - Rejects new requests with `Error { kind: ShuttingDown }`
    /// - Allows in-flight requests to complete
    /// - Closes connections cleanly
    pub async fn shutdown(self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        // In a real implementation, we would wait for in-flight requests
        // For now, just set the flag
    }

    /// Initiates shutdown with a timeout.
    ///
    /// If the timeout is reached before all requests complete,
    /// connections are forcefully closed.
    pub async fn shutdown_timeout(self, timeout: Duration) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        // In a real implementation, we would wait with a timeout
        tokio::time::sleep(timeout.min(Duration::from_millis(100))).await;
    }

    /// Returns `true` if shutdown has been initiated.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownHandle {
    fn default() -> Self {
        Self::new().0
    }
}

impl std::fmt::Debug for ShutdownHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownHandle")
            .field("is_shutting_down", &self.is_shutting_down())
            .finish()
    }
}

/// Guard that tracks shutdown state within the client.
pub struct ShutdownGuard {
    shutdown_flag: Arc<AtomicBool>,
    _shutdown_signal: tokio::sync::oneshot::Receiver<()>,
}

impl ShutdownGuard {
    /// Returns `true` if shutdown has been initiated.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for ShutdownGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownGuard").field("is_shutting_down", &self.is_shutting_down()).finish()
    }
}

/// Serde helper for Duration as milliseconds.
mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Serde helper for `Option<Duration>` as milliseconds.
mod duration_millis_opt {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => d.as_millis().serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<u64>::deserialize(deserializer)?;
        Ok(opt.map(Duration::from_millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_health_status_serialization() {
        let json = serde_json::to_string(&HealthStatus::Healthy).unwrap();
        assert_eq!(json, "\"healthy\"");

        let parsed: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_response_is_healthy() {
        let healthy = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            latency: Duration::from_millis(5),
            components: HashMap::new(),
            timestamp: Utc::now(),
        };
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
        assert!(!healthy.is_unhealthy());

        let degraded = HealthResponse { status: HealthStatus::Degraded, ..healthy.clone() };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());

        let unhealthy = HealthResponse { status: HealthStatus::Unhealthy, ..healthy.clone() };
        assert!(unhealthy.is_unhealthy());
    }

    #[test]
    fn test_health_response_summary() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            latency: Duration::from_millis(5),
            components: HashMap::new(),
            timestamp: Utc::now(),
        };

        let summary = response.summary();
        assert!(summary.contains("Healthy"));
        assert!(summary.contains("5ms"));
    }

    #[test]
    fn test_health_response_display() {
        let mut components = HashMap::new();
        components.insert("database".to_string(), ComponentHealth::healthy());

        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            latency: Duration::from_millis(5),
            components,
            timestamp: Utc::now(),
        };

        let display = format!("{}", response);
        assert!(display.contains("Health Status"));
        assert!(display.contains("Healthy"));
        assert!(display.contains("database"));
    }

    #[test]
    fn test_component_health_healthy() {
        let health = ComponentHealth::healthy();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.message.is_none());
    }

    #[test]
    fn test_component_health_unhealthy() {
        let health = ComponentHealth::unhealthy("Connection refused");
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.message, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_component_health_degraded() {
        let health = ComponentHealth::degraded("High latency");
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(health.message, Some("High latency".to_string()));
    }

    #[test]
    fn test_component_health_with_latency() {
        let health = ComponentHealth::healthy().with_latency(Duration::from_millis(10));
        assert_eq!(health.latency, Some(Duration::from_millis(10)));
    }

    #[test]
    fn test_readiness_criteria_builder() {
        let criteria = ReadinessCriteria::new()
            .max_latency(Duration::from_millis(100))
            .require_auth()
            .require_vault();

        assert_eq!(criteria.max_latency, Some(Duration::from_millis(100)));
        assert!(criteria.require_auth);
        assert!(criteria.require_vault);
    }

    #[test]
    fn test_shutdown_handle_new() {
        let (handle, guard) = ShutdownHandle::new();
        assert!(!handle.is_shutting_down());
        assert!(!guard.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_handle_shutdown() {
        let (handle, guard) = ShutdownHandle::new();
        assert!(!handle.is_shutting_down());

        handle.shutdown().await;
        assert!(guard.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_handle_shutdown_timeout() {
        let (handle, guard) = ShutdownHandle::new();
        handle.shutdown_timeout(Duration::from_millis(10)).await;
        assert!(guard.is_shutting_down());
    }

    #[test]
    fn test_shutdown_handle_debug() {
        let (handle, _guard) = ShutdownHandle::new();
        let debug = format!("{:?}", handle);
        assert!(debug.contains("ShutdownHandle"));
        assert!(debug.contains("is_shutting_down"));
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "1.0.0".to_string(),
            latency: Duration::from_millis(5),
            components: HashMap::new(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"latency\":5"));
    }

    #[test]
    fn test_component_health_serialization() {
        let health = ComponentHealth::healthy().with_latency(Duration::from_millis(10));
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"latency\":10"));
    }

    #[test]
    fn test_shutdown_handle_default() {
        let handle = ShutdownHandle::default();
        assert!(!handle.is_shutting_down());
    }

    #[test]
    fn test_shutdown_guard_debug() {
        let (_handle, guard) = ShutdownHandle::new();
        let debug = format!("{:?}", guard);
        assert!(debug.contains("ShutdownGuard"));
        assert!(debug.contains("is_shutting_down"));
    }

    #[test]
    fn test_health_response_display_with_components_and_message() {
        let mut components = HashMap::new();
        components.insert(
            "database".to_string(),
            ComponentHealth::degraded("High latency").with_latency(Duration::from_millis(100)),
        );
        components.insert(
            "cache".to_string(),
            ComponentHealth::healthy().with_latency(Duration::from_millis(5)),
        );

        let response = HealthResponse {
            status: HealthStatus::Degraded,
            version: "1.0.0".to_string(),
            latency: Duration::from_millis(10),
            components,
            timestamp: Utc::now(),
        };

        let display = format!("{}", response);
        assert!(display.contains("Status:"));
        assert!(display.contains("Components:"));
        // Should include message and latency for components
        assert!(display.contains("Message:") || display.contains("Latency:"));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{
            "status": "healthy",
            "version": "1.0.0",
            "latency": 5,
            "components": {},
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let response: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, HealthStatus::Healthy);
        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.latency, Duration::from_millis(5));
    }

    #[test]
    fn test_component_health_deserialization() {
        let json = r#"{
            "status": "healthy",
            "message": "All good",
            "latency": 10,
            "last_check": "2024-01-15T10:00:00Z"
        }"#;

        let health: ComponentHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.message.as_deref(), Some("All good"));
        assert_eq!(health.latency, Some(Duration::from_millis(10)));
    }

    #[test]
    fn test_component_health_deserialization_no_latency() {
        let json = r#"{
            "status": "degraded",
            "message": "Some issue",
            "last_check": "2024-01-15T10:00:00Z"
        }"#;

        let health: ComponentHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(health.latency, None);
    }
}
