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

Sign up for an account at [InferaDB](https://inferadb.com/) and create a new organization and vault. With these credentials, you can create a new client instance:

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

## Authorization API

```rust
let org = client.organization("org_...");
let vault = org.vault("vlt_...");
```

### Permission Checks

```rust
// Simple check - returns bool
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// With ABAC context
let allowed = vault.check("user:alice", "view", "doc:confidential")
    .with_context(Context::new()
        .with("ip_address", "10.0.0.50")
        .with("mfa_verified", true))
    .await?;

// Guard clause - returns Err(AccessDenied) if denied
vault.check("user:alice", "edit", "doc:1").require().await?;

// Batch checks - single round-trip
let results = vault.check_batch([
    ("user:alice", "view", "doc:1"),
    ("user:alice", "edit", "doc:1"),
]).await?;
```

### Relationships

#### List Relationships

```rust
let rels = vault.relationships()
    .list()
    .resource("document:readme")
    .collect()
    .await?;
```

#### Write a Relationship

```rust
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

#### Write Multiple Relationships

```rust
vault.relationships().write_batch([
    Relationship::new("folder:docs", "viewer", "group:engineering#member"),
    Relationship::new("document:readme", "parent", "folder:docs"),
]).await?;
```

#### Delete a Relationship

```rust
vault.relationships()
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

#### Delete Multiple Relationships

```rust
vault.relationships()
    .delete_where()
    .resource("document:readme")
    .execute()
    .await?;
```

### Lookups

```rust
// What can a user access?
let docs = vault.resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;

// Who can access a resource?
let users = vault.subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .collect()
    .await?;
```

### Explain & Simulate

```rust
// Debug why a permission was granted or denied
let explanation = vault.explain_permission()
    .subject("user:alice")
    .permission("edit")
    .resource("document:readme")
    .execute()
    .await?;
println!("{}", explanation.summary());

// Test what-if scenarios without persisting changes
let result = vault.simulate()
    .add_relationship(Relationship::new("doc:1", "editor", "user:bob"))
    .check("user:bob", "edit", "doc:1")
    .await?;
```

### Watch for Changes

```rust
// Real-time stream of relationship changes
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

## Management API

```rust
let org = client.organization("org_...");
let vault = org.vault("vlt_...");
```

### Organizations & Vaults

### Get Current Organization

```rust
let info = org.control().get().await?;
```

### Vaults

#### Create a Vault

```rust
let vault = org.vaults().create(CreateVaultRequest::new("production")).await?;
```

#### List Vaults

```rust
let vaults = org.vaults().list().collect().await?;
```

### Schemas

```rust
// Push a new schema version
let result = org.vault("vlt_...").schemas().push(r#"
    type user {}
    type document {
        relation viewer: user
        relation editor: user
        permission view = viewer + editor
        permission edit = editor
    }
"#).await?;

// Validate without persisting
let validation = org.vault("vlt_...").schemas().validate(schema_content).await?;

// Activate a version
org.vault("vlt_...").schemas().activate("v2").await?;

// Compare versions
let diff = org.vault("vlt_...").schemas().diff("v1", "v2").await?;
```

### Members & Teams

```rust
// Invite a member
org.members().invite(InviteMemberRequest::new("alice@example.com", OrgRole::Admin)).await?;

// Create a team
org.teams().create(CreateTeamRequest::new("Engineering")).await?;

// Add member to team
org.teams().add_member("team_...", "user_...", TeamRole::Member).await?;
```

### API Clients

```rust
// Create an API client for service-to-service auth
let api_client = org.clients().create(
    CreateApiClientRequest::new("payment-service")
).await?;

// Rotate credentials
org.clients().certificates("client_...").rotate(
    RotateCertificateRequest::new(public_key_pem)
).await?;
```

### Audit Logs

```rust
// Query audit events
let events = org.audit().list()
    .action(AuditAction::RelationshipCreated)
    .since(one_hour_ago)
    .collect()
    .await?;

// Export to file
org.audit().export()
    .format(ExportFormat::Json)
    .write_to_file("audit.json")
    .await?;
```

## Local Development

[Deploy a local instance of InferaDB](https://github.com/inferadb/deploy/), then configure your client to connect to it.

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
