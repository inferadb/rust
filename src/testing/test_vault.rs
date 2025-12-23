//! TestVault for in-memory E2E testing.
//!
//! `TestVault` provides an in-memory implementation that behaves like a real vault
//! but uses the MockTransport internally for testing without network calls.

use std::sync::Arc;

use crate::transport::mock::MockTransport;
use crate::transport::traits::TransportClient;
use crate::types::{ConsistencyToken, Context, Relationship};
use crate::Error;

/// An in-memory vault for testing.
///
/// `TestVault` provides the same API as `VaultClient` but operates entirely
/// in-memory without network calls. It's designed for testing authorization
/// logic without infrastructure dependencies.
///
/// ## Thread Safety
///
/// `TestVault` is `Clone` and thread-safe. Multiple clones share the same
/// underlying state, which is useful for testing concurrent access patterns.
///
/// ## Example
///
/// ```rust,ignore
/// use inferadb::testing::TestVault;
/// use inferadb::Relationship;
///
/// let vault = TestVault::new();
///
/// // Set up relationships
/// vault.add_relationship(Relationship::new("doc:1", "owner", "user:alice"));
///
/// // Test authorization
/// assert!(vault.check("user:alice", "owner", "doc:1").await?);
/// assert!(!vault.check("user:bob", "owner", "doc:1").await?);
/// ```
#[derive(Clone)]
pub struct TestVault {
    transport: Arc<MockTransport>,
}

impl TestVault {
    /// Creates a new empty test vault.
    pub fn new() -> Self {
        Self {
            transport: Arc::new(MockTransport::new()),
        }
    }

    /// Creates a test vault pre-populated with relationships.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::testing::TestVault;
    /// use inferadb::Relationship;
    ///
    /// let vault = TestVault::with_relationships(vec![
    ///     Relationship::new("doc:1", "viewer", "user:alice"),
    ///     Relationship::new("doc:1", "editor", "user:bob"),
    /// ]);
    /// ```
    pub fn with_relationships<'a>(relationships: Vec<Relationship<'a>>) -> Self {
        let vault = Self::new();
        for rel in relationships {
            vault.transport.add_relationship(rel.into_owned());
        }
        vault
    }

    /// Adds a relationship to the test vault.
    ///
    /// This is a synchronous method for convenient test setup.
    pub fn add_relationship<'a>(&self, relationship: Relationship<'a>) {
        self.transport.add_relationship(relationship.into_owned());
    }

    /// Clears all relationships from the vault.
    pub fn clear(&self) {
        self.transport.clear_relationships();
    }

    /// Sets the vault to simulate a failure on the next operation.
    ///
    /// The failure is consumed on use - subsequent operations will succeed
    /// unless you call this again.
    pub fn set_failure(&self, error: Error) {
        self.transport.set_failure(error);
    }

    /// Returns the number of transport requests made.
    ///
    /// Useful for verifying test behavior, e.g., ensuring batching works.
    pub fn request_count(&self) -> u64 {
        self.transport.request_count()
    }

    /// Checks if a subject has a permission on a resource.
    pub async fn check(
        &self,
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> Result<bool, Error> {
        let request = crate::transport::traits::CheckRequest {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            context: None,
            consistency: None,
            trace: false,
        };
        let response = self.transport.check(request).await?;
        Ok(response.allowed)
    }

    /// Checks if a subject has a permission on a resource with ABAC context.
    pub async fn check_with_context(
        &self,
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
        context: Context,
    ) -> Result<bool, Error> {
        let request = crate::transport::traits::CheckRequest {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            context: Some(context),
            consistency: None,
            trace: false,
        };
        let response = self.transport.check(request).await?;
        Ok(response.allowed)
    }

    /// Returns a relationships client for the test vault.
    pub fn relationships(&self) -> TestRelationshipsClient {
        TestRelationshipsClient {
            transport: self.transport.clone(),
        }
    }

    /// Returns a resources query client for the test vault.
    pub fn resources(&self) -> TestResourcesClient {
        TestResourcesClient {
            transport: self.transport.clone(),
        }
    }

    /// Returns a subjects query client for the test vault.
    pub fn subjects(&self) -> TestSubjectsClient {
        TestSubjectsClient {
            transport: self.transport.clone(),
        }
    }
}

impl Default for TestVault {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TestVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestVault")
            .field("request_count", &self.request_count())
            .finish()
    }
}

/// Client for managing relationships in a test vault.
pub struct TestRelationshipsClient {
    transport: Arc<MockTransport>,
}

impl TestRelationshipsClient {
    /// Writes a relationship to the vault.
    pub async fn write<'a>(
        &self,
        relationship: Relationship<'a>,
    ) -> Result<ConsistencyToken, Error> {
        let request = crate::transport::traits::WriteRequest {
            relationship: relationship.into_owned(),
            idempotency_key: None,
        };
        let response = self.transport.write(request).await?;
        Ok(response.consistency_token)
    }

    /// Writes multiple relationships to the vault.
    pub async fn write_batch<'a>(
        &self,
        relationships: Vec<Relationship<'a>>,
    ) -> Result<ConsistencyToken, Error> {
        let requests: Vec<_> = relationships
            .into_iter()
            .map(|r| crate::transport::traits::WriteRequest {
                relationship: r.into_owned(),
                idempotency_key: None,
            })
            .collect();
        let response = self.transport.write_batch(requests).await?;
        Ok(response.consistency_token)
    }

    /// Deletes a relationship from the vault.
    pub async fn delete<'a>(&self, relationship: Relationship<'a>) -> Result<(), Error> {
        self.transport.delete(relationship.into_owned()).await
    }

    /// Lists relationships matching the given filters.
    pub async fn list(
        &self,
        resource: Option<&str>,
        relation: Option<&str>,
        subject: Option<&str>,
    ) -> Result<Vec<Relationship<'static>>, Error> {
        let response = self
            .transport
            .list_relationships(resource, relation, subject, None, None)
            .await?;
        Ok(response.relationships)
    }
}

/// Client for querying resources in a test vault.
pub struct TestResourcesClient {
    transport: Arc<MockTransport>,
}

impl TestResourcesClient {
    /// Returns resources accessible by a subject with a permission.
    pub async fn accessible_by(
        &self,
        subject: &str,
        permission: &str,
        resource_type: Option<&str>,
    ) -> Result<Vec<String>, Error> {
        let response = self
            .transport
            .list_resources(subject, permission, resource_type, None, None)
            .await?;
        Ok(response.resources)
    }
}

/// Client for querying subjects in a test vault.
pub struct TestSubjectsClient {
    transport: Arc<MockTransport>,
}

impl TestSubjectsClient {
    /// Returns subjects with a permission on a resource.
    pub async fn with_permission(
        &self,
        permission: &str,
        resource: &str,
        subject_type: Option<&str>,
    ) -> Result<Vec<String>, Error> {
        let response = self
            .transport
            .list_subjects(permission, resource, subject_type, None, None)
            .await?;
        Ok(response.subjects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vault_basic_check() {
        let vault = TestVault::new();

        // No relationships, should deny
        let allowed = vault.check("user:alice", "view", "doc:1").await.unwrap();
        assert!(!allowed);

        // Add relationship
        vault.add_relationship(Relationship::new("doc:1", "view", "user:alice"));

        // Now should allow
        let allowed = vault.check("user:alice", "view", "doc:1").await.unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_vault_with_relationships() {
        let vault = TestVault::with_relationships(vec![
            Relationship::new("doc:1", "view", "user:alice"),
            Relationship::new("doc:1", "edit", "user:bob"),
        ]);

        assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());
        assert!(!vault.check("user:alice", "edit", "doc:1").await.unwrap());
        assert!(vault.check("user:bob", "edit", "doc:1").await.unwrap());
    }

    #[tokio::test]
    async fn test_vault_relationships_client() {
        let vault = TestVault::new();
        let rels = vault.relationships();

        // Write a relationship
        let token = rels
            .write(Relationship::new("doc:1", "viewer", "user:alice"))
            .await
            .unwrap();
        assert!(!token.is_empty());

        // List relationships
        let list = rels.list(Some("doc:1"), None, None).await.unwrap();
        assert_eq!(list.len(), 1);

        // Delete the relationship
        rels.delete(Relationship::new("doc:1", "viewer", "user:alice"))
            .await
            .unwrap();

        // Should be empty now
        let list = rels.list(Some("doc:1"), None, None).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_vault_failure_simulation() {
        let vault = TestVault::new();
        vault.set_failure(Error::unavailable("simulated outage"));

        let result = vault.check("user:alice", "view", "doc:1").await;
        assert!(result.is_err());

        // Next call should succeed
        let result = vault.check("user:alice", "view", "doc:1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_vault_clear() {
        let vault = TestVault::new();
        vault.add_relationship(Relationship::new("doc:1", "view", "user:alice"));
        assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());

        vault.clear();
        assert!(!vault.check("user:alice", "view", "doc:1").await.unwrap());
    }

    #[tokio::test]
    async fn test_vault_request_count() {
        let vault = TestVault::new();
        assert_eq!(vault.request_count(), 0);

        vault.check("user:alice", "view", "doc:1").await.unwrap();
        assert_eq!(vault.request_count(), 1);

        vault.check("user:bob", "edit", "doc:2").await.unwrap();
        assert_eq!(vault.request_count(), 2);
    }

    #[test]
    fn test_vault_default() {
        let vault = TestVault::default();
        assert_eq!(vault.request_count(), 0);
    }

    #[test]
    fn test_vault_debug() {
        let vault = TestVault::new();
        let debug = format!("{:?}", vault);
        assert!(debug.contains("TestVault"));
        assert!(debug.contains("request_count"));
    }

    #[test]
    fn test_vault_clone() {
        let vault1 = TestVault::new();
        vault1.add_relationship(Relationship::new("doc:1", "view", "user:alice"));

        let vault2 = vault1.clone();
        // Clones share state
        assert_eq!(vault1.request_count(), vault2.request_count());
    }

    #[tokio::test]
    async fn test_vault_check_with_context() {
        let vault = TestVault::new();
        vault.add_relationship(Relationship::new("doc:1", "view", "user:alice"));

        let context = Context::new().with("env", "production");
        let allowed = vault
            .check_with_context("user:alice", "view", "doc:1", context)
            .await
            .unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_vault_relationships_write_batch() {
        let vault = TestVault::new();
        let rels = vault.relationships();

        let token = rels
            .write_batch(vec![
                Relationship::new("doc:1", "viewer", "user:alice"),
                Relationship::new("doc:2", "editor", "user:bob"),
            ])
            .await
            .unwrap();
        assert!(!token.is_empty());

        let list = rels.list(None, None, None).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_vault_resources_accessible_by() {
        let vault = TestVault::new();
        vault.add_relationship(Relationship::new("doc:1", "view", "user:alice"));
        vault.add_relationship(Relationship::new("doc:2", "view", "user:alice"));

        let resources = vault.resources();
        let accessible = resources
            .accessible_by("user:alice", "view", None)
            .await
            .unwrap();
        assert_eq!(accessible.len(), 2);
    }

    #[tokio::test]
    async fn test_vault_subjects_with_permission() {
        let vault = TestVault::new();
        vault.add_relationship(Relationship::new("doc:1", "view", "user:alice"));
        vault.add_relationship(Relationship::new("doc:1", "view", "user:bob"));

        let subjects = vault.subjects();
        let with_perm = subjects
            .with_permission("view", "doc:1", None)
            .await
            .unwrap();
        assert_eq!(with_perm.len(), 2);
    }
}
