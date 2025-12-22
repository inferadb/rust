# Schema Versioning

Manage schema evolution safely as your authorization requirements change.

## Schema Lifecycle

Authorization schemas evolve as your application grows:

1. **Initial design** - Model your core entities and permissions
2. **Feature additions** - Add new types, relations, permissions
3. **Refactoring** - Rename, restructure, optimize
4. **Deprecation** - Remove unused elements

Each change requires careful consideration of existing relationships and application code.

## Change Categories

### Backward-Compatible Changes

These changes are safe to deploy without coordination:

| Change             | Example                                        | Why Safe                           |
| ------------------ | ---------------------------------------------- | ---------------------------------- |
| Add new type       | `type comment { ... }`                         | No existing relationships affected |
| Add new relation   | `relation commenter: user`                     | Existing checks unaffected         |
| Add new permission | `permission archive = owner`                   | Existing checks unaffected         |
| Expand permission  | `view = viewer` → `view = viewer \| commenter` | Grants more access, not less       |

### Breaking Changes

These changes can break existing functionality:

| Change               | Example                                         | Risk                             |
| -------------------- | ----------------------------------------------- | -------------------------------- |
| Remove type          | Remove `type comment`                           | Orphaned relationships           |
| Remove relation      | Remove `relation viewer`                        | Broken relationships             |
| Remove permission    | Remove `permission archive`                     | Broken application code          |
| Restrict permission  | `view = viewer \| editor` → `view = viewer`     | Denies previously granted access |
| Rename relation      | `viewer` → `reader`                             | Broken relationships and code    |
| Change relation type | `viewer: user` → `viewer: user \| group#member` | May work, but needs testing      |

## Safe Schema Updates

### Testing with Simulation

Always test schema changes before deploying:

```rust
use inferadb::testing::SimulationSnapshot;

// 1. Capture baseline behavior
let critical_checks = [
    ("user:admin", "manage", "organization:main"),
    ("user:alice", "edit", "document:important"),
    ("user:bob", "view", "folder:public"),
    // Add all critical permission paths
];

let baseline = SimulationSnapshot::capture(&vault, &critical_checks).await;

// 2. Simulate new schema
let simulation = vault
    .simulate()
    .with_schema(include_str!("schema_v2.ipl"))
    .build();

let after = SimulationSnapshot::capture(&simulation, &critical_checks).await;

// 3. Compare and fail if unexpected changes
let diff = baseline.diff(&after);
if !diff.is_empty() {
    for change in &diff {
        println!(
            "{} {} {} : {} -> {}",
            change.subject,
            change.permission,
            change.resource,
            change.before,
            change.after
        );
    }
    panic!("Schema change has unexpected permission changes!");
}
```

### Comprehensive Simulation Testing

```rust
async fn validate_schema_change(
    vault: &VaultClient,
    new_schema: &str,
) -> Result<ValidationReport, Error> {
    let simulation = vault
        .simulate()
        .with_schema(new_schema)
        .build();

    let mut report = ValidationReport::default();

    // Test all existing relationships still work
    let relationships = vault.export().to_vec().await?;
    for rel in &relationships {
        // Validate relationship is valid in new schema
        let valid = simulation
            .relationships()
            .validate(rel.clone())
            .await?;

        if !valid.valid {
            report.invalid_relationships.push(InvalidRelationship {
                relationship: rel.clone(),
                error: valid.error.unwrap_or_default(),
            });
        }
    }

    // Test permission paths still work
    // (requires knowing your critical paths)
    let permission_tests = load_permission_tests();
    for test in permission_tests {
        let prod = vault.check(&test.subject, &test.permission, &test.resource).await?;
        let sim = simulation.check(&test.subject, &test.permission, &test.resource).await?;

        if prod != sim {
            report.permission_changes.push(PermissionChange {
                subject: test.subject,
                permission: test.permission,
                resource: test.resource,
                before: prod,
                after: sim,
            });
        }
    }

    Ok(report)
}
```

## Migration Patterns

### Adding a New Relation

Safe to add and start using immediately:

```rust
// 1. Deploy schema with new relation
let schema_v2 = r#"
type document {
    relation viewer: user
    relation editor: user
    relation commenter: user  // NEW

    permission view = viewer | editor | commenter  // Include new relation
    permission comment = commenter | editor
    permission edit = editor
}
"#;

vault.schema().deploy(schema_v2).await?;

// 2. Start writing new relationships
vault.relationships()
    .write(Relationship::new("document:readme", "commenter", "user:alice"))
    .await?;
```

### Renaming a Relation

Requires relationship migration:

```rust
// Phase 1: Add new relation, keep old one
let schema_transition = r#"
type document {
    relation viewer: user      // OLD - keep temporarily
    relation reader: user      // NEW
    relation editor: user

    // Support both during transition
    permission view = viewer | reader | editor
    permission edit = editor
}
"#;

vault.schema().deploy(schema_transition).await?;

// Phase 2: Migrate relationships
let old_rels = vault.relationships()
    .list()
    .relation("viewer")
    .collect()
    .await?;

for rel in old_rels {
    // Write new relationship
    vault.relationships()
        .write(Relationship::new(&rel.resource, "reader", &rel.subject))
        .await?;

    // Delete old relationship
    vault.relationships()
        .delete(rel)
        .await?;
}

// Phase 3: Update application code to use "reader"
// ... deploy application changes ...

// Phase 4: Remove old relation from schema
let schema_final = r#"
type document {
    relation reader: user      // Only new relation
    relation editor: user

    permission view = reader | editor
    permission edit = editor
}
"#;

vault.schema().deploy(schema_final).await?;
```

### Restricting a Permission

Requires careful analysis and communication:

```rust
// Before: editors can delete
// permission delete = editor | owner

// After: only owners can delete
// permission delete = owner

// 1. Identify affected users BEFORE deploying
let affected = vault
    .subjects()
    .with_permission("delete")
    .on_resource_type("document")
    .collect()
    .await?;

let will_lose_access: Vec<_> = affected
    .into_iter()
    .filter(|s| {
        // Check if they have delete via editor but not owner
        let is_editor = vault.check(&s, "editor", "document:*").await.unwrap_or(false);
        let is_owner = vault.check(&s, "owner", "document:*").await.unwrap_or(false);
        is_editor && !is_owner
    })
    .collect();

// 2. Communicate change to affected users
for user in &will_lose_access {
    notify_permission_change(user, "delete", "document").await;
}

// 3. Simulate and verify
let simulation = vault
    .simulate()
    .with_schema(new_schema)
    .build();

// Verify expected behavior
for user in &will_lose_access {
    let allowed = simulation.check(user, "delete", "document:any").await?;
    assert!(!allowed, "User {} should lose delete access", user);
}

// 4. Deploy
vault.schema().deploy(new_schema).await?;
```

### Removing a Type

Clean up relationships first:

```rust
// 1. Remove all relationships involving the type
vault.relationships()
    .delete_where()
    .resource_type("deprecated_type")
    .dry_run()
    .await?;  // Preview first!

vault.relationships()
    .delete_where()
    .resource_type("deprecated_type")
    .execute()
    .await?;

vault.relationships()
    .delete_where()
    .subject_type("deprecated_type")
    .execute()
    .await?;

// 2. Update application code to stop referencing the type
// ... deploy application changes ...

// 3. Remove from schema
vault.schema().deploy(schema_without_deprecated).await?;
```

## Version Control Best Practices

### Store Schemas in Git

```text
project/
├── src/
├── schemas/
│   ├── v1.ipl          # Original schema
│   ├── v2.ipl          # Added comments feature
│   ├── v3.ipl          # Renamed viewer->reader
│   └── current.ipl     # Symlink to active version
└── migrations/
    ├── 001_initial.rs
    ├── 002_add_comments.rs
    └── 003_rename_viewer.rs
```

### Schema Deployment Script

```rust
use std::fs;

async fn deploy_schema(vault: &VaultClient, version: &str) -> Result<(), Error> {
    let schema_path = format!("schemas/{}.ipl", version);
    let schema = fs::read_to_string(&schema_path)?;

    // Validate before deploying
    let validation = vault.schema().validate(&schema).await?;
    if !validation.valid {
        return Err(Error::SchemaInvalid(validation.errors));
    }

    // Simulate to catch breaking changes
    let simulation = vault
        .simulate()
        .with_schema(&schema)
        .build();

    // Run critical checks
    let checks = load_critical_checks();
    for (subject, perm, resource, expected) in checks {
        let result = simulation.check(&subject, &perm, &resource).await?;
        if result != expected {
            return Err(Error::BreakingChange {
                subject, perm, resource, expected, actual: result
            });
        }
    }

    // Deploy
    vault.schema().deploy(&schema).await?;

    println!("Deployed schema version: {}", version);
    Ok(())
}
```

### CI/CD Integration

```yaml
# .github/workflows/schema.yml
name: Schema Validation

on:
  pull_request:
    paths:
      - "schemas/**"

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Validate Schema Syntax
        run: inferadb schema validate schemas/current.ipl

      - name: Test Against Staging
        env:
          INFERADB_URL: ${{ secrets.STAGING_URL }}
          INFERADB_CLIENT_ID: ${{ secrets.CI_CLIENT_ID }}
        run: |
          cargo run --bin schema-test -- \
            --schema schemas/current.ipl \
            --checks tests/critical-checks.json
```

## Rollback Strategies

### Immediate Rollback

If a schema change causes issues:

```rust
// Keep previous schema version
let previous_schema = include_str!("schemas/v2.ipl");

// Rollback
async fn rollback_schema(vault: &VaultClient) -> Result<(), Error> {
    vault.schema().deploy(previous_schema).await?;
    println!("Rolled back to previous schema version");
    Ok(())
}
```

### Relationship Restoration

If relationships were modified during migration:

```rust
// Before migration, export relationships
let backup = vault.export().to_vec().await?;
fs::write("relationships_backup.json", serde_json::to_string(&backup)?)?;

// If rollback needed
async fn restore_relationships(vault: &VaultClient) -> Result<(), Error> {
    let backup: Vec<Relationship> = serde_json::from_str(
        &fs::read_to_string("relationships_backup.json")?
    )?;

    vault.import()
        .mode(ImportMode::Replace)
        .confirm_replace(true)
        .from_vec(backup)
        .await?;

    Ok(())
}
```

## Schema Deprecation

### Deprecation Workflow

1. **Mark as deprecated** - Add comments, update documentation
2. **Warn in application** - Log when deprecated paths are used
3. **Set removal date** - Communicate timeline
4. **Remove** - After grace period, remove from schema

```rust
// Track usage of deprecated features
async fn check_with_deprecation_warning(
    vault: &VaultClient,
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    let result = vault.check(subject, permission, resource).await?;

    // Warn if using deprecated permission
    if permission == "legacy_view" {
        tracing::warn!(
            subject = subject,
            resource = resource,
            "Using deprecated permission 'legacy_view'. Migrate to 'view' by 2024-06-01."
        );
    }

    Ok(result)
}
```

## Best Practices Summary

1. **Always simulate first** - Test schema changes against production data
2. **Use backward-compatible changes** - Add, don't remove or rename
3. **Migrate in phases** - Add new → migrate data → remove old
4. **Keep backups** - Export relationships before major changes
5. **Version in Git** - Track all schema versions
6. **Automate validation** - Run schema tests in CI/CD
7. **Communicate breaking changes** - Notify affected users before deploying
8. **Have rollback plan** - Keep previous schema and relationship backups ready
