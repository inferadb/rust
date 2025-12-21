//! Tracing integration for observability.
//!
//! This module provides integration with the `tracing` ecosystem
//! for structured logging and distributed tracing.
//!
//! ## Features
//!
//! - Request/response logging
//! - Span propagation (W3C Trace Context)
//! - Metrics integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use tracing_subscriber::prelude::*;
//!
//! // Set up tracing subscriber
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .init();
//!
//! // SDK automatically creates spans for operations
//! let allowed = vault.check("user:alice", "view", "doc:1").await?;
//! // Logs: check{subject="user:alice", permission="view", resource="doc:1"} -> allowed=true
//! ```

// Tracing support will be fully implemented in Phase 10
