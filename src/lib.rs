//! # InferaDB Rust SDK
//!
//! Official Rust SDK for the InferaDB authorization service.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use inferadb::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), inferadb::Error> {
//!     // Create client
//!     let client = Client::builder()
//!         .url("https://api.inferadb.com")
//!         .credentials(ClientCredentialsConfig {
//!             client_id: "your-client-id".into(),
//!             private_key: Ed25519PrivateKey::from_pem_file("private-key.pem")?,
//!             certificate_id: None,
//!         })
//!         .build()
//!         .await?;
//!
//!     // Get vault context
//!     let vault = client.organization("org_...").vault("vlt_...");
//!
//!     // Check permission
//!     let allowed = vault.check("user:alice", "view", "document:readme").await?;
//!     println!("Allowed: {}", allowed);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Key Concepts
//!
//! - **Client Hierarchy**: `Client` → `OrganizationClient` → `VaultClient`
//! - **Argument Order**: `check(subject, permission, resource)` - "Can subject do X to resource?"
//! - **Relationship Order**: `Relationship::new(resource, relation, subject)` - "resource has relation subject"
//! - **Denial ≠ Error**: `check()` returns `Ok(false)` for denied access, not `Err`
//!
//! ## Features
//!
//! - `grpc` (default): Enable gRPC transport via tonic
//! - `rest` (default): Enable REST transport via reqwest
//! - `rustls` (default): Use rustls for TLS
//! - `native-tls`: Use native TLS (OpenSSL on Linux, Secure Transport on macOS)
//! - `tracing`: Enable tracing integration
//! - `blocking`: Enable blocking API
//! - `insecure`: Enable `.insecure()` for development (never use in production)

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_code)]

// Core modules
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod types;
pub mod vault;

// Transport layer
pub mod transport;

// Control plane API
pub mod control;

// Testing utilities
pub mod testing;

// Tracing support
#[cfg(feature = "tracing")]
#[cfg_attr(docsrs, doc(cfg(feature = "tracing")))]
pub mod tracing_support;

// Prelude for convenient imports
pub mod prelude;

// Re-export main types at crate root for convenience
pub use client::{Client, ClientBuilder};
pub use error::{AccessDenied, Error, ErrorKind};
pub use types::{
    ConsistencyToken, Context, ContextValue, Decision, DecisionMetadata, DecisionReason,
    Relationship,
};
pub use vault::VaultClient;

// Re-export auth types
pub use auth::{
    BearerCredentialsConfig, ClientCredentialsConfig, Credentials, CredentialsProvider,
    Ed25519PrivateKey,
};

// Re-export config types
pub use config::{
    CacheConfig, CircuitBreakerConfig, CircuitEvent, CircuitState, CircuitStats,
    DegradationConfig, FailureMode, FailurePredicate, RetryConfig, TlsConfig,
};

// Testing support
pub use testing::{AuthorizationClient, InMemoryClient, MockClient};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // Basic smoke test
        let _ = ErrorKind::Unauthorized;
    }
}
