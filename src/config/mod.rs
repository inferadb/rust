//! Configuration types for the InferaDB SDK.
//!
//! This module provides configuration options for:
//! - [`RetryConfig`]: Retry behavior for transient failures
//! - [`CacheConfig`]: Local caching of authorization decisions
//! - [`TlsConfig`]: TLS/SSL settings
//! - [`DegradationConfig`]: Graceful degradation behavior
//! - [`CircuitBreakerConfig`]: Circuit breaker for resilience

mod cache;
mod circuit_breaker;
mod degradation;
mod retry;
mod tls;

pub use cache::CacheConfig;
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitEvent, CircuitState, CircuitStats, FailurePredicate,
};
pub use degradation::{DegradationConfig, FailureMode};
pub use retry::RetryConfig;
pub use tls::TlsConfig;
