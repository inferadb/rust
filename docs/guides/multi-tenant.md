# Multi-Tenant Authorization

This guide covers patterns for implementing authorization in multi-tenant SaaS applications.

## Overview

Multi-tenant applications need to ensure complete isolation between tenants while allowing efficient authorization checks. The SDK provides ergonomic tenant scoping that automatically isolates all operations.

## Tenant-Scoped Client

Create a tenant-scoped client that automatically prefixes all operations:

```rust
use inferadb::{Client, TenantScope};

// Create a tenant-scoped client
let tenant_client = client.with_tenant("tenant:acme-corp");

// All operations automatically scoped to tenant
tenant_client.check("user:alice", "view", "doc:1").await?;
// Actually checks: tenant:acme-corp/user:alice -> view -> tenant:acme-corp/doc:1

tenant_client.write(Relationship::new("doc:1", "viewer", "user:alice")).await?;
// Actually writes in tenant:acme-corp vault
```

## Framework Integration

### Extracting Tenant from Requests

```rust
use inferadb::axum::TenantContextLayer;

// Extract tenant from subdomain (acme.app.com -> tenant:acme)
let app = Router::new()
    .layer(TenantContextLayer::from_subdomain());

// Extract tenant from header
let app = Router::new()
    .layer(TenantContextLayer::from_header("X-Tenant-ID"));

// Extract tenant from path segment
let app = Router::new()
    .layer(TenantContextLayer::from_path_segment(0));  // /tenants/{tenant_id}/...

// Custom extraction
let app = Router::new()
    .layer(TenantContextLayer::custom(|req| {
        // Extract from JWT claim, database lookup, etc.
        extract_tenant_from_jwt(req)
    }));
```

### Complete Axum Example

```rust
use axum::{extract::State, Router};
use inferadb::{Client, TenantClient};

#[derive(Clone)]
struct AppState {
    authz: Arc<Client>,
}

async fn get_document(
    State(state): State<AppState>,
    tenant: TenantContext,  // Extracted by middleware
    user: AuthenticatedUser,
    Path(doc_id): Path<String>,
) -> Result<Json<Document>, StatusCode> {
    // Create tenant-scoped client
    let authz = state.authz.with_tenant(&tenant.tenant_id);

    // All checks automatically scoped to tenant
    if !authz.check(&user.id, "view", &format!("document:{}", doc_id)).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let doc = fetch_document(&tenant.tenant_id, &doc_id).await?;
    Ok(Json(doc))
}
```

## Cross-Tenant Operations

For platform admins who need to operate across tenants:

```rust
// Admin operations across tenants (requires elevated permissions)
let admin_client = client.with_admin_context();

// Check permissions across tenant boundary
admin_client
    .check("platform:admin", "manage", "tenant:acme-corp")
    .await?;

// List all tenants user has access to
let tenants: Vec<String> = admin_client
    .list_resources("user:alice@platform", "member")
    .resource_type("tenant")
    .collect()
    .await?;
```

### Platform-Level Schema

```ipl
// Platform-level entities
entity Tenant {
    relations {
        admin: User
        member: User
    }

    permissions {
        manage: admin
        access: admin | member
    }
}

// Tenant-scoped entities
entity Document {
    relations {
        tenant: Tenant
        owner: User
        viewer: User | Group#member
    }

    permissions {
        view: (viewer | owner) & tenant->access
        edit: owner & tenant->access
        delete: owner & tenant->admin
    }
}
```

## Isolation Modes

Configure how strictly tenants are isolated:

```rust
pub enum IsolationMode {
    /// Strict isolation - no cross-tenant access
    Strict,

    /// Allow platform admins to access
    AllowPlatformAdmin,

    /// Custom isolation rules
    Custom(Arc<dyn Fn(&str, &str) -> bool + Send + Sync>),
}

let tenant_client = client
    .with_tenant("tenant:acme-corp")
    .isolation_mode(IsolationMode::AllowPlatformAdmin);
```

## Testing Tenant Isolation

Verify that tenants cannot access each other's data:

```rust
use inferadb::testing::TenantIsolationTest;

#[test]
fn test_tenant_isolation() {
    let test = TenantIsolationTest::new()
        .with_tenants(["tenant:a", "tenant:b"])
        .with_schema(include_str!("../schema.ipl"));

    // Set up data in tenant A
    test.in_tenant("tenant:a", |t| {
        t.write("doc:secret", "viewer", "user:alice");
    });

    // Verify isolation
    test.assert_isolated(
        "user:alice@tenant:a",
        "view",
        "doc:secret@tenant:b",  // Cannot access tenant B's doc
    );
}

#[test]
fn test_cross_tenant_admin() {
    let test = TenantIsolationTest::new()
        .with_tenants(["tenant:a", "tenant:b"])
        .with_platform_admin("admin:platform");

    // Platform admin can access both tenants
    test.assert_allowed("admin:platform", "manage", "tenant:a");
    test.assert_allowed("admin:platform", "manage", "tenant:b");

    // Tenant admin cannot access other tenant
    test.in_tenant("tenant:a", |t| {
        t.write("tenant:a", "admin", "user:alice");
    });
    test.assert_denied("user:alice@tenant:a", "manage", "tenant:b");
}
```

## Tenant Provisioning

When creating new tenants:

```rust
async fn provision_tenant(
    client: &Client,
    tenant_id: &str,
    admin_user: &str,
) -> Result<(), Error> {
    // Create tenant vault
    let vault = client.control()
        .vaults()
        .create(CreateVault {
            name: tenant_id.to_string(),
            ..Default::default()
        })
        .await?;

    // Apply tenant schema
    client.control()
        .schemas(&vault.id)
        .push(include_str!("tenant-schema.ipl"))
        .activate()
        .await?;

    // Set up initial admin
    let tenant_client = client.with_tenant(tenant_id);
    tenant_client.write(
        Relationship::new("tenant:root", "admin", admin_user)
    ).await?;

    Ok(())
}
```

## Type Reference

```rust
/// Tenant-scoped client wrapper
pub struct TenantClient {
    inner: Client,
    tenant_id: String,
}

impl TenantClient {
    pub fn tenant_id(&self) -> &str;

    // All Client methods available, automatically scoped
    pub async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool>;
    pub async fn write(&self, relationship: Relationship) -> Result<Token>;
    // ...
}

/// Tenant context for request handling
#[derive(Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub isolation_mode: IsolationMode,
}
```

## Best Practices

### 1. Always Use Tenant-Scoped Clients

```rust
// Good: tenant-scoped client
let authz = client.with_tenant(&tenant_id);
authz.check(&user, "view", "doc:1").await?;

// Bad: manual prefixing (error-prone)
let subject = format!("{}:{}", tenant_id, user);
let resource = format!("{}:doc:1", tenant_id);
client.check(&subject, "view", &resource).await?;
```

### 2. Extract Tenant Early in Request Pipeline

```rust
// Middleware extracts tenant, makes available to all handlers
let app = Router::new()
    .layer(TenantContextLayer::from_header("X-Tenant-ID"))
    .route("/documents/:id", get(get_document));
```

### 3. Validate Tenant Access Before Operations

```rust
async fn handler(tenant: TenantContext, user: User) -> Result<Response> {
    // Verify user belongs to this tenant
    if !user.tenants.contains(&tenant.tenant_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Proceed with tenant-scoped operations
    // ...
}
```

### 4. Use Separate Vaults Per Tenant for Strong Isolation

```rust
// Each tenant gets their own vault
// Provides physical isolation of authorization data
let vault_id = format!("vault-{}", tenant_id);
let client = client.with_vault(&vault_id);
```

### 5. Audit Cross-Tenant Operations

```rust
let admin_client = client
    .with_admin_context()
    .with_audit(AuditContext::new()
        .action_reason("Support ticket #12345")
        .custom("target_tenant", target_tenant_id));
```
