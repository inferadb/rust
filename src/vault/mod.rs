//! Vault client for authorization operations.
//!
//! [`VaultClient`] provides the primary API for authorization checks:
//!
//! - [`check()`](VaultClient::check): Check if a subject has permission on a resource
//! - [`explain_permission()`](VaultClient::explain_permission): Explain why access is allowed/denied
//! - [`simulate()`](VaultClient::simulate): Test hypothetical changes
//! - [`watch()`](VaultClient::watch): Subscribe to relationship changes
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
//!
//! // Explain why access is allowed or denied
//! let explanation = vault
//!     .explain_permission()
//!     .subject("user:alice")
//!     .permission("edit")
//!     .resource("doc:readme")
//!     .await?;
//!
//! // Simulate hypothetical changes
//! let result = vault
//!     .simulate()
//!     .add_relationship(Relationship::new("doc:readme", "editor", "user:alice"))
//!     .check("user:alice", "edit", "doc:readme")
//!     .await?;
//!
//! // Watch for real-time changes
//! use futures::StreamExt;
//! let mut stream = vault.watch().run().await?;
//! while let Some(event) = stream.next().await {
//!     println!("Change: {}", event?);
//! }
//! ```

mod client;
mod explain;
mod simulate;
pub mod watch;

pub use client::VaultClient;
pub use explain::{
    AccessSuggestion, DenialReason, ExplainBuilder, PathNode, PermissionExplanation,
};
pub use simulate::{
    SimulateBuilder, SimulateCheckBuilder, SimulateCompareBuilder, SimulationChange,
    SimulationDiff, SimulationResult,
};
pub use watch::{
    Operation, ReconnectConfig, WatchBuilder, WatchEvent, WatchFilter, WatchShutdownHandle,
    WatchStream,
};
