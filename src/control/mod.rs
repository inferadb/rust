//! Control plane API for managing InferaDB resources.
//!
//! The control plane provides administrative operations:
//!
//! - Organization management
//! - Vault management
//! - Team and member management
//! - API client management
//! - Audit logs
//! - Schema management
//!
//! ## Example
//!
//! ```rust,ignore
//! // List vaults in an organization
//! let vaults = client.control()
//!     .organization("org_123")
//!     .vaults()
//!     .list()
//!     .await?;
//!
//! // Create a new vault
//! let vault = client.control()
//!     .organization("org_123")
//!     .vaults()
//!     .create(CreateVaultRequest {
//!         name: "My Vault".to_string(),
//!         ..Default::default()
//!     })
//!     .await?;
//! ```

// Control plane API will be implemented in Phase 9
