# Common Authorization Scenarios

Real-world authorization patterns with complete, copy-paste examples.

## Multi-Tenant SaaS

Isolate data between organizations while supporting internal collaboration.

### Schema

```ipl
type organization {
    relation member: user
    relation admin: user
    relation billing_admin: user

    permission access = member | admin | billing_admin
    permission manage = admin
    permission manage_billing = billing_admin | admin
}

type workspace {
    relation org: organization
    relation member: user
    relation admin: user

    // Must be org member to access workspace
    permission view = (member | admin) & org->access
    permission edit = (member | admin) & org->access
    permission manage = admin & org->access
}

type project {
    relation workspace: workspace
    relation member: user
    relation lead: user

    permission view = member | lead | workspace->view
    permission edit = member | lead | workspace->edit
    permission manage = lead | workspace->manage
}
```

### Implementation

```rust
use inferadb::prelude::*;

// Setup: Create organization with workspaces
async fn setup_tenant(vault: &VaultClient, org_id: &str, admin_id: &str) -> Result<(), Error> {
    vault.relationships()
        .write_batch([
            // Admin is both member and admin
            Relationship::new(&format!("organization:{}", org_id), "member", &format!("user:{}", admin_id)),
            Relationship::new(&format!("organization:{}", org_id), "admin", &format!("user:{}", admin_id)),
        ])
        .await?;
    Ok(())
}

// Invite user to organization
async fn invite_to_org(vault: &VaultClient, org_id: &str, user_id: &str) -> Result<(), Error> {
    vault.relationships()
        .write(Relationship::new(
            &format!("organization:{}", org_id),
            "member",
            &format!("user:{}", user_id),
        ))
        .await
}

// Check workspace access
async fn can_access_workspace(
    vault: &VaultClient,
    user_id: &str,
    workspace_id: &str,
) -> Result<bool, Error> {
    vault.check(
        &format!("user:{}", user_id),
        "view",
        &format!("workspace:{}", workspace_id),
    ).await
}

// Middleware: Verify org membership before any operation
async fn require_org_access(
    vault: &VaultClient,
    user_id: &str,
    org_id: &str,
) -> Result<(), AccessDenied> {
    vault.check(
        &format!("user:{}", user_id),
        "access",
        &format!("organization:{}", org_id),
    )
    .require()
    .await
}
```

## Document Collaboration (Google Docs-style)

Share documents with different permission levels, support public/link sharing.

### Schema

```ipl
type user {
    // Users can be identified
}

type document {
    relation owner: user
    relation editor: user | group#member
    relation commenter: user | group#member
    relation viewer: user | group#member

    // Special sharing modes
    relation anyone_with_link: user:*          // Public link access
    relation org_anyone: organization#member   // Anyone in org

    // Permission hierarchy
    permission view = viewer | commenter | editor | owner | anyone_with_link | org_anyone
    permission comment = commenter | editor | owner
    permission edit = editor | owner
    permission delete = owner
    permission share = owner
    permission manage = owner
}
```

### Implementation

```rust
// Share document with specific user
async fn share_document(
    vault: &VaultClient,
    doc_id: &str,
    target_user: &str,
    permission: SharePermission,
) -> Result<(), Error> {
    let relation = match permission {
        SharePermission::View => "viewer",
        SharePermission::Comment => "commenter",
        SharePermission::Edit => "editor",
    };

    vault.relationships()
        .write(Relationship::new(
            &format!("document:{}", doc_id),
            relation,
            &format!("user:{}", target_user),
        ))
        .await
}

// Enable "anyone with link" access
async fn enable_link_sharing(vault: &VaultClient, doc_id: &str) -> Result<(), Error> {
    vault.relationships()
        .write(Relationship::new(
            &format!("document:{}", doc_id),
            "anyone_with_link",
            "user:*",  // Wildcard: any authenticated user
        ))
        .await
}

// Disable link sharing
async fn disable_link_sharing(vault: &VaultClient, doc_id: &str) -> Result<(), Error> {
    vault.relationships()
        .delete(Relationship::new(
            &format!("document:{}", doc_id),
            "anyone_with_link",
            "user:*",
        ))
        .await
}

// List who has access to a document
async fn list_document_access(
    vault: &VaultClient,
    doc_id: &str,
) -> Result<Vec<AccessEntry>, Error> {
    let subjects = vault
        .subjects()
        .with_permission("view")
        .on_resource(&format!("document:{}", doc_id))
        .collect()
        .await?;

    Ok(subjects.into_iter().map(|s| AccessEntry {
        subject: s.subject,
        // You'd need to query relationships to get the specific permission level
    }).collect())
}
```

## API Key Scoping

Limit what resources an API key can access.

### Schema

```ipl
type api_key {
    relation owner: user
    relation scope: resource | project | organization

    permission use = owner
}

type resource {
    relation api_key: api_key

    // API key can access if it's scoped to this resource
    permission api_access = api_key->use
}

type project {
    relation api_key: api_key

    permission api_access = api_key->use
}
```

### Implementation

```rust
// Create scoped API key
async fn create_api_key(
    vault: &VaultClient,
    key_id: &str,
    owner_id: &str,
    scopes: &[String],
) -> Result<(), Error> {
    let mut relationships = vec![
        Relationship::new(
            &format!("api_key:{}", key_id),
            "owner",
            &format!("user:{}", owner_id),
        ),
    ];

    // Add scope relationships
    for scope in scopes {
        relationships.push(Relationship::new(
            &format!("api_key:{}", key_id),
            "scope",
            scope,
        ));
    }

    vault.relationships().write_batch(relationships).await
}

// Check if API key can access resource
async fn can_api_key_access(
    vault: &VaultClient,
    api_key_id: &str,
    resource_id: &str,
) -> Result<bool, Error> {
    // Check if key is scoped to this specific resource
    let direct = vault.check(
        &format!("api_key:{}", api_key_id),
        "scope",
        resource_id,
    ).await?;

    if direct {
        return Ok(true);
    }

    // Check if key is scoped to parent project/org
    // This requires knowing the resource's parent
    Ok(false)
}

// Revoke API key
async fn revoke_api_key(vault: &VaultClient, key_id: &str) -> Result<(), Error> {
    vault.relationships()
        .delete_where()
        .subject(&format!("api_key:{}", key_id))
        .execute()
        .await?;

    vault.relationships()
        .delete_where()
        .resource(&format!("api_key:{}", key_id))
        .execute()
        .await
}
```

## Delegated Access (Acting on Behalf)

Allow services or users to act on behalf of another user.

### Schema

```ipl
type user {
    relation delegate: user | service_account

    // Delegates can act as this user
    permission impersonate = delegate
}

type service_account {
    relation owner: user
    relation can_impersonate: user

    permission act_as = can_impersonate
}

type resource {
    relation owner: user
    relation editor: user

    // Check includes both direct access and delegated access
    permission edit = editor | owner
}
```

### Implementation

```rust
// Grant delegation rights
async fn grant_delegation(
    vault: &VaultClient,
    from_user: &str,
    to_user: &str,
) -> Result<(), Error> {
    vault.relationships()
        .write(Relationship::new(
            &format!("user:{}", from_user),
            "delegate",
            &format!("user:{}", to_user),
        ))
        .await
}

// Check with delegation context
async fn check_with_delegation(
    vault: &VaultClient,
    actor: &str,          // Who is making the request
    on_behalf_of: &str,   // Who they claim to act for
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    // First verify delegation is allowed
    let can_delegate = vault.check(
        &format!("user:{}", actor),
        "impersonate",
        &format!("user:{}", on_behalf_of),
    ).await?;

    if !can_delegate {
        return Ok(false);
    }

    // Then check if the delegated user has permission
    vault.check(
        &format!("user:{}", on_behalf_of),
        permission,
        resource,
    ).await
}

// Service account impersonation
async fn service_can_act_as(
    vault: &VaultClient,
    service_account: &str,
    target_user: &str,
) -> Result<bool, Error> {
    vault.check(
        &format!("user:{}", target_user),
        "act_as",
        &format!("service_account:{}", service_account),
    ).await
}
```

## Temporary/Expiring Permissions

Grant time-limited access to resources.

### Schema

```ipl
type resource {
    relation viewer: user
    relation temp_viewer: user  // Managed externally with TTL

    permission view = viewer | (temp_viewer & context.not_expired)
}
```

### Implementation

```rust
use std::time::{Duration, SystemTime};

// Grant temporary access (store expiry in your database)
async fn grant_temporary_access(
    vault: &VaultClient,
    db: &Database,
    user_id: &str,
    resource_id: &str,
    duration: Duration,
) -> Result<(), Error> {
    let expires_at = SystemTime::now() + duration;

    // Write the relationship
    vault.relationships()
        .write(Relationship::new(
            resource_id,
            "temp_viewer",
            &format!("user:{}", user_id),
        ))
        .await?;

    // Store expiry in your database for cleanup
    db.store_temp_permission(user_id, resource_id, expires_at).await?;

    Ok(())
}

// Check with expiry context
async fn check_temp_access(
    vault: &VaultClient,
    db: &Database,
    user_id: &str,
    resource_id: &str,
) -> Result<bool, Error> {
    // Check if temp permission exists and is not expired
    let not_expired = match db.get_temp_permission_expiry(user_id, resource_id).await? {
        Some(expires_at) => SystemTime::now() < expires_at,
        None => false,
    };

    vault.check(
        &format!("user:{}", user_id),
        "view",
        resource_id,
    )
    .with_context(Context::new().insert("not_expired", not_expired))
    .await
}

// Background job: Clean up expired permissions
async fn cleanup_expired_permissions(vault: &VaultClient, db: &Database) -> Result<(), Error> {
    let expired = db.get_expired_permissions().await?;

    for perm in expired {
        vault.relationships()
            .delete(Relationship::new(
                &perm.resource_id,
                "temp_viewer",
                &format!("user:{}", perm.user_id),
            ))
            .await?;

        db.delete_temp_permission(&perm.user_id, &perm.resource_id).await?;
    }

    Ok(())
}
```

## Approval Workflows

Require approval before granting access.

### Schema

```ipl
type access_request {
    relation requester: user
    relation resource: resource
    relation approver: user
    relation approved_by: user

    permission approve = approver
    permission is_approved = approved_by
}

type resource {
    relation owner: user
    relation viewer: user
    relation pending_viewer: access_request

    permission view = viewer | owner
    permission request_access = user:*  // Anyone can request
}
```

### Implementation

```rust
// Request access to a resource
async fn request_access(
    vault: &VaultClient,
    user_id: &str,
    resource_id: &str,
    request_id: &str,
) -> Result<(), Error> {
    vault.relationships()
        .write_batch([
            Relationship::new(
                &format!("access_request:{}", request_id),
                "requester",
                &format!("user:{}", user_id),
            ),
            Relationship::new(
                &format!("access_request:{}", request_id),
                "resource",
                resource_id,
            ),
            // Resource owner can approve
            Relationship::new(
                &format!("access_request:{}", request_id),
                "approver",
                &format!("{}#owner", resource_id),
            ),
        ])
        .await
}

// Approve access request
async fn approve_access(
    vault: &VaultClient,
    approver_id: &str,
    request_id: &str,
) -> Result<(), Error> {
    let request = format!("access_request:{}", request_id);

    // Verify approver has permission
    vault.check(
        &format!("user:{}", approver_id),
        "approve",
        &request,
    )
    .require()
    .await?;

    // Get the requester and resource from your database
    // (or query relationships)
    let (requester, resource) = get_request_details(request_id).await?;

    // Grant the actual permission
    vault.relationships()
        .write(Relationship::new(&resource, "viewer", &requester))
        .await?;

    // Mark as approved
    vault.relationships()
        .write(Relationship::new(
            &request,
            "approved_by",
            &format!("user:{}", approver_id),
        ))
        .await
}
```

## Feature Flags by Permission

Control feature access through authorization.

### Schema

```ipl
type feature {
    relation enabled_for: user | organization#member | plan#subscriber

    permission access = enabled_for
}

type plan {
    relation subscriber: organization

    permission has_feature = subscriber
}
```

### Implementation

```rust
// Check if user has access to a feature
async fn has_feature(
    vault: &VaultClient,
    user_id: &str,
    feature_name: &str,
) -> Result<bool, Error> {
    vault.check(
        &format!("user:{}", user_id),
        "access",
        &format!("feature:{}", feature_name),
    ).await
}

// Enable feature for organization
async fn enable_feature_for_org(
    vault: &VaultClient,
    org_id: &str,
    feature_name: &str,
) -> Result<(), Error> {
    vault.relationships()
        .write(Relationship::new(
            &format!("feature:{}", feature_name),
            "enabled_for",
            &format!("organization:{}#member", org_id),
        ))
        .await
}

// Enable feature via plan subscription
async fn subscribe_to_plan(
    vault: &VaultClient,
    org_id: &str,
    plan_id: &str,
) -> Result<(), Error> {
    vault.relationships()
        .write(Relationship::new(
            &format!("plan:{}", plan_id),
            "subscriber",
            &format!("organization:{}", org_id),
        ))
        .await
}
```

## HTTP Handler Patterns

### Axum with Multiple Permission Checks

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};

async fn update_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
    Json(update): Json<DocumentUpdate>,
) -> Result<Json<Document>, StatusCode> {
    let resource = format!("document:{}", doc_id);

    // Check edit permission
    state.vault
        .check(&user.id, "edit", &resource)
        .require()
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    // If changing ownership, also need manage permission
    if update.new_owner.is_some() {
        state.vault
            .check(&user.id, "manage", &resource)
            .require()
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;
    }

    let doc = apply_update(&doc_id, update).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(doc))
}
```

### Filtering Lists by Permission

```rust
async fn list_accessible_documents(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<DocumentSummary>>, StatusCode> {
    // Option 1: Query what user can access (if supported by schema)
    let accessible = state.vault
        .resources()
        .accessible_by(&user.id)
        .with_permission("view")
        .resource_type("document")
        .collect()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Option 2: Batch check against known documents
    let all_docs = fetch_documents_for_user_org(&user.org_id).await?;
    let checks: Vec<_> = all_docs.iter()
        .map(|d| (&user.id, "view", format!("document:{}", d.id)))
        .collect();

    let results = state.vault.check_batch(&checks).collect().await?;

    let visible: Vec<_> = all_docs.into_iter()
        .zip(results)
        .filter_map(|(doc, (_, allowed))| allowed.then_some(doc))
        .collect();

    Ok(Json(visible))
}
```

## Best Practices

1. **Model your domain first** - Understand your entities and their relationships before writing schema
2. **Use organization scoping** - Always include org boundaries for multi-tenant apps
3. **Batch permission checks** - Use `check_batch()` when checking multiple resources
4. **Cache where appropriate** - Use SDK caching for read-heavy workloads
5. **Handle errors gracefully** - Log request IDs, fail closed on errors
6. **Test with simulation** - Validate permission logic before deploying
