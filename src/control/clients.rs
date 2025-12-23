//! API client management for the control plane.
//!
//! Provides operations for managing API clients (service accounts)
//! and their certificates for programmatic access.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for managing API clients within an organization.
///
/// Access via `client.organization("org_id").clients()`.
///
/// ## Example
///
/// ```rust,ignore
/// let clients = client.organization("org_123").clients();
///
/// // List all API clients
/// let list = clients.list().await?;
///
/// // Create a new API client
/// let api_client = clients.create(CreateApiClientRequest::new("my-service")).await?;
/// ```
#[derive(Clone)]
pub struct ApiClientsClient {
    client: Client,
    organization_id: String,
}

impl ApiClientsClient {
    /// Creates a new API clients client.
    pub(crate) fn new(client: Client, organization_id: impl Into<String>) -> Self {
        Self {
            client,
            organization_id: organization_id.into(),
        }
    }

    /// Lists all API clients in the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let clients = org.clients().list().await?;
    /// for client in clients.items {
    ///     println!("{}: {} ({:?})", client.id, client.name, client.status);
    /// }
    /// ```
    pub fn list(&self) -> ListApiClientsRequest {
        ListApiClientsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            limit: None,
            cursor: None,
            sort: None,
            status: None,
        }
    }

    /// Gets an API client by ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let api_client = org.clients().get("cli_abc123").await?;
    /// println!("Client: {} ({:?})", api_client.name, api_client.status);
    /// ```
    pub async fn get(&self, client_id: impl Into<String>) -> Result<ApiClient, Error> {
        let client_id = client_id.into();
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}",
                self.organization_id, client_id
            );
            return self.client.inner().control_get(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = client_id;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Creates a new API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let api_client = org.clients()
    ///     .create(CreateApiClientRequest::new("my-service")
    ///         .with_description("Backend service client")
    ///         .with_permissions(vec!["read:vaults", "write:relationships"]))
    ///     .await?;
    /// ```
    pub async fn create(&self, request: CreateApiClientRequest) -> Result<ApiClient, Error> {
        #[cfg(feature = "rest")]
        {
            let path = format!("/control/v1/organizations/{}/clients", self.organization_id);
            return self.client.inner().control_post(&path, &request).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = request;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Updates an API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let updated = org.clients()
    ///     .update("cli_abc123", UpdateApiClientRequest::new()
    ///         .with_description("Updated description"))
    ///     .await?;
    /// ```
    pub async fn update(
        &self,
        client_id: impl Into<String>,
        request: UpdateApiClientRequest,
    ) -> Result<ApiClient, Error> {
        let client_id = client_id.into();
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}",
                self.organization_id, client_id
            );
            return self.client.inner().control_patch(&path, &request).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = (client_id, request);
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Deletes an API client.
    ///
    /// This permanently revokes the client's access.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.clients().delete("cli_abc123").await?;
    /// ```
    pub async fn delete(&self, client_id: impl Into<String>) -> Result<(), Error> {
        let client_id = client_id.into();
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}",
                self.organization_id, client_id
            );
            return self.client.inner().control_delete(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = client_id;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Returns a client for managing certificates for a specific API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let certs = org.clients().certificates("cli_abc123");
    /// let list = certs.list().await?;
    /// ```
    pub fn certificates(&self, client_id: impl Into<String>) -> CertificatesClient {
        CertificatesClient {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            client_id: client_id.into(),
        }
    }

    /// Suspends an API client.
    ///
    /// The client will no longer be able to authenticate.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.clients().suspend("cli_abc123").await?;
    /// ```
    pub async fn suspend(&self, client_id: impl Into<String>) -> Result<ApiClient, Error> {
        let client_id = client_id.into();
        self.update(
            &client_id,
            UpdateApiClientRequest::new().with_status(ClientStatus::Suspended),
        )
        .await
    }

    /// Reactivates a suspended API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.clients().reactivate("cli_abc123").await?;
    /// ```
    pub async fn reactivate(&self, client_id: impl Into<String>) -> Result<ApiClient, Error> {
        let client_id = client_id.into();
        self.update(
            &client_id,
            UpdateApiClientRequest::new().with_status(ClientStatus::Active),
        )
        .await
    }
}

impl std::fmt::Debug for ApiClientsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiClientsClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Information about an API client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiClient {
    /// The client ID (e.g., "cli_abc123").
    pub id: String,
    /// The client name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// The client status.
    pub status: ClientStatus,
    /// When the client was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the client was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Permissions granted to this client.
    pub permissions: Vec<String>,
    /// Rate limit in requests per second (if set).
    pub rate_limit: Option<u32>,
}

/// API client status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientStatus {
    /// Client is active and can authenticate.
    Active,
    /// Client has been suspended and cannot authenticate.
    Suspended,
    /// Client has been permanently revoked.
    Revoked,
}

impl ClientStatus {
    /// Returns `true` if the client is active.
    pub fn is_active(&self) -> bool {
        matches!(self, ClientStatus::Active)
    }

    /// Returns `true` if the client is suspended.
    pub fn is_suspended(&self) -> bool {
        matches!(self, ClientStatus::Suspended)
    }

    /// Returns `true` if the client is revoked.
    pub fn is_revoked(&self) -> bool {
        matches!(self, ClientStatus::Revoked)
    }
}

impl std::fmt::Display for ClientStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientStatus::Active => write!(f, "active"),
            ClientStatus::Suspended => write!(f, "suspended"),
            ClientStatus::Revoked => write!(f, "revoked"),
        }
    }
}

/// Request to create a new API client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateApiClientRequest {
    /// The client name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Permissions to grant.
    pub permissions: Option<Vec<String>>,
    /// Rate limit in requests per second.
    pub rate_limit: Option<u32>,
    /// Initial certificate (PEM-encoded public key).
    pub certificate: Option<String>,
}

impl CreateApiClientRequest {
    /// Creates a new request with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the permissions.
    #[must_use]
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Sets the rate limit.
    #[must_use]
    pub fn with_rate_limit(mut self, rate_limit: u32) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Sets the initial certificate.
    #[must_use]
    pub fn with_certificate(mut self, certificate: impl Into<String>) -> Self {
        self.certificate = Some(certificate.into());
        self
    }
}

/// Request to update an API client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateApiClientRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// New permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    /// New rate limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<u32>,
    /// New status (for suspend/reactivate operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ClientStatus>,
}

impl UpdateApiClientRequest {
    /// Creates a new empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the permissions.
    #[must_use]
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Sets the rate limit.
    #[must_use]
    pub fn with_rate_limit(mut self, rate_limit: u32) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Sets the status.
    #[must_use]
    pub fn with_status(mut self, status: ClientStatus) -> Self {
        self.status = Some(status);
        self
    }
}

/// Request to list API clients.
pub struct ListApiClientsRequest {
    client: Client,
    organization_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
    status: Option<ClientStatus>,
}

impl ListApiClientsRequest {
    /// Sets the maximum number of results to return.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the pagination cursor.
    #[must_use]
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Sets the sort order.
    #[must_use]
    pub fn sort(mut self, order: SortOrder) -> Self {
        self.sort = Some(order);
        self
    }

    /// Filters by status.
    #[must_use]
    pub fn status(mut self, status: ClientStatus) -> Self {
        self.status = Some(status);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<ApiClient>, Error> {
        let mut path = format!("/control/v1/organizations/{}/clients", self.organization_id);

        let mut query_params = Vec::new();
        if let Some(limit) = self.limit {
            query_params.push(format!("limit={}", limit));
        }
        if let Some(ref cursor) = self.cursor {
            query_params.push(format!("cursor={}", cursor));
        }
        if let Some(ref sort) = self.sort {
            query_params.push(format!("sort={}", sort.as_str()));
        }
        if let Some(ref status) = self.status {
            let status_str = match status {
                ClientStatus::Active => "active",
                ClientStatus::Suspended => "suspended",
                ClientStatus::Revoked => "revoked",
            };
            query_params.push(format!("status={}", status_str));
        }
        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<ApiClient>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListApiClientsRequest {
    type Output = Result<Page<ApiClient>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// A certificate associated with an API client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCertificate {
    /// The certificate ID.
    pub id: String,
    /// The certificate fingerprint (SHA-256).
    pub fingerprint: String,
    /// The algorithm (e.g., "Ed25519", "RSA").
    pub algorithm: String,
    /// When the certificate was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the certificate expires (if applicable).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this certificate is active.
    pub active: bool,
}

/// Client for managing certificates for an API client.
#[derive(Clone)]
pub struct CertificatesClient {
    client: Client,
    organization_id: String,
    client_id: String,
}

impl CertificatesClient {
    /// Lists all certificates for the API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let certs = org.clients().certificates("cli_abc123").list().await?;
    /// for cert in certs.items {
    ///     println!("{}: {} ({})", cert.id, cert.fingerprint, cert.algorithm);
    /// }
    /// ```
    pub async fn list(&self) -> Result<Page<ClientCertificate>, Error> {
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}/certificates",
                self.organization_id, self.client_id
            );
            return self.client.inner().control_get(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        Err(Error::configuration("REST feature is required"))
    }

    /// Adds a new certificate to the API client.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let cert = org.clients()
    ///     .certificates("cli_abc123")
    ///     .add(AddCertificateRequest::new(public_key_pem))
    ///     .await?;
    /// ```
    pub async fn add(&self, request: AddCertificateRequest) -> Result<ClientCertificate, Error> {
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}/certificates",
                self.organization_id, self.client_id
            );
            return self.client.inner().control_post(&path, &request).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = request;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Rotates the certificate (adds new, schedules old for removal).
    ///
    /// This is the preferred way to update certificates as it allows
    /// for a grace period where both old and new certificates are valid.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let cert = org.clients()
    ///     .certificates("cli_abc123")
    ///     .rotate(RotateCertificateRequest::new(new_public_key_pem)
    ///         .with_grace_period(Duration::from_secs(3600)))
    ///     .await?;
    /// ```
    pub async fn rotate(
        &self,
        request: RotateCertificateRequest,
    ) -> Result<ClientCertificate, Error> {
        // Rotation is implemented as adding a new certificate.
        // The server handles the grace period for the old certificate.
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}/certificates",
                self.organization_id, self.client_id
            );
            return self.client.inner().control_post(&path, &request).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = request;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Revokes a certificate.
    ///
    /// The certificate will no longer be valid for authentication.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.clients()
    ///     .certificates("cli_abc123")
    ///     .revoke("crt_xyz789")
    ///     .await?;
    /// ```
    pub async fn revoke(&self, certificate_id: impl Into<String>) -> Result<(), Error> {
        let certificate_id = certificate_id.into();
        #[cfg(feature = "rest")]
        {
            let path = format!(
                "/control/v1/organizations/{}/clients/{}/certificates/{}",
                self.organization_id, self.client_id, certificate_id
            );
            return self.client.inner().control_delete(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = certificate_id;
            Err(Error::configuration("REST feature is required"))
        }
    }
}

impl std::fmt::Debug for CertificatesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertificatesClient")
            .field("organization_id", &self.organization_id)
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

/// Request to add a certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCertificateRequest {
    /// The PEM-encoded public key.
    pub public_key: String,
    /// The algorithm (auto-detected if not specified).
    pub algorithm: Option<String>,
    /// When the certificate expires.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl AddCertificateRequest {
    /// Creates a new request with the given public key.
    pub fn new(public_key: impl Into<String>) -> Self {
        Self {
            public_key: public_key.into(),
            algorithm: None,
            expires_at: None,
        }
    }

    /// Sets the algorithm.
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: impl Into<String>) -> Self {
        self.algorithm = Some(algorithm.into());
        self
    }

    /// Sets the expiration time.
    #[must_use]
    pub fn with_expires_at(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

/// Request to rotate a certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateCertificateRequest {
    /// The new PEM-encoded public key.
    pub public_key: String,
    /// The algorithm (auto-detected if not specified).
    pub algorithm: Option<String>,
    /// Grace period in seconds during which both certificates are valid.
    pub grace_period_secs: Option<u64>,
}

impl RotateCertificateRequest {
    /// Creates a new request with the given public key.
    pub fn new(public_key: impl Into<String>) -> Self {
        Self {
            public_key: public_key.into(),
            algorithm: None,
            grace_period_secs: None,
        }
    }

    /// Sets the algorithm.
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: impl Into<String>) -> Self {
        self.algorithm = Some(algorithm.into());
        self
    }

    /// Sets the grace period.
    #[must_use]
    pub fn with_grace_period(mut self, grace_period: std::time::Duration) -> Self {
        self.grace_period_secs = Some(grace_period.as_secs());
        self
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

    #[test]
    fn test_client_status() {
        assert!(ClientStatus::Active.is_active());
        assert!(!ClientStatus::Active.is_suspended());
        assert!(!ClientStatus::Active.is_revoked());

        assert!(!ClientStatus::Suspended.is_active());
        assert!(ClientStatus::Suspended.is_suspended());

        assert!(ClientStatus::Revoked.is_revoked());
    }

    #[test]
    fn test_client_status_display() {
        assert_eq!(ClientStatus::Active.to_string(), "active");
        assert_eq!(ClientStatus::Suspended.to_string(), "suspended");
        assert_eq!(ClientStatus::Revoked.to_string(), "revoked");
    }

    #[test]
    fn test_create_api_client_request() {
        let req = CreateApiClientRequest::new("my-service")
            .with_description("Test service")
            .with_permissions(vec!["read:vaults".to_string()])
            .with_rate_limit(100)
            .with_certificate("PEM_DATA");

        assert_eq!(req.name, "my-service");
        assert_eq!(req.description, Some("Test service".to_string()));
        assert_eq!(req.permissions, Some(vec!["read:vaults".to_string()]));
        assert_eq!(req.rate_limit, Some(100));
        assert_eq!(req.certificate, Some("PEM_DATA".to_string()));
    }

    #[test]
    fn test_update_api_client_request() {
        let req = UpdateApiClientRequest::new()
            .with_name("new-name")
            .with_description("New description")
            .with_permissions(vec!["write:vaults".to_string()])
            .with_rate_limit(200);

        assert_eq!(req.name, Some("new-name".to_string()));
        assert_eq!(req.description, Some("New description".to_string()));
        assert_eq!(req.permissions, Some(vec!["write:vaults".to_string()]));
        assert_eq!(req.rate_limit, Some(200));
    }

    #[test]
    fn test_add_certificate_request() {
        let req = AddCertificateRequest::new("PEM_DATA")
            .with_algorithm("Ed25519")
            .with_expires_at(chrono::Utc::now());

        assert_eq!(req.public_key, "PEM_DATA");
        assert_eq!(req.algorithm, Some("Ed25519".to_string()));
        assert!(req.expires_at.is_some());
    }

    #[test]
    fn test_rotate_certificate_request() {
        let req = RotateCertificateRequest::new("PEM_DATA")
            .with_algorithm("Ed25519")
            .with_grace_period(std::time::Duration::from_secs(3600));

        assert_eq!(req.public_key, "PEM_DATA");
        assert_eq!(req.algorithm, Some("Ed25519".to_string()));
        assert_eq!(req.grace_period_secs, Some(3600));
    }

    #[tokio::test]
    async fn test_debug_impls() {
        let client = create_test_client().await;
        let clients = ApiClientsClient::new(client, "org_test");

        assert!(format!("{:?}", clients).contains("ApiClientsClient"));
        assert!(format!("{:?}", clients.certificates("cli_abc123")).contains("CertificatesClient"));
    }

    #[tokio::test]
    async fn test_list_api_clients_request_builders() {
        let client = create_test_client().await;
        let clients = ApiClientsClient::new(client, "org_test");

        // Test all builder methods
        let _request = clients
            .list()
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending)
            .status(ClientStatus::Active);

        // Just verify the builder compiles and returns a request
    }

    #[test]
    fn test_update_api_client_request_with_status() {
        let req = UpdateApiClientRequest::new().with_status(ClientStatus::Suspended);
        assert_eq!(req.status, Some(ClientStatus::Suspended));
    }

    // Additional tests for Clone implementations and serde
    #[tokio::test]
    async fn test_api_clients_client_clone() {
        let client = create_test_client().await;
        let clients = ApiClientsClient::new(client, "org_test");
        let cloned = clients.clone();
        assert!(format!("{:?}", cloned).contains("ApiClientsClient"));
    }

    #[tokio::test]
    async fn test_certificates_client_clone() {
        let client = create_test_client().await;
        let clients = ApiClientsClient::new(client, "org_test");
        let certs = clients.certificates("cli_abc123");
        let cloned = certs.clone();
        assert!(format!("{:?}", cloned).contains("CertificatesClient"));
    }

    #[test]
    fn test_api_client_serde() {
        let json = r#"{
            "id": "cli_abc123",
            "name": "my-service",
            "description": "Test service",
            "status": "active",
            "permissions": ["read:vaults"],
            "rate_limit": 100,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        let api_client: ApiClient = serde_json::from_str(json).unwrap();
        assert_eq!(api_client.id, "cli_abc123");
        assert_eq!(api_client.name, "my-service");
        assert_eq!(api_client.status, ClientStatus::Active);
        assert_eq!(api_client.rate_limit, Some(100));
    }

    #[test]
    fn test_api_client_clone() {
        let api_client = ApiClient {
            id: "cli_123".to_string(),
            name: "test".to_string(),
            description: None,
            status: ClientStatus::Revoked,
            permissions: vec![],
            rate_limit: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let cloned = api_client.clone();
        assert_eq!(cloned.id, "cli_123");
        assert_eq!(cloned.status, ClientStatus::Revoked);
    }

    #[test]
    fn test_client_certificate_serde() {
        let json = r#"{
            "id": "cert_abc123",
            "fingerprint": "SHA256:abc123",
            "algorithm": "Ed25519",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2025-01-01T00:00:00Z",
            "active": true
        }"#;
        let cert: ClientCertificate = serde_json::from_str(json).unwrap();
        assert_eq!(cert.id, "cert_abc123");
        assert_eq!(cert.algorithm, "Ed25519");
        assert!(cert.active);
    }

    #[test]
    fn test_client_certificate_clone() {
        let cert = ClientCertificate {
            id: "cert_123".to_string(),
            fingerprint: "SHA256:fp".to_string(),
            algorithm: "Ed25519".to_string(),
            created_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now()),
            active: false,
        };
        let cloned = cert.clone();
        assert_eq!(cloned.id, "cert_123");
        assert!(!cloned.active);
    }

    #[test]
    fn test_client_status_serde() {
        let statuses = vec![
            (ClientStatus::Active, "\"active\""),
            (ClientStatus::Suspended, "\"suspended\""),
            (ClientStatus::Revoked, "\"revoked\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
            let parsed: ClientStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_create_api_client_request_clone() {
        let req = CreateApiClientRequest::new("test");
        let cloned = req.clone();
        assert_eq!(cloned.name, "test");
    }

    #[test]
    fn test_update_api_client_request_clone() {
        let req = UpdateApiClientRequest::new().with_name("new");
        let cloned = req.clone();
        assert_eq!(cloned.name, Some("new".to_string()));
    }

    #[test]
    fn test_add_certificate_request_clone() {
        let req = AddCertificateRequest::new("key");
        let cloned = req.clone();
        assert_eq!(cloned.public_key, "key");
    }

    #[test]
    fn test_rotate_certificate_request_clone() {
        let req = RotateCertificateRequest::new("key");
        let cloned = req.clone();
        assert_eq!(cloned.public_key, "key");
    }
}

#[cfg(all(test, feature = "rest"))]
mod wiremock_tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;
    use crate::Client;
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
    async fn test_list_api_clients() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "cli_1",
                        "name": "my-client",
                        "description": "Test client",
                        "status": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "permissions": ["read:vaults"],
                        "rate_limit": 100
                    }
                ],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 1
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let result = clients.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "my-client");
    }

    #[tokio::test]
    async fn test_list_api_clients_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 0
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let result = clients
            .list()
            .limit(10)
            .cursor("cursor_abc")
            .sort(SortOrder::Descending)
            .status(ClientStatus::Active)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_api_client() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/clients/cli_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cli_abc",
                "name": "test-client",
                "description": "Test client",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-02T00:00:00Z",
                "permissions": ["read:vaults", "write:relationships"],
                "rate_limit": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let result = clients.get("cli_abc").await;

        assert!(result.is_ok());
        let api_client = result.unwrap();
        assert_eq!(api_client.id, "cli_abc");
        assert_eq!(api_client.status, ClientStatus::Active);
    }

    #[tokio::test]
    async fn test_create_api_client() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/organizations/org_123/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cli_new",
                "name": "new-client",
                "description": "New client",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "permissions": [],
                "rate_limit": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let request = CreateApiClientRequest::new("new-client").with_description("New client");
        let result = clients.create(request).await;

        assert!(result.is_ok());
        let api_client = result.unwrap();
        assert_eq!(api_client.name, "new-client");
    }

    #[tokio::test]
    async fn test_update_api_client() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/control/v1/organizations/org_123/clients/cli_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cli_abc",
                "name": "updated-client",
                "description": "Updated",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-03T00:00:00Z",
                "permissions": [],
                "rate_limit": 200
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let request = UpdateApiClientRequest::new().with_name("updated-client");
        let result = clients.update("cli_abc", request).await;

        assert!(result.is_ok());
        let api_client = result.unwrap();
        assert_eq!(api_client.name, "updated-client");
    }

    #[tokio::test]
    async fn test_delete_api_client() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/organizations/org_123/clients/cli_abc"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let result = clients.delete("cli_abc").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_certificates() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/clients/cli_abc/certificates",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "cert_1",
                        "fingerprint": "sha256:abc123",
                        "algorithm": "Ed25519",
                        "active": true,
                        "created_at": "2024-01-01T00:00:00Z",
                        "expires_at": "2025-01-01T00:00:00Z"
                    }
                ],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 1
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let certs = clients.certificates("cli_abc");
        let result = certs.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].active);
    }

    #[tokio::test]
    async fn test_add_certificate() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/control/v1/organizations/org_123/clients/cli_abc/certificates",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cert_new",
                "fingerprint": "sha256:newkey",
                "algorithm": "Ed25519",
                "active": true,
                "created_at": "2024-01-01T00:00:00Z",
                "expires_at": "2025-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let certs = clients.certificates("cli_abc");
        let request = AddCertificateRequest::new("public_key_pem_here");
        let result = certs.add(request).await;

        assert!(result.is_ok());
        let cert = result.unwrap();
        assert!(cert.active);
    }

    #[tokio::test]
    async fn test_revoke_certificate() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path(
                "/control/v1/organizations/org_123/clients/cli_abc/certificates/cert_123",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let certs = clients.certificates("cli_abc");
        let result = certs.revoke("cert_123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rotate_certificate() {
        let server = MockServer::start().await;

        // rotate() posts to /certificates (adds new cert, server handles grace period)
        Mock::given(method("POST"))
            .and(path(
                "/control/v1/organizations/org_123/clients/cli_abc/certificates",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cert_rotated",
                "fingerprint": "sha256:rotated",
                "algorithm": "Ed25519",
                "active": true,
                "created_at": "2024-01-02T00:00:00Z",
                "expires_at": "2025-01-02T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let clients = ApiClientsClient::new(client, "org_123");
        let certs = clients.certificates("cli_abc");
        let request = RotateCertificateRequest::new("new_public_key_pem");
        let result = certs.rotate(request).await;

        assert!(result.is_ok());
        let cert = result.unwrap();
        assert_eq!(cert.id, "cert_rotated");
    }
}
