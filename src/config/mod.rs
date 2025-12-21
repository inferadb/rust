//! Configuration types for the InferaDB SDK.
//!
//! This module provides configuration options for:
//! - [`RetryConfig`]: Retry behavior for transient failures
//! - [`CacheConfig`]: Local caching of authorization decisions
//! - [`TlsConfig`]: TLS/SSL settings
//! - [`DegradationConfig`]: Graceful degradation behavior

mod cache;
mod degradation;
mod retry;
mod tls;

pub use cache::CacheConfig;
pub use degradation::{DegradationConfig, FailureMode};
pub use retry::RetryConfig;
pub use tls::TlsConfig;
