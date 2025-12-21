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

// Core client types
pub use crate::client::{Client, ClientBuilder};
pub use crate::vault::VaultClient;

// Error types
pub use crate::error::{AccessDenied, Error, ErrorKind, Result};

// Authentication types
pub use crate::auth::{
    BearerCredentialsConfig, ClientCredentialsConfig, Credentials, CredentialsProvider,
    Ed25519PrivateKey,
};

// Core data types
pub use crate::types::{
    ConsistencyToken, Context, ContextValue, Decision, DecisionMetadata, DecisionReason,
    Relationship,
};

// Configuration types
pub use crate::config::{CacheConfig, DegradationConfig, FailureMode, RetryConfig, TlsConfig};

// Testing support
pub use crate::testing::{AuthorizationClient, InMemoryClient, MockClient};
