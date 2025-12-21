# Migration Guide

This guide helps you migrate between SDK versions and from other authorization systems.

## Version Migrations

### From 0.1 to 0.2 (Example)

#### Client Construction

```rust
// Before (0.1)
let client = Client::new("https://api.inferadb.com", creds).await?;

// After (0.2)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()
    .await?;
```

#### Error Handling

```rust
// Before (0.1)
match result {
    Err(Error::Unauthorized(msg)) => { ... }
}

// After (0.2) - Use kind() method
match result {
    Err(e) if e.kind() == ErrorKind::Unauthorized => { ... }
}
```

## Migrating from Other Systems

### From SpiceDB

#### Authorization Checks

```rust
// SpiceDB - Verbose request construction
let request = CheckPermissionRequest {
    resource: Some(ObjectReference {
        object_type: "document".to_string(),
        object_id: "readme".to_string(),
    }),
    permission: "view".to_string(),
    subject: Some(SubjectReference {
        object: Some(ObjectReference {
            object_type: "user".to_string(),
            object_id: "alice".to_string(),
        }),
        optional_relation: String::new(),
    }),
    ..Default::default()
};
let response = client.check_permission(request).await?;
let allowed = response.permissionship == Permissionship::HasPermission;

// InferaDB - Simple and ergonomic
let allowed = client.check("user:alice", "view", "document:readme").await?;
```

#### Writing Relationships

```rust
// SpiceDB
let request = WriteRelationshipsRequest {
    updates: vec![RelationshipUpdate {
        operation: Operation::Touch,
        relationship: Some(Relationship {
            resource: Some(ObjectReference {
                object_type: "document".to_string(),
                object_id: "readme".to_string(),
            }),
            relation: "viewer".to_string(),
            subject: Some(SubjectReference {
                object: Some(ObjectReference {
                    object_type: "user".to_string(),
                    object_id: "alice".to_string(),
                }),
                optional_relation: String::new(),
            }),
        }),
    }],
    ..Default::default()
};
client.write_relationships(request).await?;

// InferaDB
client
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

### From OpenFGA

#### List Objects with Pagination

```rust
// OpenFGA - Manual pagination
let mut all_objects = vec![];
let mut continuation_token = None;
loop {
    let response = client.list_objects(ListObjectsRequest {
        user: "user:alice".into(),
        relation: "viewer".into(),
        object_type: "document".into(),
        continuation_token: continuation_token.clone(),
        ..Default::default()
    }).await?;
    all_objects.extend(response.objects);
    continuation_token = response.continuation_token;
    if continuation_token.is_none() { break; }
}

// InferaDB - Streaming handles pagination automatically
let objects: Vec<String> = client
    .list_resources("user:alice", "viewer")
    .resource_type("document")
    .collect()
    .await?;
```

#### Batch Checks

```rust
// OpenFGA - No native batch support, must loop
let mut results = vec![];
for check in checks {
    let result = client.check(check).await?;
    results.push(result);
}

// InferaDB - Native batch support
let results = client
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
        ("user:bob", "view", "doc:1"),
    ])
    .collect()
    .await?;
```

### From Oso

#### Policy Evaluation

```rust
// Oso - Embedded engine
let oso = Oso::new();
oso.load_str(r#"
    allow(user, "view", document) if
        has_role(user, "viewer", document);
"#)?;
oso.register_class(User::get_polar_class())?;
oso.register_class(Document::get_polar_class())?;
let allowed = oso.is_allowed(user, "view", document)?;

// InferaDB - Distributed service with similar semantics
let allowed = client
    .check("user:alice", "view", "document:readme")
    .await?;
```

## Schema Migration

### SpiceDB Schema to IPL

```zed
// SpiceDB schema
definition document {
    relation viewer: user | group#member
    relation owner: user

    permission view = viewer + owner
    permission edit = owner
    permission delete = owner
}
```

```ipl
// InferaDB IPL
entity Document {
    relations {
        viewer: User | Group#member
        owner: User
    }

    permissions {
        view: viewer | owner
        edit: owner
        delete: owner
    }
}
```

### OpenFGA Model to IPL

```json
// OpenFGA model
{
  "type_definitions": [
    {
      "type": "document",
      "relations": {
        "viewer": { "this": {} },
        "owner": { "this": {} }
      },
      "metadata": {
        "relations": {
          "viewer": { "directly_related_user_types": [{ "type": "user" }] },
          "owner": { "directly_related_user_types": [{ "type": "user" }] }
        }
      }
    }
  ]
}
```

```ipl
// InferaDB IPL
entity Document {
    relations {
        viewer: User
        owner: User
    }

    permissions {
        view: viewer | owner
        edit: owner
        delete: owner
    }
}
```

## Common Migration Issues

### 1. Subject Format Differences

```rust
// SpiceDB: Separate type and id
ObjectReference { object_type: "user", object_id: "alice" }

// OpenFGA: Combined string
"user:alice"

// InferaDB: Combined string (same as OpenFGA)
"user:alice"
```

### 2. Relation vs Permission

- SpiceDB: `permission` for computed, `relation` for direct
- OpenFGA: All are `relations` with computed unions
- InferaDB: Explicit `relations` and `permissions` blocks

### 3. Contextual Attributes

```rust
// OpenFGA: Condition context
client.check(CheckRequest {
    context: Some(Struct { fields: context_map }),
    ..
}).await?;

// InferaDB: ABAC context
client
    .check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.1")
        .insert("time", Utc::now()))
    .await?;
```

## Getting Help

If you encounter issues during migration:

1. Check the [Troubleshooting Guide](docs/troubleshooting.md)
2. Open an issue at https://github.com/inferadb/rust-sdk/issues
3. Join our Discord for community support
