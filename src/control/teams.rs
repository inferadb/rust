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
    pub async fn create(&self, request: CreateTeamRequest) -> Result<TeamInfo, Error> {
        // TODO: Implement actual API call
        Ok(TeamInfo {
            id: format!("team_{}", uuid::Uuid::new_v4()),
            organization_id: self.organization_id.clone(),
            name: request.name,
            description: request.description,
            member_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Gets a team by ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let team = org.teams().get("team_abc123").await?;
    /// ```
    pub async fn get(&self, team_id: impl Into<String>) -> Result<TeamInfo, Error> {
        // TODO: Implement actual API call
        let team_id = team_id.into();
        Ok(TeamInfo {
            id: team_id,
            organization_id: self.organization_id.clone(),
            name: "Team".to_string(),
            description: None,
            member_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
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
    pub async fn update(
        &self,
        team_id: impl Into<String>,
        request: UpdateTeamRequest,
    ) -> Result<TeamInfo, Error> {
        // TODO: Implement actual API call
        let _ = request;
        self.get(team_id).await
    }

    /// Deletes a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().delete("team_abc123").await?;
    /// ```
    pub async fn delete(&self, team_id: impl Into<String>) -> Result<(), Error> {
        // TODO: Implement actual API call
        let _ = (team_id.into(), &self.client);
        Ok(())
    }

    /// Adds a member to a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().add_member("team_abc123", "user_xyz").await?;
    /// ```
    pub async fn add_member(
        &self,
        team_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<(), Error> {
        // TODO: Implement actual API call
        let _ = (team_id.into(), user_id.into(), &self.client);
        Ok(())
    }

    /// Removes a member from a team.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.teams().remove_member("team_abc123", "user_xyz").await?;
    /// ```
    pub async fn remove_member(
        &self,
        team_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<(), Error> {
        // TODO: Implement actual API call
        let _ = (team_id.into(), user_id.into(), &self.client);
        Ok(())
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

    async fn execute(self) -> Result<Page<TeamInfo>, Error> {
        // TODO: Implement actual API call
        let _ = (
            &self.client,
            &self.organization_id,
            self.limit,
            self.cursor,
            self.sort,
        );
        Ok(Page::default())
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

    async fn execute(self) -> Result<Page<TeamMemberInfo>, Error> {
        // TODO: Implement actual API call
        let _ = (
            &self.client,
            &self.organization_id,
            &self.team_id,
            self.limit,
            self.cursor,
        );
        Ok(Page::default())
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
    async fn test_teams_list() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let page = teams.list().await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_teams_list_with_options() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let page = teams
            .list()
            .limit(10)
            .cursor("cursor123")
            .sort(SortOrder::Descending)
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_teams_create() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let request = CreateTeamRequest::new("Engineering").with_description("Backend team");
        let info = teams.create(request).await.unwrap();
        assert_eq!(info.name, "Engineering");
        assert_eq!(info.description, Some("Backend team".to_string()));
        assert_eq!(info.organization_id, "org_test");
    }

    #[tokio::test]
    async fn test_teams_get() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let info = teams.get("team_abc123").await.unwrap();
        assert_eq!(info.id, "team_abc123");
        assert_eq!(info.organization_id, "org_test");
    }

    #[tokio::test]
    async fn test_teams_update() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let request = UpdateTeamRequest::new()
            .with_name("New Name")
            .with_description("New description");
        let info = teams.update("team_abc123", request).await.unwrap();
        assert_eq!(info.id, "team_abc123");
    }

    #[tokio::test]
    async fn test_teams_delete() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let result = teams.delete("team_abc123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_teams_add_member() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let result = teams.add_member("team_abc123", "user_xyz").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_teams_remove_member() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let result = teams.remove_member("team_abc123", "user_xyz").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_teams_list_members() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let page = teams.list_members("team_abc123").await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_teams_list_members_with_options() {
        let client = create_test_client().await;
        let teams = TeamsClient::new(client, "org_test");
        let page = teams
            .list_members("team_abc123")
            .limit(10)
            .cursor("cursor123")
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }
}
