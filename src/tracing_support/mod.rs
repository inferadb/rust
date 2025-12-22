//! Tracing integration for observability.
//!
//! This module provides integration with the `tracing` ecosystem
//! for structured logging and distributed tracing.
//!
//! ## Features
//!
//! - Request/response logging
//! - Span propagation (W3C Trace Context)
//! - OpenTelemetry integration
//! - Metrics collection
//!
//! ## Example
//!
//! ```rust,ignore
//! use tracing_subscriber::prelude::*;
//! use inferadb::tracing_support::TraceContext;
//!
//! // Set up tracing subscriber
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .init();
//!
//! // Propagate trace context from incoming request
//! let trace_ctx = TraceContext::from_headers(&headers)?;
//!
//! // SDK automatically creates spans for operations
//! let allowed = vault.check("user:alice", "view", "doc:1")
//!     .with_trace_context(trace_ctx)
//!     .await?;
//! ```

// Allow dead code for tracing types not yet integrated
#![allow(dead_code)]

mod context;
mod propagator;
mod metrics;
mod span;

pub use context::{TraceContext, TraceId, SpanId, TraceFlags};
pub use propagator::{Propagator, W3CTraceContext, B3Propagator, HeaderExtractor, HeaderInjector};
pub use metrics::{Metrics, MetricsConfig, Counter, Histogram, Gauge};
pub use span::{SpanKind, SpanStatus, InferaDbSpan};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_roundtrip() {
        let ctx = TraceContext::new_root();
        let header = ctx.to_traceparent();
        let parsed = TraceContext::from_traceparent(&header).unwrap();
        assert_eq!(parsed.trace_id(), ctx.trace_id());
    }
}
