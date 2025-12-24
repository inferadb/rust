<div align="center">
    <p><a href="https://inferadb.com"><img src=".github/inferadb.png" width="100" /></a></p>
    <h1>InferaDB Rust SDK</h1>
    <p>
      <a href="https://crates.io/crates/inferadb"><img src="https://img.shields.io/crates/v/inferadb.svg" /></a>
      <a href="https://docs.rs/inferadb"><img src="https://docs.rs/inferadb/badge.svg" /></a>
      <a href="./LICENSE"><img src="https://img.shields.io/crates/l/inferadb.svg"></a>
    </p>
    <p>Ergonomic, type-safe access to InferaDB's authorization and management APIs</p>
</div>

<br />

[InferaDB](https://inferadb.com/) is a distributed, [Google Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/)‑inspired authorization engine that replaces ad‑hoc database lookups and scattered logic with a unified, millisecond‑latency source of truth. With this SDK, you define permissions as policy‑as‑code and wire up a type‑safe client in just a few lines.

- **Rust‑Native & Async:** Fully integrated with the ecosystem (Tokio, Tracing) so you don't have to adapt generic policy engines to your runtime.
- **Compile‑Time Safety:** Catch permission model mistakes in your build pipeline and tests, preventing surprises in production.
- **Standards‑Based & Audit‑Ready:** Built on [AuthZen](https://openid.net/wg/authzen/) with automatic multi‑tenant isolation and cryptographically verifiable audit trails out of the box.

## Quick Start

Add the [inferadb](https://crates.io/crates/inferadb) crate to your `Cargo.toml`:

```toml
[dependencies]
inferadb = "0.1"
```

```rust
use inferadb::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(ClientCredentialsConfig {
            client_id: "my_service".into(),
            private_key: Ed25519PrivateKey::from_pem_file("private-key.pem")?,
        })
        .build()
        .await?;

    let vault = client.organization("org_...").vault("vlt_...");

    let allowed = vault.check("user:alice", "view", "document:readme").await?;
    println!("Allowed: {allowed}");

    Ok(())
}
```

## Philosophy

We designed this SDK with predictability and safety in mind:

**Denial is not an error.** `check()` returns `Ok(false)` for denied access—never throws. Network failures are errors; permission denials are business logic.

**Fail-closed by default.** When something goes wrong, access is denied. Fail-open requires explicit opt-in.

**Results preserve order.** Batch operations return results matching input order—no ID correlation needed.

**Writes are acknowledged.** Write operations return only after server confirmation. No fire-and-forget surprises.

**Errors include request IDs.** Every server error exposes `request_id()` for debugging and support.

## Core API

### Authorization Checks

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Simple check
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// With ABAC context
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
// Write a single relationship
vault
    .relationships()
    .write(Relationship::new(
        "document:readme",
        "viewer",
        "user:alice",
    ))
    .await?;

// Batch write
vault
    .relationships()
    .write_batch([
        Relationship::new("folder:docs", "viewer", "group:engineering#member"),
        Relationship::new("document:readme", "parent", "folder:docs"),
    ])
    .await?;
```

### Lookups

```rust
// Resources a user can access
let docs = vault.resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;

// Users who can access a resource
let users = vault.subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .collect()
    .await?;
```

## Local Development

```rust
let client = Client::builder()
    .url("http://localhost:8080")
    .insecure()  // Disables TLS verification for local development
    .credentials(BearerCredentialsConfig {
        token: "dev-token".into(),
    })
    .build()
    .await?;
```

```yaml
# docker-compose.yml
services:
  inferadb:
    image: ghcr.io/inferadb/inferadb:latest
    ports:
      - "8080:8080"
    environment:
      INFERADB__STORAGE__BACKEND: memory
      INFERADB__AUTH__SKIP_VERIFICATION: true
```

## Testing

Use `MockClient` for unit tests:

```rust
use inferadb::testing::{AuthorizationClient, MockClient};

#[tokio::test]
async fn test_authorization() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "document:readme", true)
        .check("user:bob", "delete", "document:readme", false)
        .build();

    assert!(mock
        .check("user:alice", "view", "document:readme")
        .await
        .unwrap());
}
```

See the [Testing Guide](docs/guides/testing.md) for `InMemoryClient` (full policy evaluation) and integration testing patterns.

## Documentation

- [API Reference](https://docs.rs/inferadb) - Full rustdoc documentation

### Guides

| Topic                                                       | Description                                       |
| ----------------------------------------------------------- | ------------------------------------------------- |
| [Installation](docs/guides/installation.md)                 | Feature flags, optimized builds, TLS, MSRV        |
| [Authentication](docs/guides/authentication.md)             | Client credentials, bearer tokens, key management |
| [Integration Patterns](docs/guides/integration-patterns.md) | Axum, Actix-web, GraphQL, gRPC middleware         |
| [Error Handling](docs/guides/errors.md)                     | Error types, retries, graceful degradation        |
| [Testing](docs/guides/testing.md)                           | MockClient, InMemoryClient, TestVault             |
| [Schema Design](docs/guides/schema-design.md)               | ReBAC patterns, role hierarchy, anti-patterns     |
| [Production Checklist](docs/guides/production-checklist.md) | Deployment readiness                              |
| [Troubleshooting](docs/troubleshooting.md)                  | Common issues and solutions                       |

See [docs/README.md](docs/README.md) for the complete documentation index.

## Examples

```bash
cargo run -p inferadb-examples --bin basic_check
cargo run -p inferadb-examples --bin batch_operations
cargo run -p inferadb-examples --bin axum_middleware
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
