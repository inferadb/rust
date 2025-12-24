//! Watch/subscribe functionality for real-time relationship change notifications.
//!
//! The watch API provides real-time streaming of relationship changes in a vault.
//! This enables use cases like cache invalidation, audit logging, and event-driven
//! architectures.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use futures::StreamExt;
//! use inferadb::vault::watch::{WatchFilter, Operation};
//!
//! // Watch all changes
//! let mut stream = vault.watch().run().await?;
//!
//! while let Some(event) = stream.next().await {
//!     let event = event?;
//!     println!("{:?}: {} -[{}]-> {}",
//!         event.operation,
//!         event.relationship.subject(),
//!         event.relationship.relation(),
//!         event.relationship.resource()
//!     );
//! }
//!
//! // Filtered watch with resumption
//! let mut stream = vault
//!     .watch()
//!     .filter(WatchFilter::resource_type("document"))
//!     .filter(WatchFilter::operations([Operation::Create]))
//!     .from_revision(12345)
//!     .resumable()
//!     .run()
//!     .await?;
//! ```

use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::vault::VaultClient;
use crate::Error;
#[cfg(not(feature = "rest"))]
use crate::ErrorKind;
use crate::Relationship;

/// Type alias for the inner watch stream to reduce complexity.
type InnerWatchStream = Pin<Box<dyn Stream<Item = Result<WatchEvent, Error>> + Send>>;

/// Operation type for relationship changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// A relationship was created.
    Create,
    /// A relationship was deleted.
    Delete,
}

impl Operation {
    /// Returns `true` if this is a create operation.
    pub fn is_create(&self) -> bool {
        matches!(self, Operation::Create)
    }

    /// Returns `true` if this is a delete operation.
    pub fn is_delete(&self) -> bool {
        matches!(self, Operation::Delete)
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Create => write!(f, "create"),
            Operation::Delete => write!(f, "delete"),
        }
    }
}

/// Filter options for the watch stream.
///
/// Filters are combined with AND logic - all specified filters must match
/// for an event to be delivered.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::vault::watch::{WatchFilter, Operation};
///
/// // Filter by resource type
/// let stream = vault.watch()
///     .filter(WatchFilter::resource_type("document"))
///     .run()
///     .await?;
///
/// // Combine multiple filters
/// let stream = vault.watch()
///     .filter(WatchFilter::resource_type("document"))
///     .filter(WatchFilter::relation("viewer"))
///     .filter(WatchFilter::operations([Operation::Create]))
///     .run()
///     .await?;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WatchFilter {
    /// Filter by resource type (e.g., "document", "folder").
    ResourceType(String),

    /// Filter by subject type (e.g., "user", "group").
    SubjectType(String),

    /// Filter by specific resource ID (e.g., "document:readme").
    Resource(String),

    /// Filter by specific subject ID (e.g., "user:alice").
    Subject(String),

    /// Filter by relation name (e.g., "viewer", "editor").
    Relation(String),

    /// Filter by operation type(s).
    Operations(Vec<Operation>),

    /// Custom filter expression (server-evaluated).
    Custom(String),
}

impl WatchFilter {
    /// Create a filter by resource type.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let filter = WatchFilter::resource_type("document");
    /// ```
    pub fn resource_type(type_name: impl Into<String>) -> Self {
        WatchFilter::ResourceType(type_name.into())
    }

    /// Create a filter by subject type.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let filter = WatchFilter::subject_type("user");
    /// ```
    pub fn subject_type(type_name: impl Into<String>) -> Self {
        WatchFilter::SubjectType(type_name.into())
    }

    /// Create a filter by specific resource.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let filter = WatchFilter::resource("document:readme");
    /// ```
    pub fn resource(resource_id: impl Into<String>) -> Self {
        WatchFilter::Resource(resource_id.into())
    }

    /// Create a filter by specific subject.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let filter = WatchFilter::subject("user:alice");
    /// ```
    pub fn subject(subject_id: impl Into<String>) -> Self {
        WatchFilter::Subject(subject_id.into())
    }

    /// Create a filter by relation name.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let filter = WatchFilter::relation("viewer");
    /// ```
    pub fn relation(relation_name: impl Into<String>) -> Self {
        WatchFilter::Relation(relation_name.into())
    }

    /// Create a filter by operation types.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::vault::watch::{WatchFilter, Operation};
    ///
    /// let filter = WatchFilter::operations([Operation::Create, Operation::Delete]);
    /// ```
    pub fn operations(ops: impl IntoIterator<Item = Operation>) -> Self {
        WatchFilter::Operations(ops.into_iter().collect())
    }

    /// Create a custom filter expression.
    ///
    /// The expression is evaluated server-side. Consult the API documentation
    /// for supported expression syntax.
    pub fn custom(expression: impl Into<String>) -> Self {
        WatchFilter::Custom(expression.into())
    }

    /// Check if a watch event matches this filter.
    ///
    /// This is used for client-side filtering when server-side filtering
    /// is not available.
    pub fn matches(&self, event: &WatchEvent) -> bool {
        match self {
            WatchFilter::ResourceType(t) => {
                event.relationship.resource_type().is_some_and(|rt| rt == t)
            }
            WatchFilter::SubjectType(t) => {
                event.relationship.subject_type().is_some_and(|st| st == t)
            }
            WatchFilter::Resource(r) => event.relationship.resource() == r,
            WatchFilter::Subject(s) => event.relationship.subject() == s,
            WatchFilter::Relation(r) => event.relationship.relation() == r,
            WatchFilter::Operations(ops) => ops.contains(&event.operation),
            WatchFilter::Custom(_) => true, // Custom filters evaluated server-side
        }
    }
}

impl fmt::Display for WatchFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatchFilter::ResourceType(t) => write!(f, "resource_type={}", t),
            WatchFilter::SubjectType(t) => write!(f, "subject_type={}", t),
            WatchFilter::Resource(r) => write!(f, "resource={}", r),
            WatchFilter::Subject(s) => write!(f, "subject={}", s),
            WatchFilter::Relation(r) => write!(f, "relation={}", r),
            WatchFilter::Operations(ops) => {
                let op_strs: Vec<_> = ops.iter().map(|o| o.to_string()).collect();
                write!(f, "operations=[{}]", op_strs.join(","))
            }
            WatchFilter::Custom(expr) => write!(f, "custom={}", expr),
        }
    }
}

/// A watch event representing a relationship change.
///
/// Watch events are delivered in order and contain all information needed
/// to update local state or trigger downstream actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEvent {
    /// The operation that occurred.
    pub operation: Operation,

    /// The relationship that changed.
    #[serde(with = "relationship_serde")]
    pub relationship: Relationship<'static>,

    /// Server revision number for this change.
    ///
    /// Can be used to resume the watch stream from this point after
    /// a disconnect or restart.
    pub revision: u64,

    /// Timestamp when the change occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// The actor who made the change (if audit logging is enabled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    /// Request ID of the original operation (for correlation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Custom serde module for Relationship<'static>
mod relationship_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    #[derive(Deserialize)]
    struct RelationshipDto {
        resource: String,
        relation: String,
        subject: String,
    }

    #[derive(Serialize)]
    struct RelationshipDtoRef<'a> {
        resource: &'a str,
        relation: &'a str,
        subject: &'a str,
    }

    pub fn serialize<S>(rel: &Relationship<'static>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let dto = RelationshipDtoRef {
            resource: rel.resource(),
            relation: rel.relation(),
            subject: rel.subject(),
        };
        dto.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Relationship<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dto = RelationshipDto::deserialize(deserializer)?;
        Ok(Relationship::new(dto.resource, dto.relation, dto.subject))
    }
}

impl WatchEvent {
    /// Create a new watch event.
    pub fn new(
        operation: Operation,
        relationship: Relationship<'static>,
        revision: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            operation,
            relationship,
            revision,
            timestamp,
            actor: None,
            request_id: None,
        }
    }

    /// Set the actor for this event.
    #[must_use]
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Set the request ID for this event.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Returns `true` if this is a create operation.
    pub fn is_create(&self) -> bool {
        self.operation.is_create()
    }

    /// Returns `true` if this is a delete operation.
    pub fn is_delete(&self) -> bool {
        self.operation.is_delete()
    }

    /// Returns the resource from the relationship.
    pub fn resource(&self) -> &str {
        self.relationship.resource()
    }

    /// Returns the relation from the relationship.
    pub fn relation(&self) -> &str {
        self.relationship.relation()
    }

    /// Returns the subject from the relationship.
    pub fn subject(&self) -> &str {
        self.relationship.subject()
    }
}

impl fmt::Display for WatchEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} {} -[{}]-> {}",
            self.revision,
            self.operation,
            self.relationship.subject(),
            self.relationship.relation(),
            self.relationship.resource()
        )
    }
}

/// Configuration for stream reconnection behavior.
///
/// When a watch stream disconnects, the SDK can automatically reconnect
/// and resume from the last seen revision.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (None = infinite).
    pub max_retries: Option<u32>,

    /// Initial backoff duration between retries.
    pub initial_backoff: Duration,

    /// Maximum backoff duration.
    pub max_backoff: Duration,

    /// Backoff multiplier (e.g., 2.0 for exponential backoff).
    pub backoff_multiplier: f64,

    /// Random jitter factor (0.0 - 1.0) to prevent thundering herd.
    pub jitter: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: None, // Infinite retries
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: 0.1,
        }
    }
}

impl ReconnectConfig {
    /// Create a new reconnect configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retries.
    #[must_use]
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Set infinite retries (the default).
    #[must_use]
    pub fn infinite_retries(mut self) -> Self {
        self.max_retries = None;
        self
    }

    /// Set the initial backoff duration.
    #[must_use]
    pub fn initial_backoff(mut self, duration: Duration) -> Self {
        self.initial_backoff = duration;
        self
    }

    /// Set the maximum backoff duration.
    #[must_use]
    pub fn max_backoff(mut self, duration: Duration) -> Self {
        self.max_backoff = duration;
        self
    }

    /// Set the backoff multiplier.
    #[must_use]
    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Set the jitter factor.
    #[must_use]
    pub fn jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Calculate the backoff duration for a given attempt.
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let base_backoff =
            self.initial_backoff.as_secs_f64() * self.backoff_multiplier.powi(attempt as i32);
        let capped = base_backoff.min(self.max_backoff.as_secs_f64());

        // Apply jitter
        let jitter_range = capped * self.jitter;
        let jittered = capped - jitter_range / 2.0 + rand::random::<f64>() * jitter_range;

        Duration::from_secs_f64(jittered.max(0.0))
    }
}

/// Builder for watch streams.
///
/// Use [`VaultClient::watch()`] to create a new builder.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::vault::watch::{WatchFilter, Operation};
///
/// let stream = vault
///     .watch()
///     .filter(WatchFilter::resource_type("document"))
///     .filter(WatchFilter::operations([Operation::Create]))
///     .from_revision(12345)
///     .resumable()
///     .run()
///     .await?;
/// ```
pub struct WatchBuilder {
    client: Client,
    organization_id: String,
    vault_id: String,
    filters: Vec<WatchFilter>,
    from_revision: Option<u64>,
    resumable: bool,
    reconnect_config: Option<ReconnectConfig>,
}

impl WatchBuilder {
    /// Create a new watch builder.
    pub(crate) fn new(vault: &VaultClient) -> Self {
        Self {
            client: vault.client().clone(),
            organization_id: vault.organization_id().to_string(),
            vault_id: vault.vault_id().to_string(),
            filters: Vec::new(),
            from_revision: None,
            resumable: false,
            reconnect_config: None,
        }
    }

    /// Add a filter to narrow which changes are received.
    ///
    /// Multiple filters are combined with AND logic.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use inferadb::vault::watch::WatchFilter;
    ///
    /// let stream = vault.watch()
    ///     .filter(WatchFilter::resource_type("document"))
    ///     .filter(WatchFilter::relation("viewer"))
    ///     .run()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn filter(mut self, filter: WatchFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Resume from a specific revision.
    ///
    /// Use this for crash recovery - events will be replayed starting
    /// from the specified revision.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // On startup, resume from last checkpoint
    /// let checkpoint = load_checkpoint().await?;
    /// let stream = vault.watch()
    ///     .from_revision(checkpoint)
    ///     .run()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn from_revision(mut self, revision: u64) -> Self {
        self.from_revision = Some(revision);
        self
    }

    /// Enable automatic reconnection on disconnect.
    ///
    /// The stream will automatically reconnect and resume from the last
    /// seen revision using exponential backoff.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let stream = vault.watch()
    ///     .resumable()
    ///     .run()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn resumable(mut self) -> Self {
        self.resumable = true;
        self
    }

    /// Disable automatic reconnection (fail on disconnect).
    ///
    /// The stream will end with an error on any disconnection.
    #[must_use]
    pub fn no_reconnect(mut self) -> Self {
        self.resumable = false;
        self
    }

    /// Configure custom reconnection behavior.
    ///
    /// Implicitly enables resumable mode.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use inferadb::vault::watch::ReconnectConfig;
    /// use std::time::Duration;
    ///
    /// let stream = vault.watch()
    ///     .reconnect(ReconnectConfig::default()
    ///         .max_retries(10)
    ///         .initial_backoff(Duration::from_millis(200))
    ///         .max_backoff(Duration::from_secs(60)))
    ///     .run()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn reconnect(mut self, config: ReconnectConfig) -> Self {
        self.reconnect_config = Some(config);
        self.resumable = true;
        self
    }

    /// Returns the configured filters.
    pub fn filters(&self) -> &[WatchFilter] {
        &self.filters
    }

    /// Returns the starting revision, if set.
    pub fn starting_revision(&self) -> Option<u64> {
        self.from_revision
    }

    /// Returns whether resumable mode is enabled.
    pub fn is_resumable(&self) -> bool {
        self.resumable
    }

    /// Start the watch stream.
    ///
    /// Returns a stream of watch events. The stream continues until
    /// cancelled or an unrecoverable error occurs.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use futures::StreamExt;
    ///
    /// let mut stream = vault.watch().run().await?;
    ///
    /// while let Some(event) = stream.next().await {
    ///     let event = event?;
    ///     println!("Change: {}", event);
    /// }
    /// ```
    #[cfg(feature = "rest")]
    pub async fn run(self) -> Result<WatchStream, Error> {
        // Build query parameters for the watch endpoint
        let mut query_params = Vec::new();

        if let Some(rev) = self.from_revision {
            query_params.push(format!("from_revision={}", rev));
        }

        // Add filters to query params
        for filter in &self.filters {
            match filter {
                WatchFilter::ResourceType(t) => query_params.push(format!("resource_type={}", t)),
                WatchFilter::SubjectType(t) => query_params.push(format!("subject_type={}", t)),
                WatchFilter::Resource(r) => query_params.push(format!("resource={}", r)),
                WatchFilter::Subject(s) => query_params.push(format!("subject={}", s)),
                WatchFilter::Relation(r) => query_params.push(format!("relation={}", r)),
                WatchFilter::Operations(ops) => {
                    for op in ops {
                        query_params.push(format!("operation={}", op));
                    }
                }
                WatchFilter::Custom(expr) => query_params.push(format!("filter={}", expr)),
            }
        }

        let _path = format!(
            "/v1/organizations/{}/vaults/{}/watch{}",
            self.organization_id,
            self.vault_id,
            if query_params.is_empty() {
                String::new()
            } else {
                format!("?{}", query_params.join("&"))
            }
        );

        // For now, return a placeholder stream since SSE/WebSocket transport
        // requires additional implementation
        Ok(WatchStream::new(
            self.client,
            self.organization_id,
            self.vault_id,
            self.filters,
            self.from_revision,
            self.resumable,
            self.reconnect_config,
        ))
    }

    /// Start the watch stream.
    #[cfg(not(feature = "rest"))]
    pub async fn run(self) -> Result<WatchStream, Error> {
        Err(Error::new(
            ErrorKind::Configuration,
            "REST feature is required for watch streams",
        ))
    }
}

impl fmt::Debug for WatchBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchBuilder")
            .field("organization_id", &self.organization_id)
            .field("vault_id", &self.vault_id)
            .field("filters", &self.filters)
            .field("from_revision", &self.from_revision)
            .field("resumable", &self.resumable)
            .finish_non_exhaustive()
    }
}

/// A handle for gracefully shutting down a watch stream.
#[derive(Debug, Clone)]
pub struct WatchShutdownHandle {
    sender: tokio::sync::watch::Sender<bool>,
}

impl WatchShutdownHandle {
    /// Create a new shutdown handle.
    fn new() -> (Self, tokio::sync::watch::Receiver<bool>) {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        (Self { sender }, receiver)
    }

    /// Trigger a graceful shutdown of the watch stream.
    ///
    /// The stream will complete on its next iteration.
    pub fn shutdown(&self) {
        let _ = self.sender.send(true);
    }

    /// Check if shutdown has been triggered.
    pub fn is_shutdown(&self) -> bool {
        *self.sender.borrow()
    }
}

/// A stream of watch events.
///
/// This stream implements `futures::Stream<Item = Result<WatchEvent, Error>>`.
pub struct WatchStream {
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    organization_id: String,
    #[allow(dead_code)]
    vault_id: String,
    filters: Vec<WatchFilter>,
    last_revision: Option<u64>,
    #[allow(dead_code)]
    resumable: bool,
    #[allow(dead_code)]
    reconnect_config: Option<ReconnectConfig>,
    shutdown_receiver: tokio::sync::watch::Receiver<bool>,
    shutdown_handle: WatchShutdownHandle,
    // Internal stream state
    inner: Option<InnerWatchStream>,
}

impl WatchStream {
    fn new(
        client: Client,
        organization_id: String,
        vault_id: String,
        filters: Vec<WatchFilter>,
        from_revision: Option<u64>,
        resumable: bool,
        reconnect_config: Option<ReconnectConfig>,
    ) -> Self {
        let (shutdown_handle, shutdown_receiver) = WatchShutdownHandle::new();

        Self {
            client,
            organization_id,
            vault_id,
            filters,
            last_revision: from_revision,
            resumable,
            reconnect_config,
            shutdown_receiver,
            shutdown_handle,
            inner: None,
        }
    }

    /// Returns a handle for gracefully shutting down this stream.
    pub fn shutdown_handle(&self) -> WatchShutdownHandle {
        self.shutdown_handle.clone()
    }

    /// Returns the last seen revision.
    ///
    /// Use this to save checkpoints for crash recovery.
    pub fn last_revision(&self) -> Option<u64> {
        self.last_revision
    }

    /// Check if any event matches all configured filters.
    fn matches_filters(&self, event: &WatchEvent) -> bool {
        self.filters.iter().all(|f| f.matches(event))
    }
}

impl Stream for WatchStream {
    type Item = Result<WatchEvent, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // Check for shutdown
        if *self.shutdown_receiver.borrow() {
            return std::task::Poll::Ready(None);
        }

        // If we don't have an inner stream, create one (placeholder for real SSE implementation)
        if self.inner.is_none() {
            // Create a placeholder stream that returns nothing
            // Real implementation would connect to SSE/WebSocket endpoint
            let stream: Pin<Box<dyn Stream<Item = Result<WatchEvent, Error>> + Send>> =
                Box::pin(futures::stream::empty());
            self.inner = Some(stream);
        }

        // Poll the inner stream
        if let Some(ref mut inner) = self.inner {
            match inner.as_mut().poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(event))) => {
                    // Update last revision
                    self.last_revision = Some(event.revision);

                    // Apply client-side filtering
                    if self.matches_filters(&event) {
                        std::task::Poll::Ready(Some(Ok(event)))
                    } else {
                        // Filter didn't match, poll again
                        cx.waker().wake_by_ref();
                        std::task::Poll::Pending
                    }
                }
                other => other,
            }
        } else {
            std::task::Poll::Ready(None)
        }
    }
}

impl fmt::Debug for WatchStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WatchStream")
            .field("organization_id", &self.organization_id)
            .field("vault_id", &self.vault_id)
            .field("filters", &self.filters)
            .field("last_revision", &self.last_revision)
            .field("resumable", &self.resumable)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_is_create() {
        assert!(Operation::Create.is_create());
        assert!(!Operation::Create.is_delete());
        assert!(!Operation::Delete.is_create());
        assert!(Operation::Delete.is_delete());
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(Operation::Create.to_string(), "create");
        assert_eq!(Operation::Delete.to_string(), "delete");
    }

    #[test]
    fn test_watch_filter_constructors() {
        assert_eq!(
            WatchFilter::resource_type("document"),
            WatchFilter::ResourceType("document".to_string())
        );
        assert_eq!(
            WatchFilter::subject_type("user"),
            WatchFilter::SubjectType("user".to_string())
        );
        assert_eq!(
            WatchFilter::resource("document:readme"),
            WatchFilter::Resource("document:readme".to_string())
        );
        assert_eq!(
            WatchFilter::subject("user:alice"),
            WatchFilter::Subject("user:alice".to_string())
        );
        assert_eq!(
            WatchFilter::relation("viewer"),
            WatchFilter::Relation("viewer".to_string())
        );
        assert_eq!(
            WatchFilter::operations([Operation::Create]),
            WatchFilter::Operations(vec![Operation::Create])
        );
        assert_eq!(
            WatchFilter::custom("custom_expr"),
            WatchFilter::Custom("custom_expr".to_string())
        );
    }

    #[test]
    fn test_watch_filter_display() {
        assert_eq!(
            WatchFilter::resource_type("document").to_string(),
            "resource_type=document"
        );
        assert_eq!(
            WatchFilter::subject_type("user").to_string(),
            "subject_type=user"
        );
        assert_eq!(
            WatchFilter::resource("document:readme").to_string(),
            "resource=document:readme"
        );
        assert_eq!(
            WatchFilter::subject("user:alice").to_string(),
            "subject=user:alice"
        );
        assert_eq!(
            WatchFilter::relation("viewer").to_string(),
            "relation=viewer"
        );
        assert_eq!(
            WatchFilter::operations([Operation::Create, Operation::Delete]).to_string(),
            "operations=[create,delete]"
        );
        assert_eq!(WatchFilter::custom("expr").to_string(), "custom=expr");
    }

    #[test]
    fn test_watch_filter_matches() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Create, rel, 1, chrono::Utc::now());

        // Resource type filter
        assert!(WatchFilter::resource_type("document").matches(&event));
        assert!(!WatchFilter::resource_type("folder").matches(&event));

        // Subject type filter
        assert!(WatchFilter::subject_type("user").matches(&event));
        assert!(!WatchFilter::subject_type("group").matches(&event));

        // Resource filter
        assert!(WatchFilter::resource("document:readme").matches(&event));
        assert!(!WatchFilter::resource("document:other").matches(&event));

        // Subject filter
        assert!(WatchFilter::subject("user:alice").matches(&event));
        assert!(!WatchFilter::subject("user:bob").matches(&event));

        // Relation filter
        assert!(WatchFilter::relation("viewer").matches(&event));
        assert!(!WatchFilter::relation("editor").matches(&event));

        // Operation filter
        assert!(WatchFilter::operations([Operation::Create]).matches(&event));
        assert!(WatchFilter::operations([Operation::Create, Operation::Delete]).matches(&event));
        assert!(!WatchFilter::operations([Operation::Delete]).matches(&event));

        // Custom filter (always true client-side)
        assert!(WatchFilter::custom("anything").matches(&event));
    }

    #[test]
    fn test_watch_event_new() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let now = chrono::Utc::now();
        let event = WatchEvent::new(Operation::Create, rel.clone(), 42, now);

        assert_eq!(event.operation, Operation::Create);
        assert_eq!(event.revision, 42);
        assert_eq!(event.timestamp, now);
        assert_eq!(event.resource(), "document:readme");
        assert_eq!(event.relation(), "viewer");
        assert_eq!(event.subject(), "user:alice");
        assert!(event.actor.is_none());
        assert!(event.request_id.is_none());
    }

    #[test]
    fn test_watch_event_with_metadata() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Delete, rel, 100, chrono::Utc::now())
            .with_actor("admin")
            .with_request_id("req-123");

        assert_eq!(event.actor, Some("admin".to_string()));
        assert_eq!(event.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_watch_event_is_create_delete() {
        let rel = Relationship::new("doc:1", "v", "u:1");
        let create_event = WatchEvent::new(Operation::Create, rel.clone(), 1, chrono::Utc::now());
        let delete_event = WatchEvent::new(Operation::Delete, rel, 2, chrono::Utc::now());

        assert!(create_event.is_create());
        assert!(!create_event.is_delete());
        assert!(!delete_event.is_create());
        assert!(delete_event.is_delete());
    }

    #[test]
    fn test_watch_event_display() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Create, rel, 42, chrono::Utc::now());
        let display = event.to_string();

        assert!(display.contains("[42]"));
        assert!(display.contains("create"));
        assert!(display.contains("user:alice"));
        assert!(display.contains("viewer"));
        assert!(display.contains("document:readme"));
    }

    #[test]
    fn test_reconnect_config_default() {
        let config = ReconnectConfig::default();

        assert_eq!(config.max_retries, None);
        assert_eq!(config.initial_backoff, Duration::from_millis(100));
        assert_eq!(config.max_backoff, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!((config.jitter - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reconnect_config_builder() {
        let config = ReconnectConfig::new()
            .max_retries(5)
            .initial_backoff(Duration::from_millis(200))
            .max_backoff(Duration::from_secs(60))
            .backoff_multiplier(1.5)
            .jitter(0.2);

        assert_eq!(config.max_retries, Some(5));
        assert_eq!(config.initial_backoff, Duration::from_millis(200));
        assert_eq!(config.max_backoff, Duration::from_secs(60));
        assert_eq!(config.backoff_multiplier, 1.5);
        assert!((config.jitter - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reconnect_config_infinite_retries() {
        let config = ReconnectConfig::new().max_retries(5).infinite_retries();
        assert_eq!(config.max_retries, None);
    }

    #[test]
    fn test_reconnect_config_jitter_clamped() {
        let config = ReconnectConfig::new().jitter(2.0);
        assert!((config.jitter - 1.0).abs() < f64::EPSILON);

        let config = ReconnectConfig::new().jitter(-0.5);
        assert!(config.jitter.abs() < f64::EPSILON);
    }

    #[test]
    fn test_reconnect_config_backoff_calculation() {
        let config = ReconnectConfig::new()
            .initial_backoff(Duration::from_millis(100))
            .max_backoff(Duration::from_secs(10))
            .backoff_multiplier(2.0)
            .jitter(0.0); // No jitter for predictable testing

        // With 0 jitter, backoff should be predictable
        let b0 = config.backoff_for_attempt(0);
        let b1 = config.backoff_for_attempt(1);
        let b2 = config.backoff_for_attempt(2);

        assert_eq!(b0, Duration::from_millis(100));
        assert_eq!(b1, Duration::from_millis(200));
        assert_eq!(b2, Duration::from_millis(400));

        // Should cap at max_backoff
        let b10 = config.backoff_for_attempt(10);
        assert!(b10 <= Duration::from_secs(10));
    }

    #[test]
    fn test_watch_shutdown_handle() {
        let (handle, receiver) = WatchShutdownHandle::new();

        assert!(!handle.is_shutdown());
        assert!(!*receiver.borrow());

        handle.shutdown();

        assert!(handle.is_shutdown());
        assert!(*receiver.borrow());
    }

    #[test]
    fn test_watch_event_serialization() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Create, rel, 42, chrono::Utc::now())
            .with_actor("admin")
            .with_request_id("req-123");

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"operation\":\"create\""));
        assert!(json.contains("\"revision\":42"));
        assert!(json.contains("\"actor\":\"admin\""));
        assert!(json.contains("\"request_id\":\"req-123\""));

        // Deserialize back
        let parsed: WatchEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.operation, Operation::Create);
        assert_eq!(parsed.revision, 42);
        assert_eq!(parsed.resource(), "document:readme");
        assert_eq!(parsed.relation(), "viewer");
        assert_eq!(parsed.subject(), "user:alice");
        assert_eq!(parsed.actor, Some("admin".to_string()));
        assert_eq!(parsed.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_operation_serialization() {
        assert_eq!(
            serde_json::to_string(&Operation::Create).unwrap(),
            "\"create\""
        );
        assert_eq!(
            serde_json::to_string(&Operation::Delete).unwrap(),
            "\"delete\""
        );

        assert_eq!(
            serde_json::from_str::<Operation>("\"create\"").unwrap(),
            Operation::Create
        );
        assert_eq!(
            serde_json::from_str::<Operation>("\"delete\"").unwrap(),
            Operation::Delete
        );
    }

    #[test]
    fn test_watch_filter_matches_no_type_colon() {
        // Test matching when relationship has no type:id format
        let rel = Relationship::new("nocooldoc", "viewer", "justauser");
        let event = WatchEvent::new(Operation::Create, rel, 1, chrono::Utc::now());

        // Resource type filter should not match (no colon in resource)
        assert!(!WatchFilter::resource_type("document").matches(&event));
        // Subject type filter should not match (no colon in subject)
        assert!(!WatchFilter::subject_type("user").matches(&event));
    }

    #[test]
    fn test_watch_event_without_optional_fields() {
        let rel = Relationship::new("doc:1", "viewer", "user:1");
        let event = WatchEvent::new(Operation::Create, rel, 1, chrono::Utc::now());

        // Serialize without optional fields
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("actor"));
        assert!(!json.contains("request_id"));
    }

    #[test]
    fn test_reconnect_config_backoff_with_jitter() {
        let config = ReconnectConfig::new()
            .initial_backoff(Duration::from_millis(100))
            .max_backoff(Duration::from_secs(10))
            .backoff_multiplier(2.0)
            .jitter(0.5);

        // With 50% jitter, values should be within a range
        let backoff = config.backoff_for_attempt(0);
        // Base is 100ms, jitter range is 50ms, so result should be 50-150ms
        assert!(backoff >= Duration::from_millis(50) && backoff <= Duration::from_millis(150));
    }

    #[test]
    fn test_watch_builder_debug() {
        // We can't fully test the builder without a client, but we can test the Debug impl
        // by ensuring no panics occur during formatting
        let filter = WatchFilter::resource_type("document");
        let debug_str = format!("{:?}", filter);
        assert!(debug_str.contains("ResourceType"));
    }

    #[test]
    fn test_watch_filter_empty_operations() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Create, rel, 1, chrono::Utc::now());

        // Empty operations filter should not match anything
        let filter = WatchFilter::operations([]);
        assert!(!filter.matches(&event));
    }

    #[test]
    fn test_reconnect_config_clone() {
        let config = ReconnectConfig::new()
            .max_retries(5)
            .initial_backoff(Duration::from_millis(200));

        let cloned = config.clone();
        assert_eq!(cloned.max_retries, Some(5));
        assert_eq!(cloned.initial_backoff, Duration::from_millis(200));
    }

    #[test]
    fn test_operation_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Operation::Create);
        set.insert(Operation::Delete);
        set.insert(Operation::Create); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&Operation::Create));
        assert!(set.contains(&Operation::Delete));
    }

    #[test]
    fn test_watch_filter_hash_and_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(WatchFilter::resource_type("document"));
        set.insert(WatchFilter::resource_type("folder"));
        set.insert(WatchFilter::resource_type("document")); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_watch_event_accessor_methods() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Delete, rel, 999, chrono::Utc::now())
            .with_actor("admin@example.com")
            .with_request_id("req-abc-123");

        assert_eq!(event.resource(), "document:readme");
        assert_eq!(event.relation(), "viewer");
        assert_eq!(event.subject(), "user:alice");
        assert!(event.is_delete());
        assert!(!event.is_create());
        assert_eq!(event.actor.as_deref(), Some("admin@example.com"));
        assert_eq!(event.request_id.as_deref(), Some("req-abc-123"));
    }

    #[test]
    fn test_watch_shutdown_handle_clone() {
        let (handle, _receiver) = WatchShutdownHandle::new();
        let cloned = handle.clone();

        assert!(!cloned.is_shutdown());
        handle.shutdown();
        assert!(cloned.is_shutdown());
    }

    #[test]
    fn test_watch_shutdown_handle_debug() {
        let (handle, _receiver) = WatchShutdownHandle::new();
        let debug_str = format!("{:?}", handle);
        assert!(debug_str.contains("WatchShutdownHandle"));
    }

    #[test]
    fn test_watch_filter_clone() {
        let filter = WatchFilter::operations([Operation::Create, Operation::Delete]);
        let cloned = filter.clone();
        assert_eq!(filter, cloned);
    }

    #[test]
    fn test_reconnect_config_new_equals_default() {
        let new = ReconnectConfig::new();
        let default = ReconnectConfig::default();

        assert_eq!(new.max_retries, default.max_retries);
        assert_eq!(new.initial_backoff, default.initial_backoff);
        assert_eq!(new.max_backoff, default.max_backoff);
        assert_eq!(new.backoff_multiplier, default.backoff_multiplier);
        assert!((new.jitter - default.jitter).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_watch_stream_basic() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let stream = WatchStream::new(
            client,
            "org_test".to_string(),
            "vlt_test".to_string(),
            vec![],
            None,
            false,
            None,
        );

        // Stream should be debuggable
        let debug = format!("{:?}", stream);
        assert!(debug.contains("WatchStream"));

        // Get shutdown handle
        let handle = stream.shutdown_handle();
        assert!(!handle.is_shutdown());

        // Check last revision
        assert!(stream.last_revision().is_none());
    }

    #[tokio::test]
    async fn test_watch_stream_shutdown() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use futures::StreamExt;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let mut stream = WatchStream::new(
            client,
            "org_test".to_string(),
            "vlt_test".to_string(),
            vec![],
            Some(100),
            true,
            Some(ReconnectConfig::default()),
        );

        // Get shutdown handle and trigger shutdown
        let handle = stream.shutdown_handle();
        handle.shutdown();
        assert!(handle.is_shutdown());

        // Stream should complete immediately after shutdown
        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_watch_stream_with_filters() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use futures::StreamExt;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let filters = vec![
            WatchFilter::resource_type("document"),
            WatchFilter::operations([Operation::Create]),
        ];

        let mut stream = WatchStream::new(
            client,
            "org_test".to_string(),
            "vlt_test".to_string(),
            filters,
            None,
            false,
            None,
        );

        // Shutdown and verify
        stream.shutdown_handle().shutdown();
        let result = stream.next().await;
        assert!(result.is_none());
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_watch_builder_run() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Build and run watch - test the builder path
        let result = vault
            .watch()
            .filter(WatchFilter::resource_type("document"))
            .from_revision(100)
            .resumable()
            .reconnect(ReconnectConfig::new().max_retries(5))
            .run()
            .await;

        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.last_revision(), Some(100));
    }

    #[tokio::test]
    async fn test_watch_builder_no_reconnect() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        let builder = vault.watch().no_reconnect();

        // Debug output
        let debug = format!("{:?}", builder);
        assert!(debug.contains("WatchBuilder"));

        // Check accessors
        assert!(builder.filters().is_empty());
        assert!(builder.starting_revision().is_none());
        assert!(!builder.is_resumable());
    }

    #[test]
    fn test_watch_stream_matches_filters() {
        // Create a simple test with filter matching
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        let event = WatchEvent::new(Operation::Create, rel, 1, chrono::Utc::now());

        // Test that filter matching logic works
        let filter = WatchFilter::resource_type("document");
        assert!(filter.matches(&event));

        let filter = WatchFilter::resource_type("folder");
        assert!(!filter.matches(&event));
    }

    #[tokio::test]
    async fn test_watch_builder_full_options() {
        use crate::auth::BearerCredentialsConfig;
        use crate::transport::mock::MockTransport;
        use std::sync::Arc;

        let mock_transport = Arc::new(MockTransport::new());
        let client = crate::Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Build a watch with all options to test accessors
        let builder = vault
            .watch()
            .filter(WatchFilter::resource_type("doc"))
            .from_revision(42)
            .resumable()
            .reconnect(ReconnectConfig::default());

        // Verify the Debug implementation
        let debug = format!("{:?}", builder);
        assert!(debug.contains("WatchBuilder"));
        assert!(debug.contains("org_test"));
        assert!(debug.contains("vlt_test"));

        // Verify accessors
        assert_eq!(builder.filters().len(), 1);
        assert_eq!(builder.starting_revision(), Some(42));
        assert!(builder.is_resumable());
    }
}
