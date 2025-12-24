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
#[cfg(not(feature = "rest"))]
use std::time::Duration;

use crate::control::{
    AccountClient, ApiClientsClient, AuditLogsClient, InvitationsClient, JwksClient, MembersClient,
    OrganizationControlClient, OrganizationsClient, TeamsClient, VaultsClient,
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
        #[cfg(feature = "rest")]
        {
            // Use the liveness probe endpoint
            return match self.inner.control_get::<serde_json::Value>("/livez").await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            };
        }
        #[cfg(not(feature = "rest"))]
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

        #[cfg(feature = "rest")]
        {
            let start = std::time::Instant::now();

            // Try to get detailed health from /healthz
            #[derive(serde::Deserialize)]
            struct ServerHealth {
                status: Option<String>,
                version: Option<String>,
                #[serde(default)]
                components: HashMap<String, serde_json::Value>,
            }

            match self.inner.control_get::<ServerHealth>("/healthz").await {
                Ok(server_health) => {
                    let latency = start.elapsed();
                    let status = match server_health.status.as_deref() {
                        Some("healthy") | Some("ok") => HealthStatus::Healthy,
                        Some("degraded") => HealthStatus::Degraded,
                        _ => HealthStatus::Unhealthy,
                    };

                    let components = server_health
                        .components
                        .into_iter()
                        .map(|(name, value)| {
                            let component_status = value
                                .get("status")
                                .and_then(|s| s.as_str())
                                .map(|s| match s {
                                    "healthy" | "ok" => HealthStatus::Healthy,
                                    "degraded" => HealthStatus::Degraded,
                                    _ => HealthStatus::Unhealthy,
                                })
                                .unwrap_or(HealthStatus::Healthy);
                            (
                                name,
                                ComponentHealth {
                                    status: component_status,
                                    message: None,
                                    latency: None,
                                    last_check: chrono::Utc::now(),
                                },
                            )
                        })
                        .collect();

                    Ok(HealthResponse {
                        status,
                        version: server_health
                            .version
                            .unwrap_or_else(|| "unknown".to_string()),
                        latency,
                        components,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Err(_) => {
                    // Fall back to simple health check
                    Ok(HealthResponse {
                        status: HealthStatus::Unhealthy,
                        version: "unknown".to_string(),
                        latency: start.elapsed(),
                        components: HashMap::new(),
                        timestamp: chrono::Utc::now(),
                    })
                }
            }
        }

        #[cfg(not(feature = "rest"))]
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
        self.inner
            .shutdown_guard
            .as_ref()
            .is_some_and(|guard| guard.is_shutting_down())
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

    /// Returns a client for vault management.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let vaults = org.vaults();
    /// let list = vaults.list().await?;
    /// ```
    pub fn vaults(&self) -> VaultsClient {
        VaultsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for member management.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let members = org.members();
    /// members.invite(InviteMemberRequest::new("alice@example.com", OrgRole::Admin)).await?;
    /// ```
    pub fn members(&self) -> MembersClient {
        MembersClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for team management.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let teams = org.teams();
    /// teams.create(CreateTeamRequest::new("Engineering")).await?;
    /// ```
    pub fn teams(&self) -> TeamsClient {
        TeamsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for invitation management.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let invitations = org.invitations();
    /// let pending = invitations.list().await?;
    /// ```
    pub fn invitations(&self) -> InvitationsClient {
        InvitationsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for audit log queries.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let audit = org.audit();
    /// let events = audit.list().await?;
    /// ```
    pub fn audit(&self) -> AuditLogsClient {
        AuditLogsClient::new(self.client.clone(), self.organization_id.clone())
    }
}

impl std::fmt::Debug for OrganizationClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;

    async fn create_test_client() -> Client {
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_client_url() {
        let client = create_test_client().await;
        assert_eq!(client.url(), "https://api.example.com");
    }

    #[tokio::test]
    async fn test_client_debug() {
        let client = create_test_client().await;
        let debug = format!("{:?}", client);
        assert!(debug.contains("Client"));
        assert!(debug.contains("api.example.com"));
    }

    #[tokio::test]
    async fn test_client_clone() {
        let client = create_test_client().await;
        let cloned = client.clone();
        assert_eq!(client.url(), cloned.url());
    }

    #[tokio::test]
    async fn test_client_organization() {
        let client = create_test_client().await;
        let org = client.organization("org_test123");
        assert_eq!(org.organization_id(), "org_test123");
    }

    #[tokio::test]
    async fn test_organization_client_vault() {
        let client = create_test_client().await;
        let org = client.organization("org_test");
        let vault = org.vault("vlt_test");
        assert_eq!(vault.organization_id(), "org_test");
        assert_eq!(vault.vault_id(), "vlt_test");
    }

    #[tokio::test]
    async fn test_organization_client_client() {
        let client = create_test_client().await;
        let org = client.organization("org_test");
        let inner_client = org.client();
        assert_eq!(inner_client.url(), "https://api.example.com");
    }

    #[tokio::test]
    async fn test_organization_client_debug() {
        let client = create_test_client().await;
        let org = client.organization("org_test");
        let debug = format!("{:?}", org);
        assert!(debug.contains("OrganizationClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_organization_client_control() {
        let client = create_test_client().await;
        let org = client.organization("org_test");
        let control = org.control();
        // Verify control client can be created
        let debug = format!("{:?}", control);
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_organization_client_clients() {
        let client = create_test_client().await;
        let org = client.organization("org_test");
        let clients = org.clients();
        // Verify API clients client can be created
        let debug = format!("{:?}", clients);
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_client_account() {
        let client = create_test_client().await;
        let account = client.account();
        let debug = format!("{:?}", account);
        assert!(debug.contains("AccountClient"));
    }

    #[tokio::test]
    async fn test_client_jwks() {
        let client = create_test_client().await;
        let jwks = client.jwks();
        let debug = format!("{:?}", jwks);
        assert!(debug.contains("JwksClient"));
    }

    #[tokio::test]
    async fn test_client_organizations() {
        let client = create_test_client().await;
        let orgs = client.organizations();
        let debug = format!("{:?}", orgs);
        assert!(debug.contains("OrganizationsClient"));
    }

    #[tokio::test]
    async fn test_client_is_not_shutting_down() {
        let client = create_test_client().await;
        // Without a shutdown guard configured, should always return false
        assert!(!client.is_shutting_down());
    }

    #[tokio::test]
    async fn test_readiness_criteria_default() {
        let criteria = ReadinessCriteria::new();
        assert!(criteria.max_latency.is_none());
        assert!(!criteria.require_auth);
        assert!(!criteria.require_vault);
    }
}

#[cfg(all(test, feature = "rest"))]
mod wiremock_tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn create_mock_client(server: &MockServer) -> Client {
        Client::builder()
            .url(server.uri())
            .insecure()
            .credentials(BearerCredentialsConfig::new("test_token"))
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/livez"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/livez"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.health_check().await;
        // health_check returns Ok(false) on failure, not Err
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_health_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "healthy",
                "version": "1.0.0",
                "components": {
                    "database": {"status": "healthy"},
                    "cache": {"status": "degraded"}
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.health().await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.version, "1.0.0");
        assert_eq!(health.components.len(), 2);
    }

    #[tokio::test]
    async fn test_health_degraded() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "degraded",
                "version": "1.0.0"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.health().await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_health_failure() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.health().await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_wait_ready_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/livez"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.wait_ready(std::time::Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }
}
