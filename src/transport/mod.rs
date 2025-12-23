//! Transport layer for InferaDB communication.
//!
//! This module provides the underlying transport implementations:
//!
//! - gRPC transport (via tonic) - default, high performance
//! - REST transport (via reqwest) - for environments without gRPC support
//! - Mock transport - for testing without network
//!
//! The transport layer is internal to the SDK. Users interact with
//! the higher-level [`Client`](crate::Client) and [`VaultClient`](crate::VaultClient) APIs.
//!
//! ## Feature Flags
//!
//! - `grpc` (default): Enable gRPC transport
//! - `rest` (default): Enable REST transport
//!
//! ## Transport Selection
//!
//! ```rust
//! use inferadb::Transport;
//!
//! // Available transports
//! let grpc = Transport::Grpc;   // High performance (default)
//! let http = Transport::Http;   // Universal compatibility
//! let mock = Transport::Mock;   // For testing
//! ```

// Allow dead code for transport types not yet integrated with client
#![allow(dead_code)]

pub(crate) mod traits;

#[cfg(feature = "grpc")]
pub(crate) mod grpc;

#[cfg(feature = "rest")]
pub(crate) mod rest;

pub(crate) mod mock;

// Re-export public types
pub use traits::{
    FallbackReason, FallbackTrigger, GrpcStats, PoolConfig, RestStats, Transport, TransportEvent,
    TransportStats, TransportStrategy,
};

// Internal re-exports (used when transport is integrated with client)
pub(crate) use traits::{
    CheckRequest as TransportCheckRequest, TransportClient, WriteRequest as TransportWriteRequest,
};

// Re-export REST transport
#[cfg(feature = "rest")]
pub use rest::{RestTransport, RestTransportBuilder};

// Re-export gRPC transport
#[cfg(feature = "grpc")]
pub use grpc::{GrpcTransport, GrpcTransportBuilder};
