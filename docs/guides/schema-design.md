# Schema Design Patterns

Best practices for designing authorization schemas in InferaDB.

## Core Concepts

InferaDB uses a relationship-based access control (ReBAC) model where permissions are derived from relationships between entities. Understanding these patterns helps you design schemas that are expressive, maintainable, and performant.

## Role Hierarchy Pattern

Model roles with inherited permissions using computed relations.

### Basic Role Hierarchy

```
// Schema (IPL)
type document {
    relation viewer: user | group#member
    relation editor: user | group#member
    relation owner: user

    // Role hierarchy: owner > editor > viewer
    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
}
```

```rust
// Write relationships
vault.relationships()
    .write(Relationship::new("document:readme", "owner", "user:alice"))
    .await?;

// Alice can view, edit, and delete (as owner)
assert!(vault.check("user:alice", "view", "document:readme").await?);
assert!(vault.check("user:alice", "edit", "document:readme").await?);
assert!(vault.check("user:alice", "delete", "document:readme").await?);
```

### Extended Role Hierarchy

For applications with more granular roles:

```
type document {
    relation viewer: user | group#member
    relation commenter: user | group#member
    relation editor: user | group#member
    relation admin: user | group#member
    relation owner: user

    // Layered permissions
    permission view = viewer | commenter | editor | admin | owner
    permission comment = commenter | editor | admin | owner
    permission edit = editor | admin | owner
    permission manage = admin | owner
    permission delete = owner
}
```

## Resource Hierarchy Pattern

Model parent-child relationships where permissions cascade down.

### Folder/Document Hierarchy

```
type folder {
    relation viewer: user | group#member
    relation editor: user | group#member
    relation owner: user

    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
}

type document {
    relation parent: folder
    relation viewer: user | group#member
    relation editor: user | group#member
    relation owner: user

    // Inherit from parent folder
    permission view = viewer | editor | owner | parent->view
    permission edit = editor | owner | parent->edit
    permission delete = owner | parent->delete
}
```

```rust
// Set up hierarchy
vault.relationships()
    .write_batch([
        // Alice owns the folder
        Relationship::new("folder:engineering", "owner", "user:alice"),
        // Document is in the folder
        Relationship::new("document:design-doc", "parent", "folder:engineering"),
    ])
    .await?;

// Alice can access document via folder ownership
assert!(vault.check("user:alice", "view", "document:design-doc").await?);
assert!(vault.check("user:alice", "delete", "document:design-doc").await?);
```

### Deep Hierarchies

For deeply nested resources, use recursive relations:

```
type folder {
    relation parent: folder
    relation viewer: user | group#member
    relation editor: user | group#member

    // Recursive: inherit from parent folder
    permission view = viewer | editor | parent->view
    permission edit = editor | parent->edit
}
```

## Organization Scoping Pattern

Essential for multi-tenant SaaS applications.

### Basic Organization Isolation

```
type organization {
    relation member: user
    relation admin: user

    permission access = member | admin
    permission manage = admin
}

type project {
    relation org: organization
    relation member: user
    relation admin: user

    // Must be org member to access project
    permission view = (member | admin) & org->access
    permission manage = admin & org->access
}
```

```rust
// Set up organization
vault.relationships()
    .write_batch([
        Relationship::new("organization:acme", "member", "user:alice"),
        Relationship::new("organization:acme", "admin", "user:bob"),
        Relationship::new("project:widget", "org", "organization:acme"),
        Relationship::new("project:widget", "member", "user:alice"),
    ])
    .await?;

// Alice can view (org member + project member)
assert!(vault.check("user:alice", "view", "project:widget").await?);

// Charlie cannot view (not in org)
assert!(!vault.check("user:charlie", "view", "project:widget").await?);
```

### Organization with Teams

```
type organization {
    relation member: user
    relation admin: user
}

type team {
    relation org: organization
    relation member: user
    relation lead: user

    permission access = member | lead
}

type project {
    relation org: organization
    relation team: team
    relation member: user

    // Access via direct membership or team membership
    permission view = member | team->access
    permission manage = team->lead
}
```

## Group Membership Pattern

Use groups for managing permissions at scale.

### Basic Groups

```
type group {
    relation member: user | group#member  // Nested groups supported
    relation admin: user
}

type resource {
    relation viewer: user | group#member
    relation editor: user | group#member

    permission view = viewer | editor
    permission edit = editor
}
```

```rust
// Create group with members
vault.relationships()
    .write_batch([
        Relationship::new("group:engineering", "member", "user:alice"),
        Relationship::new("group:engineering", "member", "user:bob"),
        // Grant group access to resource
        Relationship::new("resource:api-docs", "viewer", "group:engineering#member"),
    ])
    .await?;

// Both Alice and Bob can view
assert!(vault.check("user:alice", "view", "resource:api-docs").await?);
assert!(vault.check("user:bob", "view", "resource:api-docs").await?);
```

### Nested Groups

```rust
// Engineering contains backend team
vault.relationships()
    .write_batch([
        Relationship::new("group:backend", "member", "user:alice"),
        Relationship::new("group:engineering", "member", "group:backend#member"),
        Relationship::new("resource:infra", "editor", "group:engineering#member"),
    ])
    .await?;

// Alice (in backend, which is in engineering) can edit
assert!(vault.check("user:alice", "edit", "resource:infra").await?);
```

## Attribute-Based Conditions

Combine ReBAC with runtime attributes for fine-grained control.

### IP-Based Access

```
type document {
    relation viewer: user
    relation confidential_viewer: user

    permission view = viewer
    permission view_confidential = confidential_viewer & context.ip_in_allowlist
}
```

```rust
use inferadb::Context;

// Check with runtime context
let allowed = vault
    .check("user:alice", "view_confidential", "document:secret")
    .with_context(Context::new()
        .insert("ip_in_allowlist", true))
    .await?;
```

### Time-Based Access

```
type resource {
    relation viewer: user
    relation after_hours_viewer: user

    permission view = viewer | (after_hours_viewer & context.is_business_hours)
}
```

## Common Anti-Patterns

### Anti-Pattern: Over-Flattening

```
// BAD: Duplicating permissions across types
type document {
    relation org_admin: user    // Don't duplicate org structure
    relation org_member: user   // in every resource type
    // ...
}

// GOOD: Reference organization
type document {
    relation org: organization
    permission admin_access = org->admin
}
```

### Anti-Pattern: Permission Explosion

```
// BAD: Separate relation for every permission
type document {
    relation can_view: user
    relation can_edit: user
    relation can_delete: user
    relation can_share: user
    relation can_comment: user
    // ... dozens more
}

// GOOD: Role-based with computed permissions
type document {
    relation viewer: user
    relation editor: user
    relation owner: user

    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
    permission share = owner
    permission comment = viewer | editor | owner
}
```

### Anti-Pattern: Missing Hierarchy

```
// BAD: No inheritance, must grant access at every level
type folder { relation viewer: user }
type subfolder { relation viewer: user }
type document { relation viewer: user }

// GOOD: Hierarchical inheritance
type folder {
    relation parent: folder
    relation viewer: user
    permission view = viewer | parent->view
}
```

## Performance Considerations

### Keep Hierarchies Shallow

Deep hierarchies require more graph traversal:

```
// Prefer: 2-3 levels
organization -> project -> document

// Avoid: 6+ levels
organization -> division -> department -> team -> folder -> subfolder -> document
```

### Use Direct Relations for Hot Paths

For frequently-checked permissions, consider direct relations:

```rust
// If checking document view is very frequent, consider caching
// the resolved permission as a direct relation
vault.relationships()
    .write(Relationship::new("document:hot", "cached_viewer", "user:frequent"))
    .await?;
```

### Batch Related Checks

```rust
// Instead of checking permissions one by one
let checks = documents.iter()
    .map(|d| (user_id, "view", format!("document:{}", d.id)))
    .collect::<Vec<_>>();

let results = vault.check_batch(&checks).collect().await?;
```

## Schema Evolution

See [Schema Versioning](schema-versioning.md) for managing schema changes over time.

## Best Practices Summary

1. **Start with roles** - Model your domain's roles first, then derive permissions
2. **Use hierarchies** - Leverage parent relationships to reduce relationship count
3. **Scope by organization** - Always include org scoping for multi-tenant apps
4. **Prefer groups** - Manage permissions via group membership, not individual grants
5. **Keep it shallow** - Limit hierarchy depth for performance
6. **Test with simulation** - Validate schema changes before deploying
