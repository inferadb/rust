# Testing Authorization

This guide covers testing patterns and utilities for verifying authorization logic in your applications.

## Overview

The InferaDB SDK provides multiple testing approaches:

| Approach         | Use Case                                      | Speed   | Fidelity            |
| ---------------- | --------------------------------------------- | ------- | ------------------- |
| `MockClient`     | Unit tests with predetermined responses       | Fastest | Stub responses      |
| `InMemoryClient` | Integration tests with real policy evaluation | Fast    | Full engine, no I/O |
| `TestVault`      | E2E tests against running InferaDB            | Slower  | Production behavior |

## MockClient for Unit Tests

**Start here.** `MockClient` mirrors the production API, so your tests look like production code with only the client swapped:

```rust
use inferadb::testing::MockClient;
use inferadb::AuthorizationClient;

// Your production code - accepts any AuthorizationClient
async fn get_document(
    authz: &impl AuthorizationClient,
    user: &str,
    doc_id: &str,
) -> Result<Document, AppError> {
    authz.check(user, "view", &format!("document:{}", doc_id))
        .require()
        .await?;
    fetch_document(doc_id).await
}

// Your test - swap MockClient for VaultClient
#[tokio::test]
async fn test_get_document_authorized() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "document:1", true)
        .build();

    let result = get_document(&mock, "user:alice", "1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_document_denied() {
    let mock = MockClient::builder()
        .check("user:bob", "view", "document:1", false)
        .build();

    let result = get_document(&mock, "user:bob", "1").await;
    assert!(matches!(result, Err(AppError::AccessDenied(_))));
}
```

### MockClient Features

```rust
let mock = MockClient::builder()
    // Explicit check results
    .check("user:alice", "view", "doc:1", true)
    .check("user:alice", "edit", "doc:1", false)

    // Wildcard patterns
    .check_any_subject("view", "doc:public", true)    // Anyone can view
    .check_any_resource("user:admin", "delete", true) // Admin can delete anything

    // Default behavior for unmatched
    .default_deny()

    // Verify all expectations were used
    .verify_on_drop(true)

    .build();
```

### Expectation Verification

```rust
#[tokio::test]
async fn test_authorization_flow() {
    // Create mock with stubbed results
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:bob", "view", "doc:1", false)
        .verify_on_drop(true)  // Verify all expectations were consumed
        .build();

    // ... run test ...

    // Mock verifies expectations on drop when verify_on_drop(true)
}
```

## InMemoryClient for Integration Tests

When you need real permission evaluation logic (inheritance, unions, ABAC):

```rust
use inferadb::testing::InMemoryClient;
use inferadb::Relationship;

#[tokio::test]
async fn test_permission_inheritance() {
    // Real schema, real evaluation engine
    let vault = InMemoryClient::with_schema(r#"
        entity User {}
        entity Folder {
            relations { owner: User }
            permissions { view: owner, delete: owner }
        }
        entity Document {
            relations { parent: Folder, viewer: User }
            permissions { view: viewer | parent.view, delete: parent.delete }
        }
    "#);

    // Seed data
    vault.relationships().write(Relationship::new("folder:docs", "owner", "user:alice")).await.unwrap();
    vault.relationships().write(Relationship::new("doc:readme", "parent", "folder:docs")).await.unwrap();

    // Test inheritance: alice owns folder, so can view/delete docs in it
    assert!(vault.check("user:alice", "view", "doc:readme").await.unwrap());
    assert!(vault.check("user:alice", "delete", "doc:readme").await.unwrap());
    assert!(!vault.check("user:bob", "view", "doc:readme").await.unwrap());
}
```

### InMemoryClient with Initial Data

```rust
use inferadb::testing::InMemoryClient;
use inferadb::Relationship;

let vault = InMemoryClient::with_schema_and_data(
    include_str!("schema.ipl"),
    vec![
        Relationship::new("folder:docs", "owner", "user:alice"),
        Relationship::new("doc:readme", "parent", "folder:docs"),
    ],
);
```

## TestVault for E2E Tests

For tests against a real InferaDB instance:

```rust
use inferadb::testing::{TestVault, TestConfig, test_client};
use inferadb::Relationship;

#[tokio::test]
#[ignore]  // Requires running InferaDB
async fn integration_test() {
    let config = TestConfig::new("http://localhost:8080", "test-token")
        .with_organization_id("org_test...");
    let client = test_client(config).await.unwrap();
    let org = client.organization("org_test...");
    let vault = TestVault::create(&org).await.unwrap();

    // Tests run in isolated vault
    vault.relationships().write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());

    // Vault cleaned up on drop
}
```

### Preserving Test Vaults

For debugging failed tests:

```rust
#[tokio::test]
#[ignore]
async fn debug_failing_test() {
    let org = client.organization("org_test...");
    let vault = TestVault::create(&org)
        .await
        .unwrap()
        .preserve();  // Don't clean up on drop

    // ... test code ...
    // Vault persists for inspection
}
```

### TestVault with Schema

```rust
let vault = TestVault::create_with_schema(
    &org,
    include_str!("schema.ipl"),
).await.unwrap();
```

## Decision Trace Snapshot Testing

Use `assert_decision_trace!` to catch regressions in permission evaluation logic:

```rust
use inferadb::testing::{InMemoryClient, assert_decision_trace};

#[tokio::test]
async fn test_view_permission_trace() {
    let vault = InMemoryClient::with_schema(include_str!("schema.ipl"));
    seed_test_data(&vault).await;

    // Snapshot the decision trace - fails if logic changes
    assert_decision_trace!(
        vault,
        "user:alice", "view", "doc:readme",
        @r#"
        {
          "allowed": true,
          "path": ["viewer", "parent.view"],
          "matched_rule": "view: viewer | parent.view"
        }
        "#
    );
}
```

## Simulation + Snapshot for What-If Testing

Combine `simulate()` with snapshots to test schema changes:

```rust
use inferadb::testing::{InMemoryClient, SimulationSnapshot};

#[tokio::test]
async fn test_schema_migration_preserves_access() {
    let vault = InMemoryClient::with_schema(include_str!("schema_v1.ipl"));
    seed_production_data(&vault).await;

    // Capture current behavior as baseline
    let baseline = SimulationSnapshot::capture(&vault, &[
        ("user:alice", "view", "doc:1"),
        ("user:bob", "edit", "doc:2"),
        ("user:charlie", "delete", "folder:root"),
    ]).await;

    // Simulate with new schema
    let new_schema = include_str!("schema_v2.ipl");
    let simulation = vault.simulate()
        .with_schema(new_schema)
        .build();

    let after_migration = SimulationSnapshot::capture(&simulation, &[
        ("user:alice", "view", "doc:1"),
        ("user:bob", "edit", "doc:2"),
        ("user:charlie", "delete", "folder:root"),
    ]).await;

    // Compare - fail if any permissions changed unexpectedly
    baseline.assert_unchanged(&after_migration);
}
```

## AuthorizationClient Trait

All client types implement a common trait for testability:

```rust
/// Object-safe authorization trait for dependency injection.
/// Implemented by VaultClient, MockClient, InMemoryClient.
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    // Core authorization
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    async fn check_batch(&self, checks: Vec<(&str, &str, &str)>) -> Result<Vec<bool>, Error>;

    // Relationship management
    async fn write(&self, relationship: Relationship) -> Result<(), Error>;
    async fn write_batch(&self, relationships: Vec<Relationship>) -> Result<(), Error>;
    async fn delete(&self, relationship: Relationship) -> Result<(), Error>;
    async fn delete_batch(&self, relationships: Vec<Relationship>) -> Result<(), Error>;
}
```

This allows dependency injection in your application:

```rust
use std::sync::Arc;
use inferadb::AuthorizationClient;

struct DocumentService {
    authz: Arc<dyn AuthorizationClient>,
}

impl DocumentService {
    async fn get_document(&self, user: &str, doc_id: &str) -> Result<Document, Error> {
        let resource = format!("document:{}", doc_id);
        if !self.authz.check(user, "view", &resource).await? {
            return Err(Error::Forbidden);
        }
        fetch_document(doc_id).await
    }
}

// In production
let vault = client.organization("org_...").vault("vlt_...");
let service = DocumentService { authz: Arc::new(vault) };

// In tests
let mock = MockClient::builder()
    .check("user:alice", "view", "document:1", true)
    .build();
let service = DocumentService { authz: Arc::new(mock) };
```

## Best Practices

1. **Use MockClient for unit tests**: Fast, no I/O, predetermined responses
2. **Use InMemoryClient for integration tests**: Real policy evaluation without network
3. **Use TestVault for E2E tests**: Production behavior with isolated data
4. **Test both allowed and denied cases**: Verify permission boundaries
5. **Test inheritance paths**: Verify permissions flow correctly through hierarchies
6. **Snapshot test decision traces**: Catch regressions in permission logic
7. **Use simulation for schema changes**: Verify migrations preserve expected access
