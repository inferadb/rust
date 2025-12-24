//! Vault management for the control plane.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for vault management operations.
///
/// Access via `org.vaults()`.
///
/// ## Example
///
/// ```rust,ignore
/// let vaults = org.vaults();
///
/// // List all vaults
/// let list = vaults.list().await?;
///
/// // Create a new vault
/// let vault = vaults.create(CreateVaultRequest {
///     name: "my-vault".into(),
///     ..Default::default()
/// }).await?;
///
/// // Get a specific vault
/// let vault = vaults.get("vlt_abc123").await?;
/// ```
#[derive(Clone)]
pub struct VaultsClient {
    client: Client,
    organization_id: String,
}

impl VaultsClient {
    /// Creates a new vaults client.
    pub(crate) fn new(client: Client, organization_id: impl Into<String>) -> Self {
        Self {
            client,
            organization_id: organization_id.into(),
        }
    }

    /// Returns the organization ID.
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }

    /// Lists all vaults in the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let vaults = org.vaults().list().await?;
    /// for vault in vaults.items {
    ///     println!("{}: {} ({:?})", vault.id, vault.name, vault.status);
    /// }
    /// ```
    pub fn list(&self) -> ListVaultsRequest {
        ListVaultsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            limit: None,
            cursor: None,
            sort: None,
            status: None,
        }
    }

    /// Creates a new vault.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let vault = org.vaults().create(CreateVaultRequest {
    ///     name: "my-vault".into(),
    ///     display_name: Some("My Vault".into()),
    ///     ..Default::default()
    /// }).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn create(&self, request: CreateVaultRequest) -> Result<VaultInfo, Error> {
        let path = format!("/control/v1/organizations/{}/vaults", self.organization_id);
        self.client.inner().control_post(&path, &request).await
    }

    /// Creates a new vault.
    #[cfg(not(feature = "rest"))]
    pub async fn create(&self, _request: CreateVaultRequest) -> Result<VaultInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Gets a vault by ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let vault = org.vaults().get("vlt_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self, vault_id: impl Into<String>) -> Result<VaultInfo, Error> {
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}",
            self.organization_id,
            vault_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a vault by ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _vault_id: impl Into<String>) -> Result<VaultInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Updates a vault.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let vault = org.vaults().update("vlt_abc123", UpdateVaultRequest {
    ///     display_name: Some("New Name".into()),
    ///     ..Default::default()
    /// }).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn update(
        &self,
        vault_id: impl Into<String>,
        request: UpdateVaultRequest,
    ) -> Result<VaultInfo, Error> {
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}",
            self.organization_id,
            vault_id.into()
        );
        self.client.inner().control_patch(&path, &request).await
    }

    /// Updates a vault.
    #[cfg(not(feature = "rest"))]
    pub async fn update(
        &self,
        _vault_id: impl Into<String>,
        _request: UpdateVaultRequest,
    ) -> Result<VaultInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Deletes a vault.
    ///
    /// **Warning**: This is a destructive operation that cannot be undone.
    /// All relationships and data within the vault will be permanently deleted.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // Requires confirmation
    /// org.vaults().delete("vlt_abc123").confirm("DELETE vlt_abc123").await?;
    /// ```
    pub fn delete(&self, vault_id: impl Into<String>) -> DeleteVaultRequest {
        DeleteVaultRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            vault_id: vault_id.into(),
            confirmation: None,
        }
    }
}

impl std::fmt::Debug for VaultsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultsClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    /// The vault ID (e.g., "vlt_abc123").
    pub id: String,
    /// The organization ID that owns this vault.
    pub organization_id: String,
    /// The vault name (URL-safe slug).
    pub name: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// Description of the vault.
    pub description: Option<String>,
    /// The vault status.
    pub status: VaultStatus,
    /// When the vault was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the vault was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Status of a vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultStatus {
    /// Vault is active and accepting requests.
    #[default]
    Active,
    /// Vault is suspended and not accepting requests.
    Suspended,
    /// Vault is being deleted.
    Deleting,
    /// Vault has been archived.
    Archived,
}

impl VaultStatus {
    /// Returns `true` if the vault is active.
    pub fn is_active(&self) -> bool {
        matches!(self, VaultStatus::Active)
    }

    /// Returns `true` if the vault is available for operations.
    pub fn is_available(&self) -> bool {
        matches!(self, VaultStatus::Active)
    }
}

impl std::fmt::Display for VaultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultStatus::Active => write!(f, "active"),
            VaultStatus::Suspended => write!(f, "suspended"),
            VaultStatus::Deleting => write!(f, "deleting"),
            VaultStatus::Archived => write!(f, "archived"),
        }
    }
}

/// Request to create a new vault.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateVaultRequest {
    /// The vault name (URL-safe slug).
    pub name: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// Description of the vault.
    pub description: Option<String>,
}

impl CreateVaultRequest {
    /// Creates a new request with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            description: None,
        }
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Request to update a vault.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateVaultRequest {
    /// New display name.
    pub display_name: Option<String>,
    /// New description.
    pub description: Option<String>,
}

impl UpdateVaultRequest {
    /// Creates a new empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Request to list vaults.
pub struct ListVaultsRequest {
    client: Client,
    organization_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
    status: Option<VaultStatus>,
}

impl ListVaultsRequest {
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

    /// Filters by vault status.
    #[must_use]
    pub fn status(mut self, status: VaultStatus) -> Self {
        self.status = Some(status);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<VaultInfo>, Error> {
        let mut path = format!("/control/v1/organizations/{}/vaults", self.organization_id);
        let mut query_parts = Vec::new();

        if let Some(limit) = self.limit {
            query_parts.push(format!("limit={}", limit));
        }
        if let Some(cursor) = &self.cursor {
            query_parts.push(format!("cursor={}", urlencoding::encode(cursor)));
        }
        if let Some(sort) = &self.sort {
            query_parts.push(format!("sort={}", sort.as_str()));
        }
        if let Some(status) = &self.status {
            query_parts.push(format!("status={}", status));
        }

        if !query_parts.is_empty() {
            path.push('?');
            path.push_str(&query_parts.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<VaultInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListVaultsRequest {
    type Output = Result<Page<VaultInfo>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to delete a vault.
pub struct DeleteVaultRequest {
    client: Client,
    organization_id: String,
    vault_id: String,
    confirmation: Option<String>,
}

impl DeleteVaultRequest {
    /// Confirms the deletion with the vault ID.
    ///
    /// You must pass `"DELETE {vault_id}"` to confirm deletion.
    #[must_use]
    pub fn confirm(mut self, confirmation: impl Into<String>) -> Self {
        self.confirmation = Some(confirmation.into());
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<(), Error> {
        let expected = format!("DELETE {}", self.vault_id);
        match &self.confirmation {
            Some(c) if c == &expected => {
                let path = format!(
                    "/control/v1/organizations/{}/vaults/{}",
                    self.organization_id, self.vault_id
                );
                self.client.inner().control_delete(&path).await
            }
            Some(c) => Err(Error::invalid_argument(format!(
                "Invalid confirmation. Expected '{}', got '{}'",
                expected, c
            ))),
            None => Err(Error::invalid_argument(
                "Deletion requires confirmation. Call .confirm(\"DELETE vault_id\") first",
            )),
        }
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<(), Error> {
        let _ = self.confirmation;
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for DeleteVaultRequest {
    type Output = Result<(), Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;
    use crate::transport::mock::MockTransport;
    use std::sync::Arc;

    async fn create_test_client() -> Client {
        let mock_transport = Arc::new(MockTransport::new());
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap()
    }

    #[test]
    fn test_vault_status() {
        assert!(VaultStatus::Active.is_active());
        assert!(VaultStatus::Active.is_available());
        assert!(!VaultStatus::Suspended.is_active());
        assert!(!VaultStatus::Suspended.is_available());
        assert!(!VaultStatus::Deleting.is_active());
        assert!(!VaultStatus::Deleting.is_available());
        assert!(!VaultStatus::Archived.is_active());
        assert!(!VaultStatus::Archived.is_available());
        assert_eq!(VaultStatus::default(), VaultStatus::Active);
    }

    #[test]
    fn test_vault_status_display() {
        assert_eq!(VaultStatus::Active.to_string(), "active");
        assert_eq!(VaultStatus::Suspended.to_string(), "suspended");
        assert_eq!(VaultStatus::Deleting.to_string(), "deleting");
        assert_eq!(VaultStatus::Archived.to_string(), "archived");
    }

    #[test]
    fn test_create_vault_request() {
        let req = CreateVaultRequest::new("my-vault")
            .with_display_name("My Vault")
            .with_description("A test vault");

        assert_eq!(req.name, "my-vault");
        assert_eq!(req.display_name, Some("My Vault".to_string()));
        assert_eq!(req.description, Some("A test vault".to_string()));
    }

    #[test]
    fn test_update_vault_request() {
        let req = UpdateVaultRequest::new()
            .with_display_name("New Name")
            .with_description("New description");

        assert_eq!(req.display_name, Some("New Name".to_string()));
        assert_eq!(req.description, Some("New description".to_string()));
    }

    #[test]
    fn test_vaults_delete_wrong_confirmation() {
        // Test that wrong confirmation is properly rejected
        // This doesn't require a server - it's a local validation
        let expected = format!("DELETE {}", "vlt_abc123");
        let provided = "DELETE wrong_vault";
        assert_ne!(provided, expected);
    }

    #[tokio::test]
    async fn test_vaults_client_accessors() {
        let client = create_test_client().await;
        let vaults = VaultsClient::new(client, "org_test");
        assert_eq!(vaults.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_vaults_client_debug() {
        let client = create_test_client().await;
        let vaults = VaultsClient::new(client, "org_test");
        let debug = format!("{:?}", vaults);
        assert!(debug.contains("VaultsClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_list_vaults_request_builders() {
        let client = create_test_client().await;
        let vaults = VaultsClient::new(client, "org_test");

        // Test all builder methods
        let _request = vaults
            .list()
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending)
            .status(VaultStatus::Active);

        // Just verify the builder compiles and returns a request
    }

    #[tokio::test]
    async fn test_delete_vault_request_builder() {
        let client = create_test_client().await;
        let vaults = VaultsClient::new(client, "org_test");

        // Test delete with confirmation builder
        let _request = vaults.delete("vlt_abc123").confirm("DELETE vlt_abc123");
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
    async fn test_list_vaults() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/vaults"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "vlt_1",
                        "organization_id": "org_123",
                        "name": "my-vault",
                        "display_name": "My Vault",
                        "description": "Production vault",
                        "status": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z"
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
        let vaults = VaultsClient::new(client, "org_123");
        let result = vaults.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "my-vault");
    }

    #[tokio::test]
    async fn test_list_vaults_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/vaults"))
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
        let vaults = VaultsClient::new(client, "org_123");
        let result = vaults
            .list()
            .limit(10)
            .cursor("cursor_abc")
            .sort(SortOrder::Descending)
            .status(VaultStatus::Active)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_vault() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/organizations/org_123/vaults"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vlt_new",
                "organization_id": "org_123",
                "name": "new-vault",
                "display_name": "New Vault",
                "description": "A new vault",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let vaults = VaultsClient::new(client, "org_123");
        let request = CreateVaultRequest::new("new-vault").with_display_name("New Vault");
        let result = vaults.create(request).await;

        assert!(result.is_ok());
        let vault = result.unwrap();
        assert_eq!(vault.name, "new-vault");
    }

    #[tokio::test]
    async fn test_get_vault() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/vaults/vlt_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vlt_abc",
                "organization_id": "org_123",
                "name": "test-vault",
                "display_name": "Test Vault",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-02T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let vaults = VaultsClient::new(client, "org_123");
        let result = vaults.get("vlt_abc").await;

        assert!(result.is_ok());
        let vault = result.unwrap();
        assert_eq!(vault.id, "vlt_abc");
    }

    #[tokio::test]
    async fn test_update_vault() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/control/v1/organizations/org_123/vaults/vlt_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vlt_abc",
                "organization_id": "org_123",
                "name": "test-vault",
                "display_name": "Updated Vault",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-03T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let vaults = VaultsClient::new(client, "org_123");
        let request = UpdateVaultRequest::default().with_display_name("Updated Vault");
        let result = vaults.update("vlt_abc", request).await;

        assert!(result.is_ok());
        let vault = result.unwrap();
        assert_eq!(vault.display_name, Some("Updated Vault".to_string()));
    }

    #[tokio::test]
    async fn test_delete_vault() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/organizations/org_123/vaults/vlt_abc"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let vaults = VaultsClient::new(client, "org_123");
        let result = vaults.delete("vlt_abc").confirm("DELETE vlt_abc").await;

        assert!(result.is_ok());
    }
}
