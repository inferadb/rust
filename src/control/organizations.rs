//! Organization management for the control plane.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::control::audit::AuditLogsClient;
use crate::control::members::{InvitationsClient, MembersClient};
use crate::control::teams::TeamsClient;
use crate::control::vaults::VaultsClient;
use crate::Error;

/// Client for organization-level control plane operations.
///
/// Access via `client.organization("org_id")`.
///
/// ## Example
///
/// ```rust,ignore
/// let org = client.organization("org_abc123");
///
/// // Access vault management
/// let vaults = org.vaults().list().await?;
///
/// // Get organization details
/// let info = org.get().await?;
/// ```
#[derive(Clone)]
pub struct OrganizationControlClient {
    client: Client,
    organization_id: String,
}

impl OrganizationControlClient {
    /// Creates a new organization control client.
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

    /// Returns a client for vault management.
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
    ///     name: "My Vault".into(),
    ///     ..Default::default()
    /// }).await?;
    /// ```
    pub fn vaults(&self) -> VaultsClient {
        VaultsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for member management.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let members = org.members();
    ///
    /// // List all members
    /// let list = members.list().await?;
    ///
    /// // Invite a new member
    /// members.invite(InviteMemberRequest::new("alice@example.com", OrgRole::Member)).await?;
    /// ```
    pub fn members(&self) -> MembersClient {
        MembersClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for team management.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let teams = org.teams();
    ///
    /// // List all teams
    /// let list = teams.list().await?;
    ///
    /// // Create a new team
    /// let team = teams.create(CreateTeamRequest::new("Engineering")).await?;
    /// ```
    pub fn teams(&self) -> TeamsClient {
        TeamsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for invitation management.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let invitations = org.invitations();
    ///
    /// // List pending invitations
    /// let pending = invitations.list().await?;
    ///
    /// // Resend an invitation
    /// invitations.resend("inv_abc123").await?;
    /// ```
    pub fn invitations(&self) -> InvitationsClient {
        InvitationsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Returns a client for audit log queries.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let logs = org.audit_logs();
    ///
    /// // List recent events
    /// let events = logs.list().await?;
    ///
    /// // Filter by actor
    /// let user_events = logs.list().actor("user_abc123").await?;
    /// ```
    pub fn audit_logs(&self) -> AuditLogsClient {
        AuditLogsClient::new(self.client.clone(), self.organization_id.clone())
    }

    /// Gets the organization details.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let info = org.get().await?;
    /// println!("Organization: {}", info.name);
    /// ```
    pub async fn get(&self) -> Result<OrganizationInfo, Error> {
        // TODO: Implement actual API call
        Ok(OrganizationInfo {
            id: self.organization_id.clone(),
            name: "Organization".to_string(),
            display_name: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Updates the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let updated = org.update(UpdateOrganizationRequest {
    ///     display_name: Some("New Display Name".into()),
    ///     ..Default::default()
    /// }).await?;
    /// ```
    pub async fn update(&self, request: UpdateOrganizationRequest) -> Result<OrganizationInfo, Error> {
        // TODO: Implement actual API call
        let _ = request;
        self.get().await
    }

    /// Deletes the organization.
    ///
    /// **Warning**: This is a destructive operation that cannot be undone.
    /// All vaults and data within the organization will be permanently deleted.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // Requires confirmation
    /// org.delete().confirm("DELETE org_abc123").await?;
    /// ```
    pub fn delete(&self) -> DeleteOrganizationRequest {
        DeleteOrganizationRequest {
            client: self.clone(),
            confirmation: None,
        }
    }
}

impl std::fmt::Debug for OrganizationControlClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationControlClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Client for listing and creating organizations.
///
/// Access via `client.organizations()`.
#[derive(Clone)]
pub struct OrganizationsClient {
    client: Client,
}

impl OrganizationsClient {
    /// Creates a new organizations client.
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Lists all organizations the current user has access to.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let orgs = client.organizations().list().await?;
    /// for org in orgs.items {
    ///     println!("{}: {}", org.id, org.name);
    /// }
    /// ```
    pub fn list(&self) -> ListOrganizationsRequest {
        ListOrganizationsRequest {
            client: self.client.clone(),
            limit: None,
            cursor: None,
            sort: None,
        }
    }

    /// Creates a new organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let org = client.organizations().create(CreateOrganizationRequest {
    ///     name: "my-org".into(),
    ///     display_name: Some("My Organization".into()),
    /// }).await?;
    /// ```
    pub async fn create(&self, request: CreateOrganizationRequest) -> Result<OrganizationInfo, Error> {
        // TODO: Implement actual API call
        Ok(OrganizationInfo {
            id: format!("org_{}", uuid::Uuid::new_v4()),
            name: request.name,
            display_name: request.display_name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

impl std::fmt::Debug for OrganizationsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationsClient").finish_non_exhaustive()
    }
}

/// Information about an organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    /// The organization ID (e.g., "org_abc123").
    pub id: String,
    /// The organization name (URL-safe slug).
    pub name: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
    /// When the organization was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the organization was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Request to create a new organization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    /// The organization name (URL-safe slug).
    pub name: String,
    /// Human-readable display name.
    pub display_name: Option<String>,
}

impl CreateOrganizationRequest {
    /// Creates a new request with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
        }
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

/// Request to update an organization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateOrganizationRequest {
    /// New display name.
    pub display_name: Option<String>,
}

impl UpdateOrganizationRequest {
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
}

/// Request to list organizations.
pub struct ListOrganizationsRequest {
    client: Client,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
}

impl ListOrganizationsRequest {
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

    async fn execute(self) -> Result<Page<OrganizationInfo>, Error> {
        // TODO: Implement actual API call
        let _ = (self.limit, self.cursor, self.sort);
        Ok(Page::default())
    }
}

impl std::future::IntoFuture for ListOrganizationsRequest {
    type Output = Result<Page<OrganizationInfo>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to delete an organization.
pub struct DeleteOrganizationRequest {
    client: OrganizationControlClient,
    confirmation: Option<String>,
}

impl DeleteOrganizationRequest {
    /// Confirms the deletion with the organization ID.
    ///
    /// You must pass `"DELETE {org_id}"` to confirm deletion.
    #[must_use]
    pub fn confirm(mut self, confirmation: impl Into<String>) -> Self {
        self.confirmation = Some(confirmation.into());
        self
    }

    async fn execute(self) -> Result<(), Error> {
        let expected = format!("DELETE {}", self.client.organization_id);
        match &self.confirmation {
            Some(c) if c == &expected => {
                // TODO: Implement actual API call
                Ok(())
            }
            Some(c) => Err(Error::invalid_argument(format!(
                "Invalid confirmation. Expected '{}', got '{}'",
                expected, c
            ))),
            None => Err(Error::invalid_argument(
                "Deletion requires confirmation. Call .confirm(\"DELETE org_id\") first",
            )),
        }
    }
}

impl std::future::IntoFuture for DeleteOrganizationRequest {
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

    async fn create_test_client() -> Client {
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
            .await
            .unwrap()
    }

    #[test]
    fn test_create_organization_request() {
        let req = CreateOrganizationRequest::new("my-org")
            .with_display_name("My Organization");

        assert_eq!(req.name, "my-org");
        assert_eq!(req.display_name, Some("My Organization".to_string()));
    }

    #[test]
    fn test_update_organization_request() {
        let req = UpdateOrganizationRequest::new()
            .with_display_name("New Name");

        assert_eq!(req.display_name, Some("New Name".to_string()));
    }

    #[tokio::test]
    async fn test_organization_control_client_accessors() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        assert_eq!(org.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_organization_control_client_debug() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let debug = format!("{:?}", org);
        assert!(debug.contains("OrganizationControlClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_organization_control_client_get() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let info = org.get().await.unwrap();
        assert_eq!(info.id, "org_test");
    }

    #[tokio::test]
    async fn test_organization_control_client_update() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let request = UpdateOrganizationRequest::new().with_display_name("New Name");
        let info = org.update(request).await.unwrap();
        assert_eq!(info.id, "org_test");
    }

    #[tokio::test]
    async fn test_organization_control_client_vaults() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let _ = org.vaults();
    }

    #[tokio::test]
    async fn test_organization_control_client_members() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let _ = org.members();
    }

    #[tokio::test]
    async fn test_organization_control_client_teams() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let _ = org.teams();
    }

    #[tokio::test]
    async fn test_organization_control_client_invitations() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let _ = org.invitations();
    }

    #[tokio::test]
    async fn test_organization_control_client_audit_logs() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let _ = org.audit_logs();
    }

    #[tokio::test]
    async fn test_organizations_client_debug() {
        let client = create_test_client().await;
        let orgs = OrganizationsClient::new(client);
        let debug = format!("{:?}", orgs);
        assert!(debug.contains("OrganizationsClient"));
    }

    #[tokio::test]
    async fn test_organizations_list() {
        let client = create_test_client().await;
        let orgs = OrganizationsClient::new(client);
        let page = orgs.list().await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_organizations_list_with_options() {
        let client = create_test_client().await;
        let orgs = OrganizationsClient::new(client);
        let page = orgs
            .list()
            .limit(10)
            .cursor("cursor123")
            .sort(SortOrder::Descending)
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_organizations_create() {
        let client = create_test_client().await;
        let orgs = OrganizationsClient::new(client);
        let request = CreateOrganizationRequest::new("my-org")
            .with_display_name("My Organization");
        let info = orgs.create(request).await.unwrap();
        assert_eq!(info.name, "my-org");
        assert_eq!(info.display_name, Some("My Organization".to_string()));
    }

    #[tokio::test]
    async fn test_delete_organization_with_confirmation() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let result = org.delete().confirm("DELETE org_test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_organization_wrong_confirmation() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let result = org.delete().confirm("DELETE wrong_org").await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid confirmation"));
    }

    #[tokio::test]
    async fn test_delete_organization_no_confirmation() {
        let client = create_test_client().await;
        let org = OrganizationControlClient::new(client, "org_test");
        let result = org.delete().await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("requires confirmation"));
    }
}
