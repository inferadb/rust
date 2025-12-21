//! Vault client for authorization operations.
//!
//! [`VaultClient`] provides the primary API for authorization checks:
//!
//! - [`check()`](VaultClient::check): Check if a subject has permission on a resource
//! - [`relationships()`](VaultClient::relationships): Manage relationships
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! // Get a vault client
//! let vault = client.organization("org_123").vault("vlt_456");
//!
//! // Check permission
//! let allowed = vault.check("user:alice", "view", "doc:readme").await?;
//!
//! // Use require() to get an error on denial
//! vault.check("user:alice", "edit", "doc:readme")
//!     .require()
//!     .await?;
//! ```

mod client;

pub use client::VaultClient;
