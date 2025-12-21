# Migration Guide

Migrate to InferaDB from other authorization systems.

## From Custom RBAC Middleware

Most applications start with role-based middleware that checks roles against a database.

### Before: Custom RBAC

```rust
// Typical custom RBAC pattern
async fn check_permission(
    db: &Database,
    user_id: &str,
    permission: &str,
    resource_id: &str,
) -> bool {
    // Get user's roles
    let roles = db.get_user_roles(user_id).await;

    // Check if any role has the permission
    for role in roles {
        let perms = db.get_role_permissions(&role).await;
        if perms.contains(&permission.to_string()) {
            // Check resource scope
            if db.role_has_resource_access(&role, resource_id).await {
                return true;
            }
        }
    }
    false
}

// Usage in handler
async fn get_document(user: User, doc_id: String) -> Result<Document, Error> {
    if !check_permission(&db, &user.id, "read", &doc_id).await {
        return Err(Error::Forbidden);
    }
    // ...
}
```

### After: InferaDB

```rust
use inferadb::prelude::*;

// One-time setup: migrate role assignments to relationships
async fn migrate_roles(db: &Database, vault: &VaultClient) -> Result<(), Error> {
    let role_assignments = db.get_all_role_assignments().await?;

    let relationships: Vec<_> = role_assignments.iter()
        .map(|ra| Relationship::new(
            &format!("{}:{}", ra.resource_type, ra.resource_id),
            &ra.role,  // "viewer", "editor", "admin"
            &format!("user:{}", ra.user_id),
        ))
        .collect();

    vault.relationships().write_batch(relationships).await
}

// Usage in handler - much simpler
async fn get_document(
    vault: &VaultClient,
    user: User,
    doc_id: String,
) -> Result<Document, Error> {
    vault.check(&format!("user:{}", user.id), "view", &format!("document:{}", doc_id))
        .require()
        .await?;
    // ...
}
```

### Migration Strategy

1. **Define schema** - Convert your role/permission matrix to IPL schema
2. **Export relationships** - Extract role assignments from your database
3. **Import to InferaDB** - Use `vault.import()` or batch writes
4. **Run in parallel** - Check both systems, compare results
5. **Switch over** - Once confident, remove old RBAC code

```rust
// Parallel checking during migration
async fn check_with_comparison(
    db: &Database,
    vault: &VaultClient,
    user: &str,
    perm: &str,
    resource: &str,
) -> bool {
    let old_result = check_permission_old(db, user, perm, resource).await;
    let new_result = vault.check(user, perm, resource).await.unwrap_or(false);

    if old_result != new_result {
        tracing::warn!(
            user = user,
            permission = perm,
            resource = resource,
            old = old_result,
            new = new_result,
            "Authorization mismatch during migration"
        );
    }

    old_result  // Use old system until migration complete
}
```

## From SpiceDB

SpiceDB and InferaDB share similar concepts (both inspired by Zanzibar).

### Concept Mapping

| SpiceDB              | InferaDB                             | Notes                               |
| -------------------- | ------------------------------------ | ----------------------------------- |
| Schema (Zed)         | Schema (IPL)                         | Similar structure, different syntax |
| `definition`         | `type`                               | Entity type definition              |
| `relation`           | `relation`                           | Direct relationship                 |
| `permission`         | `permission`                         | Computed permission                 |
| `WriteRelationships` | `vault.relationships().write()`      | Write operations                    |
| `CheckPermission`    | `vault.check()`                      | Permission checks                   |
| `LookupResources`    | `vault.resources().accessible_by()`  | Resource lookup                     |
| `LookupSubjects`     | `vault.subjects().with_permission()` | Subject lookup                      |
| Zed token            | Consistency token                    | Read-after-write consistency        |

### Schema Migration

```
// SpiceDB (Zed language)
definition user {}

definition document {
    relation viewer: user
    relation editor: user
    relation owner: user

    permission view = viewer + editor + owner
    permission edit = editor + owner
    permission delete = owner
}
```

```
// InferaDB (IPL)
type user {}

type document {
    relation viewer: user
    relation editor: user
    relation owner: user

    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
}
```

### Code Migration

```rust
// SpiceDB client
let client = SpiceDbClient::new("localhost:50051").await?;
let result = client
    .check_permission(CheckPermissionRequest {
        resource: ObjectReference {
            object_type: "document".into(),
            object_id: "readme".into(),
        },
        permission: "view".into(),
        subject: SubjectReference {
            object: ObjectReference {
                object_type: "user".into(),
                object_id: "alice".into(),
            },
            optional_relation: "".into(),
        },
        ..Default::default()
    })
    .await?;

let allowed = result.permissionship == Permissionship::HasPermission;
```

```rust
// InferaDB client
let client = Client::from_env().await?;
let vault = client.organization("org_...").vault("vlt_...");

let allowed = vault
    .check("user:alice", "view", "document:readme")
    .await?;
```

### Relationship Migration

```rust
// Export from SpiceDB
let relationships = spicedb_client
    .read_relationships(ReadRelationshipsRequest {
        relationship_filter: RelationshipFilter {
            resource_type: "document".into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .await?;

// Convert to InferaDB format
let inferadb_rels: Vec<_> = relationships.iter()
    .map(|r| Relationship::new(
        &format!("{}:{}", r.resource.object_type, r.resource.object_id),
        &r.relation,
        &format!("{}:{}", r.subject.object.object_type, r.subject.object.object_id),
    ))
    .collect();

// Import to InferaDB
vault.relationships().write_batch(inferadb_rels).await?;
```

## From OpenFGA

OpenFGA uses a JSON-based schema format and similar ReBAC concepts.

### Concept Mapping

| OpenFGA             | InferaDB                            | Notes                          |
| ------------------- | ----------------------------------- | ------------------------------ |
| Authorization Model | Schema (IPL)                        | Type definitions               |
| `type`              | `type`                              | Entity type                    |
| `relations`         | `relation`                          | Relationships                  |
| `define`            | `permission`                        | Computed permissions           |
| Tuple               | Relationship                        | Subject-relation-object triple |
| `Check`             | `vault.check()`                     | Permission check               |
| `ListObjects`       | `vault.resources().accessible_by()` | Resource lookup                |

### Schema Migration

```json
// OpenFGA model
{
  "type_definitions": [
    {
      "type": "document",
      "relations": {
        "viewer": {
          "this": {}
        },
        "editor": {
          "this": {}
        },
        "owner": {
          "this": {}
        },
        "can_view": {
          "union": {
            "child": [
              { "computedUserset": { "relation": "viewer" } },
              { "computedUserset": { "relation": "editor" } },
              { "computedUserset": { "relation": "owner" } }
            ]
          }
        }
      }
    }
  ]
}
```

```
// InferaDB (IPL)
type document {
    relation viewer: user
    relation editor: user
    relation owner: user

    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
}
```

### Code Migration

```rust
// OpenFGA client
let client = OpenFgaClient::new(config)?;
let response = client.check(CheckRequest {
    tuple_key: TupleKey {
        user: "user:alice".into(),
        relation: "can_view".into(),
        object: "document:readme".into(),
    },
    ..Default::default()
}).await?;

let allowed = response.allowed;
```

```rust
// InferaDB client
let vault = client.organization("org_...").vault("vlt_...");

let allowed = vault
    .check("user:alice", "view", "document:readme")
    .await?;
```

### Tuple Migration

```rust
// Export from OpenFGA
let tuples = openfga_client.read(ReadRequest {
    tuple_key: TupleKey::default(),
    ..Default::default()
}).await?.tuples;

// Convert to InferaDB format
let relationships: Vec<_> = tuples.iter()
    .map(|t| Relationship::new(&t.key.object, &t.key.relation, &t.key.user))
    .collect();

// Import to InferaDB
vault.relationships().write_batch(relationships).await?;
```

## From Oso/Polar

Oso uses a Prolog-like policy language (Polar) with a different paradigm.

### Concept Differences

| Oso/Polar           | InferaDB        | Notes                               |
| ------------------- | --------------- | ----------------------------------- |
| Polar rules         | IPL schema      | Different paradigm (logic vs graph) |
| `allow` rules       | `permission`    | Permission definitions              |
| Application classes | `type`          | Entity types                        |
| `Oso.authorize()`   | `vault.check()` | Permission checks                   |
| Local evaluation    | Remote API      | Oso runs in-process                 |

### Migration Considerations

Oso evaluates policies locally using your application objects. InferaDB stores relationships centrally and evaluates remotely. Key differences:

1. **Data model**: Move from object attributes to explicit relationships
2. **Evaluation**: From local to remote API calls
3. **Policy language**: From Prolog-like rules to graph-based permissions

### Schema Migration

```polar
# Oso/Polar
actor User {}

resource Document {
    roles = ["viewer", "editor", "owner"];
    permissions = ["view", "edit", "delete"];

    "view" if "viewer";
    "view" if "editor";
    "view" if "owner";
    "edit" if "editor";
    "edit" if "owner";
    "delete" if "owner";
}

has_role(user: User, "viewer", doc: Document) if
    doc.viewers.contains(user);
```

```
// InferaDB (IPL)
type document {
    relation viewer: user
    relation editor: user
    relation owner: user

    permission view = viewer | editor | owner
    permission edit = editor | owner
    permission delete = owner
}
```

### Code Migration

```rust
// Oso
let oso = Oso::new();
oso.load_files(["policy.polar"])?;

let user = User { id: "alice".into(), /* ... */ };
let doc = Document { id: "readme".into(), viewers: vec!["alice".into()], /* ... */ };

let allowed = oso.is_allowed(user, "view", doc)?;
```

```rust
// InferaDB
let vault = client.organization("org_...").vault("vlt_...");

// Relationships are stored centrally, not in application objects
let allowed = vault
    .check("user:alice", "view", "document:readme")
    .await?;
```

### Relationship Extraction

```rust
// Extract relationships from Oso-style objects
async fn migrate_from_oso_model(vault: &VaultClient, docs: Vec<Document>) -> Result<(), Error> {
    let mut relationships = Vec::new();

    for doc in docs {
        let doc_ref = format!("document:{}", doc.id);

        for viewer in &doc.viewers {
            relationships.push(Relationship::new(&doc_ref, "viewer", &format!("user:{}", viewer)));
        }
        for editor in &doc.editors {
            relationships.push(Relationship::new(&doc_ref, "editor", &format!("user:{}", editor)));
        }
        if let Some(owner) = &doc.owner {
            relationships.push(Relationship::new(&doc_ref, "owner", &format!("user:{}", owner)));
        }
    }

    vault.relationships().write_batch(relationships).await
}
```

## Incremental Migration Strategy

For any migration, follow this incremental approach:

### Phase 1: Setup

```rust
// 1. Define your InferaDB schema
// 2. Create vault and configure client
let client = Client::from_env().await?;
let vault = client.organization("org_...").vault("vlt_...");

// 3. Deploy schema
vault.schema()
    .deploy(include_str!("schema.ipl"))
    .await?;
```

### Phase 2: Dual-Write

Write to both systems during transition:

```rust
async fn write_permission(
    old_db: &Database,
    vault: &VaultClient,
    user: &str,
    role: &str,
    resource: &str,
) -> Result<(), Error> {
    // Write to old system
    old_db.grant_role(user, role, resource).await?;

    // Write to InferaDB
    vault.relationships()
        .write(Relationship::new(resource, role, &format!("user:{}", user)))
        .await?;

    Ok(())
}
```

### Phase 3: Shadow Checking

Compare results without affecting production:

```rust
async fn check_with_shadow(
    old_system: &OldAuthz,
    vault: &VaultClient,
    user: &str,
    perm: &str,
    resource: &str,
) -> bool {
    let old_result = old_system.check(user, perm, resource).await;

    // Shadow check InferaDB (don't block on errors)
    tokio::spawn({
        let vault = vault.clone();
        let user = user.to_string();
        let perm = perm.to_string();
        let resource = resource.to_string();
        async move {
            match vault.check(&user, &perm, &resource).await {
                Ok(new_result) if new_result != old_result => {
                    tracing::warn!(
                        user = %user,
                        permission = %perm,
                        resource = %resource,
                        old = old_result,
                        new = new_result,
                        "Authorization mismatch"
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "Shadow check failed");
                }
                _ => {}
            }
        }
    });

    old_result
}
```

### Phase 4: Gradual Rollout

Switch traffic incrementally:

```rust
async fn check_permission(
    config: &MigrationConfig,
    old_system: &OldAuthz,
    vault: &VaultClient,
    user: &str,
    perm: &str,
    resource: &str,
) -> Result<bool, Error> {
    // Feature flag / percentage rollout
    if config.use_inferadb_for_user(user) {
        vault.check(user, perm, resource).await
    } else {
        Ok(old_system.check(user, perm, resource).await)
    }
}
```

### Phase 5: Cleanup

Once fully migrated:

1. Remove dual-write code
2. Remove shadow checking
3. Remove old authorization system
4. Clean up old database tables

## Validation

Before completing migration, validate:

```rust
// Export relationships for comparison
let exported = vault.export().to_vec().await?;

// Compare counts
assert_eq!(exported.len(), old_db.count_permissions().await?);

// Spot-check critical permissions
let critical_checks = [
    ("user:admin", "manage", "organization:main"),
    ("user:alice", "edit", "document:important"),
    // ...
];

for (user, perm, resource) in critical_checks {
    let old = old_system.check(user, perm, resource).await;
    let new = vault.check(user, perm, resource).await?;
    assert_eq!(old, new, "Mismatch for {} {} {}", user, perm, resource);
}
```

## Best Practices

1. **Migrate incrementally** - Don't big-bang switch; use shadow mode first
2. **Log mismatches** - Track differences between old and new systems
3. **Validate extensively** - Test critical permission paths before cutover
4. **Keep rollback plan** - Maintain ability to revert during migration
5. **Monitor closely** - Watch error rates and latency after switching
