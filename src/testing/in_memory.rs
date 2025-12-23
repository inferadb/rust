//! InMemoryClient for testing with real graph semantics.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use crate::testing::AuthorizationClient;
use crate::types::{Context, Relationship};
use crate::Error;

/// An in-memory authorization client with real graph semantics.
///
/// Unlike [`MockClient`](super::MockClient), `InMemoryClient` actually stores
/// relationships and performs graph traversal to compute permissions.
///
/// This is useful for integration tests where you want to test actual
/// authorization logic without hitting a real server.
///
/// ## Example
///
/// ```rust
/// use inferadb::testing::InMemoryClient;
/// use inferadb::Relationship;
///
/// let client = InMemoryClient::new();
///
/// // Add relationships
/// client.write(Relationship::new("doc:1", "viewer", "user:alice"));
/// client.write(Relationship::new("doc:1", "editor", "user:bob"));
///
/// // Check permissions (direct relationships only for now)
/// // Full graph traversal will be implemented in Phase 7
/// ```
///
/// ## Limitations
///
/// Current implementation only supports direct relationship lookups.
/// Full graph traversal with schema-based permission computation
/// will be added in Phase 7.
#[derive(Clone)]
pub struct InMemoryClient {
    relationships: Arc<RwLock<HashSet<StoredRelationship>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StoredRelationship {
    resource: String,
    relation: String,
    subject: String,
}

impl From<&Relationship<'_>> for StoredRelationship {
    fn from(rel: &Relationship<'_>) -> Self {
        Self {
            resource: rel.resource().to_string(),
            relation: rel.relation().to_string(),
            subject: rel.subject().to_string(),
        }
    }
}

impl InMemoryClient {
    /// Creates a new in-memory client.
    pub fn new() -> Self {
        Self {
            relationships: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Writes a relationship to the in-memory store.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::testing::InMemoryClient;
    /// use inferadb::Relationship;
    ///
    /// let client = InMemoryClient::new();
    /// client.write(Relationship::new("doc:1", "viewer", "user:alice"));
    /// ```
    pub fn write(&self, relationship: Relationship<'_>) {
        let stored = StoredRelationship::from(&relationship);
        self.relationships.write().unwrap().insert(stored);
    }

    /// Writes multiple relationships to the in-memory store.
    pub fn write_all<'a>(&self, relationships: impl IntoIterator<Item = Relationship<'a>>) {
        let mut store = self.relationships.write().unwrap();
        for rel in relationships {
            store.insert(StoredRelationship::from(&rel));
        }
    }

    /// Deletes a relationship from the in-memory store.
    ///
    /// Returns `true` if the relationship existed.
    pub fn delete(&self, relationship: &Relationship<'_>) -> bool {
        let stored = StoredRelationship::from(relationship);
        self.relationships.write().unwrap().remove(&stored)
    }

    /// Clears all relationships from the store.
    pub fn clear(&self) {
        self.relationships.write().unwrap().clear();
    }

    /// Returns the number of stored relationships.
    pub fn len(&self) -> usize {
        self.relationships.read().unwrap().len()
    }

    /// Returns `true` if there are no stored relationships.
    pub fn is_empty(&self) -> bool {
        self.relationships.read().unwrap().is_empty()
    }

    /// Checks if a direct relationship exists.
    ///
    /// Note: This only checks for exact relationship matches.
    /// Full permission computation with graph traversal will be
    /// implemented in Phase 7.
    fn has_direct_relationship(&self, resource: &str, relation: &str, subject: &str) -> bool {
        let store = self.relationships.read().unwrap();
        store.contains(&StoredRelationship {
            resource: resource.to_string(),
            relation: relation.to_string(),
            subject: subject.to_string(),
        })
    }
}

impl Default for InMemoryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorizationClient for InMemoryClient {
    fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
        // For now, just check direct relationships
        // Full permission computation will be implemented in Phase 7
        let result = self.has_direct_relationship(resource, permission, subject);
        Box::pin(async move { Ok(result) })
    }

    fn check_with_context(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
        _context: &Context,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
        // Context-based conditions will be implemented in Phase 7
        self.check(subject, permission, resource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_client_new() {
        let client = InMemoryClient::new();
        assert!(client.is_empty());
        assert_eq!(client.len(), 0);
    }

    #[test]
    fn test_in_memory_client_write() {
        let client = InMemoryClient::new();
        client.write(Relationship::new("doc:1", "viewer", "user:alice"));
        assert_eq!(client.len(), 1);
    }

    #[test]
    fn test_in_memory_client_write_all() {
        let client = InMemoryClient::new();
        client.write_all(vec![
            Relationship::new("doc:1", "viewer", "user:alice"),
            Relationship::new("doc:1", "editor", "user:bob"),
        ]);
        assert_eq!(client.len(), 2);
    }

    #[test]
    fn test_in_memory_client_delete() {
        let client = InMemoryClient::new();
        let rel = Relationship::new("doc:1", "viewer", "user:alice");
        client.write(rel.as_borrowed());

        assert!(client.delete(&rel));
        assert!(client.is_empty());
        assert!(!client.delete(&rel)); // Already deleted
    }

    #[test]
    fn test_in_memory_client_clear() {
        let client = InMemoryClient::new();
        client.write_all(vec![
            Relationship::new("doc:1", "viewer", "user:alice"),
            Relationship::new("doc:2", "viewer", "user:bob"),
        ]);

        client.clear();
        assert!(client.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_client_check_direct() {
        let client = InMemoryClient::new();
        client.write(Relationship::new("doc:1", "viewer", "user:alice"));

        // Direct relationship exists
        let result = client.check("user:alice", "viewer", "doc:1").await.unwrap();
        assert!(result);

        // Different user
        let result = client.check("user:bob", "viewer", "doc:1").await.unwrap();
        assert!(!result);

        // Different permission
        let result = client.check("user:alice", "editor", "doc:1").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_in_memory_client_check_with_context() {
        let client = InMemoryClient::new();
        client.write(Relationship::new("doc:1", "viewer", "user:alice"));

        let context = Context::new().with("env", "test");

        // Match: context is ignored for now
        let result = client
            .check_with_context("user:alice", "viewer", "doc:1", &context)
            .await
            .unwrap();
        assert!(result);

        // No match
        let result = client
            .check_with_context("user:bob", "viewer", "doc:1", &context)
            .await
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_in_memory_client_clone() {
        let client = InMemoryClient::new();
        client.write(Relationship::new("doc:1", "viewer", "user:alice"));

        let cloned = client.clone();
        cloned.write(Relationship::new("doc:2", "viewer", "user:bob"));

        // Both clients share the same underlying store
        assert_eq!(client.len(), 2);
        assert_eq!(cloned.len(), 2);
    }
}
