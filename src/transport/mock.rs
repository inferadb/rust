//! Mock transport implementation for testing.
//!
//! This module provides a mock transport that operates entirely in-memory,
//! allowing tests to run without network dependencies.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;

use super::traits::{
    CheckRequest, CheckResponse, ListRelationshipsResponse, ListResourcesResponse,
    ListSubjectsResponse, SimulateRequest, SimulateResponse, Transport, TransportClient,
    TransportStats, WriteRequest, WriteResponse,
};
use crate::types::{ConsistencyToken, Decision, Relationship};
use crate::Error;

/// Mock transport for testing.
///
/// This transport operates entirely in-memory and can be configured
/// with expected responses for testing.
pub struct MockTransport {
    /// Stored relationships.
    relationships: RwLock<Vec<Relationship<'static>>>,
    /// Request counter.
    request_count: AtomicU64,
    /// Whether to simulate failures.
    simulate_failure: RwLock<Option<Error>>,
}

impl MockTransport {
    /// Creates a new mock transport.
    pub fn new() -> Self {
        Self {
            relationships: RwLock::new(Vec::new()),
            request_count: AtomicU64::new(0),
            simulate_failure: RwLock::new(None),
        }
    }

    /// Sets a failure to simulate on the next request.
    pub fn set_failure(&self, error: Error) {
        *self.simulate_failure.write() = Some(error);
    }

    /// Clears any simulated failure.
    pub fn clear_failure(&self) {
        *self.simulate_failure.write() = None;
    }

    /// Returns the number of requests made.
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Adds a relationship to the mock store.
    pub fn add_relationship(&self, relationship: Relationship<'static>) {
        self.relationships.write().push(relationship);
    }

    /// Clears all relationships.
    pub fn clear_relationships(&self) {
        self.relationships.write().clear();
    }

    /// Checks if a failure should be simulated.
    fn check_failure(&self) -> Result<(), Error> {
        let failure = self.simulate_failure.write().take();
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(())
    }

    /// Increments the request counter.
    fn increment_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TransportClient for MockTransport {
    async fn check(&self, request: CheckRequest) -> Result<CheckResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        // Simple mock: check if there's a relationship that grants access
        let relationships = self.relationships.read();
        let allowed = relationships.iter().any(|rel| {
            rel.subject() == request.subject
                && rel.relation() == request.permission
                && rel.resource() == request.resource
        });

        Ok(CheckResponse {
            allowed,
            decision: Decision::new(allowed),
            trace: None, // Mock doesn't provide trace data
        })
    }

    async fn check_batch(&self, requests: Vec<CheckRequest>) -> Result<Vec<CheckResponse>, Error> {
        self.increment_requests();
        self.check_failure()?;

        let mut results = Vec::with_capacity(requests.len());
        for request in requests {
            let relationships = self.relationships.read();
            let allowed = relationships.iter().any(|rel| {
                rel.subject() == request.subject
                    && rel.relation() == request.permission
                    && rel.resource() == request.resource
            });
            results.push(CheckResponse {
                allowed,
                decision: Decision::new(allowed),
                trace: None, // Mock doesn't provide trace data
            });
        }

        Ok(results)
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        self.relationships.write().push(request.relationship);

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new("mock_token"),
        })
    }

    async fn write_batch(&self, requests: Vec<WriteRequest>) -> Result<WriteResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        let mut relationships = self.relationships.write();
        for request in requests {
            relationships.push(request.relationship);
        }

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new("mock_token"),
        })
    }

    async fn delete(&self, relationship: Relationship<'static>) -> Result<(), Error> {
        self.increment_requests();
        self.check_failure()?;

        let mut relationships = self.relationships.write();
        relationships.retain(|rel| {
            !(rel.resource() == relationship.resource()
                && rel.relation() == relationship.relation()
                && rel.subject() == relationship.subject())
        });

        Ok(())
    }

    async fn list_relationships(
        &self,
        resource: Option<&str>,
        relation: Option<&str>,
        subject: Option<&str>,
        limit: Option<u32>,
        _cursor: Option<&str>,
    ) -> Result<ListRelationshipsResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        let relationships = self.relationships.read();
        let filtered: Vec<_> = relationships
            .iter()
            .filter(|rel| {
                let resource_match = resource.map_or(true, |r| rel.resource() == r);
                let relation_match = relation.map_or(true, |r| rel.relation() == r);
                let subject_match = subject.map_or(true, |s| rel.subject() == s);
                resource_match && relation_match && subject_match
            })
            .take(limit.unwrap_or(100) as usize)
            .cloned()
            .collect();

        Ok(ListRelationshipsResponse {
            relationships: filtered,
            next_cursor: None,
        })
    }

    async fn list_resources(
        &self,
        subject: &str,
        permission: &str,
        resource_type: Option<&str>,
        limit: Option<u32>,
        _cursor: Option<&str>,
    ) -> Result<ListResourcesResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        let relationships = self.relationships.read();
        let resources: Vec<_> = relationships
            .iter()
            .filter(|rel| {
                rel.subject() == subject
                    && rel.relation() == permission
                    && resource_type
                        .map_or(true, |rt| rel.resource().starts_with(&format!("{}:", rt)))
            })
            .take(limit.unwrap_or(100) as usize)
            .map(|rel| rel.resource().to_string())
            .collect();

        Ok(ListResourcesResponse {
            resources,
            next_cursor: None,
        })
    }

    async fn list_subjects(
        &self,
        permission: &str,
        resource: &str,
        subject_type: Option<&str>,
        limit: Option<u32>,
        _cursor: Option<&str>,
    ) -> Result<ListSubjectsResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        let relationships = self.relationships.read();
        let subjects: Vec<_> = relationships
            .iter()
            .filter(|rel| {
                rel.resource() == resource
                    && rel.relation() == permission
                    && subject_type
                        .map_or(true, |st| rel.subject().starts_with(&format!("{}:", st)))
            })
            .take(limit.unwrap_or(100) as usize)
            .map(|rel| rel.subject().to_string())
            .collect();

        Ok(ListSubjectsResponse {
            subjects,
            next_cursor: None,
        })
    }

    fn transport_type(&self) -> Transport {
        Transport::Mock
    }

    fn stats(&self) -> TransportStats {
        TransportStats {
            active_transport: Transport::Mock,
            fallback_count: 0,
            last_fallback_reason: None,
            last_fallback_at: None,
            grpc: None,
            rest: None,
        }
    }

    async fn health_check(&self) -> Result<(), Error> {
        self.check_failure()?;
        Ok(())
    }

    async fn simulate(&self, request: SimulateRequest) -> Result<SimulateResponse, Error> {
        self.increment_requests();
        self.check_failure()?;

        // Build a temporary relationship set with additions and without removals
        let current_relationships = self.relationships.read();
        let mut simulated_relationships: Vec<_> = current_relationships
            .iter()
            .filter(|rel| {
                !request
                    .removals
                    .iter()
                    .any(|r| r.to_string() == rel.to_string())
            })
            .cloned()
            .collect();
        simulated_relationships.extend(request.additions.clone());

        // Check if the subject has the permission on the resource in the simulated state
        let allowed = simulated_relationships.iter().any(|rel| {
            rel.resource() == request.resource
                && rel.relation() == request.permission
                && rel.subject() == request.subject
        });

        Ok(SimulateResponse {
            allowed,
            decision: Decision::new(allowed),
        })
    }
}

/// Shared mock transport for use across async contexts.
pub type SharedMockTransport = Arc<MockTransport>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_check() {
        let transport = MockTransport::new();

        // Initially no access
        let response = transport
            .check(CheckRequest {
                subject: "user:alice".to_string(),
                permission: "view".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                consistency: None,
                trace: false,
            })
            .await
            .unwrap();
        assert!(!response.allowed);

        // Add relationship
        transport.add_relationship(Relationship::new("doc:1", "view", "user:alice").into_owned());

        // Now should have access
        let response = transport
            .check(CheckRequest {
                subject: "user:alice".to_string(),
                permission: "view".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                consistency: None,
                trace: false,
            })
            .await
            .unwrap();
        assert!(response.allowed);
    }

    #[tokio::test]
    async fn test_mock_transport_write() {
        let transport = MockTransport::new();

        let response = transport
            .write(WriteRequest {
                relationship: Relationship::new("doc:1", "viewer", "user:bob").into_owned(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        assert!(!response.consistency_token.is_empty());
        assert_eq!(transport.request_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_transport_delete() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());

        transport
            .delete(Relationship::new("doc:1", "viewer", "user:alice").into_owned())
            .await
            .unwrap();

        let list = transport
            .list_relationships(Some("doc:1"), None, None, None, None)
            .await
            .unwrap();
        assert!(list.relationships.is_empty());
    }

    #[tokio::test]
    async fn test_mock_transport_list_relationships() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:1", "editor", "user:bob").into_owned());
        transport.add_relationship(Relationship::new("doc:2", "viewer", "user:alice").into_owned());

        let list = transport
            .list_relationships(Some("doc:1"), None, None, None, None)
            .await
            .unwrap();
        assert_eq!(list.relationships.len(), 2);

        let list = transport
            .list_relationships(None, Some("viewer"), None, None, None)
            .await
            .unwrap();
        assert_eq!(list.relationships.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_transport_failure_simulation() {
        let transport = MockTransport::new();
        transport.set_failure(Error::unavailable("simulated failure"));

        let result = transport
            .check(CheckRequest {
                subject: "user:alice".to_string(),
                permission: "view".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                consistency: None,
                trace: false,
            })
            .await;

        assert!(result.is_err());

        // Failure should be cleared after one use
        let result = transport
            .check(CheckRequest {
                subject: "user:alice".to_string(),
                permission: "view".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                consistency: None,
                trace: false,
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_transport_health_check() {
        let transport = MockTransport::new();
        assert!(transport.health_check().await.is_ok());

        transport.set_failure(Error::unavailable("unhealthy"));
        assert!(transport.health_check().await.is_err());
    }

    #[test]
    fn test_mock_transport_default() {
        let transport = MockTransport::default();
        assert_eq!(transport.request_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_transport_clear_failure() {
        let transport = MockTransport::new();
        transport.set_failure(Error::unavailable("test"));
        transport.clear_failure();
        // Failure should be cleared - check by doing a health check
        assert!(transport.health_check().await.is_ok());
    }

    #[test]
    fn test_mock_transport_transport_type() {
        let transport = MockTransport::new();
        assert_eq!(transport.transport_type(), Transport::Mock);
    }

    #[test]
    fn test_mock_transport_stats() {
        let transport = MockTransport::new();
        let stats = transport.stats();
        assert_eq!(stats.active_transport, Transport::Mock);
        assert_eq!(stats.fallback_count, 0);
        assert!(stats.last_fallback_reason.is_none());
        assert!(stats.grpc.is_none());
        assert!(stats.rest.is_none());
    }

    #[tokio::test]
    async fn test_mock_transport_simulate() {
        let transport = MockTransport::new();

        // Add existing relationship
        transport.add_relationship(Relationship::new("doc:1", "editor", "user:alice").into_owned());

        // Simulate adding viewer relationship
        let result = transport
            .simulate(SimulateRequest {
                subject: "user:bob".to_string(),
                permission: "viewer".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                additions: vec![
                    Relationship::new("doc:1", "viewer", "user:bob").into_owned(),
                ],
                removals: vec![],
            })
            .await
            .unwrap();

        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_mock_transport_simulate_with_removal() {
        let transport = MockTransport::new();

        // Add existing relationship
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());

        // Simulate removing the relationship
        let result = transport
            .simulate(SimulateRequest {
                subject: "user:alice".to_string(),
                permission: "viewer".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                additions: vec![],
                removals: vec![
                    Relationship::new("doc:1", "viewer", "user:alice").into_owned(),
                ],
            })
            .await
            .unwrap();

        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_mock_transport_list_resources() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:2", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("folder:1", "viewer", "user:alice").into_owned());

        let result = transport
            .list_resources("user:alice", "viewer", None, None, None)
            .await
            .unwrap();

        assert_eq!(result.resources.len(), 3);
        assert!(result.resources.contains(&"doc:1".to_string()));
        assert!(result.resources.contains(&"doc:2".to_string()));
        assert!(result.resources.contains(&"folder:1".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_list_resources_with_type_filter() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:2", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("folder:1", "viewer", "user:alice").into_owned());

        let result = transport
            .list_resources("user:alice", "viewer", Some("doc"), None, None)
            .await
            .unwrap();

        assert_eq!(result.resources.len(), 2);
        assert!(result.resources.contains(&"doc:1".to_string()));
        assert!(result.resources.contains(&"doc:2".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_list_subjects() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:bob").into_owned());
        transport.add_relationship(Relationship::new("doc:1", "viewer", "group:admins").into_owned());

        let result = transport
            .list_subjects("viewer", "doc:1", None, None, None)
            .await
            .unwrap();

        assert_eq!(result.subjects.len(), 3);
        assert!(result.subjects.contains(&"user:alice".to_string()));
        assert!(result.subjects.contains(&"user:bob".to_string()));
        assert!(result.subjects.contains(&"group:admins".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_list_subjects_with_type_filter() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:bob").into_owned());
        transport.add_relationship(Relationship::new("doc:1", "viewer", "group:admins").into_owned());

        let result = transport
            .list_subjects("viewer", "doc:1", Some("user"), None, None)
            .await
            .unwrap();

        assert_eq!(result.subjects.len(), 2);
        assert!(result.subjects.contains(&"user:alice".to_string()));
        assert!(result.subjects.contains(&"user:bob".to_string()));
    }

    #[tokio::test]
    async fn test_mock_transport_clear_relationships() {
        let transport = MockTransport::new();
        transport.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        transport.add_relationship(Relationship::new("doc:2", "viewer", "user:bob").into_owned());

        transport.clear_relationships();

        let list = transport
            .list_relationships(None, None, None, None, None)
            .await
            .unwrap();
        assert!(list.relationships.is_empty());
    }

    #[tokio::test]
    async fn test_mock_transport_list_resources_with_limit() {
        let transport = MockTransport::new();
        for i in 0..10 {
            transport.add_relationship(
                Relationship::new(&format!("doc:{}", i), "viewer", "user:alice").into_owned(),
            );
        }

        let result = transport
            .list_resources("user:alice", "viewer", None, Some(5), None)
            .await
            .unwrap();

        assert_eq!(result.resources.len(), 5);
    }

    #[tokio::test]
    async fn test_mock_transport_list_subjects_with_limit() {
        let transport = MockTransport::new();
        for i in 0..10 {
            transport.add_relationship(
                Relationship::new("doc:1", "viewer", &format!("user:{}", i)).into_owned(),
            );
        }

        let result = transport
            .list_subjects("viewer", "doc:1", None, Some(5), None)
            .await
            .unwrap();

        assert_eq!(result.subjects.len(), 5);
    }
}
