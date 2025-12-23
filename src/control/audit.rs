//! Audit log management for the control plane.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for querying audit logs.
///
/// Access via `org.audit_logs()`.
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
///
/// // Filter by action type
/// let writes = logs.list().action(AuditAction::RelationshipWrite).await?;
///
/// // Filter by time range
/// let recent = logs.list()
///     .after(Utc::now() - Duration::hours(24))
///     .await?;
/// ```
#[derive(Clone)]
pub struct AuditLogsClient {
    client: Client,
    organization_id: String,
}

impl AuditLogsClient {
    /// Creates a new audit logs client.
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

    /// Starts a query for audit log events.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let events = org.audit_logs().list().await?;
    /// for event in events.items {
    ///     println!("{}: {} {} on {}",
    ///         event.timestamp,
    ///         event.actor,
    ///         event.action,
    ///         event.resource
    ///     );
    /// }
    /// ```
    pub fn list(&self) -> ListAuditLogsRequest {
        ListAuditLogsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            vault_id: None,
            limit: None,
            cursor: None,
            sort: None,
            actor: None,
            action: None,
            resource: None,
            after: None,
            before: None,
        }
    }

    /// Gets a specific audit log event by ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let event = org.audit_logs().get("evt_abc123").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self, event_id: impl Into<String>) -> Result<AuditEvent, Error> {
        let event_id = event_id.into();
        let path = format!(
            "/control/v1/organizations/{}/audit-logs/{}",
            self.organization_id, event_id
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific audit log event by ID.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, event_id: impl Into<String>) -> Result<AuditEvent, Error> {
        let _ = event_id.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Exports audit logs to a file or stream.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// org.audit_logs().export()
    ///     .after(start_date)
    ///     .before(end_date)
    ///     .format(ExportFormat::Csv)
    ///     .to_file("audit_logs.csv")
    ///     .await?;
    /// ```
    pub fn export(&self) -> ExportAuditLogsRequest {
        ExportAuditLogsRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            vault_id: None,
            after: None,
            before: None,
            format: ExportFormat::Json,
        }
    }
}

impl std::fmt::Debug for AuditLogsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLogsClient")
            .field("organization_id", &self.organization_id)
            .finish_non_exhaustive()
    }
}

/// An audit log event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: String,
    /// The organization ID.
    pub organization_id: String,
    /// The vault ID (if applicable).
    pub vault_id: Option<String>,
    /// When the event occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Information about the actor who triggered the event.
    pub actor: ActorInfo,
    /// The action that was performed.
    pub action: AuditAction,
    /// The resource affected (if applicable).
    pub resource: Option<String>,
    /// Additional details about the event.
    pub details: Option<serde_json::Value>,
    /// The request ID for correlation.
    pub request_id: Option<String>,
    /// The outcome of the action.
    pub outcome: AuditOutcome,
}

/// Information about the actor who triggered an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorInfo {
    /// The actor's ID.
    pub id: String,
    /// The type of actor.
    pub actor_type: ActorType,
    /// The actor's email (if available).
    pub email: Option<String>,
    /// The IP address from which the action was performed.
    pub ip_address: Option<String>,
    /// The user agent string.
    pub user_agent: Option<String>,
}

/// Type of actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    /// A human user.
    #[default]
    User,
    /// An API client (service account).
    ApiClient,
    /// The system itself.
    System,
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorType::User => write!(f, "user"),
            ActorType::ApiClient => write!(f, "api_client"),
            ActorType::System => write!(f, "system"),
        }
    }
}

/// Types of auditable actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authorization actions
    /// Authorization check performed.
    #[default]
    Check,
    /// Batch authorization check performed.
    CheckBatch,

    // Relationship actions
    /// Relationship written.
    RelationshipWrite,
    /// Relationship deleted.
    RelationshipDelete,
    /// Batch relationships written.
    RelationshipWriteBatch,
    /// Batch relationships deleted.
    RelationshipDeleteBatch,

    // Schema actions
    /// Schema pushed.
    SchemaPush,
    /// Schema activated.
    SchemaActivate,

    // Vault actions
    /// Vault created.
    VaultCreate,
    /// Vault updated.
    VaultUpdate,
    /// Vault deleted.
    VaultDelete,

    // Organization actions
    /// Organization created.
    OrganizationCreate,
    /// Organization updated.
    OrganizationUpdate,
    /// Organization deleted.
    OrganizationDelete,

    // Member actions
    /// Member invited.
    MemberInvite,
    /// Member added.
    MemberAdd,
    /// Member role updated.
    MemberUpdate,
    /// Member removed.
    MemberRemove,

    // Team actions
    /// Team created.
    TeamCreate,
    /// Team updated.
    TeamUpdate,
    /// Team deleted.
    TeamDelete,
    /// Team member added.
    TeamMemberAdd,
    /// Team member removed.
    TeamMemberRemove,

    // Token actions
    /// Token created.
    TokenCreate,
    /// Token revoked.
    TokenRevoke,
    /// Token rotated.
    TokenRotate,

    // Authentication actions
    /// User logged in.
    Login,
    /// User logged out.
    Logout,
    /// Login failed.
    LoginFailed,

    // API client actions
    /// API client created.
    ApiClientCreate,
    /// API client updated.
    ApiClientUpdate,
    /// API client deleted.
    ApiClientDelete,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuditAction::Check => "check",
            AuditAction::CheckBatch => "check_batch",
            AuditAction::RelationshipWrite => "relationship.write",
            AuditAction::RelationshipDelete => "relationship.delete",
            AuditAction::RelationshipWriteBatch => "relationship.write_batch",
            AuditAction::RelationshipDeleteBatch => "relationship.delete_batch",
            AuditAction::SchemaPush => "schema.push",
            AuditAction::SchemaActivate => "schema.activate",
            AuditAction::VaultCreate => "vault.create",
            AuditAction::VaultUpdate => "vault.update",
            AuditAction::VaultDelete => "vault.delete",
            AuditAction::OrganizationCreate => "organization.create",
            AuditAction::OrganizationUpdate => "organization.update",
            AuditAction::OrganizationDelete => "organization.delete",
            AuditAction::MemberInvite => "member.invite",
            AuditAction::MemberAdd => "member.add",
            AuditAction::MemberUpdate => "member.update",
            AuditAction::MemberRemove => "member.remove",
            AuditAction::TeamCreate => "team.create",
            AuditAction::TeamUpdate => "team.update",
            AuditAction::TeamDelete => "team.delete",
            AuditAction::TeamMemberAdd => "team.member_add",
            AuditAction::TeamMemberRemove => "team.member_remove",
            AuditAction::TokenCreate => "token.create",
            AuditAction::TokenRevoke => "token.revoke",
            AuditAction::TokenRotate => "token.rotate",
            AuditAction::Login => "login",
            AuditAction::Logout => "logout",
            AuditAction::LoginFailed => "login_failed",
            AuditAction::ApiClientCreate => "api_client.create",
            AuditAction::ApiClientUpdate => "api_client.update",
            AuditAction::ApiClientDelete => "api_client.delete",
        };
        write!(f, "{}", s)
    }
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Action succeeded.
    #[default]
    Success,
    /// Action failed.
    Failure,
    /// Action was denied (authorization failure).
    Denied,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditOutcome::Success => write!(f, "success"),
            AuditOutcome::Failure => write!(f, "failure"),
            AuditOutcome::Denied => write!(f, "denied"),
        }
    }
}

/// Export format for audit logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// JSON format (one event per line).
    #[default]
    Json,
    /// CSV format.
    Csv,
}

/// Request to list audit log events.
pub struct ListAuditLogsRequest {
    client: Client,
    organization_id: String,
    vault_id: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
    actor: Option<String>,
    action: Option<AuditAction>,
    resource: Option<String>,
    after: Option<chrono::DateTime<chrono::Utc>>,
    before: Option<chrono::DateTime<chrono::Utc>>,
}

impl ListAuditLogsRequest {
    /// Filters by vault ID.
    #[must_use]
    pub fn vault(mut self, vault_id: impl Into<String>) -> Self {
        self.vault_id = Some(vault_id.into());
        self
    }

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

    /// Filters by actor ID.
    #[must_use]
    pub fn actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor = Some(actor_id.into());
        self
    }

    /// Filters by action type.
    #[must_use]
    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Filters by resource.
    #[must_use]
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Filters events after the given timestamp.
    #[must_use]
    pub fn after(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Filters events before the given timestamp.
    #[must_use]
    pub fn before(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.before = Some(timestamp);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<AuditEvent>, Error> {
        let mut path = format!(
            "/control/v1/organizations/{}/audit-logs",
            self.organization_id
        );

        let mut query_params = Vec::new();
        if let Some(ref vault_id) = self.vault_id {
            query_params.push(format!("vault_id={}", vault_id));
        }
        if let Some(limit) = self.limit {
            query_params.push(format!("limit={}", limit));
        }
        if let Some(ref cursor) = self.cursor {
            query_params.push(format!("cursor={}", cursor));
        }
        if let Some(ref sort) = self.sort {
            query_params.push(format!("sort={}", sort.as_str()));
        }
        if let Some(ref actor) = self.actor {
            query_params.push(format!("actor={}", actor));
        }
        if let Some(ref action) = self.action {
            query_params.push(format!("action={}", action));
        }
        if let Some(ref resource) = self.resource {
            query_params.push(format!("resource={}", resource));
        }
        if let Some(ref after) = self.after {
            query_params.push(format!("after={}", after.to_rfc3339()));
        }
        if let Some(ref before) = self.before {
            query_params.push(format!("before={}", before.to_rfc3339()));
        }

        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<AuditEvent>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListAuditLogsRequest {
    type Output = Result<Page<AuditEvent>, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to export audit logs.
pub struct ExportAuditLogsRequest {
    client: Client,
    organization_id: String,
    vault_id: Option<String>,
    after: Option<chrono::DateTime<chrono::Utc>>,
    before: Option<chrono::DateTime<chrono::Utc>>,
    format: ExportFormat,
}

impl ExportAuditLogsRequest {
    /// Filters by vault ID.
    #[must_use]
    pub fn vault(mut self, vault_id: impl Into<String>) -> Self {
        self.vault_id = Some(vault_id.into());
        self
    }

    /// Filters events after the given timestamp.
    #[must_use]
    pub fn after(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Filters events before the given timestamp.
    #[must_use]
    pub fn before(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Sets the export format.
    #[must_use]
    pub fn format(mut self, format: ExportFormat) -> Self {
        self.format = format;
        self
    }

    /// Writes the exported audit logs to a file.
    #[cfg(feature = "rest")]
    pub async fn write_to_file(self, file_path: impl AsRef<std::path::Path>) -> Result<(), Error> {
        use crate::error::ErrorKind;
        use std::io::Write;

        let mut api_path = format!(
            "/control/v1/organizations/{}/audit-logs/export",
            self.organization_id
        );

        let mut query_params = Vec::new();
        if let Some(ref vault_id) = self.vault_id {
            query_params.push(format!("vault_id={}", vault_id));
        }
        if let Some(ref after) = self.after {
            query_params.push(format!("after={}", after.to_rfc3339()));
        }
        if let Some(ref before) = self.before {
            query_params.push(format!("before={}", before.to_rfc3339()));
        }
        let format_str = match self.format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
        };
        query_params.push(format!("format={}", format_str));

        if !query_params.is_empty() {
            api_path.push('?');
            api_path.push_str(&query_params.join("&"));
        }

        let data: Vec<AuditEvent> = self.client.inner().control_get(&api_path).await?;

        let file_path = file_path.as_ref();
        let mut file = std::fs::File::create(file_path).map_err(|e| {
            Error::new(ErrorKind::Internal, format!("Failed to create file: {}", e))
        })?;

        match self.format {
            ExportFormat::Json => {
                for event in &data {
                    let line = serde_json::to_string(event).map_err(|e| {
                        Error::new(
                            ErrorKind::InvalidResponse,
                            format!("Failed to serialize event: {}", e),
                        )
                    })?;
                    writeln!(file, "{}", line).map_err(|e| {
                        Error::new(
                            ErrorKind::Internal,
                            format!("Failed to write to file: {}", e),
                        )
                    })?;
                }
            }
            ExportFormat::Csv => {
                // Write CSV header
                writeln!(file, "id,organization_id,vault_id,timestamp,actor_id,actor_type,action,resource,outcome")
                    .map_err(|e| Error::new(ErrorKind::Internal, format!("Failed to write to file: {}", e)))?;
                for event in &data {
                    writeln!(
                        file,
                        "{},{},{},{},{},{},{},{},{}",
                        event.id,
                        event.organization_id,
                        event.vault_id.as_deref().unwrap_or(""),
                        event.timestamp.to_rfc3339(),
                        event.actor.id,
                        event.actor.actor_type,
                        event.action,
                        event.resource.as_deref().unwrap_or(""),
                        event.outcome
                    )
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Internal,
                            format!("Failed to write to file: {}", e),
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Writes the exported audit logs to a file.
    #[cfg(not(feature = "rest"))]
    pub async fn write_to_file(self, _path: impl AsRef<std::path::Path>) -> Result<(), Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Returns a stream of events.
    #[cfg(feature = "rest")]
    pub fn stream(self) -> impl futures::Stream<Item = Result<AuditEvent, Error>> + Send + 'static {
        use futures::StreamExt;

        let client = self.client.clone();
        let organization_id = self.organization_id.clone();
        let vault_id = self.vault_id.clone();
        let after = self.after;
        let before = self.before;

        futures::stream::unfold(
            (
                client,
                organization_id,
                vault_id,
                after,
                before,
                None::<String>,
                false,
            ),
            |(client, org_id, vault_id, after, before, cursor, done)| async move {
                if done {
                    return None;
                }

                let mut path = format!("/control/v1/organizations/{}/audit-logs", org_id);
                let mut query_params = Vec::new();

                if let Some(ref vault_id) = vault_id {
                    query_params.push(format!("vault_id={}", vault_id));
                }
                if let Some(ref after) = after {
                    query_params.push(format!("after={}", after.to_rfc3339()));
                }
                if let Some(ref before) = before {
                    query_params.push(format!("before={}", before.to_rfc3339()));
                }
                if let Some(ref cursor) = cursor {
                    query_params.push(format!("cursor={}", cursor));
                }

                if !query_params.is_empty() {
                    path.push('?');
                    path.push_str(&query_params.join("&"));
                }

                let result: Result<Page<AuditEvent>, Error> =
                    client.inner().control_get(&path).await;

                match result {
                    Ok(page) => {
                        let next_cursor = page.next_cursor().map(|s| s.to_string());
                        let is_done = next_cursor.is_none();
                        let events: Vec<Result<AuditEvent, Error>> =
                            page.items.into_iter().map(Ok).collect();
                        Some((
                            futures::stream::iter(events),
                            (
                                client,
                                org_id,
                                vault_id,
                                after,
                                before,
                                next_cursor,
                                is_done,
                            ),
                        ))
                    }
                    Err(e) => Some((
                        futures::stream::iter(vec![Err(e)]),
                        (client, org_id, vault_id, after, before, None, true),
                    )),
                }
            },
        )
        .flatten()
    }

    /// Returns a stream of events.
    #[cfg(not(feature = "rest"))]
    pub fn stream(self) -> impl futures::Stream<Item = Result<AuditEvent, Error>> + Send + 'static {
        futures::stream::once(async {
            Err(Error::configuration(
                "REST feature is required for control API",
            ))
        })
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
    fn test_actor_type() {
        assert_eq!(ActorType::default(), ActorType::User);
        assert_eq!(ActorType::User.to_string(), "user");
        assert_eq!(ActorType::ApiClient.to_string(), "api_client");
        assert_eq!(ActorType::System.to_string(), "system");
    }

    #[test]
    fn test_audit_action() {
        assert_eq!(AuditAction::default(), AuditAction::Check);
        assert_eq!(AuditAction::Check.to_string(), "check");
        assert_eq!(
            AuditAction::RelationshipWrite.to_string(),
            "relationship.write"
        );
        assert_eq!(AuditAction::SchemaPush.to_string(), "schema.push");
        assert_eq!(
            AuditAction::RelationshipDelete.to_string(),
            "relationship.delete"
        );
        assert_eq!(AuditAction::SchemaActivate.to_string(), "schema.activate");
        assert_eq!(AuditAction::VaultCreate.to_string(), "vault.create");
        assert_eq!(AuditAction::VaultUpdate.to_string(), "vault.update");
        assert_eq!(AuditAction::VaultDelete.to_string(), "vault.delete");
        assert_eq!(AuditAction::MemberInvite.to_string(), "member.invite");
        assert_eq!(AuditAction::MemberUpdate.to_string(), "member.update");
        assert_eq!(AuditAction::MemberRemove.to_string(), "member.remove");
        assert_eq!(AuditAction::TeamCreate.to_string(), "team.create");
        assert_eq!(AuditAction::TeamUpdate.to_string(), "team.update");
        assert_eq!(AuditAction::TeamDelete.to_string(), "team.delete");
    }

    #[test]
    fn test_audit_action_all_variants() {
        // Cover all remaining Display implementations
        assert_eq!(AuditAction::CheckBatch.to_string(), "check_batch");
        assert_eq!(
            AuditAction::RelationshipWriteBatch.to_string(),
            "relationship.write_batch"
        );
        assert_eq!(
            AuditAction::RelationshipDeleteBatch.to_string(),
            "relationship.delete_batch"
        );
        assert_eq!(
            AuditAction::OrganizationCreate.to_string(),
            "organization.create"
        );
        assert_eq!(
            AuditAction::OrganizationUpdate.to_string(),
            "organization.update"
        );
        assert_eq!(
            AuditAction::OrganizationDelete.to_string(),
            "organization.delete"
        );
        assert_eq!(AuditAction::MemberAdd.to_string(), "member.add");
        assert_eq!(AuditAction::TeamMemberAdd.to_string(), "team.member_add");
        assert_eq!(
            AuditAction::TeamMemberRemove.to_string(),
            "team.member_remove"
        );
        assert_eq!(AuditAction::TokenCreate.to_string(), "token.create");
        assert_eq!(AuditAction::TokenRevoke.to_string(), "token.revoke");
        assert_eq!(AuditAction::TokenRotate.to_string(), "token.rotate");
        assert_eq!(AuditAction::Login.to_string(), "login");
        assert_eq!(AuditAction::Logout.to_string(), "logout");
        assert_eq!(AuditAction::LoginFailed.to_string(), "login_failed");
        assert_eq!(
            AuditAction::ApiClientCreate.to_string(),
            "api_client.create"
        );
        assert_eq!(
            AuditAction::ApiClientUpdate.to_string(),
            "api_client.update"
        );
        assert_eq!(
            AuditAction::ApiClientDelete.to_string(),
            "api_client.delete"
        );
    }

    #[test]
    fn test_audit_outcome() {
        assert_eq!(AuditOutcome::default(), AuditOutcome::Success);
        assert_eq!(AuditOutcome::Success.to_string(), "success");
        assert_eq!(AuditOutcome::Failure.to_string(), "failure");
        assert_eq!(AuditOutcome::Denied.to_string(), "denied");
    }

    #[test]
    fn test_export_format() {
        assert_eq!(ExportFormat::default(), ExportFormat::Json);
        // Test Csv variant exists
        let _csv = ExportFormat::Csv;
    }

    #[tokio::test]
    async fn test_audit_logs_client_accessors() {
        let client = create_test_client().await;
        let audit = AuditLogsClient::new(client, "org_test");
        assert_eq!(audit.organization_id(), "org_test");
    }

    #[tokio::test]
    async fn test_audit_logs_client_debug() {
        let client = create_test_client().await;
        let audit = AuditLogsClient::new(client, "org_test");
        let debug = format!("{:?}", audit);
        assert!(debug.contains("AuditLogsClient"));
        assert!(debug.contains("org_test"));
    }

    #[tokio::test]
    async fn test_list_audit_logs_request_builders() {
        let client = create_test_client().await;
        let audit = AuditLogsClient::new(client, "org_test");

        // Test all builder methods
        let now = chrono::Utc::now();
        let _request = audit
            .list()
            .vault("vlt_abc123")
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending)
            .actor("user_123")
            .action(AuditAction::RelationshipWrite)
            .resource("document:readme")
            .after(now - chrono::Duration::hours(24))
            .before(now);

        // Just verify the builder compiles and returns a request
    }

    #[tokio::test]
    async fn test_export_audit_logs_request_builders() {
        let client = create_test_client().await;
        let audit = AuditLogsClient::new(client, "org_test");

        // Test all builder methods
        let now = chrono::Utc::now();
        let _request = audit
            .export()
            .vault("vlt_abc123")
            .after(now - chrono::Duration::hours(24))
            .before(now)
            .format(ExportFormat::Csv);

        // Just verify the builder compiles and returns a request
    }

    // Additional tests for Clone implementations and serde
    #[tokio::test]
    async fn test_audit_logs_client_clone() {
        let client = create_test_client().await;
        let audit = AuditLogsClient::new(client, "org_test");
        let cloned = audit.clone();
        assert_eq!(cloned.organization_id(), "org_test");
    }

    #[test]
    fn test_actor_info_serde() {
        let json = r#"{
            "id": "user_abc123",
            "actor_type": "user",
            "email": "test@example.com",
            "ip_address": "192.168.1.1",
            "user_agent": "Mozilla/5.0"
        }"#;
        let actor: ActorInfo = serde_json::from_str(json).unwrap();
        assert_eq!(actor.id, "user_abc123");
        assert_eq!(actor.actor_type, ActorType::User);
        assert_eq!(actor.email, Some("test@example.com".to_string()));
        assert_eq!(actor.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_actor_info_clone() {
        let actor = ActorInfo {
            id: "user_123".to_string(),
            actor_type: ActorType::ApiClient,
            email: None,
            ip_address: None,
            user_agent: None,
        };
        let cloned = actor.clone();
        assert_eq!(cloned.id, "user_123");
        assert_eq!(cloned.actor_type, ActorType::ApiClient);
    }

    #[test]
    fn test_audit_event_serde() {
        let json = r#"{
            "id": "evt_abc123",
            "organization_id": "org_test",
            "vault_id": "vlt_test",
            "timestamp": "2024-01-01T00:00:00Z",
            "actor": {
                "id": "user_123",
                "actor_type": "user",
                "email": null,
                "ip_address": null,
                "user_agent": null
            },
            "action": "check",
            "resource": "document:readme",
            "details": {"key": "value"},
            "request_id": "req_123",
            "outcome": "success"
        }"#;
        let event: AuditEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "evt_abc123");
        assert_eq!(event.organization_id, "org_test");
        assert_eq!(event.vault_id, Some("vlt_test".to_string()));
        assert_eq!(event.action, AuditAction::Check);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.resource, Some("document:readme".to_string()));
    }

    #[test]
    fn test_audit_event_clone() {
        let event = AuditEvent {
            id: "evt_123".to_string(),
            organization_id: "org_123".to_string(),
            vault_id: None,
            timestamp: chrono::Utc::now(),
            actor: ActorInfo {
                id: "user_123".to_string(),
                actor_type: ActorType::System,
                email: None,
                ip_address: None,
                user_agent: None,
            },
            action: AuditAction::Login,
            resource: None,
            details: None,
            request_id: None,
            outcome: AuditOutcome::Failure,
        };
        let cloned = event.clone();
        assert_eq!(cloned.id, "evt_123");
        assert_eq!(cloned.action, AuditAction::Login);
        assert_eq!(cloned.outcome, AuditOutcome::Failure);
    }

    #[test]
    fn test_audit_action_serde() {
        // Test all action types can be serialized/deserialized
        // Note: serde uses snake_case, so RelationshipWrite becomes "relationship_write"
        let actions = vec![
            (AuditAction::Check, "\"check\""),
            (AuditAction::CheckBatch, "\"check_batch\""),
            (AuditAction::RelationshipWrite, "\"relationship_write\""),
            (AuditAction::RelationshipDelete, "\"relationship_delete\""),
        ];
        for (action, expected) in actions {
            let json = serde_json::to_string(&action).unwrap();
            assert_eq!(json, expected);
            let parsed: AuditAction = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, action);
        }
    }

    #[test]
    fn test_audit_outcome_serde() {
        let outcomes = vec![
            (AuditOutcome::Success, "\"success\""),
            (AuditOutcome::Failure, "\"failure\""),
            (AuditOutcome::Denied, "\"denied\""),
        ];
        for (outcome, expected) in outcomes {
            let json = serde_json::to_string(&outcome).unwrap();
            assert_eq!(json, expected);
            let parsed: AuditOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, outcome);
        }
    }

    #[test]
    fn test_export_format_serde() {
        let json = serde_json::to_string(&ExportFormat::Json).unwrap();
        assert_eq!(json, "\"json\"");
        let json = serde_json::to_string(&ExportFormat::Csv).unwrap();
        assert_eq!(json, "\"csv\"");
    }

    #[test]
    fn test_actor_type_serde() {
        let actors = vec![
            (ActorType::User, "\"user\""),
            (ActorType::ApiClient, "\"api_client\""),
            (ActorType::System, "\"system\""),
        ];
        for (actor, expected) in actors {
            let json = serde_json::to_string(&actor).unwrap();
            assert_eq!(json, expected);
            let parsed: ActorType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, actor);
        }
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
    async fn test_get_audit_event() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs/evt_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "evt_abc",
                "organization_id": "org_123",
                "vault_id": "vlt_456",
                "timestamp": "2024-01-01T00:00:00Z",
                "actor": {
                    "id": "user_123",
                    "actor_type": "user",
                    "email": "user@example.com",
                    "ip_address": "192.168.1.1",
                    "user_agent": "Mozilla/5.0"
                },
                "action": "relationship_write",
                "resource": "document:readme",
                "details": {"key": "value"},
                "request_id": "req_xyz",
                "outcome": "success"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let audit = AuditLogsClient::new(client, "org_123");
        let result = audit.get("evt_abc").await;

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.id, "evt_abc");
        assert_eq!(event.organization_id, "org_123");
        assert_eq!(event.vault_id, Some("vlt_456".to_string()));
        assert_eq!(event.actor.id, "user_123");
        assert_eq!(event.action, AuditAction::RelationshipWrite);
        assert_eq!(event.outcome, AuditOutcome::Success);
    }

    #[tokio::test]
    async fn test_list_audit_events() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "evt_1",
                        "organization_id": "org_123",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "actor": {
                            "id": "user_123",
                            "actor_type": "user"
                        },
                        "action": "check",
                        "outcome": "success"
                    },
                    {
                        "id": "evt_2",
                        "organization_id": "org_123",
                        "timestamp": "2024-01-01T01:00:00Z",
                        "actor": {
                            "id": "api_client_456",
                            "actor_type": "api_client"
                        },
                        "action": "relationship_write",
                        "outcome": "success"
                    }
                ],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 2
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let audit = AuditLogsClient::new(client, "org_123");
        let result = audit.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].id, "evt_1");
        assert_eq!(page.items[1].actor.actor_type, ActorType::ApiClient);
    }

    #[tokio::test]
    async fn test_list_audit_events_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs"))
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
        let audit = AuditLogsClient::new(client, "org_123");
        let now = chrono::Utc::now();
        let result = audit
            .list()
            .vault("vlt_456")
            .limit(10)
            .cursor("cursor_abc")
            .sort(SortOrder::Descending)
            .actor("user_123")
            .action(AuditAction::RelationshipWrite)
            .resource("document:readme")
            .after(now - chrono::Duration::hours(24))
            .before(now)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_export_to_json_file() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs/export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "evt_1",
                    "organization_id": "org_123",
                    "timestamp": "2024-01-01T00:00:00Z",
                    "actor": {
                        "id": "user_123",
                        "actor_type": "user"
                    },
                    "action": "check",
                    "outcome": "success"
                }
            ])))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let audit = AuditLogsClient::new(client, "org_123");

        let temp_file = std::env::temp_dir().join("test_audit_export.json");
        let result = audit
            .export()
            .format(ExportFormat::Json)
            .write_to_file(&temp_file)
            .await;

        assert!(result.is_ok());
        // Clean up
        let _ = std::fs::remove_file(&temp_file);
    }

    #[tokio::test]
    async fn test_export_to_csv_file() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs/export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "evt_1",
                    "organization_id": "org_123",
                    "timestamp": "2024-01-01T00:00:00Z",
                    "actor": {
                        "id": "user_123",
                        "actor_type": "user"
                    },
                    "action": "check",
                    "outcome": "success"
                }
            ])))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let audit = AuditLogsClient::new(client, "org_123");

        let temp_file = std::env::temp_dir().join("test_audit_export.csv");
        let result = audit
            .export()
            .format(ExportFormat::Csv)
            .write_to_file(&temp_file)
            .await;

        assert!(result.is_ok());
        // Clean up
        let _ = std::fs::remove_file(&temp_file);
    }

    #[tokio::test]
    async fn test_export_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs/export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let audit = AuditLogsClient::new(client, "org_123");

        let now = chrono::Utc::now();
        let temp_file = std::env::temp_dir().join("test_audit_export_filters.json");
        let result = audit
            .export()
            .vault("vlt_456")
            .after(now - chrono::Duration::hours(24))
            .before(now)
            .format(ExportFormat::Json)
            .write_to_file(&temp_file)
            .await;

        assert!(result.is_ok());
        // Clean up
        let _ = std::fs::remove_file(&temp_file);
    }

    #[tokio::test]
    async fn test_export_stream() {
        use futures::StreamExt;

        let server = MockServer::start().await;

        // First page
        Mock::given(method("GET"))
            .and(path("/control/v1/organizations/org_123/audit-logs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "evt_1",
                        "organization_id": "org_123",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "actor": {
                            "id": "user_123",
                            "actor_type": "user"
                        },
                        "action": "check",
                        "outcome": "success"
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
        let audit = AuditLogsClient::new(client, "org_123");

        let stream = audit.export().stream();
        let events: Vec<_> = stream.collect().await;

        assert_eq!(events.len(), 1);
        assert!(events[0].is_ok());
    }
}
