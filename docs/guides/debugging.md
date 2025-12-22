# Debugging Authorization

Techniques for diagnosing and resolving authorization issues.

## Quick Diagnosis Checklist

When a user reports "I can't access X", work through this checklist:

1. **Verify identity** - Is the subject correct? (`user:alice` vs `user:Alice`)
2. **Check relationship exists** - Does the expected relationship exist?
3. **Verify permission path** - Does the schema define a path from relationship to permission?
4. **Check hierarchy** - If using parent resources, is the chain complete?
5. **Inspect ABAC context** - Are runtime conditions being met?

## Using explain_permission()

The `explain_permission()` API shows why access was granted or denied.

### Basic Usage

```rust
let explanation = vault
    .explain_permission("user:alice", "edit", "document:readme")
    .await?;

println!("Allowed: {}", explanation.allowed);
println!("Reason: {:?}", explanation.reason);

if explanation.allowed {
    println!("Access granted via:");
    for step in &explanation.resolution_path {
        println!("  -> {}", step);
    }
} else {
    println!("Denial reasons:");
    for reason in &explanation.denial_reasons {
        println!("  - {}", reason);
    }
}
```

### Example Output

```text
Allowed: true
Access granted via:
  -> document:readme#editor <- user:alice
  -> edit = editor | owner
```

```text
Allowed: false
Denial reasons:
  - No relationship found: user:alice -> document:secret
  - Permission 'edit' requires: editor | owner
  - Checked relations: viewer, editor, owner - none matched
```

## Decision Traces

For complex permission structures, use decision traces to see the full evaluation tree.

### Enable Tracing

```rust
let decision = vault
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .await?;

println!("Allowed: {}", decision.allowed);

if let Some(trace) = &decision.trace {
    // Render as tree
    println!("{}", trace.render_tree());
}
```

### Trace Output

```text
edit (ALLOWED)
├── editor (NOT_FOUND)
│   └── Direct lookup: user:alice -> document:readme#editor ✗
├── owner (FOUND)
│   └── Direct lookup: user:alice -> document:readme#owner ✓
└── parent->edit (NOT_EVALUATED)
    └── Skipped: already satisfied by 'owner'
```

### Analyzing Traces

```rust
if let Some(trace) = &decision.trace {
    // Find what granted access
    let satisfied = trace.find_satisfied_paths();
    for path in satisfied {
        println!("Granted via: {}", path.description());
    }

    // Find failed paths (useful for debugging denials)
    let failed = trace.find_failed_paths();
    for path in failed {
        println!("Failed path: {} - {}", path.description(), path.failure_reason());
    }

    // Find slow operations
    let slow = trace.find_nodes_slower_than(Duration::from_millis(10));
    for node in slow {
        println!("Slow: {:?} took {:?}", node.operation, node.metrics.duration);
    }
}
```

## Common Issues

### Issue: Relationship Exists But Access Denied

**Symptoms**: You've written a relationship, but `check()` returns `false`.

**Diagnosis**:

```rust
// 1. Verify the relationship exists
let relationships = vault
    .relationships()
    .list()
    .subject("user:alice")
    .resource("document:readme")
    .collect()
    .await?;

println!("Found relationships: {:?}", relationships);

// 2. Check the exact permission being tested
let explanation = vault
    .explain_permission("user:alice", "edit", "document:readme")
    .await?;

println!("Explanation: {:?}", explanation);
```

**Common Causes**:

1. **Wrong relation name**: Wrote `viewer` but checking `edit` permission
2. **Case sensitivity**: `user:Alice` != `user:alice`
3. **Missing permission definition**: Schema doesn't include the relation in permission
4. **Stale read**: Check consistency tokens for read-after-write

```rust
// Ensure read-after-write consistency
let token = vault.relationships()
    .write(Relationship::new("document:readme", "editor", "user:alice"))
    .await?
    .consistency_token;

// Use token for subsequent read
let allowed = vault
    .check("user:alice", "edit", "document:readme")
    .at_least_as_fresh_as(&token)
    .await?;
```

### Issue: Inherited Permission Not Working

**Symptoms**: Parent has permission, but child resource check fails.

**Diagnosis**:

```rust
// Check the parent relationship exists
let parent = vault
    .relationships()
    .list()
    .resource("document:readme")
    .relation("parent")
    .collect()
    .await?;

println!("Parent: {:?}", parent);

// Check parent permission directly
let parent_allowed = vault
    .check("user:alice", "edit", "folder:docs")
    .await?;

println!("Parent edit permission: {}", parent_allowed);
```

**Common Causes**:

1. **Missing parent relationship**: Document not linked to folder
2. **Wrong parent relation name**: Schema uses `folder` but you wrote `parent`
3. **Schema missing inheritance**: Permission doesn't include `parent->edit`

### Issue: Group Membership Not Resolving

**Symptoms**: User is in group, group has access, but user check fails.

**Diagnosis**:

```rust
// Verify group membership
let in_group = vault
    .check("user:alice", "member", "group:engineering")
    .await?;
println!("In group: {}", in_group);

// Verify group has access
let group_access = vault
    .check("group:engineering#member", "view", "document:readme")
    .await?;
println!("Group has access: {}", group_access);

// Check relationship is using correct userset syntax
let rels = vault
    .relationships()
    .list()
    .resource("document:readme")
    .collect()
    .await?;

for rel in &rels {
    println!("{} -> {} -> {}", rel.subject, rel.relation, rel.resource);
}
```

**Common Causes**:

1. **Wrong userset syntax**: Used `group:engineering` instead of `group:engineering#member`
2. **Missing member relation**: User not added to group
3. **Schema type mismatch**: Relation expects `group#member` but got `group`

### Issue: ABAC Context Not Evaluated

**Symptoms**: Condition should be met, but access is denied.

**Diagnosis**:

```rust
// Check with explicit context
let allowed = vault
    .check("user:alice", "view_confidential", "document:secret")
    .with_context(Context::new()
        .insert("ip_in_allowlist", true)
        .insert("mfa_verified", true))
    .trace(true)
    .await?;

if let Some(trace) = &allowed.trace {
    // Look for condition evaluation
    for node in trace.all_nodes() {
        if let Some(condition) = &node.condition {
            println!("Condition '{}': {}", condition.expression, condition.result);
        }
    }
}
```

**Common Causes**:

1. **Missing context key**: Schema expects `ip_address` but you passed `ip`
2. **Type mismatch**: Schema expects boolean but got string
3. **Context not passed**: Forgot `.with_context()` on the check

## Logging Strategies

### Structured Logging for Authorization

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(vault), fields(request_id))]
async fn check_access(
    vault: &VaultClient,
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    let result = vault.check(subject, permission, resource).await;

    match &result {
        Ok(allowed) => {
            info!(
                subject = subject,
                permission = permission,
                resource = resource,
                allowed = allowed,
                "Authorization check completed"
            );
        }
        Err(e) => {
            error!(
                subject = subject,
                permission = permission,
                resource = resource,
                error = %e,
                request_id = ?e.request_id(),
                "Authorization check failed"
            );
        }
    }

    result
}
```

### Audit Logging for Compliance

```rust
async fn audited_check(
    vault: &VaultClient,
    audit_log: &AuditLog,
    actor: &str,
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    let start = std::time::Instant::now();
    let result = vault.check(subject, permission, resource).await;
    let duration = start.elapsed();

    audit_log.record(AuditEntry {
        timestamp: Utc::now(),
        actor: actor.to_string(),
        subject: subject.to_string(),
        permission: permission.to_string(),
        resource: resource.to_string(),
        allowed: result.as_ref().ok().copied(),
        error: result.as_ref().err().map(|e| e.to_string()),
        duration_ms: duration.as_millis() as u64,
    }).await;

    result
}
```

## Debugging in Production

### Request ID Tracking

Every error includes a request ID for support:

```rust
match vault.check(subject, permission, resource).await {
    Err(e) => {
        // Log request ID for support tickets
        error!(
            request_id = ?e.request_id(),
            error = %e,
            "Authorization failed"
        );

        // Include in error response (for API consumers)
        return Err(ApiError {
            message: "Authorization check failed".into(),
            request_id: e.request_id().map(|id| id.to_string()),
        });
    }
    Ok(allowed) => { /* ... */ }
}
```

### Sampling Explain Calls

In production, selectively run explain for debugging:

```rust
async fn check_with_sampling(
    vault: &VaultClient,
    subject: &str,
    permission: &str,
    resource: &str,
    sample_rate: f64,
) -> Result<bool, Error> {
    let allowed = vault.check(subject, permission, resource).await?;

    // Sample explain calls for denied access
    if !allowed && rand::random::<f64>() < sample_rate {
        if let Ok(explanation) = vault
            .explain_permission(subject, permission, resource)
            .await
        {
            tracing::debug!(
                subject = subject,
                permission = permission,
                resource = resource,
                explanation = ?explanation,
                "Sampled denial explanation"
            );
        }
    }

    Ok(allowed)
}
```

### Health Check with Authorization Test

```rust
async fn authz_health_check(vault: &VaultClient) -> Result<(), Error> {
    // Use a known-good check to verify service health
    let result = vault
        .check("healthcheck:probe", "access", "system:health")
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) if e.is_retriable() => {
            tracing::warn!(error = %e, "Authorization service degraded");
            Err(e)
        }
        Err(e) => {
            tracing::error!(error = %e, "Authorization service unhealthy");
            Err(e)
        }
    }
}
```

## Testing and Validation

### Unit Testing Authorization Logic

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_editor_can_edit() {
    let client = InMemoryClient::new();

    // Setup
    client.write_batch([
        Relationship::new("document:test", "editor", "user:alice"),
    ]).await.unwrap();

    // Test
    assert!(client.check("user:alice", "edit", "document:test").await.unwrap());
    assert!(!client.check("user:bob", "edit", "document:test").await.unwrap());
}

#[tokio::test]
async fn test_hierarchy_inheritance() {
    let client = InMemoryClient::new();

    // Setup hierarchy
    client.write_batch([
        Relationship::new("folder:root", "editor", "user:alice"),
        Relationship::new("document:readme", "parent", "folder:root"),
    ]).await.unwrap();

    // Test inheritance
    assert!(client.check("user:alice", "edit", "document:readme").await.unwrap());
}
```

### Simulation Testing

Test changes before deploying:

```rust
// Test new schema against existing relationships
let simulation = vault
    .simulate()
    .with_schema(include_str!("schema_v2.ipl"))
    .build();

// Run critical checks against simulation
let checks = [
    ("user:admin", "manage", "organization:main"),
    ("user:alice", "edit", "document:important"),
];

for (subject, permission, resource) in checks {
    let prod = vault.check(subject, permission, resource).await?;
    let sim = simulation.check(subject, permission, resource).await?;

    if prod != sim {
        panic!(
            "Schema change affects {} {} {}: {} -> {}",
            subject, permission, resource, prod, sim
        );
    }
}
```

## Tools Reference

| Tool                     | Use Case                                 |
| ------------------------ | ---------------------------------------- |
| `explain_permission()`   | Understand why access was granted/denied |
| `check().trace(true)`    | Get detailed decision tree               |
| `relationships().list()` | Verify relationships exist               |
| `simulate()`             | Test schema changes safely               |
| Request ID               | Debug production issues with support     |

## Best Practices

1. **Use explain first** - Before diving deep, use `explain_permission()` for quick diagnosis
2. **Check relationships** - Verify the relationship actually exists with the exact values
3. **Watch for case sensitivity** - Entity IDs are case-sensitive
4. **Log request IDs** - Essential for production debugging
5. **Test with simulation** - Validate schema changes before deploying
6. **Use structured logging** - Include subject, permission, resource in all auth logs
7. **Sample explain in production** - Get visibility into denials without overhead
