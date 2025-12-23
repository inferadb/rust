//! Team management for the control plane.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for team management operations.
///
/// Access via `org.teams()`.
///
/// ## Example
///
/// ```rust,ignore
/// let teams = org.teams();
///
/// // Create a team
/// let team = teams.create(CreateTeamRequest::new("Engineering")).await?;
///
/// // List all teams
/// let list = teams.list().await?;
///
/// // Add member to team
/// teams.add_member(&team.id, "user:alice").await?;
/// ```
#[derive(Clone)]
pub struct TeamsClient {
    client: Client,
    organization_id: String,
}

impl TeamsClient {
    /// Creates a new teams client.
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

    /// Lists all teams in the organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let teams = org.teams().list().await?;
    /// for team in teams.items {
    ///     println!("{}: {}", team.id, team.name);
    /// }
    /// ```
    pub fn list(&self) -> ListTeamsRequest {
        ListTeamsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            limit: None,
            cursor: None,
            sort: None,
        }
    }

    /// Creates a new team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let team = org.teams().create(CreateTeamRequest::new("Engineering")
    ///     .with_description("Backend engineering team")
    /// ).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn create(&self, request: CreateTeamRequest) -> Result<TeamInfo, Error> {
        let path = format!("/control/v1/organizations/{}/teams", self.organization_id);
        self.client.inner().control_post(&path, &request).await
    }

    /// Creates a new team.
    #[cfg(not(feature = "rest"))]
    pub async fn create(&self, _request: CreateTeamRequest) -> Result<TeamInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Gets a team by ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let team = org.teams().get("team_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self, team_id: impl Into<String>) -> Result<TeamInfo, Error> {
        let path = format!(
            "/control/v1/organizations/{}/teams/{}",
            self.organization_id,
            team_id.into()
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a team by ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, _team_id: impl Into<String>) -> Result<TeamInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Updates a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let team = org.teams().update("team_abc123", UpdateTeamRequest::new()
    ///     .with_name("New Name")
    /// ).await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn update(
        &self,
        team_id: impl Into<String>,
        request: UpdateTeamRequest,
    ) -> Result<TeamInfo, Error> {
        let path = format!(
            "/control/v1/organizations/{}/teams/{}",
            self.organization_id,
            team_id.into()
        );
        self.client.inner().control_patch(&path, &request).await
    }

    /// Updates a team.
    #[cfg(not(feature = "rest"))]
    pub async fn update(
        &self,
        _team_id: impl Into<String>,
        _request: UpdateTeamRequest,
    ) -> Result<TeamInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Deletes a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().delete("team_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn delete(&self, team_id: impl Into<String>) -> Result<(), Error> {
        let path = format!(
            "/control/v1/organizations/{}/teams/{}",
            self.organization_id,
            team_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Deletes a team.
    #[cfg(not(feature = "rest"))]
    pub async fn delete(&self, _team_id: impl Into<String>) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Adds a member to a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().add_member("team_abc123", "user_xyz").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn add_member(
        &self,
        team_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<(), Error> {
        #[derive(serde::Serialize)]
        struct AddMemberBody {
            user_id: String,
        }
        let path = format!(
            "/control/v1/organizations/{}/teams/{}/members",
            self.organization_id,
            team_id.into()
        );
        let body = AddMemberBody {
            user_id: user_id.into(),
        };
        self.client
            .inner()
            .control_post::<_, ()>(&path, &body)
            .await
    }

    /// Adds a member to a team.
    #[cfg(not(feature = "rest"))]
    pub async fn add_member(
        &self,
        _team_id: impl Into<String>,
        _user_id: impl Into<String>,
    ) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Removes a member from a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().remove_member("team_abc123", "user_xyz").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn remove_member(
        &self,
        team_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<(), Error> {
        let path = format!(
            "/control/v1/organizations/{}/teams/{}/members/{}",
            self.organization_id,
            team_id.into(),
            user_id.into()
        );
        self.client.inner().control_delete(&path).await
    }

    /// Removes a member from a team.
    #[cfg(not(feature = "rest"))]
    pub async fn remove_member(
        &self,
        _team_id: impl Into<String>,
        _user_id: impl Into<String>,
    ) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Lists members of a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let members = org.teams().list_members("team_abc123").await?;
    /// for member in members.items {
    ///     println!("{}: {}", member.user_id, member.role);
    /// }
    /// ```
    pub fn list_members(&self, team_id: impl Into<String>) -> ListTeamMembersRequest {
        ListTeamMembersRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            team_id: team_id.into(),
            limit: None,
            cursor: None,
        }
    }
}

impl std::fmt::Debug for TeamsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamsClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// Information about a team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInfo {
    /// The team ID (e.g., "team_abc123").
    pub id: String,
    /// The organization ID that owns this team.
    pub organization_id: String,
    /// The team name.
    pub name: String,
    /// Description of the team.
    pub description: Option<String>,
    /// Number of members in the team.
    pub member_count: u32,
    /// When the team was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the team was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Information about a team member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberInfo {
    /// The user ID.
    pub user_id: String,
    /// The user's email.
    pub email: String,
    /// The user's display name.
    pub name: Option<String>,
    /// Role within the team.
    pub role: TeamRole,
    /// When the member joined the team.
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Role within a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    /// Team owner with full permissions.
    Owner,
    /// Team administrator.
    Admin,
    /// Regular team member.
    #[default]
    Member,
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamRole::Owner => write!(f, "owner"),
            TeamRole::Admin => write!(f, "admin"),
            TeamRole::Member => write!(f, "member"),
        }
    }
}

/// Request to create a new team.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    /// The team name.
    pub name: String,
    /// Description of the team.
    pub description: Option<String>,
}

impl CreateTeamRequest {
    /// Creates a new request with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Request to update a team.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTeamRequest {
    /// New name for the team.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
}

impl UpdateTeamRequest {
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
}

/// Request to list teams.
pub struct ListTeamsRequest {
    client: Client,
    organization_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
}

impl ListTeamsRequest {
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

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<TeamInfo>, Error> {
        let mut path = format!("/control/v1/organizations/{}/teams", self.organization_id);
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

        if !query_parts.is_empty() {
            path.push('?');
            path.push_str(&query_parts.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<TeamInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListTeamsRequest {
    type Output = Result<Page<TeamInfo>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to list team members.
pub struct ListTeamMembersRequest {
    client: Client,
    organization_id: String,
    team_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
}

impl ListTeamMembersRequest {
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

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<TeamMemberInfo>, Error> {
        let mut path = format!(
            "/control/v1/organizations/{}/teams/{}/members",
            self.organization_id, self.team_id
        );
        let mut query_parts = Vec::new();

        if let Some(limit) = self.limit {
            query_parts.push(format!("limit={}", limit));
        }
        if let Some(cursor) = &self.cursor {
            query_parts.push(format!("cursor={}", urlencoding::encode(cursor)));
        }

        if !query_parts.is_empty() {
            path.push('?');
            path.push_str(&query_parts.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<TeamMemberInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListTeamMembersRequest {
    type Output = Result<Page<TeamMemberInfo>, Error>;
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
    fn test_team_role() {
        assert_eq!(TeamRole::default(), TeamRole::Member);
        assert_eq!(TeamRole::Owner.to_string(), "owner");
        assert_eq!(TeamRole::Admin.to_string(), "admin");
        assert_eq!(TeamRole::Member.to_string(), "member");
    }

    #[test]
    fn test_create_team_request() {
        let req = CreateTeamRequest::new("Engineering").with_description("Backend team");

        assert_eq!(req.name, "Engineering");
        assert_eq!(req.description, Some("Backend team".to_string()));
    }

    #[test]
    fn test_update_team_request() {
        let req = UpdateTeamRequest::new()
            .with_name("New Name")
            .with_description("New description");

        assert_eq!(req.name, Some("New Name".to_string()));
        assert_eq!(req.description, Some("New description".to_string()));
    }

    #[tokio::test]
    async fn test_teams_client_accessors() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        assert_eq!(teams.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_teams_client_debug() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let debug = format!("{:?}", teams);
        assert!(debug.contains("TeamsClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_list_teams_request_builders() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");

        // Test all builder methods
        let _request = teams
            .list()
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending);

        // Just verify the builder compiles and returns a request
    }

    #[tokio::test]
    async fn test_list_team_members_request_builders() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");

        // Test all builder methods
        let _request = teams
            .list_members("team_abc123")
            .limit(50)
            .cursor("cursor_xyz");

        // Just verify the builder compiles and returns a request
    }

    // Additional tests for Clone implementations and serde
    #[tokio::test]
    async fn test_teams_client_clone() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let cloned = teams.clone();
        assert_eq!(cloned.organization_id(), "org_test");
    }

    #[test]
    fn test_team_info_serde() {
        let json = r#"{
            "id": "team_abc123",
            "organization_id": "org_test",
            "name": "Engineering",
            "description": "Backend team",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "member_count": 5
        }"#;
        let team: TeamInfo = serde_json::from_str(json).unwrap();
        assert_eq!(team.id, "team_abc123");
        assert_eq!(team.name, "Engineering");
        assert_eq!(team.description, Some("Backend team".to_string()));
        assert_eq!(team.member_count, 5);
    }

    #[test]
    fn test_team_info_clone() {
        let team = TeamInfo {
            id: "team_123".to_string(),
            organization_id: "org_123".to_string(),
            name: "Test Team".to_string(),
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            member_count: 0,
        };
        let cloned = team.clone();
        assert_eq!(cloned.id, "team_123");
        assert_eq!(cloned.name, "Test Team");
    }

    #[test]
    fn test_team_member_info_serde() {
        let json = r#"{
            "user_id": "user_abc123",
            "email": "test@example.com",
            "name": "Alice",
            "role": "admin",
            "joined_at": "2024-01-01T00:00:00Z"
        }"#;
        let member: TeamMemberInfo = serde_json::from_str(json).unwrap();
        assert_eq!(member.user_id, "user_abc123");
        assert_eq!(member.email, "test@example.com");
        assert_eq!(member.role, TeamRole::Admin);
    }

    #[test]
    fn test_team_member_info_clone() {
        let member = TeamMemberInfo {
            user_id: "user_123".to_string(),
            email: "test@test.com".to_string(),
            name: Some("Test".to_string()),
            role: TeamRole::Owner,
            joined_at: chrono::Utc::now(),
        };
        let cloned = member.clone();
        assert_eq!(cloned.user_id, "user_123");
        assert_eq!(cloned.role, TeamRole::Owner);
    }

    #[test]
    fn test_team_role_serde() {
        let roles = vec![
            (TeamRole::Owner, "\"owner\""),
            (TeamRole::Admin, "\"admin\""),
            (TeamRole::Member, "\"member\""),
        ];
        for (role, expected) in roles {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, expected);
            let parsed: TeamRole = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, role);
        }
    }

    #[test]
    fn test_create_team_request_clone() {
        let req = CreateTeamRequest::new("Test").with_description("Desc");
        let cloned = req.clone();
        assert_eq!(cloned.name, "Test");
        assert_eq!(cloned.description, Some("Desc".to_string()));
    }

    #[test]
    fn test_update_team_request_clone() {
        let req = UpdateTeamRequest::new().with_name("NewName");
        let cloned = req.clone();
        assert_eq!(cloned.name, Some("NewName".to_string()));
    }
}
