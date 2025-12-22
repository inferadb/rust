//! Span types for tracing SDK operations.

use std::time::{Duration, Instant};

use crate::tracing_support::TraceContext;

/// Kind of span, indicating its role in the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpanKind {
    /// A client-side span (outgoing request).
    #[default]
    Client,
    /// A server-side span (incoming request).
    Server,
    /// An internal span.
    Internal,
    /// A producer span (async message send).
    Producer,
    /// A consumer span (async message receive).
    Consumer,
}

impl SpanKind {
    /// Returns the OpenTelemetry span kind value.
    pub fn otel_value(&self) -> i32 {
        match self {
            SpanKind::Client => 3,
            SpanKind::Server => 2,
            SpanKind::Internal => 1,
            SpanKind::Producer => 4,
            SpanKind::Consumer => 5,
        }
    }
}

impl std::fmt::Display for SpanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanKind::Client => write!(f, "client"),
            SpanKind::Server => write!(f, "server"),
            SpanKind::Internal => write!(f, "internal"),
            SpanKind::Producer => write!(f, "producer"),
            SpanKind::Consumer => write!(f, "consumer"),
        }
    }
}

/// Status of a span.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SpanStatus {
    /// Span completed successfully.
    #[default]
    Ok,
    /// Span completed with an error.
    Error(String),
    /// Span status is unset.
    Unset,
}

impl SpanStatus {
    /// Returns `true` if the span status is Ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, SpanStatus::Ok)
    }

    /// Returns `true` if the span status is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, SpanStatus::Error(_))
    }

    /// Returns the error message if this is an error status.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            SpanStatus::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl std::fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanStatus::Ok => write!(f, "ok"),
            SpanStatus::Error(msg) => write!(f, "error: {}", msg),
            SpanStatus::Unset => write!(f, "unset"),
        }
    }
}

/// A span representing an InferaDB SDK operation.
///
/// This provides a simple span abstraction that can be used with or without
/// the `tracing` crate integration.
///
/// ## Example
///
/// ```rust
/// use inferadb::tracing_support::{InferaDbSpan, SpanKind};
///
/// let span = InferaDbSpan::new("inferadb.check")
///     .with_kind(SpanKind::Client)
///     .with_attribute("subject", "user:alice")
///     .with_attribute("permission", "view")
///     .with_attribute("resource", "doc:1");
///
/// // ... perform operation ...
///
/// let finished = span.finish_ok();
/// println!("Operation took {:?}", finished.duration());
/// ```
#[derive(Debug, Clone)]
pub struct InferaDbSpan {
    /// The span name (operation name).
    name: String,
    /// The span kind.
    kind: SpanKind,
    /// The trace context.
    trace_context: Option<TraceContext>,
    /// Span attributes.
    attributes: Vec<(String, SpanValue)>,
    /// When the span started.
    start_time: Instant,
}

impl InferaDbSpan {
    /// Creates a new span with the given name.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::tracing_support::InferaDbSpan;
    ///
    /// let span = InferaDbSpan::new("inferadb.check");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: SpanKind::Client,
            trace_context: None,
            attributes: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Sets the span kind.
    #[must_use]
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the trace context.
    #[must_use]
    pub fn with_trace_context(mut self, ctx: TraceContext) -> Self {
        self.trace_context = Some(ctx);
        self
    }

    /// Adds an attribute to the span.
    #[must_use]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<SpanValue>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// Returns the span name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the span kind.
    pub fn kind(&self) -> SpanKind {
        self.kind
    }

    /// Returns the trace context, if set.
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace_context.as_ref()
    }

    /// Returns the span attributes.
    pub fn attributes(&self) -> &[(String, SpanValue)] {
        &self.attributes
    }

    /// Returns the elapsed time since the span started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Finishes the span with Ok status.
    pub fn finish_ok(self) -> FinishedSpan {
        self.finish(SpanStatus::Ok)
    }

    /// Finishes the span with an error status.
    pub fn finish_error(self, message: impl Into<String>) -> FinishedSpan {
        self.finish(SpanStatus::Error(message.into()))
    }

    /// Finishes the span with the given status.
    pub fn finish(self, status: SpanStatus) -> FinishedSpan {
        FinishedSpan {
            name: self.name,
            kind: self.kind,
            trace_context: self.trace_context,
            attributes: self.attributes,
            duration: self.start_time.elapsed(),
            status,
        }
    }
}

/// A finished span with timing information.
#[derive(Debug, Clone)]
pub struct FinishedSpan {
    /// The span name.
    name: String,
    /// The span kind.
    kind: SpanKind,
    /// The trace context.
    trace_context: Option<TraceContext>,
    /// Span attributes.
    attributes: Vec<(String, SpanValue)>,
    /// How long the span took.
    duration: Duration,
    /// The span status.
    status: SpanStatus,
}

impl FinishedSpan {
    /// Returns the span name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the span kind.
    pub fn kind(&self) -> SpanKind {
        self.kind
    }

    /// Returns the trace context, if set.
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace_context.as_ref()
    }

    /// Returns the span attributes.
    pub fn attributes(&self) -> &[(String, SpanValue)] {
        &self.attributes
    }

    /// Returns the span duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the span status.
    pub fn status(&self) -> &SpanStatus {
        &self.status
    }

    /// Returns `true` if the span completed successfully.
    pub fn is_ok(&self) -> bool {
        self.status.is_ok()
    }

    /// Returns `true` if the span completed with an error.
    pub fn is_error(&self) -> bool {
        self.status.is_error()
    }
}

/// A value that can be attached to a span as an attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanValue {
    /// A string value.
    String(String),
    /// An integer value.
    Int(i64),
    /// A float value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// An array of strings.
    StringArray(Vec<String>),
    /// An array of integers.
    IntArray(Vec<i64>),
}

impl SpanValue {
    /// Returns the value as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SpanValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an integer, if it is one.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            SpanValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the value as a float, if it is one.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            SpanValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the value as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SpanValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl std::fmt::Display for SpanValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanValue::String(s) => write!(f, "{}", s),
            SpanValue::Int(i) => write!(f, "{}", i),
            SpanValue::Float(fl) => write!(f, "{}", fl),
            SpanValue::Bool(b) => write!(f, "{}", b),
            SpanValue::StringArray(arr) => write!(f, "{:?}", arr),
            SpanValue::IntArray(arr) => write!(f, "{:?}", arr),
        }
    }
}

impl From<&str> for SpanValue {
    fn from(s: &str) -> Self {
        SpanValue::String(s.to_string())
    }
}

impl From<String> for SpanValue {
    fn from(s: String) -> Self {
        SpanValue::String(s)
    }
}

impl From<i64> for SpanValue {
    fn from(i: i64) -> Self {
        SpanValue::Int(i)
    }
}

impl From<i32> for SpanValue {
    fn from(i: i32) -> Self {
        SpanValue::Int(i as i64)
    }
}

impl From<u64> for SpanValue {
    fn from(i: u64) -> Self {
        SpanValue::Int(i as i64)
    }
}

impl From<usize> for SpanValue {
    fn from(i: usize) -> Self {
        SpanValue::Int(i as i64)
    }
}

impl From<f64> for SpanValue {
    fn from(f: f64) -> Self {
        SpanValue::Float(f)
    }
}

impl From<bool> for SpanValue {
    fn from(b: bool) -> Self {
        SpanValue::Bool(b)
    }
}

impl From<Vec<String>> for SpanValue {
    fn from(arr: Vec<String>) -> Self {
        SpanValue::StringArray(arr)
    }
}

impl From<Vec<i64>> for SpanValue {
    fn from(arr: Vec<i64>) -> Self {
        SpanValue::IntArray(arr)
    }
}

/// Common span names for InferaDB operations.
pub mod span_names {
    /// Check operation span name.
    pub const CHECK: &str = "inferadb.check";
    /// Batch check operation span name.
    pub const CHECK_BATCH: &str = "inferadb.check_batch";
    /// Relationship write span name.
    pub const RELATIONSHIP_WRITE: &str = "inferadb.relationship.write";
    /// Relationship delete span name.
    pub const RELATIONSHIP_DELETE: &str = "inferadb.relationship.delete";
    /// Relationship list span name.
    pub const RELATIONSHIP_LIST: &str = "inferadb.relationship.list";
    /// Schema push span name.
    pub const SCHEMA_PUSH: &str = "inferadb.schema.push";
    /// Schema activate span name.
    pub const SCHEMA_ACTIVATE: &str = "inferadb.schema.activate";
}

/// Common attribute keys for InferaDB spans.
pub mod attribute_keys {
    /// The subject being checked.
    pub const SUBJECT: &str = "inferadb.subject";
    /// The permission being checked.
    pub const PERMISSION: &str = "inferadb.permission";
    /// The resource being checked.
    pub const RESOURCE: &str = "inferadb.resource";
    /// Whether access was allowed.
    pub const ALLOWED: &str = "inferadb.allowed";
    /// The relation in a relationship.
    pub const RELATION: &str = "inferadb.relation";
    /// The vault ID.
    pub const VAULT_ID: &str = "inferadb.vault_id";
    /// The organization ID.
    pub const ORG_ID: &str = "inferadb.org_id";
    /// The request ID.
    pub const REQUEST_ID: &str = "inferadb.request_id";
    /// The batch size.
    pub const BATCH_SIZE: &str = "inferadb.batch_size";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_kind_display() {
        assert_eq!(SpanKind::Client.to_string(), "client");
        assert_eq!(SpanKind::Server.to_string(), "server");
        assert_eq!(SpanKind::Internal.to_string(), "internal");
    }

    #[test]
    fn test_span_status() {
        assert!(SpanStatus::Ok.is_ok());
        assert!(!SpanStatus::Ok.is_error());

        let error = SpanStatus::Error("test error".to_string());
        assert!(error.is_error());
        assert!(!error.is_ok());
        assert_eq!(error.error_message(), Some("test error"));
    }

    #[test]
    fn test_span_creation() {
        let span = InferaDbSpan::new("test.operation")
            .with_kind(SpanKind::Client)
            .with_attribute("key", "value")
            .with_attribute("count", 42i64);

        assert_eq!(span.name(), "test.operation");
        assert_eq!(span.kind(), SpanKind::Client);
        assert_eq!(span.attributes().len(), 2);
    }

    #[test]
    fn test_span_finish() {
        let span = InferaDbSpan::new("test.operation")
            .with_attribute("test", true);

        std::thread::sleep(std::time::Duration::from_millis(1));

        let finished = span.finish_ok();
        assert!(finished.is_ok());
        assert!(finished.duration() >= std::time::Duration::from_millis(1));
    }

    #[test]
    fn test_span_finish_error() {
        let span = InferaDbSpan::new("test.operation");
        let finished = span.finish_error("something went wrong");

        assert!(finished.is_error());
        assert_eq!(
            finished.status().error_message(),
            Some("something went wrong")
        );
    }

    #[test]
    fn test_span_value_conversions() {
        assert_eq!(SpanValue::from("test").as_str(), Some("test"));
        assert_eq!(SpanValue::from(42i64).as_int(), Some(42));
        assert_eq!(SpanValue::from(3.14f64).as_float(), Some(3.14));
        assert_eq!(SpanValue::from(true).as_bool(), Some(true));
    }

    #[test]
    fn test_span_with_trace_context() {
        let ctx = TraceContext::new_root();
        let span = InferaDbSpan::new("test.operation")
            .with_trace_context(ctx.clone());

        assert!(span.trace_context().is_some());
        assert_eq!(
            span.trace_context().unwrap().trace_id(),
            ctx.trace_id()
        );
    }
}
