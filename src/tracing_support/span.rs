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
#[allow(clippy::unwrap_used, clippy::panic)]
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
        let span = InferaDbSpan::new("test.operation").with_attribute("test", true);

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
        assert_eq!(finished.status().error_message(), Some("something went wrong"));
    }

    #[test]
    fn test_span_value_conversions() {
        assert_eq!(SpanValue::from("test").as_str(), Some("test"));
        assert_eq!(SpanValue::from(42i64).as_int(), Some(42));
        assert_eq!(SpanValue::from(1.23f64).as_float(), Some(1.23));
        assert_eq!(SpanValue::from(true).as_bool(), Some(true));
    }

    #[test]
    fn test_span_with_trace_context() {
        let ctx = TraceContext::new_root();
        let span = InferaDbSpan::new("test.operation").with_trace_context(ctx.clone());

        assert!(span.trace_context().is_some());
        assert_eq!(span.trace_context().unwrap().trace_id(), ctx.trace_id());
    }

    #[test]
    fn test_span_kind_otel_value() {
        assert_eq!(SpanKind::Client.otel_value(), 3);
        assert_eq!(SpanKind::Server.otel_value(), 2);
        assert_eq!(SpanKind::Internal.otel_value(), 1);
        assert_eq!(SpanKind::Producer.otel_value(), 4);
        assert_eq!(SpanKind::Consumer.otel_value(), 5);
    }

    #[test]
    fn test_span_kind_default() {
        assert_eq!(SpanKind::default(), SpanKind::Client);
    }

    #[test]
    fn test_span_kind_display_all() {
        assert_eq!(SpanKind::Producer.to_string(), "producer");
        assert_eq!(SpanKind::Consumer.to_string(), "consumer");
    }

    #[test]
    fn test_span_kind_debug() {
        let debug = format!("{:?}", SpanKind::Client);
        assert_eq!(debug, "Client");
    }

    #[test]
    fn test_span_kind_clone_eq() {
        let kind = SpanKind::Server;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    #[test]
    fn test_span_status_default() {
        assert_eq!(SpanStatus::default(), SpanStatus::Ok);
    }

    #[test]
    fn test_span_status_unset() {
        let status = SpanStatus::Unset;
        assert!(!status.is_ok());
        assert!(!status.is_error());
        assert!(status.error_message().is_none());
    }

    #[test]
    fn test_span_status_display() {
        assert_eq!(SpanStatus::Ok.to_string(), "ok");
        assert_eq!(SpanStatus::Error("test".to_string()).to_string(), "error: test");
        assert_eq!(SpanStatus::Unset.to_string(), "unset");
    }

    #[test]
    fn test_span_status_debug() {
        let debug = format!("{:?}", SpanStatus::Ok);
        assert!(debug.contains("Ok"));
    }

    #[test]
    fn test_span_status_clone_eq() {
        let status = SpanStatus::Ok;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_span_value_display() {
        assert_eq!(SpanValue::String("test".to_string()).to_string(), "test");
        assert_eq!(SpanValue::Int(42).to_string(), "42");
        assert_eq!(SpanValue::Float(1.23).to_string(), "1.23");
        assert_eq!(SpanValue::Bool(true).to_string(), "true");
        assert!(SpanValue::StringArray(vec!["a".to_string()]).to_string().contains("a"));
        assert!(SpanValue::IntArray(vec![1, 2]).to_string().contains("1"));
    }

    #[test]
    fn test_span_value_as_wrong_type() {
        let string_val = SpanValue::String("test".to_string());
        assert!(string_val.as_int().is_none());
        assert!(string_val.as_float().is_none());
        assert!(string_val.as_bool().is_none());

        let int_val = SpanValue::Int(42);
        assert!(int_val.as_str().is_none());
        assert!(int_val.as_float().is_none());
        assert!(int_val.as_bool().is_none());

        let float_val = SpanValue::Float(1.23);
        assert!(float_val.as_str().is_none());
        assert!(float_val.as_int().is_none());
        assert!(float_val.as_bool().is_none());

        let bool_val = SpanValue::Bool(true);
        assert!(bool_val.as_str().is_none());
        assert!(bool_val.as_int().is_none());
        assert!(bool_val.as_float().is_none());
    }

    #[test]
    fn test_span_value_from_i32() {
        let val: SpanValue = 42i32.into();
        assert_eq!(val.as_int(), Some(42));
    }

    #[test]
    fn test_span_value_from_u64() {
        let val: SpanValue = 100u64.into();
        assert_eq!(val.as_int(), Some(100));
    }

    #[test]
    fn test_span_value_from_usize() {
        let val: SpanValue = 50usize.into();
        assert_eq!(val.as_int(), Some(50));
    }

    #[test]
    fn test_span_value_from_string() {
        let val: SpanValue = String::from("hello").into();
        assert_eq!(val.as_str(), Some("hello"));
    }

    #[test]
    fn test_span_value_from_vec_string() {
        let val: SpanValue = vec!["a".to_string(), "b".to_string()].into();
        if let SpanValue::StringArray(arr) = val {
            assert_eq!(arr, vec!["a", "b"]);
        } else {
            panic!("Expected StringArray");
        }
    }

    #[test]
    fn test_span_value_from_vec_i64() {
        let val: SpanValue = vec![1i64, 2i64, 3i64].into();
        if let SpanValue::IntArray(arr) = val {
            assert_eq!(arr, vec![1, 2, 3]);
        } else {
            panic!("Expected IntArray");
        }
    }

    #[test]
    fn test_span_value_eq() {
        let val1 = SpanValue::Int(42);
        let val2 = SpanValue::Int(42);
        assert_eq!(val1, val2);

        let val3 = SpanValue::Int(43);
        assert_ne!(val1, val3);
    }

    #[test]
    fn test_span_elapsed() {
        let span = InferaDbSpan::new("test");
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(span.elapsed() >= std::time::Duration::from_millis(1));
    }

    #[test]
    fn test_span_finish_with_status() {
        let span = InferaDbSpan::new("test");
        let finished = span.finish(SpanStatus::Unset);
        assert_eq!(*finished.status(), SpanStatus::Unset);
    }

    #[test]
    fn test_finished_span_accessors() {
        let ctx = TraceContext::new_root();
        let span = InferaDbSpan::new("test.op")
            .with_kind(SpanKind::Server)
            .with_trace_context(ctx.clone())
            .with_attribute("key", "value");

        let finished = span.finish_ok();

        assert_eq!(finished.name(), "test.op");
        assert_eq!(finished.kind(), SpanKind::Server);
        assert!(finished.trace_context().is_some());
        assert_eq!(finished.attributes().len(), 1);
        assert!(finished.duration() >= Duration::ZERO);
        assert!(finished.is_ok());
        assert!(!finished.is_error());
    }

    #[test]
    fn test_finished_span_clone() {
        let span = InferaDbSpan::new("test");
        let finished = span.finish_ok();
        let cloned = finished.clone();
        assert_eq!(finished.name(), cloned.name());
    }

    #[test]
    fn test_finished_span_debug() {
        let span = InferaDbSpan::new("test");
        let finished = span.finish_ok();
        let debug = format!("{:?}", finished);
        assert!(debug.contains("FinishedSpan"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_span_debug() {
        let span = InferaDbSpan::new("test");
        let debug = format!("{:?}", span);
        assert!(debug.contains("InferaDbSpan"));
    }

    #[test]
    fn test_span_clone() {
        let span = InferaDbSpan::new("test").with_attribute("key", "value");
        let cloned = span.clone();
        assert_eq!(span.name(), cloned.name());
    }

    #[test]
    fn test_span_names_constants() {
        assert_eq!(span_names::CHECK, "inferadb.check");
        assert_eq!(span_names::CHECK_BATCH, "inferadb.check_batch");
        assert_eq!(span_names::RELATIONSHIP_WRITE, "inferadb.relationship.write");
        assert_eq!(span_names::RELATIONSHIP_DELETE, "inferadb.relationship.delete");
        assert_eq!(span_names::RELATIONSHIP_LIST, "inferadb.relationship.list");
        assert_eq!(span_names::SCHEMA_PUSH, "inferadb.schema.push");
        assert_eq!(span_names::SCHEMA_ACTIVATE, "inferadb.schema.activate");
    }

    #[test]
    fn test_attribute_keys_constants() {
        assert_eq!(attribute_keys::SUBJECT, "inferadb.subject");
        assert_eq!(attribute_keys::PERMISSION, "inferadb.permission");
        assert_eq!(attribute_keys::RESOURCE, "inferadb.resource");
        assert_eq!(attribute_keys::ALLOWED, "inferadb.allowed");
        assert_eq!(attribute_keys::RELATION, "inferadb.relation");
        assert_eq!(attribute_keys::VAULT_ID, "inferadb.vault_id");
        assert_eq!(attribute_keys::ORG_ID, "inferadb.org_id");
        assert_eq!(attribute_keys::REQUEST_ID, "inferadb.request_id");
        assert_eq!(attribute_keys::BATCH_SIZE, "inferadb.batch_size");
    }
}
