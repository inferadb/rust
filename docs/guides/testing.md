# Testing Authorization

This guide covers testing patterns and utilities for verifying authorization logic in your applications.

## Overview

The InferaDB SDK provides multiple testing approaches:

| Approach | Use Case | Speed | Fidelity |
|----------|----------|-------|----------|
| `MockClient` | Unit tests with predetermined responses | Fastest | Low |
| `InMemoryClient` | Integration tests with real policy evaluation | Fast | High |
| `TestVault` | E2E tests against running InferaDB | Slow | Highest |
| `AuthzTest` DSL | Comprehensive permission testing | Fast | High |

## Mock Client for Unit Tests

Use `MockClient` when you need predetermined responses without policy evaluation:

```rust
use inferadb::testing::MockClient;

#[tokio::test]
async fn test_document_access() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:alice", "edit", "doc:1", false)
        .check("user:bob", "view", "doc:1", false)
        .build();

    // Test your code
    assert!(mock.check("user:alice", "view", "doc:1").await.unwrap());
    assert!(!mock.check("user:alice", "edit", "doc:1").await.unwrap());
}
```

### Matching Patterns

```rust
let mock = MockClient::builder()
    // Exact match
    .check("user:alice", "view", "doc:1", true)

    // Wildcard subject
    .check_any_subject("view", "doc:public", true)

    // Wildcard resource
    .check_any_resource("user:admin", "delete", true)

    // Default behavior for unmatched
    .default_deny()
    .build();
```

## In-Memory Client for Integration Tests

Use `InMemoryClient` when you need real policy evaluation without a running server:

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_permission_inheritance() {
    let client = InMemoryClient::with_schema(include_str!("schema.ipl"));

    // Seed data
    client.write_batch([
        Relationship::new("folder:docs", "owner", "user:alice"),
        Relationship::new("doc:readme", "parent", "folder:docs"),
    ]).await.unwrap();

    // Test inheritance
    assert!(client.check("user:alice", "view", "doc:readme").await.unwrap());
    assert!(client.check("user:alice", "delete", "doc:readme").await.unwrap());
}
```

## Test Vault for E2E Tests

Use `TestVault` for isolated tests against a running InferaDB instance:

```rust
use inferadb::testing::TestVault;

#[tokio::test]
#[ignore]  // Requires running InferaDB
async fn integration_test() {
    let client = test_client().await;
    let vault = TestVault::create(&client).await.unwrap();

    // Tests run in isolated vault
    vault.write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());

    // Vault cleaned up on drop
}
```

## Authorization Testing DSL

For comprehensive permission testing, the SDK provides a fluent testing DSL:

```rust
use inferadb::testing::{AuthzTest, path};

#[test]
fn test_document_permissions() {
    let authz = AuthzTest::new()
        .with_schema(include_str!("../schema.ipl"))
        // Set up relationships
        .with_relationship("folder:docs", "owner", "user:alice")
        .with_relationship("doc:readme", "parent", "folder:docs")
        .with_relationship("group:engineering", "member", "user:bob")
        .with_relationship("folder:docs", "viewer", "group:engineering#member");

    // Simple assertions
    authz.assert_allowed("user:alice", "view", "doc:readme");
    authz.assert_allowed("user:alice", "edit", "doc:readme");
    authz.assert_allowed("user:alice", "delete", "doc:readme");

    authz.assert_denied("user:bob", "edit", "doc:readme");
    authz.assert_denied("user:charlie", "view", "doc:readme");

    // Assert with explanation
    authz.assert_allowed_because(
        "user:alice",
        "delete",
        "doc:readme",
        path!["owner" -> "parent"],  // Expected access path
    );

    // Assert via specific relation
    authz.assert_allowed_via(
        "user:bob",
        "view",
        "doc:readme",
        "group:engineering#member",  // Via group membership
    );
}

#[test]
fn test_bulk_permissions() {
    let authz = AuthzTest::new()
        .with_schema(include_str!("../schema.ipl"))
        .with_relationships([
            ("doc:1", "viewer", "user:alice"),
            ("doc:2", "viewer", "user:alice"),
            ("doc:3", "editor", "user:alice"),
        ]);

    // Batch assertions
    authz.assert_all_allowed("user:alice", "view", ["doc:1", "doc:2", "doc:3"]);
    authz.assert_none_allowed("user:bob", "view", ["doc:1", "doc:2", "doc:3"]);

    // Table-driven tests
    authz.assert_permissions([
        ("user:alice", "view", "doc:1", true),
        ("user:alice", "edit", "doc:1", false),
        ("user:alice", "view", "doc:3", true),
        ("user:alice", "edit", "doc:3", true),
        ("user:bob", "view", "doc:1", false),
    ]);
}
```

### Snapshot Testing

```rust
use inferadb::testing::AuthzTest;

#[test]
fn test_permission_snapshot() {
    let authz = AuthzTest::new()
        .with_schema(include_str!("../schema.ipl"))
        .with_fixture("fixtures/production-sample.json");

    // Generate permission matrix and compare to snapshot
    authz.assert_permission_matrix_snapshot(
        &["user:alice", "user:bob", "user:charlie"],
        &["view", "edit", "delete"],
        &["doc:1", "doc:2", "folder:root"],
    );
    // Creates/compares: snapshots/test_permission_snapshot.snap
}
```

### Scenario Testing

Test state changes and their permission effects:

```rust
use inferadb::testing::{AuthzTest, Scenario};

#[test]
fn test_access_revocation_scenario() {
    let authz = AuthzTest::new()
        .with_schema(include_str!("../schema.ipl"));

    authz.scenario("user gains then loses access")
        // Initial state: no access
        .assert_denied("user:alice", "view", "doc:secret")

        // Grant access
        .write("doc:secret", "viewer", "user:alice")
        .assert_allowed("user:alice", "view", "doc:secret")

        // Revoke access
        .delete("doc:secret", "viewer", "user:alice")
        .assert_denied("user:alice", "view", "doc:secret")

        .run();
}

#[test]
fn test_hierarchical_access() {
    let authz = AuthzTest::new()
        .with_schema(include_str!("../schema.ipl"));

    authz.scenario("folder access propagates to documents")
        .write("folder:root", "viewer", "user:alice")
        .write("doc:readme", "parent", "folder:root")

        // Alice can view doc via folder
        .assert_allowed("user:alice", "view", "doc:readme")

        // Remove from folder, lose access to doc
        .delete("folder:root", "viewer", "user:alice")
        .assert_denied("user:alice", "view", "doc:readme")

        .run();
}
```

## Testing Trait Abstraction

All client types implement a common trait for testability:

```rust
/// Trait for authorization operations, implemented by all client types
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    async fn check_batch(&self, checks: Vec<(&str, &str, &str)>) -> Result<Vec<bool>, Error>;
    async fn write(&self, relationship: Relationship) -> Result<(), Error>;
    async fn delete(&self, relationship: Relationship) -> Result<(), Error>;
    // ... other methods
}

// Implemented by:
impl AuthorizationClient for Client { /* real client */ }
impl AuthorizationClient for MockClient { /* mock client */ }
impl AuthorizationClient for InMemoryClient { /* in-memory client */ }
```

This allows dependency injection in your application:

```rust
struct DocumentService<C: AuthorizationClient> {
    authz: C,
    // ...
}

impl<C: AuthorizationClient> DocumentService<C> {
    async fn get_document(&self, user: &str, doc_id: &str) -> Result<Document, Error> {
        let resource = format!("document:{}", doc_id);
        if !self.authz.check(user, "view", &resource).await? {
            return Err(Error::Forbidden);
        }
        // ... fetch document
    }
}

// In production
let service = DocumentService { authz: real_client };

// In tests
let service = DocumentService { authz: mock_client };
```

## Testing Types Reference

```rust
/// Builder for authorization test scenarios
pub struct AuthzTest {
    client: InMemoryClient,
}

impl AuthzTest {
    pub fn new() -> Self;
    pub fn with_schema(self, schema: &str) -> Self;
    pub fn with_fixture(self, path: &str) -> Self;
    pub fn with_relationship(self, resource: &str, relation: &str, subject: &str) -> Self;
    pub fn with_relationships<I>(self, relationships: I) -> Self
    where
        I: IntoIterator<Item = (&str, &str, &str)>;

    // Assertions
    pub fn assert_allowed(&self, subject: &str, permission: &str, resource: &str);
    pub fn assert_denied(&self, subject: &str, permission: &str, resource: &str);
    pub fn assert_allowed_because(&self, subject: &str, permission: &str, resource: &str, path: AccessPath);
    pub fn assert_allowed_via(&self, subject: &str, permission: &str, resource: &str, via: &str);

    // Batch assertions
    pub fn assert_all_allowed<I>(&self, subject: &str, permission: &str, resources: I)
    where
        I: IntoIterator<Item = &str>;
    pub fn assert_none_allowed<I>(&self, subject: &str, permission: &str, resources: I)
    where
        I: IntoIterator<Item = &str>;
    pub fn assert_permissions<I>(&self, checks: I)
    where
        I: IntoIterator<Item = (&str, &str, &str, bool)>;

    // Scenarios
    pub fn scenario(&self, name: &str) -> ScenarioBuilder;
}

/// Macro for defining access paths
#[macro_export]
macro_rules! path {
    [$($relation:literal -> $target:literal),+ $(,)?] => {
        AccessPath::new(&[$(($relation, $target)),+])
    };
}
```

## Best Practices

1. **Use the right tool for the job**: MockClient for unit tests, InMemoryClient for integration tests, TestVault for E2E tests

2. **Test permission boundaries**: Always test both allowed and denied cases

3. **Test inheritance paths**: Verify that permissions flow correctly through hierarchies

4. **Use scenario tests for state changes**: Test grant/revoke flows and their effects

5. **Snapshot test complex permission matrices**: Catch regressions in permission configurations

6. **Keep test data minimal**: Only set up the relationships needed for each test
