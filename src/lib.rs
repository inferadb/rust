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
//! - **Relationship Order**: `Relationship::new(resource, relation, subject)` - "resource has
//!   relation subject"
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
//! - `derive`: Enable derive macros for type-safe schemas
//! - `wasm`: Enable WASM/browser support (REST only)
//!
//! ## Minimum Supported Rust Version
//!
//! This crate requires Rust **1.88.0** or later (MSRV). We target two releases
//! behind stable where possible. See the [CHANGELOG] for MSRV increase notices.
//!
//! [CHANGELOG]: https://github.com/inferadb/rust/blob/main/CHANGELOG.md

#![cfg_attr(docsrs, feature(doc_cfg))]
// Rustdoc-specific lints (not in workspace lints)
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![warn(rustdoc::invalid_codeblock_attributes)]
#![warn(rustdoc::invalid_html_tags)]
#![warn(rustdoc::bare_urls)]

// Core modules
pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod types;
pub mod vault;

// Transport layer
pub mod transport;

// User-Agent generation (internal)
mod user_agent;

// Middleware
pub mod middleware;

// Control plane API
pub mod control;

// Testing utilities
pub mod testing;

// Tracing support
#[cfg(feature = "tracing")]
#[cfg_attr(docsrs, doc(cfg(feature = "tracing")))]
pub mod tracing_support;

// WASM support
#[cfg(feature = "wasm")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm")))]
pub mod wasm;

// Prelude for convenient imports
pub mod prelude;

// Re-export main types at crate root for convenience
// Re-export auth types
pub use auth::{
    BearerCredentialsConfig, ClientCredentialsConfig, Credentials, CredentialsProvider,
    Ed25519PrivateKey,
};
pub use client::{
    Client, ClientBuilder, ComponentHealth, HealthResponse, HealthStatus, ReadinessCriteria,
    ShutdownGuard, ShutdownHandle,
};
// Re-export config types
pub use config::{
    CacheConfig, CircuitBreakerConfig, CircuitEvent, CircuitState, CircuitStats, DegradationConfig,
    FailureMode, FailurePredicate, RetryConfig, TlsConfig,
};
pub use error::{AccessDenied, Error, ErrorKind, Result};
// Testing support
pub use testing::{AuthorizationClient, InMemoryClient, MockClient};
// Re-export transport types
pub use transport::{
    FallbackReason, FallbackTrigger, GrpcStats, PoolConfig, RestStats, Transport, TransportEvent,
    TransportStats, TransportStrategy,
};
pub use types::{
    ConsistencyToken, Context, ContextValue, Decision, DecisionMetadata, DecisionReason, EntityRef,
    ParseError, Relationship, Resource, Subject, SubjectRef,
};
pub use vault::VaultClient;

// Re-export derive macros when feature is enabled
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub mod derive {
    //! Derive macros for Resource and Subject traits.
    //!
    //! Enable the `derive` feature to use these macros:
    //!
    //! ```toml
    //! [dependencies]
    //! inferadb = { version = "0.1", features = ["derive"] }
    //! ```
    //!
    //! ## Example
    //!
    //! ```rust,ignore
    //! use inferadb::derive::{Resource, Subject};
    //!
    //! #[derive(Resource)]
    //! #[resource(type = "document")]
    //! struct Document {
    //!     #[resource(id)]
    //!     id: String,
    //! }
    //!
    //! #[derive(Subject)]
    //! #[subject(type = "user")]
    //! struct User {
    //!     #[subject(id)]
    //!     id: String,
    //! }
    //! ```
    pub use inferadb_derive::{Resource, Subject};
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // Basic smoke test
        let _ = ErrorKind::Unauthorized;
    }
}
