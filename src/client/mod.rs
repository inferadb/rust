//! Client types for connecting to InferaDB.
//!
//! The SDK uses a hierarchical client structure:
//! - [`Client`]: Top-level client, manages connections and authentication
//! - `OrganizationClient`: Organization-scoped operations
//! - [`VaultClient`]: Vault-scoped authorization operations
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use inferadb::prelude::*;
//!
//! let client = Client::builder()
//!     .url("https://api.inferadb.com")
//!     .credentials(ClientCredentialsConfig {
//!         client_id: "your-client-id".into(),
//!         private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
//!         certificate_id: None,
//!     })
//!     .build()
//!     .await?;
//!
//! let vault = client.organization("org_123").vault("vlt_456");
//! let allowed = vault.check("user:alice", "view", "doc:readme").await?;
//! ```

mod builder;
mod inner;

pub use builder::ClientBuilder;

use std::sync::Arc;

use crate::vault::VaultClient;

/// The InferaDB SDK client.
///
/// This is the main entry point for the SDK. Create a client using
/// [`Client::builder()`], then navigate to organizations and vaults
/// to perform authorization operations.
///
/// ## Thread Safety
///
/// `Client` is `Clone` and thread-safe. It uses internal connection
/// pooling and can be shared across threads and async tasks.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::Client;
///
/// // Create client
/// let client = Client::builder()
///     .url("https://api.inferadb.com")
///     .credentials(config)
///     .build()
///     .await?;
///
/// // Clone for use across tasks
/// let client2 = client.clone();
/// tokio::spawn(async move {
///     let vault = client2.organization("org").vault("vlt");
///     // Use vault...
/// });
/// ```
#[derive(Clone)]
pub struct Client {
    inner: Arc<inner::ClientInner>,
}

impl Client {
    /// Creates a new client builder.
    ///
    /// This is the recommended way to create a client. The builder
    /// uses the typestate pattern to ensure required configuration
    /// is provided at compile time.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Client;
    ///
    /// let client = Client::builder()
    ///     .url("https://api.inferadb.com")
    ///     .credentials(credentials)
    ///     .build()
    ///     .await?;
    /// ```
    pub fn builder() -> ClientBuilder<builder::NoUrl, builder::NoCredentials> {
        ClientBuilder::new()
    }

    /// Returns an organization-scoped client.
    ///
    /// # Arguments
    ///
    /// * `organization_id` - The organization ID (e.g., "org_abc123")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let org = client.organization("org_abc123");
    /// let vault = org.vault("vlt_xyz789");
    /// ```
    pub fn organization(&self, organization_id: impl Into<String>) -> OrganizationClient {
        OrganizationClient {
            client: self.clone(),
            organization_id: organization_id.into(),
        }
    }

    /// Returns the base URL of the client.
    pub fn url(&self) -> &str {
        &self.inner.url
    }

    /// Creates a client from the inner implementation.
    pub(crate) fn from_inner(inner: inner::ClientInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Returns a reference to the inner client.
    pub(crate) fn inner(&self) -> &inner::ClientInner {
        &self.inner
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("url", &self.inner.url)
            .finish_non_exhaustive()
    }
}

/// An organization-scoped client.
///
/// Provides access to vaults and organization-level operations.
#[derive(Clone)]
pub struct OrganizationClient {
    client: Client,
    organization_id: String,
}

impl OrganizationClient {
    /// Returns a vault-scoped client.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - The vault ID (e.g., "vlt_xyz789")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let vault = org.vault("vlt_xyz789");
    /// let allowed = vault.check("user:alice", "view", "doc:1").await?;
    /// ```
    pub fn vault(&self, vault_id: impl Into<String>) -> VaultClient {
        VaultClient::new(
            self.client.clone(),
            self.organization_id.clone(),
            vault_id.into(),
        )
    }

    /// Returns the organization ID.
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }

    /// Returns the underlying client.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl std::fmt::Debug for OrganizationClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}
