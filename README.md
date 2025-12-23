# InferaDB Rust SDK

Provides ergonomic, type-safe access to InferaDB's authorization and management APIs.

[![Crates.io](https://img.shields.io/crates/v/inferadb.svg)](https://crates.io/crates/inferadb)
[![Documentation](https://docs.rs/inferadb/badge.svg)](https://docs.rs/inferadb)
[![License](https://img.shields.io/crates/l/inferadb.svg)](LICENSE)

## Quick Start

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
inferadb = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Minimum Viable Example

```rust
use inferadb::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create client with explicit configuration
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(ClientCredentialsConfig {
            client_id: "my_service".into(),
            private_key: Ed25519PrivateKey::from_pem_file("path/to/private-key.pem")?,
            certificate_id: None,
        })
        .build()
        .await?;

    // Get vault context (organization-first hierarchy)
    let vault = client
        .organization("org_...")
        .vault("vlt_...");

    // Check if user has permission
    let allowed = vault
        .check("user:alice", "view", "document:readme")
        .await?;

    println!("Allowed: {}", allowed);
    Ok(())
}
```

## Installation

### Default (Full Features)

```toml
[dependencies]
inferadb = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Minimal Build (REST Only)

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }
```

### Feature Flags

| Feature      | Description                            | Default |
| ------------ | -------------------------------------- | ------- |
| `grpc`       | gRPC transport (faster, streaming)     | Yes     |
| `rest`       | REST transport (broader compatibility) | Yes     |
| `rustls`     | Pure-Rust TLS                          | Yes     |
| `native-tls` | System TLS (OpenSSL/Schannel)          | No      |
| `tracing`    | Tracing spans                          | No      |
| `blocking`   | Sync/blocking API                      | No      |
| `derive`     | Proc macros for type-safe schemas      | No      |
| `wasm`       | Browser/WASM support (REST only)       | No      |
| `insecure`   | Allow HTTP for local development       | No      |

### Prelude

The prelude provides convenient access to commonly used types:

```rust
use inferadb::prelude::*;
```

This exports:

- **Client types**: `Client`, `ClientBuilder`, `VaultClient`
- **Error types**: `Error`, `ErrorKind`, `AccessDenied`, `Result`
- **Auth types**: `ClientCredentialsConfig`, `BearerCredentialsConfig`, `Ed25519PrivateKey`
- **Data types**: `Relationship`, `Context`, `Decision`, `ConsistencyToken`
- **Config types**: `CacheConfig`, `RetryConfig`, `TlsConfig`
- **Testing**: `MockClient`, `InMemoryClient`, `AuthorizationClient`

## Usage

### Authorization Checks

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Simple check - returns bool
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// require() pattern - recommended for HTTP handlers
// Returns Err(AccessDenied) on denial, integrates with ?
vault.check("user:alice", "view", "doc:1")
    .require()
    .await?;

// Check with ABAC context
vault.check("user:alice", "view", "doc:confidential")
    .with_context(Context::new()
        .with("ip_address", "10.0.0.50")
        .with("mfa_verified", true))
    .await?;

// Batch checks - single round-trip
let results: Vec<bool> = vault
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
    ])
    .await?;
```

### Relationship Management

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Write a relationship
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Write multiple relationships
vault.relationships()
    .write_batch([
        Relationship::new("folder:docs", "viewer", "group:engineering#member"),
        Relationship::new("document:readme", "parent", "folder:docs"),
    ])
    .await?;

// Delete a relationship
vault.relationships()
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

### Lookup Operations

```rust
let vault = client.organization("org_...").vault("vlt_...");

// List resources a user can access
let resources = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;

// List users who can access a resource
let subjects = vault
    .subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .collect()
    .await?;
```

## Authentication

### Client Credentials (Recommended for Services)

```rust
use inferadb::prelude::*;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
        certificate_id: None, // Optional: specific key ID
    })
    .build()
    .await?;
```

### Bearer Token

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: "eyJ...".into(),
    })
    .build()
    .await?;
```

### Full Configuration Options

```rust
let client = Client::builder()
    // Connection
    .url("https://api.inferadb.com")
    .connect_timeout(Duration::from_secs(10))
    .request_timeout(Duration::from_secs(30))

    // Authentication
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: None,
    })

    // Retry behavior
    .retry(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10)))

    // Caching
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_secs(300))
        .schema_ttl(Duration::from_secs(3600)))

    // Build
    .build()
    .await?;
```

## Local Development

### Connect to Local Instance

```rust
let client = Client::builder()
    .url("http://localhost:8080")
    .insecure()  // Allow non-TLS for local dev (requires `insecure` feature)
    .credentials(BearerCredentialsConfig {
        token: "dev-token".into(),
    })
    .build()
    .await?;
```

### MockClient for Unit Tests

```rust
use inferadb::testing::MockClient;

#[tokio::test]
async fn test_authorization() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "document:readme", true)
        .check("user:bob", "delete", "document:readme", false)
        .build();

    // Use mock in tests
    assert!(mock.check("user:alice", "view", "document:readme").await.unwrap());
    assert!(!mock.check("user:bob", "delete", "document:readme").await.unwrap());
}
```

### Docker Compose

```yaml
services:
  inferadb:
    image: ghcr.io/inferadb/inferadb:latest
    ports:
      - "8080:8080"
    environment:
      INFERADB__STORAGE__BACKEND: memory
      INFERADB__AUTH__SKIP_VERIFICATION: true # Dev only!
```

## Design Guarantees

| Guarantee                         | Description                                                                                |
| --------------------------------- | ------------------------------------------------------------------------------------------ |
| **Denial is not an error**        | `check()` returns `Ok(false)` for denied access; only `require()` converts denial to error |
| **Fail-closed by default**        | Errors default to denying access; fail-open must be explicit                               |
| **Results preserve order**        | Batch operations return results in the same order as inputs                                |
| **Writes are acknowledged**       | Write operations return only after server confirmation                                     |
| **Cache never changes semantics** | Cached results are identical to fresh results                                              |
| **Errors include request IDs**    | All server errors include a `request_id()` for debugging                                   |

## Error Handling

```rust
use inferadb::{Error, ErrorKind};

match vault.check("user:alice", "view", "doc:1").await {
    Ok(allowed) => println!("Allowed: {}", allowed),
    Err(e) => {
        match e.kind() {
            ErrorKind::Unauthorized => println!("Auth failed"),
            ErrorKind::Forbidden => println!("Insufficient permissions"),
            ErrorKind::NotFound => println!("Resource not found"),
            ErrorKind::RateLimited => {
                let retry_after = e.retry_after();
                println!("Rate limited, retry after {:?}", retry_after);
            }
            _ => println!("Error: {}", e),
        }
    }
}
```

## Documentation

- [API Documentation](https://docs.rs/inferadb) - Full API reference
- [Documentation Index](docs/README.md) - Complete guide to all documentation

### Getting Started

- [Integration Patterns](docs/guides/integration-patterns.md) - Axum, Actix-web, GraphQL, gRPC
- [Testing Guide](docs/guides/testing.md) - MockClient, InMemoryClient, TestVault
- [Error Handling](docs/guides/errors.md) - Error types and retry strategies
- [Migration Guide](docs/guides/migration.md) - From SpiceDB, OpenFGA, Oso, custom RBAC

### Schema & Design

- [Schema Design](docs/guides/schema-design.md) - ReBAC patterns, role hierarchy, anti-patterns
- [Authorization Scenarios](docs/guides/authorization-scenarios.md) - Multi-tenant SaaS, document sharing, API keys

### Production

- [Performance Tuning](docs/guides/performance-tuning.md) - Optimization guide
- [Production Checklist](docs/guides/production-checklist.md) - Deployment readiness
- [Debugging](docs/guides/debugging.md) - Diagnosis, explain API, common issues
- [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions

## Examples

See the [examples](examples/) directory for complete working examples:

```bash
# Run from the SDK root directory
cargo run -p inferadb-examples --bin basic_check
cargo run -p inferadb-examples --bin batch_operations
cargo run -p inferadb-examples --bin axum_middleware
```

Examples are in a separate workspace member to keep the main SDK dependencies lean.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
