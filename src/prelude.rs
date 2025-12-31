//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used types for easy importing:
//!
//! ```rust
//! use inferadb::prelude::*;
//! ```
//!
//! This provides access to:
//! - Core client types
//! - Error types
//! - Authentication types
//! - Common data types

pub use crate::{
    auth::{
        BearerCredentialsConfig, ClientCredentialsConfig, Credentials, CredentialsProvider,
        Ed25519PrivateKey,
    },
    client::{
        Client, ClientBuilder, ComponentHealth, HealthResponse, HealthStatus, ReadinessCriteria,
        ShutdownGuard, ShutdownHandle,
    },
    config::{CacheConfig, DegradationConfig, FailureMode, RetryConfig, TlsConfig},
    error::{AccessDenied, Error, ErrorKind, Result},
    testing::{AuthorizationClient, InMemoryClient, MockClient},
    types::{
        ConsistencyToken, Context, ContextValue, Decision, DecisionMetadata, DecisionReason,
        Relationship,
    },
    vault::VaultClient,
};
