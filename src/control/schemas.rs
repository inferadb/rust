//! Schema management for vaults.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::{Page, SortOrder};
use crate::Error;

/// Client for vault schema management operations.
///
/// Access via `vault.schemas()`.
///
/// ## Example
///
/// ```rust,ignore
/// let schemas = vault.schemas();
///
/// // Get active schema
/// let active = schemas.get_active().await?;
///
/// // Push new schema
/// let result = schemas.push(r#"
///     entity User {}
///     entity Document {
///         relations { owner: User }
///         permissions { view: owner, edit: owner }
///     }
/// "#).await?;
///
/// // Activate specific version
/// schemas.activate(&version_id).await?;
/// ```
#[derive(Clone)]
pub struct SchemasClient {
    client: Client,
    organization_id: String,
    vault_id: String,
}

impl SchemasClient {
    /// Creates a new schemas client.
    pub(crate) fn new(
        client: Client,
        organization_id: impl Into<String>,
        vault_id: impl Into<String>,
    ) -> Self {
        Self {
            client,
            organization_id: organization_id.into(),
            vault_id: vault_id.into(),
        }
    }

    /// Returns the organization ID.
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }

    /// Returns the vault ID.
    pub fn vault_id(&self) -> &str {
        &self.vault_id
    }

    /// Gets the currently active schema.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let schema = vault.schemas().get_active().await?;
    /// println!("Version: {}", schema.version);
    /// println!("Content:\n{}", schema.content);
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get_active(&self) -> Result<SchemaInfo, Error> {
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/active",
            self.organization_id, self.vault_id
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets the currently active schema.
    #[cfg(not(feature = "rest"))]
    pub async fn get_active(&self) -> Result<SchemaInfo, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Lists all schema versions.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let schemas = vault.schemas().list().await?;
    /// for schema in schemas.items {
    ///     println!("{}: {:?}", schema.version, schema.status);
    /// }
    /// ```
    pub fn list(&self) -> ListSchemasRequest {
        ListSchemasRequest {
            client: self.client.clone(),
            organization_id: self.organization_id.clone(),
            vault_id: self.vault_id.clone(),
            limit: None,
            cursor: None,
            sort: None,
            status: None,
        }
    }

    /// Gets a specific schema version.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let schema = vault.schemas().get("1").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        let version = version.into();
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/{}",
            self.organization_id, self.vault_id, version
        );
        self.client.inner().control_get(&path).await
    }

    /// Gets a specific schema version.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        let _ = version.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Pushes a new schema version.
    ///
    /// The schema will be validated but not activated. Use [`activate`](Self::activate)
    /// to make it the active schema.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let result = vault.schemas().push(r#"
    ///     entity User {}
    ///     entity Document {
    ///         relations { owner: User, viewer: User }
    ///         permissions { view: viewer | owner, edit: owner }
    ///     }
    /// "#).await?;
    ///
    /// println!("Created version: {}", result.version);
    /// ```
    #[cfg(feature = "rest")]
    pub async fn push(&self, content: impl Into<String>) -> Result<PushSchemaResult, Error> {
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas",
            self.organization_id, self.vault_id
        );
        let body = PushSchemaRequest {
            content: content.into(),
        };
        self.client.inner().control_post(&path, &body).await
    }

    /// Pushes a new schema version.
    #[cfg(not(feature = "rest"))]
    pub async fn push(&self, content: impl Into<String>) -> Result<PushSchemaResult, Error> {
        let _ = content.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Validates a schema without pushing it.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let validation = vault.schemas().validate(schema_content).await?;
    /// if !validation.is_valid() {
    ///     for error in &validation.errors {
    ///         eprintln!("Line {}: {}", error.line, error.message);
    ///     }
    /// }
    /// ```
    #[cfg(feature = "rest")]
    pub async fn validate(&self, content: impl Into<String>) -> Result<ValidationResult, Error> {
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/validate",
            self.organization_id, self.vault_id
        );
        let body = ValidateSchemaRequest {
            content: content.into(),
        };
        self.client.inner().control_post(&path, &body).await
    }

    /// Validates a schema without pushing it.
    #[cfg(not(feature = "rest"))]
    pub async fn validate(&self, content: impl Into<String>) -> Result<ValidationResult, Error> {
        let _ = content.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Activates a specific schema version.
    ///
    /// This makes the specified version the active schema for authorization checks.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// vault.schemas().activate("2").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn activate(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        let version = version.into();
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/{}/activate",
            self.organization_id, self.vault_id, version
        );
        self.client.inner().control_post_empty(&path).await
    }

    /// Activates a specific schema version.
    #[cfg(not(feature = "rest"))]
    pub async fn activate(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        let _ = version.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Deletes a schema version.
    ///
    /// The active schema cannot be deleted. Activate a different version first.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// vault.schemas().delete("1").await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn delete(&self, version: impl Into<String>) -> Result<(), Error> {
        let version = version.into();
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/{}",
            self.organization_id, self.vault_id, version
        );
        self.client.inner().control_delete(&path).await
    }

    /// Deletes a schema version.
    #[cfg(not(feature = "rest"))]
    pub async fn delete(&self, version: impl Into<String>) -> Result<(), Error> {
        let _ = version.into();
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }

    /// Compares two schema versions.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let diff = vault.schemas().diff("1", "2").await?;
    /// for change in &diff.changes {
    ///     println!("{:?}: {}", change.change_type, change.description);
    /// }
    /// ```
    #[cfg(feature = "rest")]
    pub async fn diff(
        &self,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Result<SchemaDiff, Error> {
        let from = from_version.into();
        let to = to_version.into();
        let path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas/diff?from={}&to={}",
            self.organization_id, self.vault_id, from, to
        );
        self.client.inner().control_get(&path).await
    }

    /// Compares two schema versions.
    #[cfg(not(feature = "rest"))]
    pub async fn diff(
        &self,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Result<SchemaDiff, Error> {
        let _ = (from_version.into(), to_version.into());
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::fmt::Debug for SchemasClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemasClient")
            .field("organization_id", &self.organization_id)
            .field("vault_id", &self.vault_id)
            .finish_non_exhaustive()
    }
}

/// Information about a schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// The schema ID.
    pub id: String,
    /// The vault ID.
    pub vault_id: String,
    /// The schema version number.
    pub version: String,
    /// The schema content (IPL source code).
    pub content: String,
    /// The schema status.
    pub status: SchemaStatus,
    /// When the schema was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the schema was activated (if ever).
    pub activated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Status of a schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaStatus {
    /// Schema is currently active.
    Active,
    /// Schema is not active.
    #[default]
    Inactive,
    /// Schema is being activated.
    Activating,
    /// Schema was deprecated.
    Deprecated,
}

impl SchemaStatus {
    /// Returns `true` if the schema is active.
    pub fn is_active(&self) -> bool {
        matches!(self, SchemaStatus::Active)
    }
}

impl std::fmt::Display for SchemaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaStatus::Active => write!(f, "active"),
            SchemaStatus::Inactive => write!(f, "inactive"),
            SchemaStatus::Activating => write!(f, "activating"),
            SchemaStatus::Deprecated => write!(f, "deprecated"),
        }
    }
}

/// Result of pushing a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSchemaResult {
    /// The created schema.
    pub schema: SchemaInfo,
    /// Validation result.
    pub validation: ValidationResult,
}

/// Request body for pushing a schema.
#[derive(Debug, Clone, Serialize)]
struct PushSchemaRequest {
    /// The schema content (IPL source code).
    content: String,
}

/// Request body for validating a schema.
#[derive(Debug, Clone, Serialize)]
struct ValidateSchemaRequest {
    /// The schema content (IPL source code).
    content: String,
}

/// Result of schema validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the schema is valid.
    pub is_valid: bool,
    /// Validation errors.
    pub errors: Vec<ValidationIssue>,
    /// Validation warnings.
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Returns `true` if the schema is valid (no errors).
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Returns `true` if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// A validation issue (error or warning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Line number where the issue occurred (1-indexed).
    pub line: u32,
    /// Column number where the issue occurred (1-indexed).
    pub column: u32,
    /// The issue message.
    pub message: String,
    /// The issue code.
    pub code: String,
}

/// Difference between two schema versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    /// The source version.
    pub from_version: String,
    /// The target version.
    pub to_version: String,
    /// List of changes.
    pub changes: Vec<SchemaChange>,
    /// Whether the change is backward compatible.
    pub is_backward_compatible: bool,
}

/// A single schema change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    /// The type of change.
    pub change_type: SchemaChangeType,
    /// Human-readable description.
    pub description: String,
    /// The entity type affected (if applicable).
    pub entity_type: Option<String>,
    /// The relation affected (if applicable).
    pub relation: Option<String>,
    /// The permission affected (if applicable).
    pub permission: Option<String>,
    /// Whether this is a breaking change.
    pub is_breaking: bool,
}

/// Type of schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaChangeType {
    /// Entity type added.
    EntityAdded,
    /// Entity type removed.
    EntityRemoved,
    /// Relation added.
    RelationAdded,
    /// Relation removed.
    RelationRemoved,
    /// Relation modified.
    RelationModified,
    /// Permission added.
    PermissionAdded,
    /// Permission removed.
    PermissionRemoved,
    /// Permission modified.
    PermissionModified,
}

impl std::fmt::Display for SchemaChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaChangeType::EntityAdded => write!(f, "entity_added"),
            SchemaChangeType::EntityRemoved => write!(f, "entity_removed"),
            SchemaChangeType::RelationAdded => write!(f, "relation_added"),
            SchemaChangeType::RelationRemoved => write!(f, "relation_removed"),
            SchemaChangeType::RelationModified => write!(f, "relation_modified"),
            SchemaChangeType::PermissionAdded => write!(f, "permission_added"),
            SchemaChangeType::PermissionRemoved => write!(f, "permission_removed"),
            SchemaChangeType::PermissionModified => write!(f, "permission_modified"),
        }
    }
}

/// Request to list schemas.
pub struct ListSchemasRequest {
    client: Client,
    organization_id: String,
    vault_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    sort: Option<SortOrder>,
    status: Option<SchemaStatus>,
}

impl ListSchemasRequest {
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
    pub fn status(mut self, status: SchemaStatus) -> Self {
        self.status = Some(status);
        self
    }

    #[cfg(feature = "rest")]
    async fn execute(self) -> Result<Page<SchemaInfo>, Error> {
        let mut path = format!(
            "/control/v1/organizations/{}/vaults/{}/schemas",
            self.organization_id, self.vault_id
        );

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
            query_params.push(format!("status={}", status));
        }

        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client.inner().control_get(&path).await
    }

    #[cfg(not(feature = "rest"))]
    async fn execute(self) -> Result<Page<SchemaInfo>, Error> {
        Err(Error::configuration(
            "REST feature is required for control API",
        ))
    }
}

impl std::future::IntoFuture for ListSchemasRequest {
    type Output = Result<Page<SchemaInfo>, Error>;
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
    fn test_schema_status() {
        assert_eq!(SchemaStatus::default(), SchemaStatus::Inactive);
        assert!(SchemaStatus::Active.is_active());
        assert!(!SchemaStatus::Inactive.is_active());
        assert!(!SchemaStatus::Deprecated.is_active());
        assert_eq!(SchemaStatus::Active.to_string(), "active");
        assert_eq!(SchemaStatus::Inactive.to_string(), "inactive");
        assert_eq!(SchemaStatus::Deprecated.to_string(), "deprecated");
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        assert!(valid.is_valid());
        assert!(!valid.has_warnings());

        let with_warnings = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![ValidationIssue {
                line: 1,
                column: 1,
                message: "Unused entity".to_string(),
                code: "W001".to_string(),
            }],
        };
        assert!(with_warnings.is_valid());
        assert!(with_warnings.has_warnings());

        let invalid = ValidationResult {
            is_valid: false,
            errors: vec![ValidationIssue {
                line: 5,
                column: 10,
                message: "Syntax error".to_string(),
                code: "E001".to_string(),
            }],
            warnings: vec![],
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_schema_change_type() {
        assert_eq!(SchemaChangeType::EntityAdded.to_string(), "entity_added");
        assert_eq!(
            SchemaChangeType::EntityRemoved.to_string(),
            "entity_removed"
        );
        assert_eq!(
            SchemaChangeType::RelationAdded.to_string(),
            "relation_added"
        );
        assert_eq!(
            SchemaChangeType::RelationRemoved.to_string(),
            "relation_removed"
        );
        assert_eq!(
            SchemaChangeType::RelationModified.to_string(),
            "relation_modified"
        );
        assert_eq!(
            SchemaChangeType::PermissionAdded.to_string(),
            "permission_added"
        );
        assert_eq!(
            SchemaChangeType::PermissionRemoved.to_string(),
            "permission_removed"
        );
        assert_eq!(
            SchemaChangeType::PermissionModified.to_string(),
            "permission_modified"
        );
    }

    #[tokio::test]
    async fn test_schemas_client_accessors() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        assert_eq!(schemas.organization_id(), "org_test");
        assert_eq!(schemas.vault_id(), "vlt_abc123");
    }

    #[tokio::test]
    async fn test_schemas_client_debug() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let debug = format!("{:?}", schemas);
        assert!(debug.contains("SchemasClient"));
        assert!(debug.contains("org_test"));
        assert!(debug.contains("vlt_abc123"));
    }

    #[tokio::test]
    async fn test_list_schemas_request_builders() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");

        // Test all builder methods
        let _request = schemas
            .list()
            .limit(50)
            .cursor("cursor_xyz")
            .sort(SortOrder::Descending)
            .status(SchemaStatus::Active);

        // Just verify the builder compiles and returns a request
    }

    #[test]
    fn test_schema_status_activating() {
        assert_eq!(SchemaStatus::Activating.to_string(), "activating");
        assert!(!SchemaStatus::Activating.is_active());
    }

    #[tokio::test]
    async fn test_schemas_client_clone() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let cloned = schemas.clone();
        assert_eq!(cloned.organization_id(), "org_test");
        assert_eq!(cloned.vault_id(), "vlt_abc123");
    }

    #[test]
    fn test_schema_info_clone() {
        let info = SchemaInfo {
            id: "sch_123".to_string(),
            vault_id: "vlt_abc".to_string(),
            version: "1".to_string(),
            content: "entity user {}".to_string(),
            status: SchemaStatus::Active,
            created_at: chrono::Utc::now(),
            activated_at: Some(chrono::Utc::now()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, "sch_123");
        assert_eq!(cloned.version, "1");
    }

    #[test]
    fn test_push_schema_result_clone() {
        let result = PushSchemaResult {
            schema: SchemaInfo {
                id: "sch_123".to_string(),
                vault_id: "vlt_abc".to_string(),
                version: "1".to_string(),
                content: "entity user {}".to_string(),
                status: SchemaStatus::Inactive,
                created_at: chrono::Utc::now(),
                activated_at: None,
            },
            validation: ValidationResult {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            },
        };
        let cloned = result.clone();
        assert_eq!(cloned.schema.id, "sch_123");
        assert!(cloned.validation.is_valid());
    }

    #[test]
    fn test_validation_issue_clone() {
        let issue = ValidationIssue {
            line: 10,
            column: 5,
            message: "test error".to_string(),
            code: "E001".to_string(),
        };
        let cloned = issue.clone();
        assert_eq!(cloned.line, 10);
        assert_eq!(cloned.column, 5);
        assert_eq!(cloned.message, "test error");
    }

    #[test]
    fn test_schema_diff_clone() {
        let diff = SchemaDiff {
            from_version: "1".to_string(),
            to_version: "2".to_string(),
            changes: vec![SchemaChange {
                change_type: SchemaChangeType::EntityAdded,
                description: "Added User entity".to_string(),
                entity_type: Some("User".to_string()),
                relation: None,
                permission: None,
                is_breaking: false,
            }],
            is_backward_compatible: true,
        };
        let cloned = diff.clone();
        assert_eq!(cloned.from_version, "1");
        assert_eq!(cloned.changes.len(), 1);
    }

    #[test]
    fn test_schema_change_clone() {
        let change = SchemaChange {
            change_type: SchemaChangeType::RelationRemoved,
            description: "Removed viewer relation".to_string(),
            entity_type: Some("Document".to_string()),
            relation: Some("viewer".to_string()),
            permission: None,
            is_breaking: true,
        };
        let cloned = change.clone();
        assert_eq!(cloned.change_type, SchemaChangeType::RelationRemoved);
        assert!(cloned.is_breaking);
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
    async fn test_get_active_schema() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/active",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sch_abc",
                "vault_id": "vlt_456",
                "version": "2",
                "content": "entity User {}",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "activated_at": "2024-01-02T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.get_active().await;

        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.version, "2");
        assert!(schema.status.is_active());
    }

    #[tokio::test]
    async fn test_list_schemas() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "sch_1",
                        "vault_id": "vlt_456",
                        "version": "1",
                        "content": "entity User {}",
                        "status": "inactive",
                        "created_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "sch_2",
                        "vault_id": "vlt_456",
                        "version": "2",
                        "content": "entity User {} entity Doc {}",
                        "status": "active",
                        "created_at": "2024-01-02T00:00:00Z",
                        "activated_at": "2024-01-02T01:00:00Z"
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
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn test_list_schemas_with_filters() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas",
            ))
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
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas
            .list()
            .limit(10)
            .cursor("cursor_abc")
            .sort(SortOrder::Descending)
            .status(SchemaStatus::Active)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_schema_by_version() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sch_1",
                "vault_id": "vlt_456",
                "version": "1",
                "content": "entity User {}",
                "status": "inactive",
                "created_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.get("1").await;

        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.version, "1");
    }

    #[tokio::test]
    async fn test_push_schema() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "schema": {
                    "id": "sch_new",
                    "vault_id": "vlt_456",
                    "version": "3",
                    "content": "entity NewUser {}",
                    "status": "inactive",
                    "created_at": "2024-01-03T00:00:00Z"
                },
                "validation": {
                    "is_valid": true,
                    "errors": [],
                    "warnings": []
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.push("entity NewUser {}").await;

        assert!(result.is_ok());
        let push_result = result.unwrap();
        assert_eq!(push_result.schema.version, "3");
        assert!(push_result.validation.is_valid());
    }

    #[tokio::test]
    async fn test_validate_schema() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/validate",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "is_valid": true,
                "errors": [],
                "warnings": [
                    {
                        "line": 1,
                        "column": 1,
                        "message": "Consider adding relations",
                        "code": "W001"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.validate("entity User {}").await;

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(validation.is_valid());
        assert_eq!(validation.warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_activate_schema() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/2/activate",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sch_2",
                "vault_id": "vlt_456",
                "version": "2",
                "content": "entity User {}",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "activated_at": "2024-01-02T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.activate("2").await;

        assert!(result.is_ok());
        let schema = result.unwrap();
        assert!(schema.status.is_active());
    }

    #[tokio::test]
    async fn test_delete_schema() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/1",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.delete("1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_diff_schemas() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/control/v1/organizations/org_123/vaults/vlt_456/schemas/diff",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "from_version": "1",
                "to_version": "2",
                "changes": [
                    {
                        "change_type": "entity_added",
                        "description": "Added Document entity",
                        "entity_type": "Document",
                        "is_breaking": false
                    }
                ],
                "is_backward_compatible": true
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let schemas = SchemasClient::new(client, "org_123", "vlt_456");
        let result = schemas.diff("1", "2").await;

        assert!(result.is_ok());
        let diff = result.unwrap();
        assert_eq!(diff.changes.len(), 1);
        assert!(diff.is_backward_compatible);
    }
}
