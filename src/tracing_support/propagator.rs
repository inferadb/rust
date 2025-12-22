//! Trace context propagation.

use crate::tracing_support::context::{TraceContext, TraceContextError};

/// A trait for extracting values from headers.
pub trait HeaderExtractor {
    /// Gets a header value by name.
    fn get(&self, key: &str) -> Option<&str>;
}

/// A trait for injecting values into headers.
pub trait HeaderInjector {
    /// Sets a header value.
    fn set(&mut self, key: &str, value: String);
}

/// Implement HeaderExtractor for HashMap-like types.
impl HeaderExtractor for std::collections::HashMap<String, String> {
    fn get(&self, key: &str) -> Option<&str> {
        self.get(key).map(|s| s.as_str())
    }
}

impl HeaderInjector for std::collections::HashMap<String, String> {
    fn set(&mut self, key: &str, value: String) {
        self.insert(key.to_string(), value);
    }
}

/// A propagator for trace context.
pub trait Propagator {
    /// Extracts a trace context from headers.
    fn extract<E: HeaderExtractor>(&self, extractor: &E) -> Result<TraceContext, TraceContextError>;

    /// Injects a trace context into headers.
    fn inject<I: HeaderInjector>(&self, context: &TraceContext, injector: &mut I);
}

/// W3C Trace Context propagator.
///
/// Implements the [W3C Trace Context](https://www.w3.org/TR/trace-context/) specification.
///
/// ## Example
///
/// ```rust
/// use std::collections::HashMap;
/// use inferadb::tracing_support::{W3CTraceContext, TraceContext, Propagator, HeaderInjector};
///
/// let propagator = W3CTraceContext;
///
/// // Inject trace context into headers
/// let ctx = TraceContext::new_root();
/// let mut headers = HashMap::new();
/// propagator.inject(&ctx, &mut headers);
///
/// assert!(headers.contains_key("traceparent"));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct W3CTraceContext;

impl W3CTraceContext {
    /// The traceparent header name.
    pub const TRACEPARENT: &'static str = "traceparent";
    /// The tracestate header name.
    pub const TRACESTATE: &'static str = "tracestate";
}

impl Propagator for W3CTraceContext {
    fn extract<E: HeaderExtractor>(&self, extractor: &E) -> Result<TraceContext, TraceContextError> {
        let traceparent = extractor
            .get(Self::TRACEPARENT)
            .ok_or(TraceContextError::InvalidFormat)?;

        let mut ctx = TraceContext::from_traceparent(traceparent)?;

        if let Some(tracestate) = extractor.get(Self::TRACESTATE) {
            ctx = ctx.with_tracestate(tracestate);
        }

        Ok(ctx)
    }

    fn inject<I: HeaderInjector>(&self, context: &TraceContext, injector: &mut I) {
        injector.set(Self::TRACEPARENT, context.to_traceparent());

        if let Some(tracestate) = context.tracestate() {
            injector.set(Self::TRACESTATE, tracestate.to_string());
        }
    }
}

/// B3 propagator for Zipkin compatibility.
///
/// Implements the [B3 Propagation](https://github.com/openzipkin/b3-propagation) format
/// used by Zipkin and compatible systems.
///
/// ## Example
///
/// ```rust
/// use std::collections::HashMap;
/// use inferadb::tracing_support::{B3Propagator, TraceContext, Propagator, HeaderInjector};
///
/// let propagator = B3Propagator::single();
///
/// // Inject trace context into headers
/// let ctx = TraceContext::new_root();
/// let mut headers = HashMap::new();
/// propagator.inject(&ctx, &mut headers);
///
/// assert!(headers.contains_key("b3"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct B3Propagator {
    /// Whether to use single header format.
    single_header: bool,
}

impl B3Propagator {
    /// B3 single header name.
    pub const B3: &'static str = "b3";
    /// X-B3-TraceId header name.
    pub const X_B3_TRACE_ID: &'static str = "x-b3-traceid";
    /// X-B3-SpanId header name.
    pub const X_B3_SPAN_ID: &'static str = "x-b3-spanid";
    /// X-B3-Sampled header name.
    pub const X_B3_SAMPLED: &'static str = "x-b3-sampled";
    /// X-B3-ParentSpanId header name.
    pub const X_B3_PARENT_SPAN_ID: &'static str = "x-b3-parentspanid";

    /// Creates a new B3 propagator using the single header format.
    pub fn single() -> Self {
        Self { single_header: true }
    }

    /// Creates a new B3 propagator using multiple headers.
    pub fn multi() -> Self {
        Self { single_header: false }
    }
}

impl Default for B3Propagator {
    fn default() -> Self {
        Self::single()
    }
}

impl Propagator for B3Propagator {
    fn extract<E: HeaderExtractor>(&self, extractor: &E) -> Result<TraceContext, TraceContextError> {
        // Try single header first
        if let Some(b3) = extractor.get(Self::B3) {
            return parse_b3_single(b3);
        }

        // Try multi-header format
        let trace_id = extractor
            .get(Self::X_B3_TRACE_ID)
            .ok_or(TraceContextError::InvalidFormat)?;
        let span_id = extractor
            .get(Self::X_B3_SPAN_ID)
            .ok_or(TraceContextError::InvalidFormat)?;

        let sampled = extractor
            .get(Self::X_B3_SAMPLED)
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        let trace_id = crate::tracing_support::TraceId::from_hex(trace_id)?;
        let span_id = crate::tracing_support::SpanId::from_hex(span_id)?;

        Ok(TraceContext::new(trace_id, span_id).with_sampled(sampled))
    }

    fn inject<I: HeaderInjector>(&self, context: &TraceContext, injector: &mut I) {
        if self.single_header {
            let sampled = if context.is_sampled() { "1" } else { "0" };
            injector.set(
                Self::B3,
                format!("{}-{}-{}", context.trace_id(), context.span_id(), sampled),
            );
        } else {
            injector.set(Self::X_B3_TRACE_ID, context.trace_id().to_string());
            injector.set(Self::X_B3_SPAN_ID, context.span_id().to_string());
            injector.set(
                Self::X_B3_SAMPLED,
                if context.is_sampled() { "1" } else { "0" }.to_string(),
            );
            if let Some(parent) = context.parent_span_id() {
                injector.set(Self::X_B3_PARENT_SPAN_ID, parent.to_string());
            }
        }
    }
}

/// Parses the B3 single header format.
fn parse_b3_single(b3: &str) -> Result<TraceContext, TraceContextError> {
    // Format: {trace_id}-{span_id}-{sampling_state}[-{parent_span_id}]
    // Or: {sampling_state} (just "0" or "1" for deny/accept)
    if b3 == "0" {
        return Ok(TraceContext::new_root().with_sampled(false));
    }
    if b3 == "1" {
        return Ok(TraceContext::new_root().with_sampled(true));
    }

    let parts: Vec<&str> = b3.split('-').collect();
    if parts.len() < 2 {
        return Err(TraceContextError::InvalidFormat);
    }

    let trace_id = crate::tracing_support::TraceId::from_hex(parts[0])?;
    let span_id = crate::tracing_support::SpanId::from_hex(parts[1])?;

    let sampled = if parts.len() > 2 {
        parts[2] == "1" || parts[2].eq_ignore_ascii_case("true") || parts[2] == "d"
    } else {
        true
    };

    Ok(TraceContext::new(trace_id, span_id).with_sampled(sampled))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_w3c_inject_extract() {
        let propagator = W3CTraceContext;
        let ctx = TraceContext::new_root();

        let mut headers = HashMap::new();
        propagator.inject(&ctx, &mut headers);

        assert!(headers.contains_key("traceparent"));

        let extracted = propagator.extract(&headers).unwrap();
        assert_eq!(extracted.trace_id(), ctx.trace_id());
        assert_eq!(extracted.span_id(), ctx.span_id());
    }

    #[test]
    fn test_w3c_with_tracestate() {
        let propagator = W3CTraceContext;
        let ctx = TraceContext::new_root().with_tracestate("vendor=value");

        let mut headers = HashMap::new();
        propagator.inject(&ctx, &mut headers);

        assert_eq!(headers.get("tracestate"), Some(&"vendor=value".to_string()));

        let extracted = propagator.extract(&headers).unwrap();
        assert_eq!(extracted.tracestate(), Some("vendor=value"));
    }

    #[test]
    fn test_b3_single_inject_extract() {
        let propagator = B3Propagator::single();
        let ctx = TraceContext::new_root();

        let mut headers = HashMap::new();
        propagator.inject(&ctx, &mut headers);

        assert!(headers.contains_key("b3"));

        let extracted = propagator.extract(&headers).unwrap();
        assert_eq!(extracted.trace_id(), ctx.trace_id());
        assert_eq!(extracted.span_id(), ctx.span_id());
    }

    #[test]
    fn test_b3_multi_inject_extract() {
        let propagator = B3Propagator::multi();
        let ctx = TraceContext::new_root();

        let mut headers = HashMap::new();
        propagator.inject(&ctx, &mut headers);

        assert!(headers.contains_key("x-b3-traceid"));
        assert!(headers.contains_key("x-b3-spanid"));
        assert!(headers.contains_key("x-b3-sampled"));

        let extracted = propagator.extract(&headers).unwrap();
        assert_eq!(extracted.trace_id(), ctx.trace_id());
        assert_eq!(extracted.span_id(), ctx.span_id());
    }

    #[test]
    fn test_b3_single_deny() {
        let propagator = B3Propagator::single();
        let mut headers = HashMap::new();
        headers.insert("b3".to_string(), "0".to_string());

        let ctx = propagator.extract(&headers).unwrap();
        assert!(!ctx.is_sampled());
    }
}
