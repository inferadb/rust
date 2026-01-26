//! Member management for the control plane.

use serde::{Deserialize, Serialize};

use crate::{
    Error,
    client::Client,
    control::{Page, SortOrder},
};

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
        Self { client, organization_id: organization_id.into() }
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
            "/control/v1/organizations/{}/members/{}",
            self.organization_id,
            user_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific member by user ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _user_id: impl Into<String>) -> Result<MemberInfo, Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
        let path = format!("/control/v1/organizations/{}/invitations", self.organization_id);
        self.client.inner().control_post(&path, &request).await
    }

    /// Invites a new member to the organization.
    #[cfg(not(feature = "rest"))]
    pub async fn invite(&self, _request: InviteMemberRequest) -> Result<InvitationInfo, Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
            "/control/v1/organizations/{}/members/{}",
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
        Err(Error::configuration("REST feature is required for control API"))
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
            "/control/v1/organizations/{}/members/{}",
            self.organization_id,
            user_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Removes a member from the organization.
    #[cfg(not(feature = "rest"))]
    pub async fn remove(&self, _user_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
        Self { client, organization_id: organization_id.into() }
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
            "/control/v1/organizations/{}/invitations/{}",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific invitation by ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _invitation_id: impl Into<String>) -> Result<InvitationInfo, Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
            "/control/v1/organizations/{}/invitations/{}/resend",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_post_empty::<()>(&path).await
    }

    /// Resends an invitation email.
    #[cfg(not(feature = "rest"))]
    pub async fn resend(&self, _invitation_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
            "/control/v1/organizations/{}/invitations/{}",
            self.organization_id,
            invitation_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Revokes a pending invitation.
    #[cfg(not(feature = "rest"))]
    pub async fn revoke(&self, _invitation_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration("REST feature is required for control API"))
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
        Self { email: email.into(), role, message: None }
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
        let mut path = format!("/control/v1/organizations/{}/members", self.organization_id);
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
        Err(Error::configuration("REST feature is required for control API"))
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
        let mut path = format!("/control/v1/organizations/{}/invitations", self.organization_id);
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
        Err(Error::configuration("REST feature is required for control API"))
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
    use std::sync::Arc;

    use super::*;
    use crate::{auth::BearerCredentialsConfig, transport::mock::MockTransport};

    async fn create_test_client() -> Client {
        let mock_transport = Arc::new(MockTransport::new().into_any());
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
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
    async fn test_members_client_accessors() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        assert_eq!(members.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_members_client_debug() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let debug = format!("{:?}", members);
        assert!(debug.contains("MembersClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_invitations_client_accessors() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        assert_eq!(invitations.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_invitations_client_debug() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let debug = format!("{:?}", invitations);
        assert!(debug.contains("InvitationsClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_list_members_request_builders() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");

        // Test all builder methods
        let _request = members
            .list()
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending)
            .role(OrgRole::Admin);

        // Just verify the builder compiles and returns a request
    }

    #[tokio::test]
    async fn test_list_invitations_request_builders() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");

        // Test all builder methods
        let _request =
            invitations.list().limit(50).cursor("cursor_xyz").status(InvitationStatus::Pending);

        // Just verify the builder compiles and returns a request
    }

    // Additional tests for Clone implementations and serde
    #[tokio::test]
    async fn test_members_client_clone() {
        let client = create_test_client().await;
        let members = MembersClient::new(client, "org_test");
        let cloned = members.clone();
        assert_eq!(cloned.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_invitations_client_clone() {
        let client = create_test_client().await;
        let invitations = InvitationsClient::new(client, "org_test");
        let cloned = invitations.clone();
        assert_eq!(cloned.organization_id(), "org_test");
    }

    #[test]
    fn test_member_info_serde() {
        let json = r#"{
            "user_id": "user_xyz",
            "organization_id": "org_test",
            "email": "test@example.com",
            "name": "Alice",
            "role": "admin",
            "status": "active",
            "joined_at": "2024-01-01T00:00:00Z"
        }"#;
        let member: MemberInfo = serde_json::from_str(json).unwrap();
        assert_eq!(member.user_id, "user_xyz");
        assert_eq!(member.email, "test@example.com");
        assert_eq!(member.role, OrgRole::Admin);
        assert_eq!(member.status, MemberStatus::Active);
    }

    #[test]
    fn test_member_info_clone() {
        let member = MemberInfo {
            user_id: "user_123".to_string(),
            organization_id: "org_123".to_string(),
            email: "test@test.com".to_string(),
            name: Some("Test".to_string()),
            role: OrgRole::Owner,
            status: MemberStatus::Active,
            joined_at: chrono::Utc::now(),
        };
        let cloned = member.clone();
        assert_eq!(cloned.user_id, "user_123");
        assert_eq!(cloned.role, OrgRole::Owner);
    }

    #[test]
    fn test_invitation_info_serde() {
        let json = r#"{
            "id": "inv_abc123",
            "organization_id": "org_test",
            "email": "invited@example.com",
            "role": "member",
            "status": "pending",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-02-01T00:00:00Z"
        }"#;
        let inv: InvitationInfo = serde_json::from_str(json).unwrap();
        assert_eq!(inv.id, "inv_abc123");
        assert_eq!(inv.email, "invited@example.com");
        assert_eq!(inv.role, OrgRole::Member);
        assert_eq!(inv.status, InvitationStatus::Pending);
    }

    #[test]
    fn test_invitation_info_clone() {
        let inv = InvitationInfo {
            id: "inv_123".to_string(),
            organization_id: "org_123".to_string(),
            email: "test@test.com".to_string(),
            role: OrgRole::Billing,
            status: InvitationStatus::Accepted,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now(),
        };
        let cloned = inv.clone();
        assert_eq!(cloned.id, "inv_123");
        assert_eq!(cloned.role, OrgRole::Billing);
        assert_eq!(cloned.status, InvitationStatus::Accepted);
    }

    #[test]
    fn test_org_role_serde() {
        let roles = vec![
            (OrgRole::Owner, "\"owner\""),
            (OrgRole::Admin, "\"admin\""),
            (OrgRole::Member, "\"member\""),
            (OrgRole::Billing, "\"billing\""),
            (OrgRole::Viewer, "\"viewer\""),
        ];
        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);
            let parsed: OrgRole = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, role);
        }
    }

    #[test]
    fn test_member_status_serde() {
        let statuses = vec![
            (MemberStatus::Active, "\"active\""),
            (MemberStatus::Suspended, "\"suspended\""),
            (MemberStatus::Deactivated, "\"deactivated\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
            let parsed: MemberStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_invitation_status_serde() {
        let statuses = vec![
            (InvitationStatus::Pending, "\"pending\""),
            (InvitationStatus::Accepted, "\"accepted\""),
            (InvitationStatus::Expired, "\"expired\""),
            (InvitationStatus::Revoked, "\"revoked\""),
        ];
        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
            let parsed: InvitationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_invite_member_request_clone() {
        let req = InviteMemberRequest::new("test@test.com", OrgRole::Admin);
        let cloned = req.clone();
        assert_eq!(cloned.email, "test@test.com");
        assert_eq!(cloned.role, OrgRole::Admin);
    }

    #[test]
    fn test_update_member_request_clone() {
        let req = UpdateMemberRequest::new().with_role(OrgRole::Viewer);
        let cloned = req.clone();
        assert_eq!(cloned.role, Some(OrgRole::Viewer));
    }
}

#[cfg(all(test, feature = "rest"))]
mod wiremock_tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;
    use crate::{Client, auth::BearerCredentialsConfig};

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
    async fn test_list_members() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/members"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "user_id": "user_1",
                        "organization_id": "org_123",
                        "email": "user@example.com",
                        "name": "Test User",
                        "role": "owner",
                        "status": "active",
                        "joined_at": "2024-01-01T00:00:00Z"
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
        let members = MembersClient::new(client, "org_123");
        let result = members.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].role, OrgRole::Owner);
    }

    #[tokio::test]
    async fn test_list_members_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/members"))
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
        let members = MembersClient::new(client, "org_123");
        let result = members
            .list()
            .limit(10)
            .cursor("cursor_abc")
            .sort(SortOrder::Descending)
            .role(OrgRole::Admin)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_member() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/members/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "user_abc",
                "organization_id": "org_123",
                "email": "user@example.com",
                "name": "Test User",
                "role": "admin",
                "status": "active",
                "joined_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let members = MembersClient::new(client, "org_123");
        let result = members.get("user_abc").await;

        assert!(result.is_ok());
        let member = result.unwrap();
        assert_eq!(member.user_id, "user_abc");
        assert_eq!(member.role, OrgRole::Admin);
    }

    #[tokio::test]
    async fn test_invite_member() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/organizations/org_123/invitations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "inv_new",
                "organization_id": "org_123",
                "email": "new@example.com",
                "role": "member",
                "status": "pending",
                "created_at": "2024-01-01T00:00:00Z",
                "expires_at": "2024-01-08T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let members = MembersClient::new(client, "org_123");
        let request = InviteMemberRequest::new("new@example.com", OrgRole::Member);
        let result = members.invite(request).await;

        assert!(result.is_ok());
        let invitation = result.unwrap();
        assert_eq!(invitation.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_update_member() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/control/v1/organizations/org_123/members/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "user_abc",
                "organization_id": "org_123",
                "email": "user@example.com",
                "name": "Test User",
                "role": "admin",
                "status": "active",
                "joined_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let members = MembersClient::new(client, "org_123");
        let request = UpdateMemberRequest::new().with_role(OrgRole::Admin);
        let result = members.update("user_abc", request).await;

        assert!(result.is_ok());
        let member = result.unwrap();
        assert_eq!(member.role, OrgRole::Admin);
    }

    #[tokio::test]
    async fn test_remove_member() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/organizations/org_123/members/user_abc"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let members = MembersClient::new(client, "org_123");
        let result = members.remove("user_abc").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_invitations() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/invitations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "inv_1",
                        "organization_id": "org_123",
                        "email": "invite@example.com",
                        "role": "member",
                        "status": "pending",
                        "created_at": "2024-01-01T00:00:00Z",
                        "expires_at": "2024-01-08T00:00:00Z"
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
        let invitations = InvitationsClient::new(client, "org_123");
        let result = invitations.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn test_revoke_invitation() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/organizations/org_123/invitations/inv_abc"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let invitations = InvitationsClient::new(client, "org_123");
        let result = invitations.revoke("inv_abc").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resend_invitation() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/organizations/org_123/invitations/inv_abc/resend"))
            .respond_with(ResponseTemplate::new(200).set_body_string("null"))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let invitations = InvitationsClient::new(client, "org_123");
        let result = invitations.resend("inv_abc").await;

        assert!(result.is_ok());
    }
}
