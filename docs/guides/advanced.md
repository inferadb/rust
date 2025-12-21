# Advanced Features

Simulation, permission explanation, export/import, and type-safe schemas.

## Simulation (What-If Testing)

Test schema changes or relationship modifications without affecting production data.

### Schema Migration Testing

```rust
// Test a new schema before deploying
let simulation = vault
    .simulate()
    .with_schema(include_str!("schema_v2.ipl"))
    .build();

// Run checks against simulated schema
let allowed = simulation
    .check("user:alice", "view", "doc:1")
    .await?;

// Compare with production
let production_allowed = vault
    .check("user:alice", "view", "doc:1")
    .await?;

if allowed != production_allowed {
    eprintln!("Schema change affects user:alice access to doc:1");
}
```

### Relationship Testing

```rust
// Test adding relationships without committing
let simulation = vault
    .simulate()
    .with_relationships([
        Relationship::new("doc:1", "viewer", "user:bob"),
    ])
    .build();

// Check if Bob would have access
let allowed = simulation
    .check("user:bob", "view", "doc:1")
    .await?;

assert!(allowed);
```

### Combined Simulation

```rust
let simulation = vault
    .simulate()
    .with_schema(new_schema)
    .with_relationships(new_relationships)
    .without_relationships(removed_relationships)
    .build();
```

### Simulation Snapshot Testing

Compare behavior before and after changes:

```rust
use inferadb::testing::SimulationSnapshot;

// Capture current behavior
let baseline = SimulationSnapshot::capture(&vault, &[
    ("user:alice", "view", "doc:1"),
    ("user:bob", "edit", "doc:2"),
    ("user:charlie", "delete", "folder:root"),
]).await;

// Simulate new schema
let simulation = vault.simulate()
    .with_schema(new_schema)
    .build();

let after = SimulationSnapshot::capture(&simulation, &[
    ("user:alice", "view", "doc:1"),
    ("user:bob", "edit", "doc:2"),
    ("user:charlie", "delete", "folder:root"),
]).await;

// Fail if behavior changed unexpectedly
baseline.assert_unchanged(&after);
```

## Explain Permission

Understand why access was granted or denied.

### Basic Explanation

```rust
let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .await?;

if explanation.allowed {
    println!("Access granted via: {:?}", explanation.resolution_path);
} else {
    println!("Access denied. Reasons:");
    for reason in &explanation.denial_reasons {
        println!("  - {}", reason);
    }
}
```

### Decision Traces

Get detailed traces for debugging complex permissions:

```rust
let decision = vault
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .await?;

if let Some(trace) = &decision.trace {
    // Print decision tree
    println!("{}", trace.render_tree());

    // Find what granted access
    let paths = trace.find_satisfied_paths();
    for path in paths {
        println!("Granted via: {}", path.description());
    }

    // Find slow operations
    let slow = trace.find_nodes_slower_than(Duration::from_millis(10));
    for node in slow {
        println!("Slow: {:?} took {:?}", node.operation, node.metrics.duration);
    }
}
```

### Decision Structure

```rust
pub struct Decision {
    pub allowed: bool,
    pub reason: DecisionReason,
    pub trace: Option<DecisionNode>,
    pub metadata: DecisionMetadata,
}

pub enum DecisionReason {
    DirectRelationship(OwnedRelationship),
    ComputedPermission { path: Vec<String> },
    GroupMembership { group: String, relation: String },
    NoValidPath,
    ExplicitDenial { rule: String },
    ConditionResult { condition: String, satisfied: bool },
}
```

## Expand Operation

Show why a permission would be granted, including the full relationship graph:

```rust
let expansion = vault
    .expand("document:readme", "edit")
    .await?;

fn print_tree(node: &ExpansionNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{:?}: {}", indent, node.operation, node.description);
    for child in &node.children {
        print_tree(child, depth + 1);
    }
}
print_tree(&expansion, 0);

// Output:
// Union: edit permission
//   Direct: user:alice is owner of document:readme
//   Intersection: edit via folder
//     Direct: document:readme parent is folder:docs
//     Union: folder edit permission
//       Computed: user:alice has folder:docs#editor via group:engineering
```

## Export & Import

Backup and restore vault data.

### Export Relationships

```rust
// Export to file
vault.export()
    .to_file("backup.json")
    .await?;

// Export with schema
vault.export()
    .include_schema(true)
    .to_file("backup.json")
    .await?;

// Export with metadata
vault.export()
    .include_metadata(true)  // created_at, created_by
    .to_file("backup.json")
    .await?;

// Stream export (large vaults)
let stream = vault.export().stream();
while let Some(batch) = stream.next().await {
    process_batch(batch?);
}
```

### Export Formats

```rust
// JSON (default)
vault.export()
    .format(ExportFormat::Json)
    .to_file("backup.json")
    .await?;

// JSON Lines (one per line, for streaming)
vault.export()
    .format(ExportFormat::JsonLines)
    .to_file("backup.jsonl")
    .await?;

// CSV
vault.export()
    .format(ExportFormat::Csv)
    .to_file("backup.csv")
    .await?;

// With compression
vault.export()
    .compression(Compression::Gzip)
    .to_file("backup.json.gz")
    .await?;
```

### Import Relationships

```rust
// Basic import
vault.import()
    .from_file("backup.json")
    .await?;

// Merge mode (skip conflicts, default)
vault.import()
    .from_file("backup.json")
    .mode(ImportMode::Merge)
    .await?;

// Upsert mode (update existing)
vault.import()
    .from_file("backup.json")
    .mode(ImportMode::Upsert)
    .await?;

// Replace mode (full replacement, dangerous!)
vault.import()
    .from_file("backup.json")
    .mode(ImportMode::Replace)
    .confirm_replace(true)  // Required safety flag
    .await?;
```

### Conflict Handling

```rust
let result = vault.import()
    .from_file("backup.json")
    .on_conflict(ConflictResolution::Skip)
    .report_conflicts(true)
    .await?;

println!("Created: {}, Skipped: {}", result.created, result.skipped);

for conflict in &result.conflicts {
    println!("Conflict: {} ({})", conflict.relationship, conflict.reason);
}
```

### Async Import (Large Files)

```rust
// Start background import
let job = vault.import()
    .from_file("large-backup.json")
    .start_async()
    .await?;

// Monitor progress
loop {
    let status = vault.import_status(&job.id).await?;

    match status.state {
        JobState::Running => {
            println!("Progress: {:.1}%", status.progress_percent());
        }
        JobState::Completed => {
            println!("Done: {} imported", status.processed);
            break;
        }
        JobState::Failed => {
            eprintln!("Failed: {}", status.error.unwrap());
            break;
        }
        _ => {}
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}

// Cancel if needed
vault.cancel_import(&job.id).await?;
```

## Type-Safe Schemas

Generate Rust types from your schema for compile-time safety.

### Enable Feature

```toml
[dependencies]
inferadb = { version = "0.1", features = ["derive"] }
```

### Schema Macro

```rust
use inferadb::derive::schema;

// Generate types from schema
schema!("schema.ipl");

// Now you have typed entities
let doc = Document::new("readme");
let user = User::new("alice");

// Type-safe relations
doc.viewer().add(&user);     // Compiles
doc.owner().add(&user);      // Compiles
// doc.viewer().add(&doc);   // Error: expected User, found Document

// Type-safe checks
let allowed = doc.can_view(&user).check(&vault).await?;
```

### Derive Macros

```rust
use inferadb::derive::{Entity, Relation};

#[derive(Entity)]
#[entity(type = "document")]
pub struct Document {
    id: String,
}

#[derive(Entity)]
#[entity(type = "user")]
pub struct User {
    id: String,
}

// Generated methods:
impl Document {
    pub fn viewer(&self) -> RelationBuilder<User>;
    pub fn can_view(&self, subject: &User) -> CheckBuilder;
}
```

### Type-Safe Relationships

```rust
// Stringly-typed (still works)
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Type-safe (preferred)
vault.relationships()
    .write(doc.viewer().is(&user))
    .await?;
```

## Strong ID Types

Prevent mixing up identifiers with strongly-typed wrappers.

### EntityRef and SubjectRef

```rust
use inferadb::{EntityRef, SubjectRef};

// Parse from strings
let resource = EntityRef::parse("document:readme")?;
let subject = SubjectRef::parse("user:alice")?;

// Or construct directly
let resource = EntityRef::new("document", "readme");
let subject = SubjectRef::new("user", "alice");

// Usersets (group members)
let members = SubjectRef::userset(
    EntityRef::new("group", "admins"),
    "member"
);
// Represents: group:admins#member

// Use in checks
vault.check(&subject, "view", &resource).await?;
```

### Benefits

```rust
// Without strong types - easy to swap parameters
check_permission("doc:readme", "user:alice", "view");  // Oops! Wrong order

// With strong types - compiler catches mistakes
fn check_permission(
    subject: SubjectRef<'_>,
    permission: &str,
    resource: EntityRef<'_>
) { /* ... */ }

// check_permission(resource, "view", subject);  // Compile error!
```

## Bulk Delete

Delete relationships matching a query.

```rust
// Delete all access for a departing user
vault.relationships()
    .delete_where()
    .subject("user:departed")
    .execute()
    .await?;

// Delete by resource type
vault.relationships()
    .delete_where()
    .resource_type("temp_document")
    .execute()
    .await?;

// Dry run first
let preview = vault.relationships()
    .delete_where()
    .subject("user:departed")
    .dry_run()
    .await?;

println!("Would delete {} relationships", preview.count);
for rel in &preview.sample {
    println!("  {}", rel);
}

// Require confirmation for large deletes
vault.relationships()
    .delete_where()
    .resource_type("document")
    .confirm_above(100)  // Error if > 100 would be deleted
    .execute()
    .await?;
```

## Preconditions

Conditional writes with optimistic concurrency control.

```rust
// Only write if relationship doesn't exist
vault.relationships()
    .write(Relationship::new("doc:1", "owner", "user:alice"))
    .unless_exists()
    .await?;

// Write with precondition
vault.relationships()
    .write(Relationship::new("doc:1", "owner", "user:bob"))
    .precondition(Precondition::exists("doc:1", "owner", "user:alice"))
    .await?;

// Optimistic locking with consistency token
let result = vault.relationships().list().resource("doc:1").collect().await?;
let token = result.consistency_token;

vault.relationships()
    .write(Relationship::new("doc:1", "editor", "user:charlie"))
    .precondition(Precondition::token_matches(token))
    .await?;
```

## Relationship History

Query change history for auditing:

```rust
// Get history for a specific relationship
let history = vault
    .relationships()
    .history("user:alice", "viewer", "document:readme")
    .await?;

for event in history {
    println!("{}: {} by {}",
        event.timestamp,
        event.action,  // Created or Deleted
        event.actor.unwrap_or("system".into())
    );
}
```

### History Query Builder

```rust
let history = vault
    .relationships()
    .history_query()
    .resource("document:readme")
    .from(Utc::now() - Duration::days(30))
    .to(Utc::now())
    .include_actor(true)
    .limit(100)
    .stream();

while let Some(event) = history.next().await {
    process_event(event?);
}
```

## Relationship Validation

Validate relationships against the schema before writing:

```rust
// Validate a single relationship
let result = vault
    .relationships()
    .validate(Relationship::new("user:alice", "viewer", "document:readme"))
    .await?;

if !result.valid {
    println!("Invalid: {}", result.error.unwrap());
    for suggestion in result.suggestions {
        println!("  Did you mean: {}", suggestion);
    }
}
```

### Batch Validation

```rust
let results = vault
    .relationships()
    .validate_batch([
        Relationship::new("user:alice", "viewer", "document:readme"),
        Relationship::new("user:bob", "invalid_relation", "document:readme"),
    ])
    .collect()
    .await?;

for (rel, result) in relationships.iter().zip(results) {
    if !result.valid {
        println!("{}: {}", rel.subject, result.error.unwrap());
    }
}
```

### Dry-Run Writes

```rust
// Preview what a write would do without committing
let preview = vault.relationships()
    .write(Relationship::new("user:alice", "viewer", "document:readme"))
    .dry_run(true)
    .await?;

println!("Would create: {}", preview.would_create);
println!("Validation: {:?}", preview.validation);
```

## Best Practices

1. **Simulate before deploying** - Test schema changes with simulation
2. **Use explain for debugging** - Understand complex permission denials
3. **Export regularly** - Maintain backups of critical vaults
4. **Prefer type-safe schemas** - Catch errors at compile time
5. **Use preconditions** - Prevent race conditions in concurrent writes
6. **Dry-run bulk deletes** - Preview impact before executing
7. **Validate before batch writes** - Catch schema violations early
8. **Query history for auditing** - Track who changed what and when
