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
    pub async fn get_active(&self) -> Result<SchemaInfo, Error> {
        // TODO: Implement actual API call
        Ok(SchemaInfo {
            id: format!("sch_{}", uuid::Uuid::new_v4()),
            vault_id: self.vault_id.clone(),
            version: "1".to_string(),
            content: String::new(),
            status: SchemaStatus::Active,
            created_at: chrono::Utc::now(),
            activated_at: Some(chrono::Utc::now()),
        })
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
    pub async fn get(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        // TODO: Implement actual API call
        let version = version.into();
        Ok(SchemaInfo {
            id: format!("sch_{}", uuid::Uuid::new_v4()),
            vault_id: self.vault_id.clone(),
            version,
            content: String::new(),
            status: SchemaStatus::Inactive,
            created_at: chrono::Utc::now(),
            activated_at: None,
        })
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
    pub async fn push(&self, content: impl Into<String>) -> Result<PushSchemaResult, Error> {
        // TODO: Implement actual API call
        let content = content.into();
        Ok(PushSchemaResult {
            schema: SchemaInfo {
                id: format!("sch_{}", uuid::Uuid::new_v4()),
                vault_id: self.vault_id.clone(),
                version: "2".to_string(),
                content,
                status: SchemaStatus::Inactive,
                created_at: chrono::Utc::now(),
                activated_at: None,
            },
            validation: ValidationResult {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            },
        })
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
    pub async fn validate(&self, content: impl Into<String>) -> Result<ValidationResult, Error> {
        // TODO: Implement actual API call
        let _ = content.into();
        Ok(ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        })
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
    pub async fn activate(&self, version: impl Into<String>) -> Result<SchemaInfo, Error> {
        // TODO: Implement actual API call
        let version = version.into();
        Ok(SchemaInfo {
            id: format!("sch_{}", uuid::Uuid::new_v4()),
            vault_id: self.vault_id.clone(),
            version,
            content: String::new(),
            status: SchemaStatus::Active,
            created_at: chrono::Utc::now(),
            activated_at: Some(chrono::Utc::now()),
        })
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
    pub async fn delete(&self, version: impl Into<String>) -> Result<(), Error> {
        // TODO: Implement actual API call
        let _ = (version.into(), &self.client);
        Ok(())
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
    pub async fn diff(
        &self,
        from_version: impl Into<String>,
        to_version: impl Into<String>,
    ) -> Result<SchemaDiff, Error> {
        // TODO: Implement actual API call
        let _ = (from_version.into(), to_version.into());
        Ok(SchemaDiff {
            from_version: "1".to_string(),
            to_version: "2".to_string(),
            changes: vec![],
            is_backward_compatible: true,
        })
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

    async fn execute(self) -> Result<Page<SchemaInfo>, Error> {
        // TODO: Implement actual API call
        let _ = (
            &self.client,
            &self.organization_id,
            &self.vault_id,
            self.limit,
            self.cursor,
            self.sort,
            self.status,
        );
        Ok(Page::default())
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

    async fn create_test_client() -> Client {
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
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
    async fn test_schemas_get_active() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let info = schemas.get_active().await.unwrap();
        assert_eq!(info.status, SchemaStatus::Active);
    }

    #[tokio::test]
    async fn test_schemas_get() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let info = schemas.get("v1.0.0").await.unwrap();
        assert_eq!(info.version, "v1.0.0");
        assert_eq!(info.vault_id, "vlt_abc123");
    }

    #[tokio::test]
    async fn test_schemas_list() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let page = schemas.list().await.unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_schemas_list_with_options() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let page = schemas
            .list()
            .limit(10)
            .cursor("cursor123")
            .sort(SortOrder::Descending)
            .status(SchemaStatus::Active)
            .await
            .unwrap();
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_schemas_push() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let result = schemas.push("entity user {}").await.unwrap();
        assert!(result.validation.is_valid);
    }

    #[tokio::test]
    async fn test_schemas_validate() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let result = schemas.validate("entity user {}").await.unwrap();
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_schemas_activate() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let result = schemas.activate("v1.0.0").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_schemas_diff() {
        let client = create_test_client().await;
        let schemas = SchemasClient::new(client, "org_test", "vlt_abc123");
        let diff = schemas.diff("v1.0.0", "v1.1.0").await.unwrap();
        // The mock returns hardcoded "1" and "2" for now
        assert_eq!(diff.from_version, "1");
        assert_eq!(diff.to_version, "2");
        assert!(diff.is_backward_compatible);
    }
}
