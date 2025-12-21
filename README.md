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
    // Create client from environment variables
    let client = Client::from_env().await?;

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

```bash
# Set environment variables and run
export INFERADB_URL=https://api.inferadb.com
export INFERADB_CLIENT_ID=my_service
export INFERADB_PRIVATE_KEY_PATH=./private_key.pem
export INFERADB_VAULT_ID=my_vault

cargo run
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

| Feature         | Description                            | Default |
| --------------- | -------------------------------------- | ------- |
| `grpc`          | gRPC transport (faster, streaming)     | Yes     |
| `rest`          | REST transport (broader compatibility) | Yes     |
| `rustls`        | Pure-Rust TLS                          | Yes     |
| `native-tls`    | System TLS (OpenSSL/Schannel)          | No      |
| `tracing`       | Tracing spans                          | No      |
| `metrics`       | Metrics emission                       | No      |
| `opentelemetry` | OTLP integration                       | No      |
| `blocking`      | Sync/blocking API                      | No      |
| `derive`        | Proc macros for type-safe schemas      | No      |
| `serde`         | Serialization support                  | No      |

### Prelude Options

```rust
// Standard prelude - most applications
use inferadb::prelude::*;

// Minimal prelude - libraries, minimal footprint
use inferadb::prelude::core::*;

// Extended prelude - includes testing utilities
use inferadb::prelude::extended::*;
```

| Prelude    | Use Case                 | Includes                            |
| ---------- | ------------------------ | ----------------------------------- |
| `core`     | Libraries, minimal deps  | Client, VaultClient, Error, traits  |
| Default    | Most applications        | Core + config types, credentials    |
| `extended` | Tests, feature-rich apps | Full + MockClient, derive macros    |

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
        .insert("ip_address", "10.0.0.50")
        .insert("mfa_verified", true))
    .await?;

// Batch checks - single round-trip
let results = vault
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
    ])
    .collect()
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

### Watch for Changes

```rust
use futures::StreamExt;

let vault = client.organization("org_...").vault("vlt_...");

let mut stream = vault
    .watch()
    .filter(WatchFilter::resource_type("document"))
    .run()
    .await?;

while let Some(change) = stream.next().await {
    let change = change?;
    println!("{:?}: {} -> {}",
        change.operation,
        change.relationship.subject,
        change.relationship.resource
    );
}
```

## Authentication

### Client Credentials (Recommended for Services)

```rust
use inferadb::auth::{ClientCredentials, Ed25519PrivateKey};

let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
    certificate_id: None,
};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .default_vault("my_vault")
    .build()
    .await?;
```

### Bearer Token

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .bearer_token("eyJ...")
    .default_vault("my_vault")
    .build()
    .await?;
```

### Environment Variables

| Variable                    | Description                      |
| --------------------------- | -------------------------------- |
| `INFERADB_URL`              | Service URL                      |
| `INFERADB_CLIENT_ID`        | Client ID                        |
| `INFERADB_PRIVATE_KEY_PATH` | Path to Ed25519 private key      |
| `INFERADB_PRIVATE_KEY`      | Ed25519 private key PEM contents |
| `INFERADB_VAULT_ID`         | Default vault ID                 |
| `INFERADB_CERTIFICATE_ID`   | Specific certificate KID         |

## Local Development

### Connect to Local Instance

```rust
let client = Client::builder()
    .url("http://localhost:8080")
    .insecure()  // Allow non-TLS for local dev
    .default_vault("dev-vault")
    .build()
    .await?;
```

### In-Memory Client for Unit Tests

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_authorization() {
    let client = InMemoryClient::new();

    client.write_batch([
        Relationship::new("document:readme", "owner", "user:alice"),
        Relationship::new("document:readme", "viewer", "user:bob"),
    ]).await.unwrap();

    assert!(client.check("user:alice", "delete", "document:readme").await.unwrap());
    assert!(!client.check("user:bob", "delete", "document:readme").await.unwrap());
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

| Guarantee | Description |
|-----------|-------------|
| **Denial is not an error** | `check()` returns `Ok(false)` for denied access; only `require()` converts denial to error |
| **Fail-closed by default** | Errors default to denying access; fail-open must be explicit |
| **Results preserve order** | Batch operations return results in the same order as inputs |
| **Writes are acknowledged** | Write operations return only after server confirmation |
| **Cache never changes semantics** | Cached results are identical to fresh results |
| **Errors include request IDs** | All server errors include a `request_id()` for debugging |

## Error Handling

```rust
use inferadb::{Error, ErrorKind};

match client.check("user:alice", "view", "doc:1").await {
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
- [Integration Patterns](docs/guides/integration-patterns.md) - Axum, Actix-web, GraphQL, gRPC
- [Testing Guide](docs/guides/testing.md) - MockClient, InMemoryClient, TestVault
- [Error Handling](docs/guides/errors.md) - Error types and retry strategies
- [Consistency & Watch](docs/guides/consistency.md) - Consistency tokens, real-time streams
- [Control API](docs/guides/control-api.md) - Organizations, schemas, members, audit
- [Advanced Features](docs/guides/advanced.md) - Simulation, explain, export/import
- [Caching](docs/guides/caching.md) - Cache configuration and invalidation
- [Performance Tuning](docs/guides/performance-tuning.md) - Optimization guide
- [Production Checklist](docs/guides/production-checklist.md) - Deployment readiness
- [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions

## Examples

See the [examples](examples/) directory for complete working examples:

```bash
cargo run --example basic_check
cargo run --example batch_check
cargo run --example watch_changes
cargo run --example middleware_axum
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
