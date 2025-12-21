//! Transport layer for InferaDB communication.
//!
//! This module provides the underlying transport implementations:
//!
//! - gRPC transport (via tonic) - default, high performance
//! - REST transport (via reqwest) - for environments without gRPC support
//!
//! The transport layer is internal to the SDK. Users interact with
//! the higher-level [`Client`](crate::Client) and [`VaultClient`](crate::VaultClient) APIs.
//!
//! ## Feature Flags
//!
//! - `grpc` (default): Enable gRPC transport
//! - `rest` (default): Enable REST transport

// Transport implementation is internal for now
// Will be implemented in Phase 6

pub(crate) mod traits;

#[cfg(feature = "grpc")]
pub(crate) mod grpc;

#[cfg(feature = "rest")]
pub(crate) mod rest;
