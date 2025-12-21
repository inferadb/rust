# Competitive Analysis

Internal document comparing InferaDB Rust SDK against competing authorization solutions.

## Executive Summary

InferaDB's Rust SDK provides a superior developer experience through ergonomic APIs, comprehensive type safety, and native async support while maintaining competitive performance characteristics.

## Competitor Overview

| Solution     | Type             | Language Support             | Pricing Model       |
| ------------ | ---------------- | ---------------------------- | ------------------- |
| SpiceDB      | OSS + Enterprise | Go, Java, Python, Node, Ruby | Open source / SaaS  |
| OpenFGA      | OSS (CNCF)       | Go, Java, Python, Node, .NET | Open source only    |
| Oso          | Embedded + Cloud | Rust, Python, Node, Go, Java | Open source / Cloud |
| Auth0 FGA    | SaaS             | REST API                     | Usage-based         |
| Permit.io    | SaaS             | REST API, SDKs               | Freemium            |
| **InferaDB** | OSS + Enterprise | Rust, Go, Python (planned)   | Open source / SaaS  |

## API Ergonomics Comparison

### Authorization Check

**SpiceDB:**

```rust
// Verbose request construction
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
```

**OpenFGA:**

```rust
// Multiple builder calls
let allowed = client
    .check(ClientCheckRequest {
        user: "user:alice".to_string(),
        relation: "viewer".to_string(),
        object: "document:readme".to_string(),
        context: None,
    })
    .await?
    .allowed;
```

**InferaDB:**

```rust
// Get vault context (once at startup)
let vault = client.organization("org_...").vault("vlt_...");

// Simple, fluent API
let allowed = vault.check("user:alice", "view", "document:readme").await?;

// With context when needed
let allowed = vault
    .check("user:alice", "view", "document:readme")
    .with_context(Context::new().insert("ip", "10.0.0.1"))
    .await?;
```

**Advantage:** InferaDB requires 80% fewer lines for common operations.

### Batch Operations

**SpiceDB:** No native batch API - must loop or use streaming.

**OpenFGA:**

```rust
// No native batch check
for check in checks {
    let result = client.check(check).await?;
    results.push(result);
}
```

**InferaDB:**

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Native batch support
let results = vault
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
        ("user:bob", "view", "doc:1"),
    ])
    .collect()
    .await?;
```

**Advantage:** InferaDB provides native batching with single round-trip.

### Streaming Results

**SpiceDB:** gRPC streaming but manual pagination handling.

**OpenFGA:**

```rust
// Manual continuation token handling
let mut all_objects = vec![];
let mut continuation_token = None;
loop {
    let response = client.list_objects(ListObjectsRequest {
        user: "user:alice".into(),
        relation: "viewer".into(),
        object_type: "document".into(),
        continuation_token,
        ..Default::default()
    }).await?;
    all_objects.extend(response.objects);
    continuation_token = response.continuation_token;
    if continuation_token.is_none() { break; }
}
```

**InferaDB:**

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Automatic pagination via streams
let objects: Vec<String> = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("viewer")
    .resource_type("document")
    .collect()
    .await?;

// Or stream for memory efficiency
let mut stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("viewer")
    .stream();
while let Some(obj) = stream.next().await {
    process(obj?);
}
```

**Advantage:** InferaDB handles pagination automatically.

## Schema Language Comparison

### SpiceDB (Zed)

```zed
definition document {
    relation viewer: user | group#member
    relation owner: user

    permission view = viewer + owner
    permission edit = owner
    permission delete = owner
}
```

### OpenFGA (JSON DSL)

```json
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

### InferaDB (IPL)

```ipl
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

**Analysis:**

- SpiceDB Zed: Concise but custom syntax
- OpenFGA JSON: Verbose, hard to read
- InferaDB IPL: Rust-like familiarity, clear separation of relations/permissions

## Policy Language Comparison (vs Cedar/Rego)

### Cedar (AWS Verified Permissions)

```cedar
permit(
    principal == User::"alice",
    action == Action::"view",
    resource == Document::"readme"
);

permit(
    principal in Group::"engineering",
    action in [Action::"view", Action::"edit"],
    resource
) when {
    resource.classification != "confidential"
};
```

**Characteristics:**

- Explicit permit/forbid rules
- Built-in ABAC with `when` clauses
- Static analysis for policy conflicts
- No native ReBAC (must encode as attributes)

### Rego (Open Policy Agent)

```rego
package authz

default allow := false

allow {
    input.user == "alice"
    input.action == "view"
}

allow {
    input.user == data.groups["engineering"].members[_]
    input.action in ["view", "edit"]
    data.resources[input.resource].classification != "confidential"
}
```

**Characteristics:**

- General-purpose policy language
- Powerful but complex
- Requires data loading
- No native graph traversal

### InferaDB IPL + ABAC

```ipl
entity Document {
    attributes {
        classification: String
    }

    relations {
        viewer: User | Group#member
        owner: User
    }

    permissions {
        view: (viewer | owner) & !confidential_without_clearance
    }

    rules {
        confidential_without_clearance:
            @classification == "confidential" & !@subject.has_clearance
    }
}
```

**Characteristics:**

- Native ReBAC with graph traversal
- ABAC integrated via rules
- Type-safe schema
- Optimized for authorization queries

### Comparison Matrix

| Feature         | Cedar  | Rego    | InferaDB IPL |
| --------------- | ------ | ------- | ------------ |
| ReBAC           | Manual | Manual  | Native       |
| ABAC            | Native | Native  | Integrated   |
| Graph traversal | No     | Manual  | Native       |
| Type safety     | Strong | Weak    | Strong       |
| Static analysis | Yes    | Limited | Yes          |
| Learning curve  | Medium | High    | Low          |
| Performance     | Fast   | Medium  | Fast         |

## Error Handling Comparison

### SpiceDB

```rust
// Manual error inspection
match client.check_permission(request).await {
    Ok(response) => { /* ... */ }
    Err(status) => {
        if status.code() == Code::Unauthenticated {
            // Handle auth error
        }
    }
}
```

### OpenFGA

```rust
// Generic error types
match client.check(request).await {
    Ok(response) => { /* ... */ }
    Err(e) => {
        // Limited error categorization
        eprintln!("Error: {}", e);
    }
}
```

### InferaDB

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Rich error types with context
match vault.check("user:alice", "view", "doc:1").await {
    Ok(allowed) => println!("Allowed: {}", allowed),
    Err(e) => {
        match e.kind() {
            ErrorKind::Unauthorized => { /* Auth failed */ }
            ErrorKind::Forbidden => { /* Insufficient permissions */ }
            ErrorKind::NotFound => { /* Resource not found */ }
            ErrorKind::RateLimited => {
                let retry_after = e.retry_after();
                /* Back off */
            }
            ErrorKind::SchemaViolation => {
                let details = e.schema_error();
                /* Invalid relation/permission */
            }
            _ => { /* Other error */ }
        }
        // Always available
        if let Some(request_id) = e.request_id() {
            eprintln!("Request ID for support: {}", request_id);
        }
    }
}
```

**Advantage:** InferaDB provides semantic error types with actionable context.

## Performance Characteristics

### Latency (Single Check, p50)

| Solution | REST | gRPC |
| -------- | ---- | ---- |
| SpiceDB  | 15ms | 8ms  |
| OpenFGA  | 18ms | N/A  |
| InferaDB | 12ms | 6ms  |

### Throughput (Batch of 100)

| Solution | Checks/sec |
| -------- | ---------- |
| SpiceDB  | 2,500      |
| OpenFGA  | 1,800      |
| InferaDB | 4,000      |

### Memory Footprint

| Solution        | Base | Per Connection |
| --------------- | ---- | -------------- |
| SpiceDB client  | 8MB  | 100KB          |
| OpenFGA client  | 5MB  | 80KB           |
| InferaDB client | 3MB  | 50KB           |

_Note: Benchmarks from internal testing. Results may vary by deployment._

## Feature Comparison Matrix

| Feature          | SpiceDB      | OpenFGA       | Oso         | InferaDB   |
| ---------------- | ------------ | ------------- | ----------- | ---------- |
| ReBAC            | ✅           | ✅            | ✅          | ✅         |
| ABAC             | ⚠️ Caveats   | ✅ Conditions | ✅ Native   | ✅ Native  |
| Async Rust       | ✅           | ❌            | ✅          | ✅         |
| Batch API        | ❌           | ❌            | ❌          | ✅         |
| Streaming        | ✅ gRPC      | ⚠️ Polling    | ❌          | ✅ Native  |
| Watch/Subscribe  | ✅           | ❌            | ❌          | ✅         |
| Local cache      | ❌           | ❌            | ✅ Embedded | ✅         |
| Type-safe schema | ⚠️ Generated | ❌            | ✅          | ✅         |
| Mock client      | ❌           | ❌            | ✅          | ✅         |
| OpenTelemetry    | ⚠️ Manual    | ❌            | ❌          | ✅ Native  |
| Multi-vault      | ✅           | ✅            | N/A         | ✅         |
| Schema migration | ⚠️ Manual    | ⚠️ Manual     | N/A         | ✅ Tooling |

Legend: ✅ Full support, ⚠️ Partial/workaround, ❌ Not supported

## SDK Quality Indicators

| Indicator                 | SpiceDB   | OpenFGA | InferaDB   |
| ------------------------- | --------- | ------- | ---------- |
| API Guidelines compliance | Medium    | Low     | High       |
| Documentation             | Good      | Medium  | Excellent  |
| Test coverage             | High      | Medium  | High       |
| Example coverage          | Good      | Medium  | Excellent  |
| Error messages            | Technical | Generic | Actionable |
| Type safety               | Medium    | Low     | High       |
| IDE support               | Medium    | Low     | High (LSP) |

## Key Differentiators

### InferaDB Strengths

1. **Developer Experience**
   - Minimal boilerplate for common operations
   - Fluent builder APIs
   - Excellent error messages with request IDs

2. **Performance**
   - Native batch operations
   - Built-in caching
   - Optimized connection pooling

3. **Type Safety**
   - Compile-time schema validation (with derive feature)
   - Rich error types
   - IDE autocomplete support

4. **Observability**
   - First-class OpenTelemetry integration
   - Built-in metrics
   - Request tracing

5. **Testing**
   - MockClient for unit tests
   - InMemoryClient for integration tests
   - TestVault for isolated testing

### Areas for Improvement

1. **Ecosystem maturity** - SpiceDB/OpenFGA have larger communities
2. **Language coverage** - More SDK languages needed (Go, Python, Node)
3. **Cloud deployment** - Less proven at massive scale
4. **Third-party integrations** - Fewer pre-built connectors

## Migration Recommendations

### From SpiceDB

- Similar conceptual model
- Schema migration mostly 1:1
- API calls simplify significantly
- Watch streams are compatible

### From OpenFGA

- Model translates directly
- Batch operations are a significant improvement
- Remove manual pagination code
- Add streaming for large result sets

### From Oso

- Architectural shift (embedded → distributed)
- Policy requires translation to IPL
- Significant performance improvement at scale
- Lose embedded engine benefits

## Conclusion

InferaDB's Rust SDK provides the best developer experience in the authorization SDK space through:

- **80% less code** for common operations
- **Native batch support** for efficient bulk operations
- **Automatic pagination** via streaming APIs
- **Rich error types** with actionable context
- **First-class testing support** with mocks and in-memory clients
- **Superior observability** with native OpenTelemetry

Primary competitive disadvantage is ecosystem maturity, which will improve with adoption.
