//! Trace context for distributed tracing.

use std::fmt;

/// A distributed trace context following W3C Trace Context specification.
///
/// This type carries trace information across service boundaries,
/// enabling correlation of requests in distributed systems.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::tracing_support::TraceContext;
///
/// // Create from incoming headers
/// let ctx = TraceContext::from_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")?;
///
/// // Generate a new root trace
/// let ctx = TraceContext::new_root();
///
/// // Create a child span context
/// let child = ctx.child();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// The trace ID (16 bytes).
    trace_id: TraceId,
    /// The span ID (8 bytes).
    span_id: SpanId,
    /// The parent span ID (if any).
    parent_span_id: Option<SpanId>,
    /// Trace flags.
    flags: TraceFlags,
    /// Tracestate for vendor-specific data.
    tracestate: Option<String>,
}

impl TraceContext {
    /// Creates a new root trace context with random IDs.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::tracing_support::TraceContext;
    ///
    /// let ctx = TraceContext::new_root();
    /// assert!(ctx.is_sampled());
    /// ```
    pub fn new_root() -> Self {
        Self {
            trace_id: TraceId::random(),
            span_id: SpanId::random(),
            parent_span_id: None,
            flags: TraceFlags::SAMPLED,
            tracestate: None,
        }
    }

    /// Creates a new trace context with the given trace and span IDs.
    pub fn new(trace_id: TraceId, span_id: SpanId) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            flags: TraceFlags::SAMPLED,
            tracestate: None,
        }
    }

    /// Creates a child span context from this context.
    ///
    /// The child inherits the trace ID and uses the current span ID as its parent.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: SpanId::random(),
            parent_span_id: Some(self.span_id.clone()),
            flags: self.flags,
            tracestate: self.tracestate.clone(),
        }
    }

    /// Creates a trace context from a W3C traceparent header value.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::tracing_support::TraceContext;
    ///
    /// let ctx = TraceContext::from_traceparent(
    ///     "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    /// ).unwrap();
    ///
    /// assert_eq!(ctx.trace_id().to_string(), "4bf92f3577b34da6a3ce929d0e0e4736");
    /// assert_eq!(ctx.span_id().to_string(), "00f067aa0ba902b7");
    /// assert!(ctx.is_sampled());
    /// ```
    pub fn from_traceparent(traceparent: &str) -> Result<Self, TraceContextError> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 {
            return Err(TraceContextError::InvalidFormat);
        }

        let version = parts[0];
        if version != "00" {
            return Err(TraceContextError::UnsupportedVersion);
        }

        let trace_id = TraceId::from_hex(parts[1])?;
        let span_id = SpanId::from_hex(parts[2])?;
        let flags = TraceFlags::from_hex(parts[3])?;

        Ok(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            flags,
            tracestate: None,
        })
    }

    /// Returns the traceparent header value.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::tracing_support::TraceContext;
    ///
    /// let ctx = TraceContext::new_root();
    /// let header = ctx.to_traceparent();
    /// assert!(header.starts_with("00-"));
    /// ```
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id, self.span_id, self.flags.0
        )
    }

    /// Returns the trace ID.
    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    /// Returns the span ID.
    pub fn span_id(&self) -> &SpanId {
        &self.span_id
    }

    /// Returns the parent span ID, if any.
    pub fn parent_span_id(&self) -> Option<&SpanId> {
        self.parent_span_id.as_ref()
    }

    /// Returns the trace flags.
    pub fn flags(&self) -> TraceFlags {
        self.flags
    }

    /// Returns `true` if the trace is sampled.
    pub fn is_sampled(&self) -> bool {
        self.flags.is_sampled()
    }

    /// Sets the tracestate header value.
    pub fn with_tracestate(mut self, tracestate: impl Into<String>) -> Self {
        self.tracestate = Some(tracestate.into());
        self
    }

    /// Returns the tracestate header value, if any.
    pub fn tracestate(&self) -> Option<&str> {
        self.tracestate.as_deref()
    }

    /// Sets the sampled flag.
    pub fn with_sampled(mut self, sampled: bool) -> Self {
        if sampled {
            self.flags = self.flags | TraceFlags::SAMPLED;
        } else {
            self.flags = TraceFlags(self.flags.0 & !TraceFlags::SAMPLED.0);
        }
        self
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new_root()
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_traceparent())
    }
}

/// A 128-bit trace identifier.
#[derive(Clone, PartialEq, Eq)]
pub struct TraceId([u8; 16]);

impl TraceId {
    /// Creates a new random trace ID.
    pub fn random() -> Self {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes");
        Self(bytes)
    }

    /// Creates a trace ID from bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Creates a trace ID from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, TraceContextError> {
        if hex.len() != 32 {
            return Err(TraceContextError::InvalidTraceId);
        }
        let mut bytes = [0u8; 16];
        hex::decode_to_slice(hex, &mut bytes)
            .map_err(|_| TraceContextError::InvalidTraceId)?;

        // Check for invalid all-zero trace ID
        if bytes == [0u8; 16] {
            return Err(TraceContextError::InvalidTraceId);
        }

        Ok(Self(bytes))
    }

    /// Returns the trace ID as bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TraceId({})", self)
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// A 64-bit span identifier.
#[derive(Clone, PartialEq, Eq)]
pub struct SpanId([u8; 8]);

impl SpanId {
    /// Creates a new random span ID.
    pub fn random() -> Self {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes");
        Self(bytes)
    }

    /// Creates a span ID from bytes.
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Creates a span ID from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, TraceContextError> {
        if hex.len() != 16 {
            return Err(TraceContextError::InvalidSpanId);
        }
        let mut bytes = [0u8; 8];
        hex::decode_to_slice(hex, &mut bytes)
            .map_err(|_| TraceContextError::InvalidSpanId)?;

        // Check for invalid all-zero span ID
        if bytes == [0u8; 8] {
            return Err(TraceContextError::InvalidSpanId);
        }

        Ok(Self(bytes))
    }

    /// Returns the span ID as bytes.
    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
}

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpanId({})", self)
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Trace flags as defined by W3C Trace Context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraceFlags(u8);

impl TraceFlags {
    /// No flags set.
    pub const NONE: Self = Self(0);
    /// The trace is sampled.
    pub const SAMPLED: Self = Self(0x01);

    /// Creates trace flags from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, TraceContextError> {
        if hex.len() != 2 {
            return Err(TraceContextError::InvalidFlags);
        }
        let value = u8::from_str_radix(hex, 16)
            .map_err(|_| TraceContextError::InvalidFlags)?;
        Ok(Self(value))
    }

    /// Returns `true` if the sampled flag is set.
    pub fn is_sampled(&self) -> bool {
        self.0 & Self::SAMPLED.0 != 0
    }

    /// Returns the raw flag value.
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for TraceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Error parsing trace context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceContextError {
    /// Invalid traceparent format.
    InvalidFormat,
    /// Unsupported version.
    UnsupportedVersion,
    /// Invalid trace ID.
    InvalidTraceId,
    /// Invalid span ID.
    InvalidSpanId,
    /// Invalid flags.
    InvalidFlags,
}

impl fmt::Display for TraceContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceContextError::InvalidFormat => write!(f, "invalid traceparent format"),
            TraceContextError::UnsupportedVersion => write!(f, "unsupported trace context version"),
            TraceContextError::InvalidTraceId => write!(f, "invalid trace ID"),
            TraceContextError::InvalidSpanId => write!(f, "invalid span ID"),
            TraceContextError::InvalidFlags => write!(f, "invalid trace flags"),
        }
    }
}

impl std::error::Error for TraceContextError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_new_root() {
        let ctx = TraceContext::new_root();
        assert!(ctx.is_sampled());
        assert!(ctx.parent_span_id().is_none());
    }

    #[test]
    fn test_trace_context_child() {
        let parent = TraceContext::new_root();
        let child = parent.child();

        assert_eq!(child.trace_id(), parent.trace_id());
        assert_ne!(child.span_id(), parent.span_id());
        assert_eq!(child.parent_span_id(), Some(parent.span_id()));
    }

    #[test]
    fn test_trace_context_from_traceparent() {
        let ctx = TraceContext::from_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        ).unwrap();

        assert_eq!(ctx.trace_id().to_string(), "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(ctx.span_id().to_string(), "00f067aa0ba902b7");
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_trace_context_to_traceparent() {
        let ctx = TraceContext::from_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        ).unwrap();

        assert_eq!(
            ctx.to_traceparent(),
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        );
    }

    #[test]
    fn test_trace_context_not_sampled() {
        let ctx = TraceContext::from_traceparent(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00"
        ).unwrap();

        assert!(!ctx.is_sampled());
    }

    #[test]
    fn test_trace_context_invalid_format() {
        assert!(TraceContext::from_traceparent("invalid").is_err());
        assert!(TraceContext::from_traceparent("00-abc-def-01").is_err());
    }

    #[test]
    fn test_trace_context_with_tracestate() {
        let ctx = TraceContext::new_root()
            .with_tracestate("vendor=value");

        assert_eq!(ctx.tracestate(), Some("vendor=value"));
    }

    #[test]
    fn test_trace_context_with_sampled() {
        let ctx = TraceContext::new_root().with_sampled(false);
        assert!(!ctx.is_sampled());

        let ctx = ctx.with_sampled(true);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_trace_id_from_hex() {
        let id = TraceId::from_hex("4bf92f3577b34da6a3ce929d0e0e4736").unwrap();
        assert_eq!(id.to_string(), "4bf92f3577b34da6a3ce929d0e0e4736");
    }

    #[test]
    fn test_trace_id_invalid_all_zeros() {
        assert!(TraceId::from_hex("00000000000000000000000000000000").is_err());
    }

    #[test]
    fn test_span_id_from_hex() {
        let id = SpanId::from_hex("00f067aa0ba902b7").unwrap();
        assert_eq!(id.to_string(), "00f067aa0ba902b7");
    }

    #[test]
    fn test_span_id_invalid_all_zeros() {
        assert!(SpanId::from_hex("0000000000000000").is_err());
    }

    #[test]
    fn test_trace_flags() {
        assert!(!TraceFlags::NONE.is_sampled());
        assert!(TraceFlags::SAMPLED.is_sampled());
        assert!((TraceFlags::NONE | TraceFlags::SAMPLED).is_sampled());
    }
}
