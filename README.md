<div align="center">
    <p><a href="https://inferadb.com"><img src=".github/inferadb.png" width="100" /></a></p>
    <h1>InferaDB Rust SDK</h1>
    <p>
        <a href="https://crates.io/crates/inferadb"><img src="https://img.shields.io/crates/v/inferadb.svg" /></a>
        <a href="https://docs.rs/inferadb"><img src="https://docs.rs/inferadb/badge.svg" /></a>
        <a href="https://codecov.io/github/inferadb/rust"><img src="https://codecov.io/github/inferadb/rust/branch/main/graph/badge.svg?token=HBlxbXsvny" /></a>
        <a href="./LICENSE"><img src="https://img.shields.io/crates/l/inferadb.svg"></a>
    </p>
    <p>Ergonomic, type-safe access to InferaDB's authorization and management APIs</p>
</div>

> [!IMPORTANT]
> Under active development. Not production-ready.

[InferaDB](https://inferadb.com/) is a distributed, [Google Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/)‑inspired authorization engine that replaces ad‑hoc database lookups and scattered logic with a unified, millisecond‑latency source of truth. With this SDK, you define permissions as policy‑as‑code and wire up a type‑safe client in just a few lines.

- **Rust‑Native & Async:** Fully integrated with the ecosystem ([Tokio](https://crates.io/crates/tokio), [Tracing](https://crates.io/crates/tracing)) so you don't have to adapt generic policy engines to your runtime.
- **Compile‑Time Safety:** Catch permission model mistakes in your build pipeline and tests, preventing surprises in production.
- **Standards‑Based & Audit‑Ready:** Built on [AuthZen](https://openid.net/wg/authzen/) with automatic multi‑tenant isolation and cryptographically verifiable audit trails out of the box.

## Quick Start

1. Sign up for an account at [InferaDB](https://inferadb.com/) and create a new organization and vault.

2. Run the following Cargo command in your project directory:

   ```shell
   cargo add inferadb
   ```

3. In your project, create and configure a client instance:

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

## In Action

### "Can this user do this?"

The most common question in any app. One line:

```rust
if vault.check("user:alice", "edit", "document:readme").await? {
    // allow the edit
}
```

### "Who can access this?"

Building a share dialog or audit view? List everyone with access:

```rust
let viewers = vault.subjects()
    .with_permission("view")
    .on_resource("document:readme")
    .collect()
    .await?;
// ["user:alice", "user:bob", "team:engineering"]
```

### "What can this user see?"

Filtering a dashboard or search results by what the user can actually access:

```rust
let docs = vault.resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .of_type("document")
    .collect()
    .await?;
```

### "Grant access to a team"

When Alice shares a folder with her team, everyone on that team gets access:

```rust
vault.relationships()
    .write(Relationship::new("folder:designs", "viewer", "team:engineering"))
    .await?;
```

### "Inherit permissions from a parent"

Documents inside a folder should inherit the folder's permissions:

```rust
vault.relationships()
    .write(Relationship::new("document:spec", "parent", "folder:designs"))
    .await?;

// Now anyone who can view the folder can view the document
```

### "Check multiple permissions at once"

Rendering a UI with edit, delete, and share buttons? Check them all in one round-trip:

```rust
let [can_edit, can_delete, can_share] = vault.batch_check(&[
    ("user:alice", "edit", "document:readme"),
    ("user:alice", "delete", "document:readme"),
    ("user:alice", "share", "document:readme"),
]).await?[..] else { unreachable!() };
```

## Usage

### Authorization API

```rust
let vault = client.organization("org_...").vault("vlt_...");
```

#### Permission Checks

```rust
let allowed = vault.check("user:alice", "view", "doc:1").await?;
```

#### Relationships

```rust
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

#### Lookups

```rust
let docs = vault.resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .collect()
    .await?;
```

See the [Authorization API Guide](docs/guides/authorization-api.md) for ABAC context, batch checks, explain, simulate, watch, and more.

### Management API

```rust
let org = client.organization("org_...");
```

#### Vaults

```rust
let vault = org.vaults()
    .create(CreateVaultRequest::new("production"))
    .await?;
```

#### Schemas

```rust
vault.schemas().push(r#"
type user {}

type document {
    relation viewer: user
    permission view = viewer
}
"#).await?;
```

#### Members & Teams

```rust
org.members()
    .invite(InviteMemberRequest::new("alice@example.com", OrgRole::Admin))
    .await?;

org.teams()
    .create(CreateTeamRequest::new("Engineering"))
    .await?;
```

#### Audit Logs

```rust
let events = org.audit().list().collect().await?;
```

See the [Management API Guide](docs/guides/management-api.md) for organizations, API clients, schema versioning, and more.

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
| [Authorization API](docs/guides/authorization-api.md)       | Permission checks, relationships, lookups, watch  |
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
