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

// Allow dead code for client internals not yet integrated
#![allow(dead_code)]

mod builder;
mod health;
mod inner;

pub use builder::ClientBuilder;
pub use health::{
    ComponentHealth, HealthResponse, HealthStatus, ReadinessCriteria, ShutdownGuard, ShutdownHandle,
};

use std::sync::Arc;

use crate::control::{
    AccountClient, ApiClientsClient, JwksClient, OrganizationControlClient, OrganizationsClient,
};
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

    /// Returns the transport client, if available.
    #[cfg(feature = "rest")]
    pub(crate) fn transport(
        &self,
    ) -> Option<&std::sync::Arc<dyn crate::transport::TransportClient + Send + Sync>> {
        self.inner.transport.as_ref()
    }

    // Control plane methods

    /// Returns a client for managing the current user's account.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let account = client.account();
    /// let info = account.get().await?;
    /// ```
    pub fn account(&self) -> AccountClient {
        AccountClient::new(self.clone())
    }

    /// Returns a client for JWKS operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let jwks = client.jwks().get().await?;
    /// if let Some(key) = jwks.find_key("key_id") {
    ///     // Use key for verification
    /// }
    /// ```
    pub fn jwks(&self) -> JwksClient {
        JwksClient::new(self.clone())
    }

    /// Returns a client for listing and creating organizations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let orgs = client.organizations().list().await?;
    /// ```
    pub fn organizations(&self) -> OrganizationsClient {
        OrganizationsClient::new(self.clone())
    }

    // Health check methods

    /// Performs a simple health check.
    ///
    /// Returns `true` if the service is reachable and responding.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if client.health_check().await? {
    ///     println!("Service is healthy");
    /// }
    /// ```
    pub async fn health_check(&self) -> Result<bool, crate::Error> {
        // TODO: Implement actual health check via transport
        // For now, return true as a placeholder
        Ok(true)
    }

    /// Performs a detailed health check.
    ///
    /// Returns comprehensive health information including component
    /// status and latency measurements.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let health = client.health().await?;
    /// println!("Status: {:?}", health.status);
    /// println!("Latency: {:?}", health.latency);
    /// ```
    pub async fn health(&self) -> Result<HealthResponse, crate::Error> {
        use std::collections::HashMap;
        use std::time::Duration;

        // TODO: Implement actual health check via transport
        // For now, return a placeholder response
        Ok(HealthResponse {
            status: HealthStatus::Healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            latency: Duration::from_millis(1),
            components: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Waits for the service to become ready.
    ///
    /// This is useful during application startup to ensure the
    /// authorization service is available before accepting traffic.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for readiness
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// client.wait_ready(Duration::from_secs(30)).await?;
    /// println!("Service is ready");
    /// ```
    pub async fn wait_ready(&self, timeout: std::time::Duration) -> Result<(), crate::Error> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if self.health_check().await.unwrap_or(false) {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Err(crate::Error::timeout(
            "Timed out waiting for service readiness",
        ))
    }

    /// Waits for the service to become ready with custom criteria.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for readiness
    /// * `criteria` - Custom readiness criteria
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// client.wait_ready_with(Duration::from_secs(30), ReadinessCriteria::new()
    ///     .max_latency(Duration::from_millis(100))
    ///     .require_auth()
    /// ).await?;
    /// ```
    pub async fn wait_ready_with(
        &self,
        timeout: std::time::Duration,
        criteria: ReadinessCriteria,
    ) -> Result<(), crate::Error> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            let health = self.health().await?;

            let mut ready = health.is_healthy();

            if let Some(max_latency) = criteria.max_latency {
                ready = ready && health.latency <= max_latency;
            }

            if ready {
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Err(crate::Error::timeout(
            "Timed out waiting for service readiness",
        ))
    }

    /// Returns `true` if the client is in shutdown mode.
    ///
    /// When shutting down, new requests may be rejected.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if client.is_shutting_down() {
    ///     return Err(Error::shutting_down("Client is shutting down"));
    /// }
    /// ```
    pub fn is_shutting_down(&self) -> bool {
        // TODO: Implement actual shutdown state tracking
        false
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

    // Control plane methods

    /// Returns a control plane client for this organization.
    ///
    /// The control plane client provides administrative operations like
    /// managing vaults, members, teams, and API clients.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let control = org.control();
    /// let vaults = control.vaults().list().await?;
    /// ```
    pub fn control(&self) -> OrganizationControlClient {
        OrganizationControlClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for managing API clients in this organization.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let clients = org.clients();
    /// let list = clients.list().await?;
    /// ```
    pub fn clients(&self) -> ApiClientsClient {
        ApiClientsClient::new(self.client.clone(), self.organization_id.clone())
    }
}

impl std::fmt::Debug for OrganizationClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}
