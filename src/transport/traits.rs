//! Transport trait definitions and common types.
//!
//! This module defines the core transport abstraction and related types
//! for communication with InferaDB services.

use std::time::{Duration, Instant};

use crate::types::{ConsistencyToken, Context, Decision, Relationship};
use crate::Error;

// ============================================================================
// Transport Enum
// ============================================================================

/// Available transport implementations.
///
/// The SDK supports multiple transport protocols with automatic fallback:
///
/// - **gRPC** (default): Best performance, native streaming via HTTP/2
/// - **Http**: Universal compatibility, firewall-friendly HTTP/1.1
/// - **Mock**: In-memory transport for testing
///
/// ## Example
///
/// ```rust
/// use inferadb::Transport;
///
/// // Check transport type
/// let transport = Transport::Grpc;
/// assert!(transport.is_grpc());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Transport {
    /// gRPC over HTTP/2 (default) - best performance, streaming support.
    #[default]
    Grpc,
    /// REST over HTTP/1.1 - universal compatibility, firewall-friendly.
    Http,
    /// In-memory mock - for testing without network.
    Mock,
}

impl Transport {
    /// Returns `true` if this is gRPC transport.
    pub fn is_grpc(&self) -> bool {
        matches!(self, Transport::Grpc)
    }

    /// Returns `true` if this is HTTP/REST transport.
    pub fn is_http(&self) -> bool {
        matches!(self, Transport::Http)
    }

    /// Returns `true` if this is mock transport.
    pub fn is_mock(&self) -> bool {
        matches!(self, Transport::Mock)
    }
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transport::Grpc => write!(f, "gRPC"),
            Transport::Http => write!(f, "HTTP/REST"),
            Transport::Mock => write!(f, "Mock"),
        }
    }
}

// ============================================================================
// Transport Strategy
// ============================================================================

/// Transport fallback configuration.
///
/// When both `grpc` and `rest` features are enabled, the SDK can automatically
/// handle transport fallback for resilience.
///
/// ## Example
///
/// ```rust
/// use inferadb::{TransportStrategy, FallbackTrigger};
///
/// // Prefer gRPC with automatic fallback
/// let strategy = TransportStrategy::PreferGrpc {
///     fallback_on: FallbackTrigger::default(),
/// };
///
/// // Use gRPC only, fail if unavailable
/// let strategy = TransportStrategy::GrpcOnly;
/// ```
#[derive(Debug, Clone)]
pub enum TransportStrategy {
    /// Use gRPC only, fail if unavailable.
    GrpcOnly,
    /// Use REST only.
    RestOnly,
    /// Prefer gRPC, automatically fall back to REST on failure (default).
    PreferGrpc {
        /// Conditions that trigger fallback.
        fallback_on: FallbackTrigger,
    },
    /// Prefer REST, automatically fall back to gRPC on failure.
    PreferRest {
        /// Conditions that trigger fallback.
        fallback_on: FallbackTrigger,
    },
}

impl Default for TransportStrategy {
    fn default() -> Self {
        TransportStrategy::PreferGrpc {
            fallback_on: FallbackTrigger::default(),
        }
    }
}

impl TransportStrategy {
    /// Returns the preferred transport for this strategy.
    pub fn preferred_transport(&self) -> Transport {
        match self {
            TransportStrategy::GrpcOnly | TransportStrategy::PreferGrpc { .. } => Transport::Grpc,
            TransportStrategy::RestOnly | TransportStrategy::PreferRest { .. } => Transport::Http,
        }
    }

    /// Returns the fallback transport, if any.
    pub fn fallback_transport(&self) -> Option<Transport> {
        match self {
            TransportStrategy::GrpcOnly | TransportStrategy::RestOnly => None,
            TransportStrategy::PreferGrpc { .. } => Some(Transport::Http),
            TransportStrategy::PreferRest { .. } => Some(Transport::Grpc),
        }
    }

    /// Returns `true` if fallback is enabled.
    pub fn has_fallback(&self) -> bool {
        self.fallback_transport().is_some()
    }
}

// ============================================================================
// Fallback Trigger
// ============================================================================

/// Conditions that trigger transport fallback.
///
/// ## Example
///
/// ```rust
/// use inferadb::FallbackTrigger;
///
/// // Default: fallback on connection errors, protocol errors, and timeouts
/// let trigger = FallbackTrigger::default();
///
/// // Custom: only fallback on connection errors
/// let trigger = FallbackTrigger {
///     connection_error: true,
///     protocol_error: false,
///     status_codes: vec![],
///     connect_timeout: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct FallbackTrigger {
    /// Trigger fallback on connection failures (TCP, TLS handshake).
    pub connection_error: bool,
    /// Trigger fallback on HTTP/2 protocol errors (gRPC requires HTTP/2).
    pub protocol_error: bool,
    /// Trigger fallback on specific HTTP status codes (e.g., 502, 503).
    pub status_codes: Vec<u16>,
    /// Trigger fallback on connection timeout.
    pub connect_timeout: bool,
}

impl Default for FallbackTrigger {
    fn default() -> Self {
        Self {
            connection_error: true,
            protocol_error: true,
            status_codes: vec![502, 503],
            connect_timeout: true,
        }
    }
}

impl FallbackTrigger {
    /// Creates a trigger that falls back on any error.
    pub fn on_any_error() -> Self {
        Self {
            connection_error: true,
            protocol_error: true,
            status_codes: vec![500, 502, 503, 504],
            connect_timeout: true,
        }
    }

    /// Creates a trigger that only falls back on connection errors.
    pub fn on_connection_only() -> Self {
        Self {
            connection_error: true,
            protocol_error: false,
            status_codes: vec![],
            connect_timeout: true,
        }
    }

    /// Returns `true` if the given status code should trigger fallback.
    pub fn should_fallback_on_status(&self, status: u16) -> bool {
        self.status_codes.contains(&status)
    }
}

// ============================================================================
// Transport Stats
// ============================================================================

/// Transport layer statistics.
///
/// Provides visibility into transport behavior for monitoring and debugging.
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    /// Currently active transport.
    pub active_transport: Transport,
    /// Number of times fallback was triggered.
    pub fallback_count: u64,
    /// Reason for the last fallback (if any).
    pub last_fallback_reason: Option<FallbackReason>,
    /// Timestamp of last fallback.
    pub last_fallback_at: Option<Instant>,
    /// gRPC-specific stats (if gRPC enabled).
    pub grpc: Option<GrpcStats>,
    /// REST-specific stats (if REST enabled).
    pub rest: Option<RestStats>,
}

/// Reason for transport fallback.
#[derive(Debug, Clone)]
pub enum FallbackReason {
    /// Connection was refused.
    ConnectionRefused,
    /// Protocol error occurred.
    ProtocolError(String),
    /// HTTP status code triggered fallback.
    StatusCode(u16),
    /// Connection timeout.
    ConnectTimeout,
}

impl std::fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallbackReason::ConnectionRefused => write!(f, "connection refused"),
            FallbackReason::ProtocolError(msg) => write!(f, "protocol error: {}", msg),
            FallbackReason::StatusCode(code) => write!(f, "HTTP status {}", code),
            FallbackReason::ConnectTimeout => write!(f, "connect timeout"),
        }
    }
}

/// gRPC transport statistics.
#[derive(Debug, Clone, Default)]
pub struct GrpcStats {
    /// Total requests sent.
    pub requests_sent: u64,
    /// Failed requests.
    pub requests_failed: u64,
    /// Streams opened.
    pub streams_opened: u64,
    /// Currently active streams.
    pub streams_active: u32,
}

/// REST transport statistics.
#[derive(Debug, Clone, Default)]
pub struct RestStats {
    /// Total requests sent.
    pub requests_sent: u64,
    /// Failed requests.
    pub requests_failed: u64,
    /// SSE connections opened.
    pub sse_connections: u64,
    /// Currently active SSE connections.
    pub sse_active: u32,
}

// ============================================================================
// Transport Events
// ============================================================================

/// Events emitted by the transport layer.
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// Transport fallback was triggered.
    FallbackTriggered {
        /// Transport we fell back from.
        from: Transport,
        /// Transport we fell back to.
        to: Transport,
        /// Reason for fallback.
        reason: FallbackReason,
    },
    /// Primary transport was restored.
    Restored {
        /// Transport that was restored.
        transport: Transport,
    },
}

impl std::fmt::Display for TransportEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportEvent::FallbackTriggered { from, to, reason } => {
                write!(f, "fallback {} -> {}: {}", from, to, reason)
            }
            TransportEvent::Restored { transport } => {
                write!(f, "restored {}", transport)
            }
        }
    }
}

// ============================================================================
// Check Request/Response
// ============================================================================

/// Request for a single authorization check.
#[derive(Debug, Clone)]
pub struct CheckRequest {
    /// Subject to check (e.g., "user:alice").
    pub subject: String,
    /// Permission to check (e.g., "view").
    pub permission: String,
    /// Resource to check (e.g., "document:readme").
    pub resource: String,
    /// Optional ABAC context.
    pub context: Option<Context>,
    /// Optional consistency requirement.
    pub consistency: Option<ConsistencyToken>,
    /// Whether to include detailed evaluation trace (for explain).
    pub trace: bool,
}

/// Response from an authorization check.
#[derive(Debug, Clone)]
pub struct CheckResponse {
    /// Whether access is allowed.
    pub allowed: bool,
    /// Decision with metadata.
    pub decision: Decision,
    /// Detailed evaluation trace (only present if trace was requested).
    pub trace: Option<DecisionTrace>,
}

/// Detailed trace of an authorization decision.
#[derive(Debug, Clone)]
pub struct DecisionTrace {
    /// Time taken to evaluate in microseconds.
    pub duration_micros: u64,
    /// Number of relationships read during evaluation.
    pub relationships_read: u64,
    /// Number of relations evaluated.
    pub relations_evaluated: u64,
    /// Root node of the evaluation tree.
    pub root: Option<EvaluationNode>,
}

/// A node in the evaluation tree.
#[derive(Debug, Clone)]
pub struct EvaluationNode {
    /// Type of this node.
    pub node_type: EvaluationNodeType,
    /// Result at this node.
    pub result: bool,
    /// Child nodes.
    pub children: Vec<EvaluationNode>,
}

/// Type of evaluation node.
#[derive(Debug, Clone)]
pub enum EvaluationNodeType {
    /// Direct relationship check.
    DirectCheck {
        resource: String,
        relation: String,
        subject: String,
    },
    /// Computed userset.
    ComputedUserset { relation: String },
    /// Related object userset (tupleset rewrite).
    RelatedObjectUserset {
        relationship: String,
        computed: String,
    },
    /// Union of child nodes.
    Union,
    /// Intersection of child nodes.
    Intersection,
    /// Exclusion (difference) of child nodes.
    Exclusion,
    /// WASM module evaluation.
    WasmModule { module_name: String },
}

// ============================================================================
// Write Request/Response
// ============================================================================

/// Request to write a relationship.
#[derive(Debug, Clone)]
pub struct WriteRequest {
    /// The relationship to write.
    pub relationship: Relationship<'static>,
    /// Optional idempotency key.
    pub idempotency_key: Option<String>,
}

/// Response from a write operation.
#[derive(Debug, Clone)]
pub struct WriteResponse {
    /// Consistency token for read-after-write.
    pub consistency_token: ConsistencyToken,
}

// ============================================================================
// Simulate Request/Response
// ============================================================================

/// Request for a simulated authorization check.
#[derive(Debug, Clone)]
pub struct SimulateRequest {
    /// Subject to check (e.g., "user:alice").
    pub subject: String,
    /// Permission to check (e.g., "view").
    pub permission: String,
    /// Resource to check (e.g., "document:readme").
    pub resource: String,
    /// Optional ABAC context.
    pub context: Option<Context>,
    /// Hypothetical relationships to add for the simulation.
    pub additions: Vec<Relationship<'static>>,
    /// Hypothetical relationships to remove for the simulation.
    pub removals: Vec<Relationship<'static>>,
}

/// Response from a simulated authorization check.
#[derive(Debug, Clone)]
pub struct SimulateResponse {
    /// Whether access would be allowed in the simulated state.
    pub allowed: bool,
    /// Decision with metadata.
    pub decision: Decision,
}

// ============================================================================
// Transport Trait
// ============================================================================

/// Core transport trait for InferaDB communication.
///
/// This trait is implemented by gRPC, HTTP, and mock transports.
/// It is internal to the SDK and not exposed to users.
#[async_trait::async_trait]
pub trait TransportClient: Send + Sync {
    /// Performs an authorization check.
    async fn check(&self, request: CheckRequest) -> Result<CheckResponse, Error>;

    /// Performs a batch of authorization checks.
    async fn check_batch(&self, requests: Vec<CheckRequest>) -> Result<Vec<CheckResponse>, Error>;

    /// Writes a relationship.
    async fn write(&self, request: WriteRequest) -> Result<WriteResponse, Error>;

    /// Writes a batch of relationships.
    async fn write_batch(&self, requests: Vec<WriteRequest>) -> Result<WriteResponse, Error>;

    /// Deletes a relationship.
    async fn delete(&self, relationship: Relationship<'static>) -> Result<(), Error>;

    /// Lists relationships matching a filter.
    async fn list_relationships(
        &self,
        resource: Option<&str>,
        relation: Option<&str>,
        subject: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRelationshipsResponse, Error>;

    /// Lists resources accessible by a subject with a permission.
    async fn list_resources(
        &self,
        subject: &str,
        permission: &str,
        resource_type: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListResourcesResponse, Error>;

    /// Lists subjects with a permission on a resource.
    async fn list_subjects(
        &self,
        permission: &str,
        resource: &str,
        subject_type: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListSubjectsResponse, Error>;

    /// Returns the transport type.
    fn transport_type(&self) -> Transport;

    /// Returns transport statistics.
    fn stats(&self) -> TransportStats;

    /// Checks if the transport is healthy.
    async fn health_check(&self) -> Result<(), Error>;

    /// Performs a simulated authorization check with hypothetical changes.
    async fn simulate(&self, request: SimulateRequest) -> Result<SimulateResponse, Error>;
}

/// Response from listing relationships.
#[derive(Debug, Clone)]
pub struct ListRelationshipsResponse {
    /// The relationships.
    pub relationships: Vec<Relationship<'static>>,
    /// Cursor for next page, if any.
    pub next_cursor: Option<String>,
}

/// Response from listing resources.
#[derive(Debug, Clone)]
pub struct ListResourcesResponse {
    /// The resource IDs.
    pub resources: Vec<String>,
    /// Cursor for next page, if any.
    pub next_cursor: Option<String>,
}

/// Response from listing subjects.
#[derive(Debug, Clone)]
pub struct ListSubjectsResponse {
    /// The subject IDs.
    pub subjects: Vec<String>,
    /// Cursor for next page, if any.
    pub next_cursor: Option<String>,
}

// ============================================================================
// Connection Pool Config
// ============================================================================

/// Connection pool configuration.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum connections in the pool.
    pub max_connections: u32,
    /// Idle connection timeout.
    pub idle_timeout: Duration,
    /// Maximum idle connections per host.
    pub max_idle_per_host: u32,
    /// Timeout waiting for a connection from the pool.
    pub pool_timeout: Duration,
    /// Force HTTP/2 (required for gRPC).
    pub http2_only: bool,
    /// HTTP/2 keepalive interval.
    pub http2_keepalive: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            idle_timeout: Duration::from_secs(90),
            max_idle_per_host: 10,
            pool_timeout: Duration::from_secs(30),
            http2_only: false,
            http2_keepalive: Duration::from_secs(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_default() {
        assert_eq!(Transport::default(), Transport::Grpc);
    }

    #[test]
    fn test_transport_checks() {
        assert!(Transport::Grpc.is_grpc());
        assert!(!Transport::Grpc.is_http());
        assert!(!Transport::Grpc.is_mock());

        assert!(!Transport::Http.is_grpc());
        assert!(Transport::Http.is_http());
        assert!(!Transport::Http.is_mock());

        assert!(!Transport::Mock.is_grpc());
        assert!(!Transport::Mock.is_http());
        assert!(Transport::Mock.is_mock());
    }

    #[test]
    fn test_transport_display() {
        assert_eq!(Transport::Grpc.to_string(), "gRPC");
        assert_eq!(Transport::Http.to_string(), "HTTP/REST");
        assert_eq!(Transport::Mock.to_string(), "Mock");
    }

    #[test]
    fn test_transport_strategy_default() {
        let strategy = TransportStrategy::default();
        assert_eq!(strategy.preferred_transport(), Transport::Grpc);
        assert_eq!(strategy.fallback_transport(), Some(Transport::Http));
        assert!(strategy.has_fallback());
    }

    #[test]
    fn test_transport_strategy_grpc_only() {
        let strategy = TransportStrategy::GrpcOnly;
        assert_eq!(strategy.preferred_transport(), Transport::Grpc);
        assert_eq!(strategy.fallback_transport(), None);
        assert!(!strategy.has_fallback());
    }

    #[test]
    fn test_fallback_trigger_default() {
        let trigger = FallbackTrigger::default();
        assert!(trigger.connection_error);
        assert!(trigger.protocol_error);
        assert!(trigger.connect_timeout);
        assert!(trigger.should_fallback_on_status(502));
        assert!(trigger.should_fallback_on_status(503));
        assert!(!trigger.should_fallback_on_status(500));
    }

    #[test]
    fn test_fallback_reason_display() {
        assert_eq!(
            FallbackReason::ConnectionRefused.to_string(),
            "connection refused"
        );
        assert_eq!(
            FallbackReason::StatusCode(502).to_string(),
            "HTTP status 502"
        );
    }

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.idle_timeout, Duration::from_secs(90));
    }

    #[test]
    fn test_transport_event_display() {
        let fallback_event = TransportEvent::FallbackTriggered {
            from: Transport::Grpc,
            to: Transport::Http,
            reason: FallbackReason::ConnectionRefused,
        };
        assert!(fallback_event.to_string().contains("gRPC"));
        assert!(fallback_event.to_string().contains("HTTP"));

        let restored_event = TransportEvent::Restored {
            transport: Transport::Grpc,
        };
        assert!(restored_event.to_string().contains("restored"));
        assert!(restored_event.to_string().contains("gRPC"));
    }

    #[test]
    fn test_transport_stats_default() {
        let stats = TransportStats::default();
        assert_eq!(stats.active_transport, Transport::default());
        assert_eq!(stats.fallback_count, 0);
        assert!(stats.grpc.is_none());
        assert!(stats.rest.is_none());
    }

    #[test]
    fn test_grpc_stats_default() {
        let stats = GrpcStats::default();
        assert_eq!(stats.requests_sent, 0);
        assert_eq!(stats.requests_failed, 0);
        assert_eq!(stats.streams_opened, 0);
        assert_eq!(stats.streams_active, 0);
    }

    #[test]
    fn test_rest_stats_default() {
        let stats = RestStats::default();
        assert_eq!(stats.requests_sent, 0);
        assert_eq!(stats.requests_failed, 0);
        assert_eq!(stats.sse_connections, 0);
        assert_eq!(stats.sse_active, 0);
    }

    #[test]
    fn test_fallback_reason_connect_timeout() {
        assert_eq!(
            FallbackReason::ConnectTimeout.to_string(),
            "connect timeout"
        );
    }

    #[test]
    fn test_fallback_reason_protocol_error() {
        let reason = FallbackReason::ProtocolError("invalid frame".to_string());
        assert!(reason.to_string().contains("protocol error"));
        assert!(reason.to_string().contains("invalid frame"));
    }

    #[test]
    fn test_transport_strategy_rest_only() {
        let strategy = TransportStrategy::RestOnly;
        assert_eq!(strategy.preferred_transport(), Transport::Http);
        assert_eq!(strategy.fallback_transport(), None);
        assert!(!strategy.has_fallback());
    }

    #[test]
    fn test_transport_strategy_prefer_rest() {
        let strategy = TransportStrategy::PreferRest {
            fallback_on: FallbackTrigger::default(),
        };
        assert_eq!(strategy.preferred_transport(), Transport::Http);
        assert_eq!(strategy.fallback_transport(), Some(Transport::Grpc));
        assert!(strategy.has_fallback());
    }

    #[test]
    fn test_fallback_trigger_on_any_error() {
        let trigger = FallbackTrigger::on_any_error();
        assert!(trigger.connection_error);
        assert!(trigger.protocol_error);
        assert!(trigger.connect_timeout);
        assert!(trigger.should_fallback_on_status(500));
        assert!(trigger.should_fallback_on_status(502));
        assert!(trigger.should_fallback_on_status(503));
        assert!(trigger.should_fallback_on_status(504));
    }

    #[test]
    fn test_fallback_trigger_on_connection_only() {
        let trigger = FallbackTrigger::on_connection_only();
        assert!(trigger.connection_error);
        assert!(!trigger.protocol_error);
        assert!(trigger.connect_timeout);
        assert!(trigger.status_codes.is_empty());
        assert!(!trigger.should_fallback_on_status(502));
    }
}
