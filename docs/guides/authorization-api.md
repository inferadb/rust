# Authorization API

The Authorization API is the core of InferaDB's SDK, providing permission checks, relationship management, lookups, and real-time change streaming.

## Getting Started

```rust
let org = client.organization("org_...");
let vault = org.vault("vlt_...");
```

## Permission Checks

### Basic Check

```rust
let allowed = vault.check("user:alice", "view", "doc:1").await?;
if allowed {
    // Grant access
}
```

### Check with ABAC Context

Add attribute-based context to permission checks for fine-grained access control:

```rust
let allowed = vault.check("user:alice", "view", "doc:confidential")
    .with_context(Context::new()
        .with("ip_address", "10.0.0.50")
        .with("mfa_verified", true)
        .with("department", "engineering"))
    .await?;
```

Context values are evaluated against conditions defined in your authorization schema.

### Require Permission (Guard Clause)

Use `require()` when denial should be an error (e.g., in middleware):

```rust
vault.check("user:alice", "edit", "doc:1").require().await?;
// If denied, returns Err(AccessDenied)
```

### Batch Checks

Check multiple permissions efficiently in a single request:

```rust
let results = vault.check_batch([
    ("user:alice", "view", "doc:1"),
    ("user:alice", "edit", "doc:1"),
    ("user:alice", "delete", "doc:1"),
]).await?;

// Results maintain input order
for (i, allowed) in results.iter().enumerate() {
    println!("Check {}: {}", i, allowed);
}

// Convenience methods
if results.all_allowed() {
    // All permissions granted
}

let denied = results.denied_indices();
```

### Consistency Tokens

For read-after-write consistency, use consistency tokens:

```rust
// Write returns a token
let token = vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

// Use token for consistent read
let allowed = vault.check("user:alice", "view", "doc:1")
    .at_least_as_fresh(token)
    .await?;
```

## Relationships

### Write a Relationship

```rust
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

### Write Multiple Relationships

```rust
vault.relationships().write_batch([
    Relationship::new("folder:docs", "viewer", "group:engineering#member"),
    Relationship::new("document:readme", "parent", "folder:docs"),
]).await?;
```

### List Relationships

```rust
let rels = vault.relationships()
    .list()
    .resource("document:readme")
    .collect()
    .await?;

for rel in rels {
    println!("{} -> {} -> {}", rel.resource(), rel.relation(), rel.subject());
}
```

Filter by relation or subject:

```rust
let viewers = vault.relationships()
    .list()
    .resource("document:readme")
    .relation("viewer")
    .collect()
    .await?;
```

### Delete a Relationship

```rust
vault.relationships()
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

### Delete Multiple Relationships

Delete all relationships matching a filter:

```rust
vault.relationships()
    .delete_where()
    .resource("document:readme")
    .execute()
    .await?;
```

Filter by relation or subject:

```rust
vault.relationships()
    .delete_where()
    .resource("document:readme")
    .relation("viewer")
    .subject("user:alice")
    .execute()
    .await?;
```

## Lookups

### List Accessible Resources

Find all resources a subject can access:

```rust
let docs = vault.resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;

for doc in docs {
    println!("Alice can view: {}", doc);
}
```

### List Subjects with Access

Find all subjects that have access to a resource:

```rust
let users = vault.subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .collect()
    .await?;

for user in users {
    println!("{} can view document:readme", user);
}
```

Filter by subject type:

```rust
let groups = vault.subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .subject_type("group")
    .collect()
    .await?;
```

## Explain & Simulate

### Explain a Permission Decision

Understand why access was allowed or denied:

```rust
let explanation = vault.explain_permission()
    .subject("user:alice")
    .permission("edit")
    .resource("document:readme")
    .execute()
    .await?;

println!("{}", explanation.summary());

if explanation.allowed {
    // Show the path that granted access
    for node in &explanation.path {
        println!("  via {} on {}", node.relation, node.resource);
    }
} else {
    // Show why access was denied
    println!("Denied: {:?}", explanation.reason);
}
```

### Simulate What-If Scenarios

Test hypothetical changes without modifying data:

```rust
let result = vault.simulate()
    .add_relationship(Relationship::new("doc:1", "editor", "user:bob"))
    .check("user:bob", "edit", "doc:1")
    .await?;

if result.allowed {
    println!("Adding this relationship would grant access");
}
```

Simulate multiple changes:

```rust
let result = vault.simulate()
    .add_relationship(Relationship::new("folder:docs", "viewer", "user:bob"))
    .add_relationship(Relationship::new("doc:1", "parent", "folder:docs"))
    .remove_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
    .check("user:bob", "view", "doc:1")
    .await?;
```

## Watch for Changes

Stream real-time relationship changes:

```rust
use futures::StreamExt;

let mut stream = vault.watch()
    .filter(WatchFilter::resource_type("document"))
    .run()
    .await?;

while let Some(event) = stream.next().await {
    let event = event?;
    println!("{}: {} {} {}",
        event.operation, event.resource, event.relation, event.subject);
}
```

### Filter Options

```rust
// Filter by resource type
.filter(WatchFilter::resource_type("document"))

// Filter by specific resource
.filter(WatchFilter::resource("document:readme"))

// Filter by relation
.filter(WatchFilter::relation("viewer"))

// Filter by operation type
.filter(WatchFilter::operations([Operation::Create, Operation::Delete]))

// Combine multiple filters
.filter(WatchFilter::resource_type("document"))
.filter(WatchFilter::relation("editor"))
```

### Resumable Streams

For reliable processing, use resumable streams with revision tracking:

```rust
let mut stream = vault.watch()
    .from_revision(last_processed_revision)
    .resumable()
    .run()
    .await?;
```

## Error Handling

Permission checks return `Ok(false)` for denial, not errors:

```rust
match vault.check("user:alice", "view", "doc:1").await {
    Ok(true) => println!("Access granted"),
    Ok(false) => println!("Access denied"),
    Err(e) => println!("Error: {}", e),  // Network, auth, etc.
}
```

Use `require()` when you want denial to be an error:

```rust
vault.check("user:alice", "view", "doc:1")
    .require()
    .await?;  // Returns Err(AccessDenied) if denied
```

## See Also

- [Error Handling](errors.md) - Detailed error handling patterns
- [Testing](testing.md) - MockClient and InMemoryClient for testing
- [Consistency](consistency.md) - Consistency tokens and guarantees
- [Integration Patterns](integration-patterns.md) - Framework middleware examples
