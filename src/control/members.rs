//! Member management for the control plane.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for organization member management operations.
///
/// Access via `org.members()`.
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
///
/// // Update member role
/// members.update("user_abc123", UpdateMemberRequest::new()
///     .with_role(OrgRole::Admin)
/// ).await?;
/// ```
#[derive(Clone)]
pub struct MembersClient {
    client: Client,
    organization_id: String,
}

impl MembersClient {
    /// Creates a new members client.
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

    /// Lists all members in the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let members = org.members().list().await?;
    /// for member in members.items {
    ///     println!("{}: {} ({})", member.user_id, member.email, member.role);
    /// }
    /// ```
    pub fn list(&self) -> ListMembersRequest {
        ListMembersRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            limit: None,
            cursor: None,
            sort: None,
            role: None,
        }
    }

    /// Gets a specific member by user ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let member = org.members().get("user_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self, user_id: impl Into<String>) -> Result<MemberInfo, Error> {
        let path = format!(
            "/v1/organizations/{}/members/{}",
            self.organization_id,
            user_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific member by user ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _user_id: impl Into<String>) -> Result<MemberInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Invites a new member to the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.members().invite(InviteMemberRequest::new("alice@example.com", OrgRole::Member)
    ///     .with_message("Welcome to the team!")
    /// ).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn invite(&self, request: InviteMemberRequest) -> Result<InvitationInfo, Error> {
        let path = format!("/v1/organizations/{}/invitations", self.organization_id);
        self.client.inner().control_post(&path, &request).await
    }

    /// Invites a new member to the organization.
    #[cfg(not(feature = "rest"))]
    pub async fn invite(&self, _request: InviteMemberRequest) -> Result<InvitationInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Updates a member's role or status.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.members().update("user_abc123", UpdateMemberRequest::new()
    ///     .with_role(OrgRole::Admin)
    /// ).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn update(
        &self,
        user_id: impl Into<String>,
        request: UpdateMemberRequest,
    ) -> Result<MemberInfo, Error> {
        let path = format!(
            "/v1/organizations/{}/members/{}",
            self.organization_id,
            user_id.into()
        );
        self.client.inner().control_patch(&path, &request).await
    }

    /// Updates a member's role or status.
    #[cfg(not(feature = "rest"))]
    pub async fn update(
        &self,
        _user_id: impl Into<String>,
        _request: UpdateMemberRequest,
    ) -> Result<MemberInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Removes a member from the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.members().remove("user_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn remove(&self, user_id: impl Into<String>) -> Result<(), Error> {
        let path = format!(
            "/v1/organizations/{}/members/{}",
            self.organization_id,
            user_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Removes a member from the organization.
    #[cfg(not(feature = "rest"))]
    pub async fn remove(&self, _user_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::fmt::Debug for MembersClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MembersClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Client for managing invitations.
///
/// Access via `org.invitations()`.
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
///
/// // Revoke an invitation
/// invitations.revoke("inv_abc123").await?;
/// ```
#[derive(Clone)]
pub struct InvitationsClient {
    client: Client,
    organization_id: String,
}

impl InvitationsClient {
    /// Creates a new invitations client.
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

    /// Lists all pending invitations.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let invitations = org.invitations().list().await?;
    /// ```
    pub fn list(&self) -> ListInvitationsRequest {
        ListInvitationsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            limit: None,
            cursor: None,
            status: None,
        }
    }

    /// Gets a specific invitation by ID.
    #[cfg(feature = "rest")]
    pub async fn get(&self, invitation_id: impl Into<String>) -> Result<InvitationInfo, Error> {
        let path = format!(
            "/v1/organizations/{}/invitations/{}",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific invitation by ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _invitation_id: impl Into<String>) -> Result<InvitationInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Resends an invitation email.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.invitations().resend("inv_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn resend(&self, invitation_id: impl Into<String>) -> Result<(), Error> {
        let path = format!(
            "/v1/organizations/{}/invitations/{}/resend",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_post_empty::<()>(&path).await
    }

    /// Resends an invitation email.
    #[cfg(not(feature = "rest"))]
    pub async fn resend(&self, _invitation_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Revokes a pending invitation.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.invitations().revoke("inv_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn revoke(&self, invitation_id: impl Into<String>) -> Result<(), Error> {
        let path = format!(
            "/v1/organizations/{}/invitations/{}",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Revokes a pending invitation.
    #[cfg(not(feature = "rest"))]
    pub async fn revoke(&self, _invitation_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::fmt::Debug for InvitationsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvitationsClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Information about an organization member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    /// The user ID.
    pub user_id: String,
    /// The organization ID.
    pub organization_id: String,
    /// The member's email address.
    pub email: String,
    /// The member's display name.
    pub name: Option<String>,
    /// The member's role in the organization.
    pub role: OrgRole,
    /// The member's status.
    pub status: MemberStatus,
    /// When the member joined.
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Information about an invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationInfo {
    /// The invitation ID.
    pub id: String,
    /// The organization ID.
    pub organization_id: String,
    /// The invited email address.
    pub email: String,
    /// The role that will be assigned upon acceptance.
    pub role: OrgRole,
    /// The invitation status.
    pub status: InvitationStatus,
    /// When the invitation expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// When the invitation was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Role within an organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    /// Organization owner with full permissions.
    Owner,
    /// Administrator with most permissions.
    Admin,
    /// Regular member.
    #[default]
    Member,
    /// Billing administrator (can manage billing only).
    Billing,
    /// Read-only viewer.
    Viewer,
}

impl OrgRole {
    /// Returns `true` if this is an admin role (owner or admin).
    pub fn is_admin(&self) -> bool {
        matches!(self, OrgRole::Owner | OrgRole::Admin)
    }

    /// Returns `true` if this is the owner role.
    pub fn is_owner(&self) -> bool {
        matches!(self, OrgRole::Owner)
    }
}

impl std::fmt::Display for OrgRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrgRole::Owner => write!(f, "owner"),
            OrgRole::Admin => write!(f, "admin"),
            OrgRole::Member => write!(f, "member"),
            OrgRole::Billing => write!(f, "billing"),
            OrgRole::Viewer => write!(f, "viewer"),
        }
    }
}

/// Status of an organization member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    /// Member is active.
    #[default]
    Active,
    /// Member is suspended.
    Suspended,
    /// Member has been deactivated.
    Deactivated,
}

impl std::fmt::Display for MemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberStatus::Active => write!(f, "active"),
            MemberStatus::Suspended => write!(f, "suspended"),
            MemberStatus::Deactivated => write!(f, "deactivated"),
        }
    }
}

/// Status of an invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    /// Invitation is pending acceptance.
    #[default]
    Pending,
    /// Invitation was accepted.
    Accepted,
    /// Invitation expired.
    Expired,
    /// Invitation was revoked.
    Revoked,
}

impl std::fmt::Display for InvitationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvitationStatus::Pending => write!(f, "pending"),
            InvitationStatus::Accepted => write!(f, "accepted"),
            InvitationStatus::Expired => write!(f, "expired"),
            InvitationStatus::Revoked => write!(f, "revoked"),
        }
    }
}

/// Request to invite a new member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteMemberRequest {
    /// Email address to invite.
    pub email: String,
    /// Role to assign upon acceptance.
    pub role: OrgRole,
    /// Optional message to include in the invitation email.
    pub message: Option<String>,
}

impl InviteMemberRequest {
    /// Creates a new invite request.
    pub fn new(email: impl Into<String>, role: OrgRole) -> Self {
        Self {
            email: email.into(),
            role,
            message: None,
        }
    }

    /// Sets a custom message for the invitation email.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Request to update a member.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMemberRequest {
    /// New role for the member.
    pub role: Option<OrgRole>,
    /// New status for the member.
    pub status: Option<MemberStatus>,
}

impl UpdateMemberRequest {
    /// Creates a new empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the role.
    #[must_use]
    pub fn with_role(mut self, role: OrgRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Sets the status.
    #[must_use]
    pub fn with_status(mut self, status: MemberStatus) -> Self {
        self.status = Some(status);
        self
    }
}

/// Request to list members.
pub struct ListMembersRequest {
    client: Client,
    organization_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
    role: Option<OrgRole>,
}

impl ListMembersRequest {
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

    /// Filters by role.
    #[must_use]
    pub fn role(mut self, role: OrgRole) -> Self {
        self.role = Some(role);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<MemberInfo>, Error> {
        let mut path = format!("/v1/organizations/{}/members", self.organization_id);
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
        if let Some(role) = &self.role {
            query_parts.push(format!("role={}", role));
        }

        if !query_parts.is_empty() {
            path.push('?');
            path.push_str(&query_parts.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<MemberInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListMembersRequest {
    type Output = Result<Page<MemberInfo>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to list invitations.
pub struct ListInvitationsRequest {
    client: Client,
    organization_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    status: Option<InvitationStatus>,
}

impl ListInvitationsRequest {
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

    /// Filters by status.
    #[must_use]
    pub fn status(mut self, status: InvitationStatus) -> Self {
        self.status = Some(status);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<InvitationInfo>, Error> {
        let mut path = format!("/v1/organizations/{}/invitations", self.organization_id);
        let mut query_parts = Vec::new();

        if let Some(limit) = self.limit {
            query_parts.push(format!("limit={}", limit));
        }
        if let Some(cursor) = &self.cursor {
            query_parts.push(format!("cursor={}", urlencoding::encode(cursor)));
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
    async fn execute(self) -> Result<Page<InvitationInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListInvitationsRequest {
    type Output = Result<Page<InvitationInfo>, Error>;
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
    fn test_org_role() {
        assert_eq!(OrgRole::default(), OrgRole::Member);
        assert!(OrgRole::Owner.is_admin());
        assert!(OrgRole::Admin.is_admin());
        assert!(!OrgRole::Member.is_admin());
        assert!(!OrgRole::Billing.is_admin());
        assert!(!OrgRole::Viewer.is_admin());
        assert!(OrgRole::Owner.is_owner());
        assert!(!OrgRole::Admin.is_owner());
    }

    #[test]
    fn test_org_role_display() {
        assert_eq!(OrgRole::Owner.to_string(), "owner");
        assert_eq!(OrgRole::Admin.to_string(), "admin");
        assert_eq!(OrgRole::Member.to_string(), "member");
        assert_eq!(OrgRole::Billing.to_string(), "billing");
        assert_eq!(OrgRole::Viewer.to_string(), "viewer");
    }

    #[test]
    fn test_member_status() {
        assert_eq!(MemberStatus::default(), MemberStatus::Active);
        assert_eq!(MemberStatus::Active.to_string(), "active");
        assert_eq!(MemberStatus::Suspended.to_string(), "suspended");
        assert_eq!(MemberStatus::Deactivated.to_string(), "deactivated");
    }

    #[test]
    fn test_invitation_status() {
        assert_eq!(InvitationStatus::default(), InvitationStatus::Pending);
        assert_eq!(InvitationStatus::Pending.to_string(), "pending");
        assert_eq!(InvitationStatus::Accepted.to_string(), "accepted");
        assert_eq!(InvitationStatus::Expired.to_string(), "expired");
        assert_eq!(InvitationStatus::Revoked.to_string(), "revoked");
    }

    #[test]
    fn test_invite_member_request() {
        let req =
            InviteMemberRequest::new("alice@example.com", OrgRole::Admin).with_message("Welcome!");

        assert_eq!(req.email, "alice@example.com");
        assert_eq!(req.role, OrgRole::Admin);
        assert_eq!(req.message, Some("Welcome!".to_string()));
    }

    #[test]
    fn test_update_member_request() {
        let req = UpdateMemberRequest::new()
            .with_role(OrgRole::Admin)
            .with_status(MemberStatus::Suspended);

        assert_eq!(req.role, Some(OrgRole::Admin));
        assert_eq!(req.status, Some(MemberStatus::Suspended));
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_client_accessors() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        assert_eq!(members.organization_id(), "org_test");
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_client_debug() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let debug = format!("{:?}", members);
        assert!(debug.contains("MembersClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_list() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let page = members.list().await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_list_with_options() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let page = members
            .list()
            .limit(10)
            .cursor("cursor123")
            .sort(SortOrder::Ascending)
            .role(OrgRole::Admin)
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_get() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let info = members.get("user_abc123").await.unwrap();
        assert_eq!(info.user_id, "user_abc123");
        assert_eq!(info.organization_id, "org_test");
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_update() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let request = UpdateMemberRequest::new()
            .with_role(OrgRole::Admin)
            .with_status(MemberStatus::Suspended);
        let info = members.update("user_abc123", request).await.unwrap();
        assert_eq!(info.user_id, "user_abc123");
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_remove() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let result = members.remove("user_abc123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_members_invite() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let request =
            InviteMemberRequest::new("alice@example.com", OrgRole::Member).with_message("Welcome!");
        let info = members.invite(request).await.unwrap();
        assert_eq!(info.email, "alice@example.com");
        assert_eq!(info.role, OrgRole::Member);
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_client_accessors() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        assert_eq!(invitations.organization_id(), "org_test");
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_client_debug() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let debug = format!("{:?}", invitations);
        assert!(debug.contains("InvitationsClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_list() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let page = invitations.list().await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_list_with_options() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let page = invitations
            .list()
            .limit(10)
            .cursor("cursor123")
            .status(InvitationStatus::Pending)
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_get() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let info = invitations.get("inv_abc123").await.unwrap();
        assert_eq!(info.id, "inv_abc123");
        assert_eq!(info.organization_id, "org_test");
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_revoke() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let result = invitations.revoke("inv_abc123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires running server"]
    async fn test_invitations_resend() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let result = invitations.resend("inv_abc123").await;
        assert!(result.is_ok());
    }
}
