# InferaDB Rust SDK Development Overview

A best-in-class Rust SDK for InferaDB that provides ergonomic, type-safe access to both the Engine (authorization) and Control (management) APIs through a single unified client.

---

## Table of Contents

### Part 1: Getting Started

- [Quick Start](#quick-start) _(start here)_
- [Installation](#installation)
- [Your First Authorization Check](#your-first-authorization-check)
- [Local Development](#local-development)

### Part 2: Vision & Architecture

- [Design Philosophy](#design-philosophy)
- [Competitive Analysis](#competitive-analysis)
- [Architecture Overview](#architecture-overview)
- [Crate Structure](#crate-structure)

### Part 3: Client Configuration

- [Client Builder](#client-builder)
- [Authentication](#authentication)
- [Connection Management](#connection-management)
- [Configuration Options](#configuration-options)
- [Middleware & Interceptors](#middleware--interceptors)

### Part 4: Engine API (Authorization)

- [Authorization Checks](#authorization-checks)
- [Batch Evaluations](#batch-evaluations)
- [Relationship Management](#relationship-management)
- [Lookup Operations](#lookup-operations)
- [Streaming & Watch](#streaming--watch)
- [Simulation](#simulation)
- [Caching](#caching)

### Part 5: Control API (Management)

- [Organization Management](#organization-management)
- [Vault Management](#vault-management)
- [Client & Certificate Management](#client--certificate-management)
- [Team Management](#team-management)
- [Schema Management](#schema-management)
- [Audit Logs](#audit-logs)

### Part 6: Developer Experience

- [Error Handling](#error-handling)
- [Retry & Resilience](#retry--resilience)
- [Graceful Degradation](#graceful-degradation)
- [Observability](#observability)
- [Testing Support](#testing-support)
- [Sync API](#sync-api)

### Part 7: Common Patterns & Recipes

- [Multi-Tenant SaaS](#multi-tenant-saas)
- [API Gateway Integration](#api-gateway-integration)
- [GraphQL & DataLoader](#graphql--dataloader)
- [Background Jobs](#background-jobs)
- [Audit Trail Enrichment](#audit-trail-enrichment)

### Part 8: Implementation

- [Type System](#type-system)
- [Protocol Support](#protocol-support)
- [Feature Flags](#feature-flags)
- [Performance](#performance)
- [Release Strategy](#release-strategy)

### Part 9: Reference

- [Troubleshooting](#troubleshooting)
- [Migration Guide](#migration-guide)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 1: GETTING STARTED
     ═══════════════════════════════════════════════════════════════════════════ -->

## Quick Start

Get up and running with InferaDB in under 5 minutes.

> **TL;DR**: Add `inferadb = "0.1"` to Cargo.toml, create a `Client`, call `.check()`. That's it.

### Minimum Viable Example

Copy-paste this complete working example:

```rust
// main.rs - Complete working example
use inferadb::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create client from environment variables:
    // INFERADB_URL, INFERADB_CLIENT_ID, INFERADB_PRIVATE_KEY_PATH, INFERADB_VAULT_ID
    let client = Client::from_env().await?;

    // Check if user has permission
    let allowed = client
        .check("user:alice", "view", "document:readme")
        .await?;

    if allowed {
        println!("Access granted!");
    } else {
        println!("Access denied.");
    }

    Ok(())
}
```

```toml
# Cargo.toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"

[dependencies]
inferadb = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```bash
# Run it
export INFERADB_URL=https://api.inferadb.com
export INFERADB_CLIENT_ID=my_service
export INFERADB_PRIVATE_KEY_PATH=./private_key.pem
export INFERADB_VAULT_ID=my_vault

cargo run
```

### Installation

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
inferadb = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

For minimal builds (REST only, smaller binary):

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }
```

### Your First Authorization Check

```rust
use inferadb::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to InferaDB
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials("your_client_id", "path/to/private_key.pem")
        .default_vault("your_vault_id")
        .build()
        .await?;

    // Check if alice can view the readme document
    let allowed = client
        .check("user:alice", "view", "document:readme")
        .await?;

    println!("Access allowed: {}", allowed);

    Ok(())
}
```

That's it! The SDK handles authentication, token refresh, and connection management automatically.

### Next Steps

```rust
// Add a relationship
client
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Check with ABAC context
let allowed = client
    .check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.50")
        .insert("mfa_verified", true))
    .await?;

// Get detailed decision with trace
let decision = client
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .detailed()
    .await?;

println!("Allowed: {}, Reason: {:?}", decision.allowed, decision.reason);
```

---

## Local Development

The SDK provides first-class support for local development workflows.

### Connecting to Local InferaDB

```rust
// Connect to local development instance
let client = Client::local()
    .port(8080)  // Default: 8080
    .vault("dev-vault")
    .build()
    .await?;

// Or explicit local URL
let client = Client::builder()
    .url("http://localhost:8080")
    .insecure()  // Allow non-TLS for local dev
    .default_vault("dev-vault")
    .build()
    .await?;
```

### In-Memory Client for Unit Tests

For pure unit tests that don't need a running server:

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_authorization_logic() {
    // Create an in-memory client with no network calls
    let client = InMemoryClient::new();

    // Seed with relationships
    client.write_batch([
        Relationship::new("document:readme", "owner", "user:alice"),
        Relationship::new("document:readme", "viewer", "user:bob"),
    ]).await?;

    // Test authorization
    assert!(client.check("user:alice", "delete", "document:readme").await?);
    assert!(client.check("user:bob", "view", "document:readme").await?);
    assert!(!client.check("user:bob", "delete", "document:readme").await?);
}
```

### Docker Compose Integration

For integration tests with a real InferaDB instance:

```yaml
# docker-compose.test.yml
services:
  inferadb:
    image: ghcr.io/inferadb/inferadb:latest
    ports:
      - "8080:8080"
    environment:
      INFERADB__STORAGE__BACKEND: memory
      INFERADB__AUTH__SKIP_VERIFICATION: true # Dev only!
```

```rust
use inferadb::testing::TestContainer;

#[tokio::test]
async fn integration_test() {
    // Automatically starts/stops InferaDB container
    let container = TestContainer::start().await?;
    let client = container.client().await?;

    // Run integration tests
    client.write(...).await?;
    assert!(client.check(...).await?);

    // Container cleaned up on drop
}
```

### Development Workflow with Watch Mode

The SDK integrates with `cargo watch` for rapid iteration:

```bash
# Terminal 1: Start local InferaDB
cd deploy && ./scripts/dev-up.sh

# Terminal 2: Run tests on file change
cargo watch -x 'test --features test-utils'
```

### Schema Development

```rust
use inferadb::testing::SchemaTestHarness;

#[tokio::test]
async fn test_schema_changes() {
    let harness = SchemaTestHarness::new()
        .with_schema(include_str!("../schemas/v2.ipl"))
        .build()
        .await?;

    // Test that new schema works as expected
    harness.write(Relationship::new("folder:docs", "parent", "folder:root")).await?;

    // Verify permission inheritance works
    assert!(harness.check("user:alice", "view", "folder:docs").await?);
}
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 2: VISION & ARCHITECTURE
     ═══════════════════════════════════════════════════════════════════════════ -->

## Design Philosophy

### Core Principles

1. **Zero-friction authentication**: SDK self-manages tokens, refresh cycles, and credential rotation. Developers provide credentials once and forget about auth.

2. **Unified service URL**: Single endpoint routes to both Engine and Control APIs transparently. No separate clients or configuration.

3. **Type-safe by default**: Leverage Rust's type system to prevent invalid states. Relationship tuples, permissions, and resources are typed at compile time.

4. **Streaming-first**: All list operations support streaming for memory efficiency. Batch operations stream results as they complete.

5. **Protocol flexibility**: Support both gRPC (high performance) and REST (universal compatibility) with feature flags.

6. **Observability built-in**: First-class tracing, metrics, and structured logging without configuration.

7. **Testing as a feature**: Mock clients, simulation mode, and test utilities are first-class SDK features.

### What Sets Us Apart

| Capability          | InferaDB SDK                          | SpiceDB                 | OpenFGA            | Oso             |
| ------------------- | ------------------------------------- | ----------------------- | ------------------ | --------------- |
| Self-managing auth  | Yes (client assertions, auto-refresh) | Manual token management | Manual credentials | API key only    |
| Unified API surface | Single client for auth + management   | Separate clients        | Separate SDK/admin | Separate        |
| Streaming responses | Native async streams                  | gRPC streams only       | Pagination only    | Pagination only |
| Type-safe tuples    | Compile-time validation               | String-based            | String-based       | String-based    |
| Built-in simulation | `simulate()` for what-if testing      | Separate tooling        | No                 | No              |
| Watch/real-time     | SSE + gRPC streaming                  | gRPC only               | No                 | No              |
| Trace/explain       | Full decision trace                   | Limited                 | Limited            | No              |
| Protocol choice     | gRPC + REST                           | gRPC only               | HTTP only          | HTTP only       |

---

## Architecture Overview

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                              InferaDB Rust SDK                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                             Client                                  │   │
│  │  (Unified entry point - routes to Engine or Control automatically)  │   │
│  └───────────────────────────────┬─────────────────────────────────────┘   │
│                                  │                                         │
│  ┌───────────────────────────────┴─────────────────────────────────────┐   │
│  │                      AuthManager (internal)                         │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │   │
│  │  │ ClientAssertion │  │  TokenCache     │  │  RefreshScheduler   │  │   │
│  │  │ (Ed25519 JWT)   │  │  (vault-scoped) │  │  (background task)  │  │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
│  ┌─────────────────────────────┐  ┌─────────────────────────────────────┐  │
│  │      EngineClient           │  │         ControlClient               │  │
│  │  ┌───────────────────────┐  │  │  ┌───────────────────────────────┐  │  │
│  │  │ check()               │  │  │  │ organizations()               │  │  │
│  │  │ check_batch()         │  │  │  │ vaults()                      │  │  │
│  │  │ expand()              │  │  │  │ clients()                     │  │  │
│  │  │ list_resources()      │  │  │  │ teams()                       │  │  │
│  │  │ list_subjects()       │  │  │  │ schemas()                     │  │  │
│  │  │ list_relationships()  │  │  │  │ audit_logs()                  │  │  │
│  │  │ write()               │  │  │  │ users()                       │  │  │
│  │  │ delete()              │  │  │  │ sessions()                    │  │  │
│  │  │ watch()               │  │  │  │ tokens()                      │  │  │
│  │  │ simulate()            │  │  │  └───────────────────────────────┘  │  │
│  │  └───────────────────────┘  │  │                                     │  │
│  └─────────────────────────────┘  └─────────────────────────────────────┘  │
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       Transport Layer                               │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │   │
│  │  │ GrpcTransport   │  │ HttpTransport   │  │ MockTransport       │  │   │
│  │  │ (tonic)         │  │ (reqwest)       │  │ (testing)           │  │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Request Routing

The SDK automatically routes requests to the correct backend based on the operation:

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                     Single Service URL (e.g., api.inferadb.com)          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Control API Routes:                   Engine API Routes:                │
│  ─────────────────────                 ───────────────────               │
│  /control/v1/auth/*                    /access/v1/evaluation             │
│  /control/v1/organizations/*           /access/v1/evaluations            │
│  /control/v1/users/*                   /access/v1/evaluate               │
│  /control/v1/vaults/*                  /access/v1/expand                 │
│  /control/v1/tokens/*                  /access/v1/relationships/*        │
│  /control/v1/clients/*                 /access/v1/resources/*            │
│  /control/v1/teams/*                   /access/v1/subjects/*             │
│  /control/v1/schemas/*                 /access/v1/simulate               │
│  /.well-known/jwks.json                /access/v1/watch                  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Structure

```text
inferadb-sdk/
├── Cargo.toml                    # Workspace manifest
├── inferadb/                     # Main SDK crate (re-exports everything)
│   ├── src/
│   │   ├── lib.rs               # Public API surface
│   │   └── prelude.rs           # Common imports
│   └── Cargo.toml
├── inferadb-client/              # Core client implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs            # Client
│   │   ├── engine.rs            # EngineClient
│   │   ├── control.rs           # ControlClient
│   │   ├── auth/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs       # AuthManager
│   │   │   ├── assertion.rs     # Client assertion JWT
│   │   │   ├── token.rs         # Token cache
│   │   │   └── refresh.rs       # Background refresh
│   │   ├── transport/
│   │   │   ├── mod.rs
│   │   │   ├── grpc.rs          # Tonic transport
│   │   │   ├── http.rs          # Reqwest transport
│   │   │   └── mock.rs          # Test transport
│   │   └── config.rs            # ClientConfig, Builder
│   └── Cargo.toml
├── inferadb-types/               # Shared types
│   ├── src/
│   │   ├── lib.rs
│   │   ├── relationship.rs      # Relationship, Subject, Resource
│   │   ├── decision.rs          # Decision, Trace
│   │   ├── vault.rs             # Vault, VaultRole
│   │   ├── organization.rs      # Organization, Member
│   │   └── error.rs             # Error types
│   └── Cargo.toml
├── inferadb-macros/              # Procedural macros
│   ├── src/
│   │   ├── lib.rs
│   │   ├── resource.rs          # #[derive(Resource)]
│   │   └── relation.rs          # #[derive(Relation)]
│   └── Cargo.toml
└── inferadb-test/                # Testing utilities
    ├── src/
    │   ├── lib.rs
    │   ├── mock.rs              # MockClient
    │   ├── fixtures.rs          # Test data builders
    │   └── assertions.rs        # Custom assertions
    └── Cargo.toml
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 2: CLIENT CONFIGURATION
     ═══════════════════════════════════════════════════════════════════════════ -->

## Client Builder

### Quick Start

```rust
use inferadb::prelude::*;

// Minimal setup with client credentials
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials("client_id", "path/to/private_key.pem")
    .default_vault("vault_id")
    .build()
    .await?;

// Authorization check
let allowed = client.check("user:alice", "view", "document:readme").await?;
```

### Full Configuration

```rust
use inferadb::{
    Client,
    ClientConfig,
    RetryConfig,
    auth::{ClientCredentials, Ed25519PrivateKey},
};
use std::time::Duration;

let client = Client::builder()
    // Connection
    .url("https://api.inferadb.com")
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(5))

    // Authentication (client assertion - recommended for services)
    .client_credentials(ClientCredentials {
        client_id: "client_id".into(),
        private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
        certificate_id: Some("cert_kid".into()),  // Optional: specific cert
    })

    // Target vault (can be overridden per-request)
    .default_vault("production_vault_id")
    .default_organization("org_id")  // Optional: for control operations

    // Protocol selection
    .prefer_grpc()  // Use gRPC when available, fall back to REST
    // .rest_only()  // Force REST (useful for restrictive networks)
    // .grpc_only()  // Force gRPC (maximum performance)

    // Retry configuration
    .retry(RetryConfig {
        max_attempts: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(10),
        backoff_multiplier: 2.0,
        retryable_status_codes: vec![429, 503, 504],
    })

    // Connection pooling
    .max_connections(100)
    .idle_timeout(Duration::from_secs(90))

    // Observability
    .tracing(true)  // Emit tracing spans
    .metrics(true)  // Emit metrics

    .build()
    .await?;
```

### Environment-Based Configuration

```rust
// Reads from environment variables:
// INFERADB_URL, INFERADB_CLIENT_ID, INFERADB_PRIVATE_KEY_PATH,
// INFERADB_VAULT, INFERADB_ORGANIZATION
let client = Client::from_env().await?;

// Or with a prefix for multiple environments
let staging = Client::from_env_with_prefix("STAGING_INFERADB").await?;
let prod = Client::from_env_with_prefix("PROD_INFERADB").await?;
```

#### Environment Variable Error Messages

The SDK provides clear, actionable error messages for configuration issues:

```rust
// Missing required variable
let result = Client::from_env().await;
// Error: Missing required environment variable 'INFERADB_URL'
//        Set this variable or use Client::builder() for explicit configuration.
//        Example: export INFERADB_URL=https://api.inferadb.com

// Invalid URL format
// INFERADB_URL=not-a-url
// Error: Invalid value for 'INFERADB_URL': 'not-a-url'
//        Expected a valid URL (e.g., https://api.inferadb.com)

// File not found
// INFERADB_PRIVATE_KEY_PATH=/nonexistent/key.pem
// Error: Cannot read private key from 'INFERADB_PRIVATE_KEY_PATH'
//        File not found: /nonexistent/key.pem
//        Ensure the file exists and is readable.

// Invalid PEM format
// Error: Invalid private key in 'INFERADB_PRIVATE_KEY'
//        Expected Ed25519 private key in PEM format.
//        Key should start with '-----BEGIN PRIVATE KEY-----'

// Conflicting variables
// Both INFERADB_PRIVATE_KEY and INFERADB_PRIVATE_KEY_PATH set
// Error: Conflicting environment variables
//        Both 'INFERADB_PRIVATE_KEY' and 'INFERADB_PRIVATE_KEY_PATH' are set.
//        Use only one: inline key content OR file path.
```

#### Validation Before Connection

```rust
// Validate configuration without connecting
match Client::validate_env() {
    Ok(config) => {
        println!("Configuration valid:");
        println!("  URL: {}", config.url);
        println!("  Client ID: {}", config.client_id);
        println!("  Vault: {:?}", config.default_vault);
    }
    Err(errors) => {
        eprintln!("Configuration errors:");
        for error in errors {
            eprintln!("  - {}", error);
        }
        std::process::exit(1);
    }
}
```

---

## Authentication

> **TL;DR**: Use `ClientCredentials` with Ed25519 keys for services. Token refresh is automatic.

The SDK supports multiple authentication methods, all with automatic token management.

### Client Credentials (Recommended for Services)

Uses OAuth 2.0 JWT Bearer client assertion (RFC 7523) with Ed25519 signatures.

```rust
use inferadb::auth::{ClientCredentials, Ed25519PrivateKey};

// From PEM file
let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
    certificate_id: None,  // Uses default certificate
};

// From PEM string (e.g., from secret manager)
let pem = std::env::var("PRIVATE_KEY_PEM")?;
let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem(&pem)?,
    certificate_id: Some("specific_cert_kid".into()),
};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()
    .await?;
```

**How it works internally:**

1. SDK generates JWT assertion signed with Ed25519 private key
2. Exchanges assertion for vault-scoped access token (5 min TTL)
3. Receives refresh token (30 day TTL, single-use)
4. Background task refreshes tokens before expiry
5. On refresh, receives new access + refresh token pair
6. All token management is invisible to the caller

### User Session (CLI/Interactive Apps)

For CLI tools or applications with user interaction.

```rust
use inferadb::auth::SessionToken;

// From CLI login flow (OAuth PKCE)
let session = SessionToken::from_cli_login().await?;

// Or from existing session
let session = SessionToken::new("session_id_from_cookie");

let client = Client::builder()
    .url("https://api.inferadb.com")
    .session(session)
    .build()
    .await?;
```

### Bearer Token (Simple/Testing)

For cases where you have a pre-obtained token.

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .bearer_token("eyJ...")
    .build()
    .await?;
```

### Custom JWT Claims

Add custom claims to client assertions for service-to-service context propagation:

```rust
use inferadb::auth::{ClientCredentials, CustomClaims};

let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
    certificate_id: None,
    // Add custom claims to every JWT assertion
    custom_claims: Some(CustomClaims::new()
        .insert("service_name", "order-processor")
        .insert("environment", "production")
        .insert("deployment_id", "deploy-abc123")),
};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()
    .await?;
```

#### Dynamic Claims Per-Request

For claims that vary per request (correlation IDs, tenant hints):

```rust
// Set claims that will be included in token refresh requests
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Dynamic claims evaluated on each token request
    .dynamic_claims(|| {
        CustomClaims::new()
            .insert("correlation_id", Uuid::new_v4().to_string())
            .insert("request_time", Utc::now().to_rfc3339())
    })
    .build()
    .await?;
```

#### Per-Request Claim Override

```rust
// Override claims for a specific operation
let allowed = client
    .check("user:alice", "view", "doc:1")
    .with_claims(CustomClaims::new()
        .insert("audit_reason", "support_ticket_12345")
        .insert("operator_id", "admin:bob"))
    .await?;
```

#### Claim Validation

Custom claims are validated before being added to assertions:

```rust
// Claims must follow these rules:
// - Keys must be strings matching [a-z_][a-z0-9_]*
// - Values must be JSON-serializable
// - Reserved claim names (iss, sub, aud, exp, iat, jti) cannot be overridden
// - Total claims size must not exceed 4KB

let claims = CustomClaims::new()
    .insert("valid_key", "value")           // ✅ OK
    .insert("nested", json!({"a": 1}))      // ✅ OK - JSON object
    .insert("iss", "override")?;            // ❌ Error: reserved claim

// Validate claims before use
match claims.validate() {
    Ok(_) => println!("Claims valid"),
    Err(e) => eprintln!("Invalid claims: {}", e),
}
```

#### Use Cases for Custom Claims

| Claim | Purpose | Example |
|-------|---------|---------|
| `correlation_id` | Distributed tracing | `"corr-abc123"` |
| `tenant_hint` | Multi-tenant routing | `"tenant:acme"` |
| `audit_reason` | Compliance logging | `"GDPR data request"` |
| `operator_id` | Admin impersonation tracking | `"admin:bob"` |
| `deployment_id` | Canary/blue-green identification | `"deploy-v2.3.1"` |
| `region` | Geographic context | `"us-east-1"` |

### Authentication Flow Diagram

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Client Credentials Authentication Flow                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Initial Authentication                                                  │
│  ─────────────────────────                                                  │
│                                                                             │
│  SDK                              Control API                               │
│   │                                   │                                     │
│   │  POST /control/v1/token           │                                     │
│   │  ┌─────────────────────────────┐  │                                     │
│   │  │ grant_type: client_assertion│  │                                     │
│   │  │ assertion: <ed25519_jwt>    │  │                                     │
│   │  │ vault_id: <target_vault>    │──│                                     │
│   │  │ scopes: [check, write, ...] │  │                                     │
│   │  └─────────────────────────────┘  │                                     │
│   │                                   │                                     │
│   │  ┌─────────────────────────────┐  │                                     │
│   │  │ access_token: <jwt>         │  │                                     │
│   │◄─│ refresh_token: <opaque>     │──│                                     │
│   │  │ expires_in: 300             │  │                                     │
│   │  └─────────────────────────────┘  │                                     │
│   │                                   │                                     │
│                                                                             │
│  2. Token Usage (Automatic)                                                 │
│  ──────────────────────────                                                 │
│                                                                             │
│  SDK                              Engine API                                │
│   │                                   │                                     │
│   │  POST /access/v1/evaluation       │                                     │
│   │  Authorization: Bearer <access>   │                                     │
│   │──────────────────────────────────►│                                     │
│   │                                   │                                     │
│   │◄──────────────────────────────────│                                     │
│   │  { "decision": true }             │                                     │
│   │                                   │                                     │
│                                                                             │
│  3. Background Token Refresh (at ~4 minutes)                                │
│  ───────────────────────────────────────────                                │
│                                                                             │
│  SDK (background)                 Control API                               │
│   │                                   │                                     │
│   │  POST /control/v1/tokens/refresh  │                                     │
│   │  Authorization: Bearer <refresh>  │                                     │
│   │──────────────────────────────────►│                                     │
│   │                                   │                                     │
│   │  ┌─────────────────────────────┐  │                                     │
│   │◄─│ access_token: <new_jwt>     │──│                                     │
│   │  │ refresh_token: <new_opaque> │  │  (old refresh token invalidated)    │
│   │  └─────────────────────────────┘  │                                     │
│   │                                   │                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Connection Management

### Connection Lifecycle

Understanding connection lifecycle helps debug connectivity issues and optimize performance.

#### Connection State Machine

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Connection Pool State Machine                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐                                                           │
│   │   Created   │  Client::builder().build()                                │
│   └──────┬──────┘                                                           │
│          │                                                                  │
│          ▼                                                                  │
│   ┌─────────────┐                                                           │
│   │    Idle     │◄────────────────────────────────────────┐                 │
│   │  (no conn)  │                                         │                 │
│   └──────┬──────┘                                         │                 │
│          │ First request                                  │                 │
│          ▼                                                │                 │
│   ┌─────────────┐     Success      ┌─────────────┐       │                 │
│   │ Connecting  │─────────────────►│   Active    │       │                 │
│   │             │                  │ (pooled)    │       │                 │
│   └──────┬──────┘                  └──────┬──────┘       │                 │
│          │                                │               │                 │
│          │ Failure                        │ Idle timeout  │                 │
│          ▼                                ▼               │                 │
│   ┌─────────────┐                  ┌─────────────┐       │                 │
│   │   Retry     │                  │   Closing   │───────┘                 │
│   │  (backoff)  │                  │             │                         │
│   └──────┬──────┘                  └─────────────┘                         │
│          │                                                                  │
│          │ Max retries                                                      │
│          ▼                                                                  │
│   ┌─────────────┐                                                           │
│   │   Failed    │──► Returns Error to caller                                │
│   └─────────────┘                                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Token Refresh Lifecycle

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Token Refresh Lifecycle                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Timeline ────────────────────────────────────────────────────────►        │
│                                                                             │
│   t=0          t=4min        t=5min                   t=30 days             │
│    │              │             │                         │                 │
│    ▼              ▼             ▼                         ▼                 │
│  ┌────┐       ┌────────┐    ┌────────┐              ┌────────────┐          │
│  │Auth│       │Refresh │    │ Token  │              │  Refresh   │          │
│  │    │       │Started │    │Expires │              │Token Expires│          │
│  └────┘       └────────┘    └────────┘              └────────────┘          │
│    │              │             │                         │                 │
│    │              │             │                         │                 │
│  Access         New access    Old token                Re-auth              │
│  token          token         invalid                  required             │
│  issued         ready                                                       │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Background refresh at ~80% of token lifetime (4 min for 5 min)  │        │
│  │ Ensures seamless operation without request delays               │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Watch Stream Reconnection

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Watch Stream Reconnection                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐                                                          │
│   │  Connected   │◄─────────────────────────────────┐                       │
│   │  (streaming) │                                  │                       │
│   └──────┬───────┘                                  │                       │
│          │                                          │                       │
│          │ Disconnect (network/server)              │                       │
│          ▼                                          │                       │
│   ┌──────────────┐                                  │                       │
│   │ Disconnected │                                  │                       │
│   │              │                                  │                       │
│   └──────┬───────┘                                  │                       │
│          │                                          │                       │
│          │ Start backoff                            │ Success               │
│          ▼                                          │                       │
│   ┌──────────────┐    Retry with    ┌────────────┐ │                       │
│   │   Backoff    │───────────────►  │ Reconnect  │─┘                       │
│   │  (1s→2s→4s)  │     cursor       │ from cursor│                         │
│   └──────┬───────┘                  └────────────┘                         │
│          │                                                                  │
│          │ Max backoff (5 min)                                              │
│          ▼                                                                  │
│   ┌──────────────┐                                                          │
│   │    Error     │──► Emits WatchError, caller decides                      │
│   │  (give up)   │                                                          │
│   └──────────────┘                                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### API Version Negotiation

The SDK handles API version compatibility automatically:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Specify minimum required API version
    .min_api_version("2024.1")
    // Or require specific version
    .api_version("2024.2")
    .build()
    .await?;
```

**How version negotiation works:**

1. SDK sends `Accept-Version: 2024.2` header
2. Server responds with `API-Version: 2024.2` if supported
3. If server only supports older version, returns error with supported versions
4. SDK caches negotiated version for connection lifetime

```rust
// Check negotiated version
let info = client.server_info().await?;
println!("Server API version: {}", info.api_version);
println!("Supported versions: {:?}", info.supported_versions);

// Conditional feature usage based on version
if info.api_version >= Version::new(2024, 2) {
    // Use new batch API
    client.check_batch_v2(checks).await?;
} else {
    // Fall back to original
    client.check_batch(checks).collect().await?;
}
```

#### Lazy vs Eager Connection

Connections are established **lazily** by default:

```rust
// No connections made yet
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()  // Token fetch happens here
    .await?;

// First connection established on first request
let allowed = client.check("user:alice", "view", "doc:1").await?;
```

For eager connection establishment (useful for fail-fast at startup):

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .eager_connect(true)  // Establish connection on build()
    .build()
    .await?;  // Fails here if server unreachable

// Connection already established
let allowed = client.check("user:alice", "view", "doc:1").await?;
```

### Connection Pooling

The SDK maintains connection pools for both gRPC and HTTP transports.

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)

    // HTTP connection pool (for REST endpoints)
    .http_pool_max_idle_per_host(10)
    .http_pool_idle_timeout(Duration::from_secs(90))

    // gRPC connection pool (for streaming endpoints)
    .grpc_concurrency_limit(100)
    .grpc_keep_alive(Duration::from_secs(60))

    .build()
    .await?;
```

### Multi-Vault Support

```rust
// Client with default vault
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .default_vault("vault_a")
    .build()
    .await?;

// Use default vault
client.check("user:alice", "view", "document:readme").await?;

// Override vault for specific operation
client
    .with_vault("vault_b")
    .check("user:alice", "view", "document:readme")
    .await?;

// Or use scoped client for many operations
let vault_b = client.scoped_to_vault("vault_b");
vault_b.check("user:alice", "view", "doc:1").await?;
vault_b.check("user:bob", "edit", "doc:2").await?;
```

### Clone Semantics

The `Client` is cheap to clone and safe to share across threads and tasks:

```rust
// Client uses Arc internally - cloning is O(1)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()
    .await?;

// Clone for use in multiple tasks
let client_clone = client.clone();  // Cheap, shares connection pool

tokio::spawn(async move {
    client_clone.check("user:alice", "view", "doc:1").await
});

// Original client still usable
client.check("user:bob", "view", "doc:2").await?;
```

#### What's Shared vs Cloned

| Component          | Behavior | Notes                          |
| ------------------ | -------- | ------------------------------ |
| Connection pools   | Shared   | HTTP and gRPC connections      |
| Token cache        | Shared   | All clones use same tokens     |
| Configuration      | Shared   | Immutable after build          |
| Middleware stack   | Shared   | Same middleware for all clones |
| Metrics/tracing    | Shared   | Single set of metrics          |
| Cache (if enabled) | Shared   | Same cache backend             |

#### Thread Safety

```rust
// Client is Send + Sync - safe to use from any thread
fn assert_send_sync<T: Send + Sync>() {}
assert_send_sync::<Client>();

// Safe to store in lazy_static or once_cell
use once_cell::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::new();

async fn init_client() {
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(creds)
        .build()
        .await
        .unwrap();

    CLIENT.set(client).unwrap();
}

async fn get_client() -> &'static Client {
    CLIENT.get().expect("Client not initialized")
}
```

### Graceful Shutdown

Properly drain connections and complete in-flight requests:

```rust
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(creds)
        .build()
        .await?;

    // Start your application
    let app_handle = tokio::spawn(run_application(client.clone()));

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");

    // Graceful shutdown with timeout
    match client.shutdown_timeout(Duration::from_secs(30)).await {
        Ok(stats) => {
            tracing::info!(
                "Shutdown complete: {} requests drained, {} connections closed",
                stats.requests_drained,
                stats.connections_closed
            );
        }
        Err(e) => {
            tracing::warn!("Shutdown timeout: {}", e);
        }
    }

    // Wait for application to finish
    app_handle.await??;

    Ok(())
}
```

#### Shutdown Behavior

```rust
// Immediate shutdown - cancels in-flight requests
client.shutdown().await?;

// Graceful shutdown - waits for in-flight requests
client.shutdown_timeout(Duration::from_secs(30)).await?;

// Check if client is shutting down
if client.is_shutting_down() {
    return Err(Error::ShuttingDown);
}

// Shutdown hook for cleanup
client
    .on_shutdown(|| async {
        tracing::info!("Client shutting down, saving state...");
        save_cursor_position().await?;
        Ok(())
    })
    .build()
    .await?;
```

#### Watch Stream Shutdown

```rust
// Watch streams respect shutdown signals
let (watch, shutdown_tx) = client
    .watch()
    .with_shutdown_signal()
    .run()
    .await?;

// Later, signal shutdown
shutdown_tx.send(()).await?;

// Or automatically via client shutdown
// client.shutdown() will stop all watch streams
```

### Resource Cleanup and Drop Behavior

Understanding how the SDK handles resource cleanup is critical for long-running applications.

#### What Happens on Drop

When a `Client` is dropped without calling `shutdown()`:

```rust
{
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(creds)
        .build()
        .await?;

    // Use client...

}  // Client dropped here - what happens?
```

**Automatic cleanup on drop:**

- ✅ HTTP connections returned to OS (via reqwest/hyper)
- ✅ gRPC channels closed
- ✅ Memory freed
- ⚠️ Background token refresh task cancelled (may log warning)
- ⚠️ Watch streams abruptly terminated (no graceful close)
- ⚠️ In-flight requests cancelled (may leave server-side state inconsistent)

**Recommendation**: Always call `shutdown()` for production applications:

```rust
// Explicit shutdown is cleaner
client.shutdown_timeout(Duration::from_secs(5)).await?;
// Now safe to drop
```

#### Async Drop Pattern

Rust doesn't have async `Drop`. For cleanup that requires async operations, use explicit shutdown:

```rust
pub struct MyService {
    client: Client,
}

impl MyService {
    /// Call this before dropping
    pub async fn close(self) -> Result<(), Error> {
        // Stop accepting new requests
        self.client.shutdown_timeout(Duration::from_secs(30)).await?;
        Ok(())
    }
}

// Usage
let service = MyService::new(client);
// ... use service ...
service.close().await?;  // Explicit cleanup
// service is now consumed and cannot be used
```

#### Detecting Resource Leaks

Enable leak detection in development:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Warn if dropped without shutdown (debug builds only)
    .warn_on_drop_without_shutdown(true)
    .build()
    .await?;
```

This emits a warning if the client is dropped without `shutdown()`:

```text
[WARN inferadb] Client dropped without shutdown(). This may leave resources
               unreleased. Call client.shutdown() for clean cleanup.
               Created at: src/main.rs:42
```

#### Preventing Leaks in Long-Running Applications

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ApplicationState {
    client: Arc<Client>,
    shutdown_complete: Arc<Mutex<bool>>,
}

impl ApplicationState {
    pub async fn shutdown(&self) {
        let mut complete = self.shutdown_complete.lock().await;
        if !*complete {
            if let Err(e) = self.client.shutdown_timeout(Duration::from_secs(30)).await {
                tracing::error!("Client shutdown error: {}", e);
            }
            *complete = true;
        }
    }
}

// Ensure shutdown on process exit
impl Drop for ApplicationState {
    fn drop(&mut self) {
        if !*self.shutdown_complete.blocking_lock() {
            tracing::warn!("ApplicationState dropped without shutdown()");
        }
    }
}
```

#### Background Task Lifecycle

The SDK spawns background tasks that must be cleaned up:

| Task | Created When | Cleaned Up By |
|------|--------------|---------------|
| Token refresh | Client with credentials | `shutdown()` or Drop |
| Watch streams | `client.watch().run()` | Stream drop or `shutdown()` |
| Metrics reporter | `metrics(true)` | `shutdown()` or Drop |
| Health monitor | `health_check_interval()` | `shutdown()` or Drop |

```rust
// Check for orphaned background tasks
let stats = client.background_task_stats();
println!("Active tasks: {}", stats.active_count);
println!("Token refresh running: {}", stats.token_refresh_active);
println!("Watch streams: {}", stats.watch_stream_count);
```

---

## Configuration Options

### Full Configuration Reference

| Option                     | Type       | Default  | Description                                     |
| -------------------------- | ---------- | -------- | ----------------------------------------------- |
| `url`                      | `String`   | Required | Service URL (routes to both Engine and Control) |
| `timeout`                  | `Duration` | 30s      | Request timeout                                 |
| `connect_timeout`          | `Duration` | 5s       | Connection establishment timeout                |
| `default_vault`            | `String`   | None     | Default vault for Engine operations             |
| `default_organization`     | `String`   | None     | Default organization for Control operations     |
| `prefer_grpc`              | `bool`     | true     | Use gRPC when available                         |
| `max_connections`          | `usize`    | 100      | HTTP connection pool size                       |
| `idle_timeout`             | `Duration` | 90s      | Idle connection timeout                         |
| `retry.max_attempts`       | `u32`      | 3        | Maximum retry attempts                          |
| `retry.initial_backoff`    | `Duration` | 100ms    | Initial retry delay                             |
| `retry.max_backoff`        | `Duration` | 10s      | Maximum retry delay                             |
| `retry.backoff_multiplier` | `f64`      | 2.0      | Exponential backoff factor                      |
| `tracing`                  | `bool`     | false    | Enable tracing spans                            |
| `metrics`                  | `bool`     | false    | Enable metrics emission                         |

### Environment Variables

| Variable                    | Description                      |
| --------------------------- | -------------------------------- |
| `INFERADB_URL`              | Service URL                      |
| `INFERADB_CLIENT_ID`        | Client ID for authentication     |
| `INFERADB_PRIVATE_KEY_PATH` | Path to Ed25519 private key PEM  |
| `INFERADB_PRIVATE_KEY`      | Ed25519 private key PEM contents |
| `INFERADB_CERTIFICATE_ID`   | Specific certificate KID to use  |
| `INFERADB_VAULT`            | Default vault ID                 |
| `INFERADB_ORGANIZATION`     | Default organization ID          |
| `INFERADB_TIMEOUT_SECS`     | Request timeout in seconds       |
| `INFERADB_PREFER_GRPC`      | "true" or "false"                |

---

## Middleware & Interceptors

The SDK provides a composable middleware stack for customizing request/response handling.

### Built-in Middleware

```rust
use inferadb::middleware::{LoggingMiddleware, MetricsMiddleware, TracingMiddleware};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Add middleware (executed in order)
    .with_middleware(TracingMiddleware::new())
    .with_middleware(MetricsMiddleware::new())
    .with_middleware(LoggingMiddleware::new().level(log::Level::Debug))
    .build()
    .await?;
```

### Custom Headers Middleware

Inject headers into every request (useful for correlation IDs, tenant context):

```rust
use inferadb::middleware::HeadersMiddleware;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .with_middleware(HeadersMiddleware::new()
        .static_header("X-Service-Name", "my-api")
        .dynamic_header("X-Request-ID", || uuid::Uuid::new_v4().to_string()))
    .build()
    .await?;
```

### Custom Middleware

Implement your own middleware for advanced use cases:

```rust
use inferadb::middleware::{Middleware, Next, Request, Response};
use async_trait::async_trait;

struct AuditMiddleware {
    audit_log: Arc<AuditLogger>,
}

#[async_trait]
impl Middleware for AuditMiddleware {
    async fn handle(&self, request: Request, next: Next<'_>) -> Result<Response, Error> {
        let start = std::time::Instant::now();
        let operation = request.operation().to_string();

        // Call the next middleware/handler
        let response = next.run(request).await;

        // Log after completion
        self.audit_log.record(AuditEntry {
            operation,
            duration: start.elapsed(),
            success: response.is_ok(),
            request_id: response.as_ref().ok().and_then(|r| r.request_id()),
        }).await;

        response
    }
}

// Use it
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .with_middleware(AuditMiddleware { audit_log })
    .build()
    .await?;
```

### Request/Response Transformation

```rust
use inferadb::middleware::TransformMiddleware;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .with_middleware(TransformMiddleware::new()
        .on_request(|req| {
            // Add tenant prefix to all resources
            req.transform_resource(|r| format!("tenant_123:{}", r))
        })
        .on_response(|resp| {
            // Strip tenant prefix from responses
            resp.transform_resource(|r| r.strip_prefix("tenant_123:").unwrap_or(r).to_string())
        }))
    .build()
    .await?;
```

### Middleware Order

Middleware executes in the order added, wrapping around the core client:

```text
Request Flow:
┌─────────────────────────────────────────────────────────────┐
│  Your Code                                                  │
│      │                                                      │
│      ▼                                                      │
│  ┌─────────────────┐                                        │
│  │ TracingMiddleware │ ◄─── Outermost (first added)         │
│  └────────┬────────┘                                        │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ MetricsMiddleware│                                       │
│  └────────┬────────┘                                        │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ LoggingMiddleware│ ◄─── Innermost (last added)           │
│  └────────┬────────┘                                        │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │   Core Client   │ ◄─── Actual HTTP/gRPC call             │
│  └─────────────────┘                                        │
│                                                             │
│  Response flows back up through the same stack              │
└─────────────────────────────────────────────────────────────┘
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 4: ENGINE API (AUTHORIZATION)
     ═══════════════════════════════════════════════════════════════════════════ -->

## Authorization Checks

### Simple Check

```rust
// Boolean result
let allowed = client.check("user:alice", "view", "document:readme").await?;

if allowed {
    // Grant access
}
```

### Check with Context (ABAC)

```rust
use inferadb::Context;

let allowed = client
    .check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.50")
        .insert("time_of_day", "14:30")
        .insert("mfa_verified", true))
    .await?;
```

### Check with Decision Details

```rust
use inferadb::CheckOptions;

let decision = client
    .check("user:alice", "edit", "document:readme")
    .with_options(CheckOptions {
        trace: true,  // Include evaluation trace
    })
    .detailed()
    .await?;

println!("Allowed: {}", decision.allowed);
println!("Reason: {:?}", decision.reason);

if let Some(trace) = decision.trace {
    println!("Evaluation path:");
    for step in trace.steps {
        println!("  {} -> {}", step.relation, step.result);
    }
}
```

### Type-Safe Checks (with macros)

The SDK provides derive macros for compile-time type safety.

#### Basic Resource and Permission Types

```rust
use inferadb::prelude::*;

// Define your schema types
#[derive(Resource)]
#[inferadb(type = "document")]
struct Document {
    id: String,
}

#[derive(Resource)]
#[inferadb(type = "user")]
struct User {
    id: String,
}

#[derive(Permission)]
enum DocumentPermission {
    View,
    Edit,
    Delete,
    Share,
}

// Type-safe check
let doc = Document { id: "readme".into() };
let user = User { id: "alice".into() };

let allowed = client
    .check_typed(&user, DocumentPermission::View, &doc)
    .await?;
```

#### Advanced Macro Attributes

```rust
use inferadb::prelude::*;

// Resource with custom ID field
#[derive(Resource)]
#[inferadb(type = "document", id_field = "doc_id")]
struct Document {
    doc_id: Uuid,
    title: String,
}

// Resource with composite ID
#[derive(Resource)]
#[inferadb(type = "document", id_expr = "format!(\"{}/{}\", self.tenant, self.id)")]
struct TenantDocument {
    tenant: String,
    id: String,
}

// Subject with userset support
#[derive(Subject)]
#[inferadb(type = "group")]
struct Group {
    id: String,
    #[inferadb(relation)]
    relation: Option<String>,  // For group:eng#member
}

impl Group {
    fn members(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            relation: Some("member".into()),
        }
    }
}

// Usage
let allowed = client
    .check_typed(
        &Group::members("engineering"),
        DocumentPermission::View,
        &doc,
    )
    .await?;
```

#### Permission Hierarchies

```rust
#[derive(Permission)]
#[inferadb(entity = "document")]
enum DocumentPermission {
    #[inferadb(name = "view")]
    View,

    #[inferadb(name = "edit", implies = ["view"])]
    Edit,

    #[inferadb(name = "delete", implies = ["edit", "view"])]
    Delete,

    #[inferadb(name = "share")]
    Share,

    #[inferadb(name = "admin", implies = ["delete", "share"])]
    Admin,
}

// Compile-time validation
// If schema doesn't define "admin" permission, macro emits error
```

#### Compile-Time Schema Validation

```rust
// Enable schema validation at compile time
#[derive(Resource)]
#[inferadb(
    type = "document",
    schema = "schemas/production.ipl",  // Path to schema file
    validate = true,                     // Enable validation
)]
struct Document {
    id: String,
}

// If "document" entity doesn't exist in schema, compilation fails:
// error: Entity 'document' not found in schema 'schemas/production.ipl'
//        Available entities: user, folder, team
```

#### Relation Types

```rust
#[derive(Relation)]
#[inferadb(from = "document", to = "user")]
struct DocumentViewer;

#[derive(Relation)]
#[inferadb(from = "folder", to = "document")]
struct FolderContains;

// Type-safe relationship creation
let rel = DocumentViewer::new(&doc, &user);
client.write(rel).await?;

// Compile error if types don't match:
// let rel = DocumentViewer::new(&folder, &user);  // ERROR: expected Document, found Folder
```

#### Macro Error Messages

The macros provide helpful error messages:

```rust
#[derive(Resource)]
#[inferadb(type = "document")]
struct Document {
    // Missing id field!
}
// error: Resource 'Document' must have an 'id' field or specify 'id_field' attribute
//   --> src/models.rs:5:1
//    |
// 5  | struct Document {
//    | ^^^^^^^^^^^^^^^^
//    |
//    = help: Add a field named 'id' or use #[inferadb(id_field = "your_field")]

#[derive(Permission)]
enum InvalidPermission {
    // Empty enum!
}
// error: Permission enum must have at least one variant
//   --> src/models.rs:10:1

#[derive(Resource)]
#[inferadb(type = "has spaces")]  // Invalid type name
struct Bad;
// error: Resource type must match pattern [a-z][a-z0-9_]*
//   --> src/models.rs:15:17
//    |
// 15 | #[inferadb(type = "has spaces")]
//    |                   ^^^^^^^^^^^^
```

#### Generated Code Example

For transparency, here's what the macros generate:

```rust
// Input:
#[derive(Resource)]
#[inferadb(type = "document")]
struct Document {
    id: String,
}

// Generated:
impl inferadb::Resource for Document {
    fn resource_type() -> &'static str {
        "document"
    }

    fn resource_id(&self) -> String {
        self.id.clone()
    }

    fn to_resource_string(&self) -> String {
        format!("document:{}", self.id)
    }
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "document:{}", self.id)
    }
}

impl From<Document> for inferadb::types::Resource {
    fn from(doc: Document) -> Self {
        inferadb::types::Resource::new("document", doc.id)
    }
}

impl From<&Document> for inferadb::types::Resource {
    fn from(doc: &Document) -> Self {
        inferadb::types::Resource::new("document", &doc.id)
    }
}
```

---

## Batch Evaluations

### Parallel Batch Check

```rust
use inferadb::BatchCheck;

let checks = vec![
    ("user:alice", "view", "document:1"),
    ("user:alice", "edit", "document:1"),
    ("user:bob", "view", "document:1"),
    ("user:bob", "view", "document:2"),
];

// Returns results as they complete (streaming)
let mut results = client.check_batch(checks).await?;

while let Some(result) = results.next().await {
    let (index, decision) = result?;
    println!("Check {}: {}", index, decision.allowed);
}
```

### Batch with Shared Context

```rust
let context = Context::new()
    .insert("ip_address", "10.0.0.50");

let results = client
    .check_batch(checks)
    .with_context(context)
    .collect()  // Collect all results
    .await?;

// results: Vec<(usize, Decision)>
```

### Batch Check All/Any

```rust
// Check if ALL permissions are granted
let all_allowed = client
    .check_batch(checks)
    .all()
    .await?;

// Check if ANY permission is granted
let any_allowed = client
    .check_batch(checks)
    .any()
    .await?;
```

---

## Relationship Management

### Write Relationships

```rust
use inferadb::Relationship;

// Single relationship
client
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Multiple relationships
let relationships = vec![
    Relationship::new("document:readme", "viewer", "user:alice"),
    Relationship::new("document:readme", "viewer", "user:bob"),
    Relationship::new("document:readme", "editor", "user:charlie"),
    Relationship::new("folder:docs", "viewer", "group:engineering#member"),
];

let result = client.write_batch(relationships).await?;
println!("Written: {}, Revision: {}", result.count, result.revision);
```

### Write with Touch (Idempotent)

```rust
// Touch operation: create if not exists, update timestamp if exists
client
    .touch(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;
```

### Delete Relationships

```rust
// Delete specific relationship
client
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Delete with filter (bulk)
use inferadb::RelationshipFilter;

let deleted = client
    .delete_where(RelationshipFilter::new()
        .resource("document:readme")
        .relation("viewer"))
    .await?;

println!("Deleted {} relationships", deleted.count);
```

### Conditional Write (Optimistic Concurrency)

```rust
// Write only if at specific revision
let result = client
    .write_batch(relationships)
    .expect_revision("abc123")
    .await;

match result {
    Ok(r) => println!("Written at revision {}", r.revision),
    Err(Error::RevisionMismatch { expected, actual }) => {
        println!("Conflict: expected {}, actual {}", expected, actual);
    }
    Err(e) => return Err(e.into()),
}
```

### Transaction Semantics

Understanding atomicity guarantees for relationship operations.

#### Batch Atomicity

```rust
// Batch writes are atomic - all succeed or all fail
let result = client.write_batch([
    Relationship::new("doc:1", "owner", "user:alice"),
    Relationship::new("doc:1", "viewer", "user:bob"),
    Relationship::new("doc:1", "viewer", "group:eng#member"),
]).await?;

// All three relationships are written atomically
// If any fails (e.g., validation error), none are written
println!("All {} relationships written at revision {}", result.count, result.revision);
```

#### Partial Failure Handling

```rust
use inferadb::WriteMode;

// Default: Atomic (all-or-nothing)
let result = client
    .write_batch(relationships)
    .mode(WriteMode::Atomic)
    .await?;

// Alternative: Best-effort (write as many as possible)
let result = client
    .write_batch(relationships)
    .mode(WriteMode::BestEffort)
    .await?;

// Check for partial failures
if result.failed.len() > 0 {
    for failure in &result.failed {
        tracing::warn!(
            index = failure.index,
            error = %failure.error,
            "Relationship write failed"
        );
    }
}
println!("Written: {}, Failed: {}", result.succeeded, result.failed.len());
```

#### Multi-Resource Transactions

For operations spanning multiple resources that must be atomic:

```rust
use inferadb::Transaction;

// Start a transaction
let tx = client.transaction().await?;

// Queue multiple operations
tx.write(Relationship::new("folder:docs", "owner", "user:alice")).await?;
tx.write(Relationship::new("doc:readme", "parent", "folder:docs")).await?;
tx.delete(Relationship::new("doc:readme", "owner", "user:old-owner")).await?;

// Commit atomically
let result = tx.commit().await?;
println!("Transaction committed at revision {}", result.revision);

// Or rollback on error
// tx.rollback().await?;
```

#### Precondition Checks

Ensure state hasn't changed before writing:

```rust
use inferadb::Precondition;

let result = client
    .write_batch([
        Relationship::new("doc:1", "owner", "user:alice"),
    ])
    .preconditions([
        // Only write if doc:1 doesn't already have an owner
        Precondition::must_not_exist("doc:1", "owner", "*"),

        // Only write if user:alice is a member of org
        Precondition::must_exist("org:acme", "member", "user:alice"),
    ])
    .await;

match result {
    Ok(r) => println!("Written successfully"),
    Err(Error::PreconditionFailed { failed }) => {
        for precondition in failed {
            println!("Precondition failed: {:?}", precondition);
        }
    }
    Err(e) => return Err(e.into()),
}
```

#### Idempotency Keys

For safe retries without duplicate writes:

```rust
use uuid::Uuid;

let idempotency_key = Uuid::new_v4().to_string();

// First attempt
let result = client
    .write_batch(relationships)
    .idempotency_key(&idempotency_key)
    .await?;

// Retry with same key - returns same result, no duplicate writes
let retry_result = client
    .write_batch(relationships)
    .idempotency_key(&idempotency_key)
    .await?;

assert_eq!(result.revision, retry_result.revision);
```

---

## Lookup Operations

### List Resources (What can user access?)

```rust
// Stream resources user can view
let mut resources = client
    .list_resources("user:alice", "view", "document")
    .await?;

while let Some(resource) = resources.next().await {
    let resource = resource?;
    println!("Can view: {}", resource);
}

// Collect all (be careful with large result sets)
let all_resources: Vec<String> = client
    .list_resources("user:alice", "view", "document")
    .collect()
    .await?;

// With filtering
let filtered = client
    .list_resources("user:alice", "view", "document")
    .filter_id("project-*")  // Wildcard pattern
    .limit(100)
    .await?;
```

### List Subjects (Who can access resource?)

```rust
// Stream users who can edit document
let mut subjects = client
    .list_subjects("document:readme", "edit")
    .await?;

while let Some(subject) = subjects.next().await {
    let subject = subject?;
    println!("Can edit: {}", subject);
}

// Filter by subject type
let editors = client
    .list_subjects("document:readme", "edit")
    .subject_type("user")
    .collect()
    .await?;
```

### List Relationships

```rust
use inferadb::RelationshipFilter;

// All relationships for a resource
let mut rels = client
    .list_relationships()
    .resource("document:readme")
    .await?;

// With multiple filters
let rels = client
    .list_relationships()
    .resource("document:readme")
    .relation("viewer")
    .subject_type("user")
    .limit(50)
    .collect()
    .await?;
```

### Expand (Userset Tree)

```rust
// Get all subjects in the "viewer" userset
let expansion = client
    .expand("document:readme", "viewer")
    .await?;

println!("Direct viewers:");
for user in expansion.direct {
    println!("  {}", user);
}

println!("Via groups:");
for (group, members) in expansion.via_groups {
    println!("  {} ->", group);
    for member in members {
        println!("    {}", member);
    }
}

// Full tree structure
println!("Tree: {:?}", expansion.tree);
```

---

## Streaming & Watch

### Watch for Changes (Real-time)

```rust
// Watch all changes
let mut changes = client.watch().await?;

while let Some(change) = changes.next().await {
    let change = change?;
    match change.operation {
        Operation::Create => println!("Created: {:?}", change.relationship),
        Operation::Delete => println!("Deleted: {:?}", change.relationship),
    }
    println!("Revision: {}", change.revision);
}

// Watch specific resource types
let mut doc_changes = client
    .watch()
    .resource_types(["document", "folder"])
    .await?;

// Resume from cursor (e.g., after reconnection)
let mut changes = client
    .watch()
    .cursor("last_seen_cursor")
    .await?;
```

### Watch with Handler

```rust
use inferadb::WatchHandler;

client
    .watch()
    .resource_types(["document"])
    .on_change(|change| async move {
        // Update local cache
        cache.invalidate(&change.relationship.resource);
        Ok(())
    })
    .on_error(|err| async move {
        tracing::error!("Watch error: {}", err);
        // Return true to reconnect, false to stop
        true
    })
    .run()
    .await?;
```

### Watch Reconnection Strategy

The SDK provides robust reconnection handling for long-lived watch streams.

#### Automatic Reconnection with Backoff

```rust
use inferadb::watch::{WatchConfig, BackoffConfig};

let watch = client
    .watch()
    .config(WatchConfig {
        // Reconnection settings
        reconnect: true,
        backoff: BackoffConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: 0.1,  // Add 10% random jitter
        },
        max_reconnect_attempts: None,  // Unlimited retries

        // Heartbeat detection
        heartbeat_interval: Duration::from_secs(30),
        heartbeat_timeout: Duration::from_secs(90),
    })
    .await?;
```

#### Cursor Management for Gap-Free Streaming

```rust
use inferadb::watch::CursorStore;

// Persist cursors to survive restarts
struct RedisCursorStore {
    redis: redis::Client,
    key: String,
}

#[async_trait]
impl CursorStore for RedisCursorStore {
    async fn save(&self, cursor: &str) -> Result<(), Error> {
        self.redis.set(&self.key, cursor).await?;
        Ok(())
    }

    async fn load(&self) -> Result<Option<String>, Error> {
        Ok(self.redis.get(&self.key).await?)
    }
}

// Use persistent cursor store
let watch = client
    .watch()
    .cursor_store(RedisCursorStore {
        redis: redis_client,
        key: "inferadb:watch:cursor".into(),
    })
    .on_change(|change| async move {
        process_change(change).await?;
        // Cursor automatically saved after successful processing
        Ok(())
    })
    .run()
    .await?;
```

#### Handling Cursor Gaps

When reconnecting after extended downtime, cursors may expire:

```rust
client
    .watch()
    .cursor_store(cursor_store)
    .on_cursor_expired(|expired_cursor| async move {
        tracing::warn!(
            cursor = %expired_cursor,
            "Watch cursor expired, performing full resync"
        );

        // Option 1: Full resync of affected data
        resync_all_relationships().await?;

        // Option 2: Start from a known-good snapshot
        Ok(CursorRecovery::StartFromSnapshot("snapshot_id"))

        // Option 3: Start fresh (may miss changes)
        // Ok(CursorRecovery::StartFresh)
    })
    .run()
    .await?;
```

#### Tombstone Handling

Deleted relationships may be compacted after some time:

```rust
client
    .watch()
    .on_change(|change| async move {
        match change.operation {
            Operation::Create => {
                // Add to local state
                local_store.insert(change.relationship).await?;
            }
            Operation::Delete => {
                // Remove from local state
                local_store.remove(&change.relationship).await?;
            }
            Operation::Tombstone { deleted_at, compacted } => {
                // Historical delete - may be from compaction
                if compacted {
                    tracing::debug!(
                        "Received compacted tombstone from {}",
                        deleted_at
                    );
                }
                local_store.remove(&change.relationship).await?;
            }
        }
        Ok(())
    })
    .run()
    .await?;
```

#### Health Monitoring for Watch Streams

```rust
use inferadb::watch::WatchHealth;

let (watch, health_handle) = client
    .watch()
    .with_health_handle()
    .run()
    .await?;

// Monitor watch health in background
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;

        let health = health_handle.status();
        metrics::gauge!("inferadb.watch.connected")
            .set(if health.connected { 1.0 } else { 0.0 });
        metrics::gauge!("inferadb.watch.lag_seconds")
            .set(health.lag.as_secs_f64());
        metrics::counter!("inferadb.watch.reconnects")
            .absolute(health.reconnect_count);

        if health.lag > Duration::from_secs(60) {
            tracing::warn!("Watch stream lagging by {:?}", health.lag);
        }
    }
});
```

---

## Simulation

Test authorization decisions without persisting changes.

```rust
// What if we add these relationships?
let result = client
    .simulate()
    .with_relationships([
        Relationship::new("document:secret", "viewer", "user:alice"),
    ])
    .check("user:alice", "view", "document:secret")
    .await?;

assert!(result.decision.allowed);
assert_eq!(result.context_relationships_used, 1);
```

### Complex Simulation

```rust
// Simulate removing relationships
let result = client
    .simulate()
    .without_relationships([
        Relationship::new("document:readme", "viewer", "user:alice"),
    ])
    .check("user:alice", "view", "document:readme")
    .await?;

assert!(!result.decision.allowed);

// Simulate both additions and removals
let result = client
    .simulate()
    .with_relationships([
        Relationship::new("folder:docs", "viewer", "user:alice"),
    ])
    .without_relationships([
        Relationship::new("document:readme", "viewer", "user:alice"),
    ])
    .check("user:alice", "view", "document:readme")
    .await?;
```

---

## Caching

Authorization checks are latency-sensitive. The SDK provides built-in caching with automatic invalidation.

### Enable Caching

```rust
use inferadb::cache::{CacheConfig, MemoryCache};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .default_vault("vault_id")
    .cache(CacheConfig {
        backend: MemoryCache::new(),
        ttl: Duration::from_secs(60),           // Cache decisions for 60 seconds
        max_entries: 10_000,                     // Maximum cached decisions
        negative_ttl: Duration::from_secs(30),  // Cache denials shorter
    })
    .build()
    .await?;

// First call hits the server
let allowed = client.check("user:alice", "view", "doc:1").await?;

// Second call served from cache (sub-millisecond)
let allowed = client.check("user:alice", "view", "doc:1").await?;
```

### Cache Invalidation with Watch

Combine caching with real-time invalidation for consistency:

```rust
use inferadb::cache::{CacheConfig, WatchInvalidation};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .default_vault("vault_id")
    .cache(CacheConfig {
        backend: MemoryCache::new(),
        ttl: Duration::from_secs(300),  // 5 minute TTL
        invalidation: WatchInvalidation::new()
            .on_relationship_change(|change| {
                // Invalidate cache entries affected by this change
                vec![
                    CacheKey::resource(&change.relationship.resource),
                    CacheKey::subject(&change.relationship.subject),
                ]
            }),
    })
    .build()
    .await?;

// Cache is automatically invalidated when relationships change
```

### External Cache Backend (Redis)

For distributed deployments:

```rust
use inferadb::cache::RedisCache;

let redis_cache = RedisCache::new("redis://localhost:6379")
    .prefix("inferadb:")
    .build()
    .await?;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .cache(CacheConfig {
        backend: redis_cache,
        ttl: Duration::from_secs(60),
    })
    .build()
    .await?;
```

### Cache Bypass

For sensitive operations that must hit the server:

```rust
// Bypass cache for this check
let allowed = client
    .check("user:alice", "delete", "document:sensitive")
    .no_cache()
    .await?;

// Force cache refresh
let allowed = client
    .check("user:alice", "view", "document:readme")
    .refresh_cache()
    .await?;
```

### Cache Statistics

```rust
let stats = client.cache_stats();

println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Hits: {}, Misses: {}", stats.hits, stats.misses);
println!("Entries: {}, Size: {} bytes", stats.entries, stats.size_bytes);
println!("Evictions: {}", stats.evictions);
```

### Consistency Guarantees

| Mode              | Consistency        | Latency | Use Case                |
| ----------------- | ------------------ | ------- | ----------------------- |
| No cache          | Strong             | High    | Financial transactions  |
| TTL-based         | Eventual (bounded) | Low     | Most applications       |
| Watch-invalidated | Near-real-time     | Low     | Real-time collaboration |
| Write-through     | Strong             | Medium  | Write-heavy workloads   |

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 5: CONTROL API (MANAGEMENT)
     ═══════════════════════════════════════════════════════════════════════════ -->

## Organization Management

Access organization management through `client.control().organizations()`.

```rust
// List organizations user belongs to
let orgs = client.control().organizations().list().await?;

for org in orgs {
    println!("{}: {} (role: {:?})", org.id, org.name, org.my_role);
}

// Get specific organization
let org = client.control().organizations().get("org_id").await?;

// Create organization
let new_org = client
    .control()
    .organizations()
    .create("My Company")
    .await?;

// Update organization
client
    .control()
    .organizations()
    .update("org_id")
    .name("New Name")
    .await?;

// Delete organization (requires owner role)
client.control().organizations().delete("org_id").await?;
```

### Organization Members

```rust
let members = client
    .control()
    .organizations()
    .members("org_id")
    .list()
    .await?;

// Update member role
client
    .control()
    .organizations()
    .members("org_id")
    .update("user_id")
    .role(OrgRole::Admin)
    .await?;

// Remove member
client
    .control()
    .organizations()
    .members("org_id")
    .remove("user_id")
    .await?;
```

### Organization Invitations

```rust
// List pending invitations
let invitations = client
    .control()
    .organizations()
    .invitations("org_id")
    .list()
    .await?;

// Create invitation
let invitation = client
    .control()
    .organizations()
    .invitations("org_id")
    .create("new_user@example.com")
    .role(OrgRole::Member)
    .await?;

// Accept invitation (from invitee's perspective)
client
    .control()
    .organizations()
    .accept_invitation("invitation_token")
    .await?;

// Delete/cancel invitation
client
    .control()
    .organizations()
    .invitations("org_id")
    .delete("invitation_id")
    .await?;
```

---

## Vault Management

```rust
// List vaults in organization
let vaults = client
    .control()
    .vaults("org_id")
    .list()
    .await?;

// Get vault details
let vault = client.control().vaults("org_id").get("vault_id").await?;

// Create vault
let new_vault = client
    .control()
    .vaults("org_id")
    .create("Production Vault")
    .await?;

// Update vault
client
    .control()
    .vaults("org_id")
    .update("vault_id")
    .name("Renamed Vault")
    .await?;

// Delete vault
client.control().vaults("org_id").delete("vault_id").await?;
```

### Vault Access Grants

```rust
// List user grants
let grants = client
    .control()
    .vaults("org_id")
    .user_grants("vault_id")
    .list()
    .await?;

// Grant user access
client
    .control()
    .vaults("org_id")
    .user_grants("vault_id")
    .create("user_id", VaultRole::Writer)
    .await?;

// Update grant
client
    .control()
    .vaults("org_id")
    .user_grants("vault_id")
    .update("grant_id")
    .role(VaultRole::Admin)
    .await?;

// Revoke grant
client
    .control()
    .vaults("org_id")
    .user_grants("vault_id")
    .delete("grant_id")
    .await?;
```

### Vault Tokens

```rust
// Generate vault access token
let token = client
    .control()
    .tokens()
    .generate_vault_token("org_id", "vault_id")
    .scopes([Scope::Check, Scope::Write])
    .ttl(Duration::from_secs(3600))
    .await?;

println!("Access token: {}", token.access_token);
println!("Refresh token: {}", token.refresh_token);
println!("Expires in: {}s", token.expires_in);
```

---

## Client & Certificate Management

### Clients

```rust
// List clients
let clients = client.control().clients("org_id").list().await?;

// Create client (for service-to-service auth)
let new_client = client
    .control()
    .clients("org_id")
    .create("my-backend-service")
    .await?;

// Get client
let service_client = client.control().clients("org_id").get("client_id").await?;

// Update client
client
    .control()
    .clients("org_id")
    .update("client_id")
    .name("renamed-service")
    .await?;

// Delete client
client.control().clients("org_id").delete("client_id").await?;
```

### Certificates

```rust
// List certificates for client
let certs = client
    .control()
    .clients("org_id")
    .certificates("client_id")
    .list()
    .await?;

// Create new certificate (returns private key ONCE)
let cert = client
    .control()
    .clients("org_id")
    .certificates("client_id")
    .create()
    .await?;

// IMPORTANT: Save private key securely - it's only returned once!
println!("Certificate ID (KID): {}", cert.id);
println!("Public Key: {}", cert.public_key_pem);
println!("Private Key: {}", cert.private_key_pem.unwrap());

// Save to file
std::fs::write("private_key.pem", cert.private_key_pem.unwrap())?;

// Revoke certificate
client
    .control()
    .clients("org_id")
    .certificates("client_id")
    .revoke("cert_id")
    .await?;
```

---

## Team Management

```rust
// List teams
let teams = client.control().teams("org_id").list().await?;

// Create team
let team = client
    .control()
    .teams("org_id")
    .create("Engineering")
    .await?;

// Get team
let team = client.control().teams("org_id").get("team_id").await?;

// Update team
client
    .control()
    .teams("org_id")
    .update("team_id")
    .name("Platform Engineering")
    .await?;

// Delete team
client.control().teams("org_id").delete("team_id").await?;
```

### Team Members

```rust
// List team members
let members = client
    .control()
    .teams("org_id")
    .members("team_id")
    .list()
    .await?;

// Add member
client
    .control()
    .teams("org_id")
    .members("team_id")
    .add("user_id")
    .role(TeamRole::Member)
    .await?;

// Update member role
client
    .control()
    .teams("org_id")
    .members("team_id")
    .update("user_id")
    .role(TeamRole::Lead)
    .await?;

// Remove member
client
    .control()
    .teams("org_id")
    .members("team_id")
    .remove("user_id")
    .await?;
```

### Team Permissions

```rust
// List team permissions
let perms = client
    .control()
    .teams("org_id")
    .permissions("team_id")
    .list()
    .await?;

// Grant permission
client
    .control()
    .teams("org_id")
    .permissions("team_id")
    .grant(TeamPermission::ManageVaults)
    .await?;

// Revoke permission
client
    .control()
    .teams("org_id")
    .permissions("team_id")
    .revoke("permission_id")
    .await?;
```

---

## Schema Management

### Schema Introspection

Programmatically inspect loaded schema at runtime:

```rust
// Get schema metadata
let schema = client.schema().await?;

println!("Schema version: {}", schema.version());
println!("Entities: {:?}", schema.entity_names());

// Inspect specific entity
if let Some(entity) = schema.entity("Document") {
    println!("Document relations:");
    for relation in entity.relations() {
        println!("  - {}: {:?}", relation.name(), relation.target_types());
    }

    println!("Document permissions:");
    for permission in entity.permissions() {
        println!("  - {}: {}", permission.name(), permission.expression());
    }
}

// Check if relation exists (useful for version compatibility)
if schema.has_relation("Document", "commenter") {
    // New relation available - use it
    client.write(Relationship::new("doc:1", "commenter", "user:alice")).await?;
} else {
    // Fall back to older relation
    client.write(Relationship::new("doc:1", "viewer", "user:alice")).await?;
}
```

#### Schema Diffing

Compare schemas for migration planning:

```rust
let current = client.schema().await?;
let proposed = Schema::parse(new_schema_content)?;

let diff = current.diff(&proposed);

for change in diff.changes() {
    match change {
        SchemaChange::EntityAdded(name) => {
            println!("+ entity {}", name);
        }
        SchemaChange::EntityRemoved(name) => {
            println!("- entity {} (BREAKING)", name);
        }
        SchemaChange::RelationAdded { entity, relation } => {
            println!("+ {}.{}", entity, relation);
        }
        SchemaChange::RelationRemoved { entity, relation } => {
            println!("- {}.{} (BREAKING)", entity, relation);
        }
        SchemaChange::PermissionChanged { entity, permission, .. } => {
            println!("~ {}.{}", entity, permission);
        }
    }
}

if diff.has_breaking_changes() {
    println!("WARNING: Schema has breaking changes!");
}
```

### Schema Operations

```rust
// List schema versions
let schemas = client
    .control()
    .schemas("org_id", "vault_id")
    .list()
    .await?;

// Get current active schema
let current = client
    .control()
    .schemas("org_id", "vault_id")
    .current()
    .await?;

println!("Version: {}", current.version);
println!("Content:\n{}", current.content);

// Get specific version
let schema = client
    .control()
    .schemas("org_id", "vault_id")
    .get(version)
    .await?;

// Deploy new schema
let result = client
    .control()
    .schemas("org_id", "vault_id")
    .deploy(r#"
        schema inferadb v1.0

        entity User {
            attributes {
                id: UUID
                email: String @unique
            }
        }

        entity Document {
            relations {
                owner: User
                viewer: User | owner
            }

            permissions {
                view: viewer
                delete: owner
            }
        }
    "#)
    .await?;

println!("Deployed version: {}", result.version);

// Activate specific version
client
    .control()
    .schemas("org_id", "vault_id")
    .activate(version)
    .await?;

// Diff two versions
let diff = client
    .control()
    .schemas("org_id", "vault_id")
    .diff(version_a, version_b)
    .await?;

println!("Changes:\n{}", diff.summary);

// Rollback to previous version
client
    .control()
    .schemas("org_id", "vault_id")
    .rollback()
    .to_version(previous_version)
    .await?;
```

### Schema Validation

Validate schemas locally before deploying to catch errors early:

```rust
use inferadb::schema::{Schema, ValidationResult};

// Parse and validate schema locally
let schema_content = include_str!("../schemas/v2.ipl");
let schema = Schema::parse(schema_content)?;

// Check for syntax errors
if let Err(errors) = schema.validate() {
    for error in errors {
        eprintln!("Line {}: {}", error.line, error.message);
    }
    return Err("Schema validation failed".into());
}

// Check compatibility with current production schema
let current = client
    .control()
    .schemas("org_id", "vault_id")
    .current()
    .await?;

let compatibility = schema.check_compatibility(&current)?;

match compatibility {
    Compatibility::FullyCompatible => {
        println!("✅ Schema is fully backwards compatible");
    }
    Compatibility::BackwardsCompatible { warnings } => {
        println!("⚠️ Schema is backwards compatible with warnings:");
        for warning in warnings {
            println!("  - {}", warning);
        }
    }
    Compatibility::Breaking { changes } => {
        println!("❌ Schema has breaking changes:");
        for change in changes {
            println!("  - {}: {}", change.kind, change.description);
        }
        return Err("Breaking changes detected".into());
    }
}

// Safe to deploy
client
    .control()
    .schemas("org_id", "vault_id")
    .deploy(schema_content)
    .await?;
```

#### Breaking Change Detection

```rust
use inferadb::schema::BreakingChangeKind;

let changes = schema.detect_breaking_changes(&current)?;

for change in changes {
    match change.kind {
        BreakingChangeKind::RemovedEntity { name } => {
            println!("Entity '{}' was removed", name);
        }
        BreakingChangeKind::RemovedRelation { entity, relation } => {
            println!("Relation '{}.{}' was removed", entity, relation);
        }
        BreakingChangeKind::RemovedPermission { entity, permission } => {
            println!("Permission '{}.{}' was removed", entity, permission);
        }
        BreakingChangeKind::ChangedRelationType { entity, relation, from, to } => {
            println!(
                "Relation '{}.{}' type changed from '{}' to '{}'",
                entity, relation, from, to
            );
        }
        BreakingChangeKind::RemovedAttribute { entity, attribute } => {
            println!("Attribute '{}.{}' was removed", entity, attribute);
        }
    }
}
```

#### Dry-Run Deployment

```rust
// Validate deployment without actually activating
let result = client
    .control()
    .schemas("org_id", "vault_id")
    .deploy(schema_content)
    .dry_run(true)  // Validate only
    .await?;

println!("Validation result: {:?}", result.validation);
println!("Would create version: {}", result.version);
println!("Compatibility: {:?}", result.compatibility);

// If satisfied, deploy for real
if result.validation.is_ok() {
    client
        .control()
        .schemas("org_id", "vault_id")
        .deploy(schema_content)
        .await?;
}
```

### Schema Evolution During Rolling Deployments

Handling schema changes safely during rolling deployments when old and new SDK versions run simultaneously.

#### The Version Mismatch Problem

During a rolling deployment:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  Rolling Deployment Timeline                                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  t0: All instances on SDK v1 + Schema v1                            │
│      ├── Instance A (v1) ──────────────────────────────────────►    │
│      ├── Instance B (v1) ──────────────────────────────────────►    │
│      └── Instance C (v1) ──────────────────────────────────────►    │
│                                                                     │
│  t1: Schema v2 deployed, SDK v2 rolling out                         │
│      ├── Instance A (v2) ──────────────────────────────────────►    │
│      ├── Instance B (v1) ◄── Still using old types!                 │
│      └── Instance C (v1) ◄── Still using old types!                 │
│                                                                     │
│  t2: All instances upgraded                                         │
│      ├── Instance A (v2) ──────────────────────────────────────►    │
│      ├── Instance B (v2) ──────────────────────────────────────►    │
│      └── Instance C (v2) ──────────────────────────────────────►    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

#### Safe Schema Evolution Strategies

##### Strategy 1: Additive-Only Changes

```rust
// Schema v1
entity Document {
    relations {
        owner: User
        viewer: User
    }
}

// Schema v2 - Only additions, no removals
entity Document {
    relations {
        owner: User
        viewer: User
        editor: User      // ✅ New relation - old SDK ignores it
        commenter: User   // ✅ New relation - old SDK ignores it
    }
}
```

##### Strategy 2: Schema Version Pinning

```rust
// Pin SDK to specific schema version during deployment
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Only accept responses compatible with this schema version
    .schema_version("v1.2.3")
    .build()
    .await?;

// Server will reject requests if schema has incompatible changes
```

##### Strategy 3: Graceful Degradation for Unknown Types

```rust
use inferadb::UnknownRelationPolicy;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // How to handle relations not in compile-time types
    .unknown_relation_policy(UnknownRelationPolicy::Warn)  // Log but continue
    // .unknown_relation_policy(UnknownRelationPolicy::Error)  // Fail fast
    // .unknown_relation_policy(UnknownRelationPolicy::Ignore) // Silent skip
    .build()
    .await?;
```

#### Multi-Version Type Support

Define types that work across schema versions:

```rust
use inferadb::prelude::*;

// Version-aware permission enum
#[derive(Permission)]
#[inferadb(entity = "document")]
enum DocumentPermission {
    View,
    Edit,
    Delete,

    // Added in schema v2 - gracefully ignored on v1
    #[inferadb(since = "2.0", fallback = "edit")]
    Comment,

    // Deprecated in v2 - maps to new permission
    #[inferadb(deprecated = "2.0", maps_to = "edit")]
    Modify,
}
```

#### Deployment Checklist for Schema Changes

```rust
use inferadb::schema::MigrationChecker;

async fn safe_schema_deploy(
    client: &Client,
    new_schema: &str,
) -> Result<(), Error> {
    let checker = MigrationChecker::new(client);

    // 1. Validate new schema
    let validation = checker.validate(new_schema).await?;
    if !validation.is_valid() {
        return Err(Error::SchemaInvalid(validation.errors));
    }

    // 2. Check for breaking changes
    let breaking = checker.detect_breaking_changes(new_schema).await?;
    if !breaking.is_empty() {
        tracing::error!("Breaking changes detected:");
        for change in &breaking {
            tracing::error!("  - {}", change);
        }
        return Err(Error::BreakingChanges(breaking));
    }

    // 3. Verify all running SDK versions are compatible
    let compatibility = checker.check_sdk_compatibility(new_schema).await?;
    if !compatibility.all_compatible {
        tracing::warn!(
            "SDK versions {} may have issues with new schema",
            compatibility.incompatible_versions.join(", ")
        );
    }

    // 4. Deploy with canary
    let result = client
        .control()
        .schemas("org_id", "vault_id")
        .deploy(new_schema)
        .canary_percentage(10)  // Only 10% of traffic initially
        .await?;

    // 5. Monitor for errors
    tokio::time::sleep(Duration::from_secs(300)).await;  // 5 min observation

    let errors = client
        .control()
        .schemas("org_id", "vault_id")
        .canary_metrics(result.version)
        .await?;

    if errors.error_rate > 0.01 {  // >1% error rate
        tracing::error!("Canary failed, rolling back");
        client
            .control()
            .schemas("org_id", "vault_id")
            .rollback()
            .await?;
        return Err(Error::CanaryFailed(errors));
    }

    // 6. Promote to 100%
    client
        .control()
        .schemas("org_id", "vault_id")
        .activate(result.version)
        .await?;

    Ok(())
}
```

#### Runtime Schema Refresh

For long-running services, refresh schema types periodically:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Refresh schema metadata every 5 minutes
    .schema_refresh_interval(Duration::from_secs(300))
    // Callback when schema changes detected
    .on_schema_change(|old_version, new_version| {
        tracing::info!(
            "Schema changed from {} to {}",
            old_version, new_version
        );
        // Optionally trigger application reload
    })
    .build()
    .await?;
```

---

## Audit Logs

```rust
use inferadb::AuditFilter;
use chrono::{Utc, Duration};

// List recent audit logs
let logs = client
    .control()
    .audit_logs("org_id")
    .list()
    .await?;

for log in logs {
    println!(
        "[{}] {} performed {} on {}",
        log.timestamp, log.actor, log.action, log.resource
    );
}

// Filter audit logs
let filtered = client
    .control()
    .audit_logs("org_id")
    .filter(AuditFilter::new()
        .actor("user:alice")
        .action("create")
        .resource_type("vault")
        .since(Utc::now() - Duration::days(7)))
    .await?;
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 5: DEVELOPER EXPERIENCE
     ═══════════════════════════════════════════════════════════════════════════ -->

## Error Handling

> **TL;DR**: All errors are in `Error` enum. Use `.is_retryable()` for retry decisions, `.kind()` for matching.

### Error Types

```rust
use inferadb::{Error, Result};

fn example() -> Result<()> {
    let result = client.check("user:alice", "view", "doc:readme").await;

    match result {
        Ok(allowed) => {
            println!("Decision: {}", allowed);
        }
        Err(Error::Unauthorized) => {
            // Token expired or invalid
            // SDK should auto-refresh, but handle edge cases
        }
        Err(Error::Forbidden { message }) => {
            // Insufficient permissions for this operation
            println!("Access denied: {}", message);
        }
        Err(Error::NotFound { resource }) => {
            // Resource doesn't exist
            println!("Not found: {}", resource);
        }
        Err(Error::RevisionMismatch { expected, actual }) => {
            // Optimistic concurrency conflict
            println!("Conflict: expected {}, got {}", expected, actual);
        }
        Err(Error::RateLimited { retry_after }) => {
            // Rate limited, wait and retry
            tokio::time::sleep(retry_after).await;
        }
        Err(Error::ValidationError { field, message }) => {
            // Invalid request
            println!("Invalid {}: {}", field, message);
        }
        Err(Error::Network(e)) => {
            // Network error
            println!("Network error: {}", e);
        }
        Err(Error::Internal(e)) => {
            // Server error
            println!("Server error: {}", e);
        }
    }

    Ok(())
}
```

### Error Context

```rust
use inferadb::Error;

// Errors include context for debugging
let err = client.check("bad:subject", "view", "doc:1").await.unwrap_err();

println!("Error: {}", err);
println!("Request ID: {:?}", err.request_id());
println!("Retry safe: {}", err.is_retryable());

// Display shows user-friendly message
// Debug shows full context including request ID
```

---

## Retry & Resilience

### Automatic Retries

The SDK automatically retries on transient failures:

```rust
// Default retry behavior
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .retry(RetryConfig::default())  // 3 attempts, exponential backoff
    .build()
    .await?;

// Custom retry configuration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .retry(RetryConfig {
        max_attempts: 5,
        initial_backoff: Duration::from_millis(50),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        retryable_status_codes: vec![429, 500, 502, 503, 504],
        retryable_errors: vec![
            RetryableError::ConnectionReset,
            RetryableError::Timeout,
        ],
    })
    .build()
    .await?;

// Disable retries for specific operation
let result = client
    .check("user:alice", "view", "doc:readme")
    .no_retry()
    .await?;
```

### Circuit Breaker

```rust
use inferadb::CircuitBreakerConfig;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .circuit_breaker(CircuitBreakerConfig {
        failure_threshold: 5,      // Open after 5 failures
        success_threshold: 3,      // Close after 3 successes
        half_open_timeout: Duration::from_secs(30),
    })
    .build()
    .await?;
```

---

## Graceful Degradation

Handle InferaDB unavailability gracefully with configurable fallback strategies.

### Fail-Open vs Fail-Closed

```rust
use inferadb::fallback::{FallbackPolicy, FallbackDecision};

// Fail-closed (default): Deny access when InferaDB is unavailable
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .fallback(FallbackPolicy::Deny)  // Default behavior
    .build()
    .await?;

// Fail-open: Allow access when InferaDB is unavailable (use with caution!)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .fallback(FallbackPolicy::Allow)
    .build()
    .await?;
```

### Custom Fallback Logic

```rust
use inferadb::fallback::{FallbackPolicy, FallbackContext};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .fallback(FallbackPolicy::Custom(|ctx: FallbackContext| {
        // Allow read operations, deny writes when degraded
        match ctx.permission.as_str() {
            "view" | "read" | "list" => FallbackDecision::Allow,
            _ => FallbackDecision::Deny,
        }
    }))
    .build()
    .await?;
```

### Cached Fallback

Use cached decisions as fallback when server is unavailable:

```rust
use inferadb::fallback::CachedFallback;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .cache(CacheConfig { ttl: Duration::from_secs(300), .. })
    .fallback(CachedFallback::new()
        .max_age(Duration::from_secs(600))  // Use cache up to 10 min old
        .on_miss(FallbackPolicy::Deny))     // Deny if not in cache
    .build()
    .await?;
```

### Health Checks

Integrate with your service health checks:

```rust
// Check InferaDB health
let health = client.health().await?;

println!("Status: {:?}", health.status);  // Healthy, Degraded, Unhealthy
println!("Latency: {:?}", health.latency);
println!("Version: {}", health.version);

// Use in health endpoint
async fn health_check(client: &Client) -> impl IntoResponse {
    match client.health().await {
        Ok(h) if h.status == HealthStatus::Healthy => StatusCode::OK,
        Ok(h) if h.status == HealthStatus::Degraded => StatusCode::OK,  // Still operational
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
```

### Deadline Propagation

Propagate request deadlines through authorization checks to prevent cascading timeouts.

#### Basic Deadline

```rust
use std::time::Duration;

// Set deadline for this request
let allowed = client
    .check("user:alice", "view", "document:readme")
    .deadline(Duration::from_millis(100))  // Fail fast if > 100ms
    .await?;
```

#### Inheriting Deadlines from Incoming Requests

For service-to-service calls, inherit the caller's deadline:

```rust
// Axum handler with deadline propagation
async fn get_document(
    State(client): State<Client>,
    headers: HeaderMap,
    Path(doc_id): Path<String>,
) -> Result<Json<Document>, StatusCode> {
    // Extract deadline from gRPC-style header
    let deadline = headers
        .get("grpc-timeout")
        .and_then(|v| parse_grpc_timeout(v))
        .unwrap_or(Duration::from_secs(30));

    // Reserve time for our own processing
    let auth_deadline = deadline.saturating_sub(Duration::from_millis(50));

    let allowed = client
        .check(&current_user(), "view", &format!("document:{}", doc_id))
        .deadline(auth_deadline)
        .await
        .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?;

    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    // Remaining time for database query
    let db_deadline = deadline.saturating_sub(Duration::from_millis(100));
    let doc = db.fetch_with_timeout(doc_id, db_deadline).await?;

    Ok(Json(doc))
}
```

#### Tower/Tonic Deadline Integration

```rust
use tower::timeout::Timeout;
use tonic::Request;

// Extract deadline from tonic request
fn deadline_from_tonic<T>(req: &Request<T>) -> Option<Duration> {
    req.metadata()
        .get("grpc-timeout")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_grpc_timeout)
}

// Or use tonic's built-in deadline
fn deadline_from_request<T>(req: &Request<T>) -> Option<Duration> {
    req.extensions()
        .get::<tower::timeout::TimeoutLayer>()
        .map(|t| t.timeout())
}

// gRPC service implementation
#[tonic::async_trait]
impl MyService for MyServiceImpl {
    async fn get_resource(
        &self,
        request: Request<GetResourceRequest>,
    ) -> Result<Response<GetResourceResponse>, Status> {
        // Propagate deadline to authorization
        let deadline = deadline_from_tonic(&request)
            .unwrap_or(Duration::from_secs(30));

        let allowed = self.auth_client
            .check(&request.get_ref().user_id, "view", &request.get_ref().resource_id)
            .deadline(deadline.saturating_sub(Duration::from_millis(10)))
            .await
            .map_err(|e| Status::deadline_exceeded(e.to_string()))?;

        // ...
    }
}
```

#### Deadline Budget Tracking

Track deadline budget across multiple operations:

```rust
use inferadb::deadline::DeadlineBudget;

async fn complex_operation(client: &Client, user: &str) -> Result<(), Error> {
    // Start with 500ms budget
    let mut budget = DeadlineBudget::new(Duration::from_millis(500));

    // First check consumes some budget
    let allowed1 = client
        .check(user, "view", "resource:1")
        .deadline(budget.remaining())
        .await?;
    budget.record_elapsed();  // Updates remaining time

    // Second check uses remaining budget
    let allowed2 = client
        .check(user, "edit", "resource:1")
        .deadline(budget.remaining())
        .await?;
    budget.record_elapsed();

    // Check if we have time left for more work
    if budget.remaining() < Duration::from_millis(50) {
        return Err(Error::DeadlineExceeded {
            budget: Duration::from_millis(500),
            elapsed: budget.elapsed(),
        });
    }

    // Continue with remaining budget...
    Ok(())
}
```

#### Deadline Behavior

| Scenario | Behavior |
|----------|----------|
| Deadline expires during request | Returns `Error::DeadlineExceeded` |
| Deadline expires during retry | No more retries attempted |
| Zero deadline | Request executed without timeout |
| Negative remaining deadline | Immediate `Error::DeadlineExceeded` |

```rust
// Deadline affects retry behavior
let result = client
    .check("user:alice", "view", "doc:1")
    .deadline(Duration::from_millis(100))
    .retry(RetryConfig { max_attempts: 3, .. })  // Won't retry if deadline exceeded
    .await;

match result {
    Err(Error::DeadlineExceeded { elapsed, deadline }) => {
        tracing::warn!(
            "Authorization timed out after {:?} (deadline: {:?})",
            elapsed, deadline
        );
    }
    // ...
}
```

### Degradation Events

Subscribe to degradation events for monitoring:

```rust
client
    .on_degradation(|event| {
        match event {
            DegradationEvent::CircuitOpen { since, failures } => {
                tracing::warn!("Circuit breaker opened after {} failures", failures);
                metrics::counter!("inferadb.circuit_open").increment(1);
            }
            DegradationEvent::FallbackUsed { reason, decision } => {
                tracing::info!("Using fallback: {:?} -> {:?}", reason, decision);
            }
            DegradationEvent::Recovered { downtime } => {
                tracing::info!("InferaDB recovered after {:?}", downtime);
            }
        }
    })
    .build()
    .await?;
```

---

## Observability

### Tracing Integration

```rust
use tracing_subscriber;

// Enable tracing
tracing_subscriber::fmt::init();

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(true)
    .build()
    .await?;

// Operations emit spans:
// inferadb.check{subject="user:alice" resource="doc:readme" permission="view"}
// inferadb.http.request{method="POST" path="/access/v1/evaluation"}
```

### Metrics

```rust
use inferadb::metrics::Metrics;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .metrics(true)
    .build()
    .await?;

// Access metrics
let metrics = client.metrics();

println!("Total requests: {}", metrics.requests_total());
println!("Request latency p99: {:?}", metrics.latency_p99());
println!("Active connections: {}", metrics.active_connections());
println!("Token refreshes: {}", metrics.token_refreshes());

// Export to Prometheus
let prometheus_output = metrics.to_prometheus();
```

### Structured Logging

```rust
// SDK logs at appropriate levels:
// ERROR: Unrecoverable errors
// WARN:  Retries, token refresh failures
// INFO:  Connection established, major operations
// DEBUG: Request/response details
// TRACE: Wire-level details

// Configure log level
std::env::set_var("RUST_LOG", "inferadb=debug");
```

### OpenTelemetry Integration

For production observability pipelines, integrate with OpenTelemetry:

#### Setup

```toml
[dependencies]
inferadb = { version = "0.1", features = ["opentelemetry"] }
opentelemetry = "0.21"
opentelemetry-otlp = "0.14"
opentelemetry_sdk = "0.21"
tracing-opentelemetry = "0.22"
```

#### Configure OTLP Exporter

```rust
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace::Config, Resource};
use opentelemetry::KeyValue;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Initialize OpenTelemetry
fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(Config::default().with_resource(Resource::new(vec![
            KeyValue::new("service.name", "my-service"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])))
        .install_batch(runtime::Tokio)?;

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

// Create client with OpenTelemetry enabled
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(true)  // Emits spans to configured subscriber
    .build()
    .await?;
```

#### Span Attributes

The SDK emits rich span attributes for debugging:

```text
Span: inferadb.check
├── inferadb.subject = "user:alice"
├── inferadb.permission = "view"
├── inferadb.resource = "document:readme"
├── inferadb.vault_id = "vault_123"
├── inferadb.decision = true
├── inferadb.cached = false
├── inferadb.latency_ms = 2.5
└── inferadb.request_id = "req_abc123"

Span: inferadb.http.request
├── http.method = "POST"
├── http.url = "https://api.inferadb.com/access/v1/evaluation"
├── http.status_code = 200
├── http.request_content_length = 156
├── http.response_content_length = 42
└── net.peer.name = "api.inferadb.com"
```

#### Metrics with OpenTelemetry

```rust
use opentelemetry::metrics::MeterProvider;
use opentelemetry_otlp::MetricsExporterBuilder;

// Initialize metrics
let meter_provider = opentelemetry_otlp::new_pipeline()
    .metrics(runtime::Tokio)
    .with_exporter(
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://localhost:4317"),
    )
    .build()?;

global::set_meter_provider(meter_provider);

// Client automatically emits metrics
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .metrics(true)
    .build()
    .await?;

// Metrics emitted:
// - inferadb_requests_total{operation="check", status="success"}
// - inferadb_request_duration_seconds{operation="check", quantile="0.99"}
// - inferadb_connections_active{transport="grpc"}
// - inferadb_cache_hits_total
// - inferadb_cache_misses_total
// - inferadb_token_refreshes_total{status="success"}
```

#### Instrumentation Customization

Fine-tune which operations are instrumented and how.

##### Custom Span Names

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(TracingConfig {
        // Customize span names
        span_name_prefix: "authz",  // "authz.check" instead of "inferadb.check"
        // Include resource type in span name
        include_resource_type: true,  // "authz.check.document"
    })
    .build()
    .await?;
```

##### Filtering Instrumented Operations

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(TracingConfig {
        // Only trace slow operations
        min_duration_to_trace: Some(Duration::from_millis(10)),
        // Skip health checks
        operations_to_skip: vec!["health", "quota"],
        // Trace all cache misses
        trace_cache_misses: true,
        trace_cache_hits: false,  // Skip cache hits (too noisy)
    })
    .build()
    .await?;
```

##### Custom Span Attributes

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(TracingConfig {
        // Add custom attributes to all spans
        static_attributes: vec![
            ("deployment.environment", "production"),
            ("deployment.region", "us-east-1"),
        ],
        // Dynamic attributes computed per-request
        dynamic_attributes: Some(Arc::new(|op| {
            vec![
                ("request.timestamp", Utc::now().to_rfc3339()),
            ]
        })),
    })
    .build()
    .await?;
```

##### Per-Request Instrumentation Override

```rust
// Disable tracing for this specific request (e.g., in tests)
let allowed = client
    .check("user:alice", "view", "doc:1")
    .without_tracing()
    .await?;

// Add extra attributes to this request's span
let allowed = client
    .check("user:alice", "view", "doc:1")
    .with_span_attributes([
        ("audit.reason", "user_request"),
        ("audit.ticket_id", "TICKET-123"),
    ])
    .await?;
```

##### Sampling Control

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .tracing(TracingConfig {
        // Sample 10% of successful requests, 100% of errors
        sampling: SamplingConfig {
            success_rate: 0.1,
            error_rate: 1.0,
            // Always trace slow requests
            slow_threshold: Duration::from_millis(100),
            slow_rate: 1.0,
        },
    })
    .build()
    .await?;
```

#### Context Propagation

Propagate trace context from incoming requests:

```rust
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

// Extract context from incoming HTTP headers
fn extract_context(headers: &HeaderMap) -> opentelemetry::Context {
    let propagator = TraceContextPropagator::new();
    let extractor = HeaderExtractor(headers);
    propagator.extract(&extractor)
}

// Use in your handler
async fn handle_request(
    headers: HeaderMap,
    client: &Client,
) -> Result<Response, Error> {
    // Extract parent context
    let parent_ctx = extract_context(&headers);

    // Create span with parent context
    let span = tracing::info_span!("handle_request");
    span.set_parent(parent_ctx);
    let _guard = span.enter();

    // SDK operations will be children of this span
    let allowed = client
        .check("user:alice", "view", "document:readme")
        .await?;

    Ok(Response::new(allowed))
}
```

#### Request ID Propagation

```rust
// Get request ID from SDK responses for correlation
let decision = client
    .check("user:alice", "view", "doc:1")
    .detailed()
    .await?;

if let Some(request_id) = decision.request_id() {
    tracing::info!(
        request_id = %request_id,
        "Authorization check completed"
    );
}

// Include in error responses for support
match result {
    Err(e) => {
        let request_id = e.request_id().unwrap_or("unknown");
        tracing::error!(
            request_id = %request_id,
            error = %e,
            "Authorization failed"
        );
        // Return request_id to client for support tickets
        Err(ApiError::new(e).with_request_id(request_id))
    }
}
```

#### Baggage Propagation

Pass contextual information through the authorization chain:

```rust
use opentelemetry::baggage::BaggageExt;

// Set baggage in your service
let cx = opentelemetry::Context::current()
    .with_baggage(vec![
        KeyValue::new("tenant.id", "acme-corp"),
        KeyValue::new("user.tier", "enterprise"),
    ]);

// SDK propagates baggage in outgoing requests
let _guard = cx.attach();
let allowed = client.check("user:alice", "view", "doc:1").await?;
```

### Rate Limit Visibility

Monitor your API usage and quotas:

```rust
// Get current rate limit status
let quota = client.quota().await?;

println!("Requests remaining: {}/{}", quota.remaining, quota.limit);
println!("Resets at: {:?}", quota.reset_at);
println!("Current usage: {}%", (1.0 - quota.remaining as f64 / quota.limit as f64) * 100.0);

// Rate limit info also available in responses
let decision = client
    .check("user:alice", "view", "doc:1")
    .detailed()
    .await?;

if let Some(rate_limit) = decision.rate_limit() {
    if rate_limit.remaining < 100 {
        tracing::warn!(
            remaining = rate_limit.remaining,
            "Approaching rate limit"
        );
    }
}

// Subscribe to rate limit warnings
client
    .on_rate_limit_warning(|warning| {
        metrics::gauge!("inferadb.rate_limit.remaining")
            .set(warning.remaining as f64);

        if warning.remaining < 50 {
            alert_ops_team("InferaDB rate limit critical");
        }
    })
    .build()
    .await?;
```

---

## Testing Support

> **TL;DR**: Use `MockClient` for unit tests, `TestVault` for integration tests, `AuthorizationClient` trait for testable code.

### Mock Client

```rust
use inferadb_test::{MockClient, MockBuilder};

#[tokio::test]
async fn test_authorization() {
    let mock = MockClient::builder()
        // Mock check responses
        .check("user:alice", "view", "doc:readme", true)
        .check("user:alice", "edit", "doc:readme", false)
        // Mock with patterns
        .check_pattern("user:alice", "view", "doc:*", true)
        // Mock errors
        .check_error("user:banned", "*", "*", Error::Forbidden {
            message: "User banned".into()
        })
        .build();

    assert!(mock.check("user:alice", "view", "doc:readme").await?);
    assert!(!mock.check("user:alice", "edit", "doc:readme").await?);
}
```

### Test Fixtures

```rust
use inferadb_test::{fixtures, TestVault};

#[tokio::test]
async fn test_with_fixture() {
    // Create isolated test vault
    let vault = TestVault::create(&client).await?;

    // Seed with relationships
    vault.seed(fixtures::document_hierarchy()).await?;

    // Run tests
    let allowed = vault.check("user:alice", "view", "doc:readme").await?;

    // Cleanup happens automatically on drop
}
```

### Property-Based Testing

```rust
use inferadb_test::proptest_strategies::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn check_is_deterministic(
        subject in subject_strategy(),
        permission in permission_strategy(),
        resource in resource_strategy(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result1 = client.check(&subject, &permission, &resource).await;
            let result2 = client.check(&subject, &permission, &resource).await;
            assert_eq!(result1, result2);
        });
    }
}
```

### Testable Code with Trait Abstraction

Write application code that works with both production and mock clients using the `AuthorizationClient` trait:

```rust
use inferadb::traits::AuthorizationClient;
use async_trait::async_trait;

// Your application code depends on the trait, not concrete Client
pub struct DocumentService<C: AuthorizationClient> {
    auth: C,
    db: Database,
}

impl<C: AuthorizationClient> DocumentService<C> {
    pub fn new(auth: C, db: Database) -> Self {
        Self { auth, db }
    }

    pub async fn get_document(&self, user: &str, doc_id: &str) -> Result<Document, Error> {
        // Works with both real Client and MockClient
        let allowed = self.auth.check(user, "view", &format!("document:{}", doc_id)).await?;
        if !allowed {
            return Err(Error::Forbidden);
        }
        self.db.fetch(doc_id).await
    }
}
```

#### The `AuthorizationClient` Trait

```rust
/// Core trait implemented by both Client and MockClient
#[async_trait]
pub trait AuthorizationClient: Send + Sync + Clone {
    /// Check if subject has permission on resource
    async fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error>;

    /// Check with ABAC context
    async fn check_with_context(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
        context: &Context,
    ) -> Result<bool, Error>;

    /// Batch check multiple permissions
    async fn check_batch<'a>(
        &self,
        checks: impl IntoIterator<Item = (&'a str, &'a str, &'a str)> + Send,
    ) -> Result<Vec<bool>, Error>;

    /// Write a relationship
    async fn write(&self, relationship: Relationship) -> Result<WriteResult, Error>;

    /// Delete a relationship
    async fn delete(&self, relationship: Relationship) -> Result<DeleteResult, Error>;
}

// Both Client and MockClient implement this trait
impl AuthorizationClient for Client { /* ... */ }
impl AuthorizationClient for MockClient { /* ... */ }
```

#### Testing with the Trait

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inferadb_test::MockClient;

    #[tokio::test]
    async fn test_get_document_authorized() {
        let mock = MockClient::builder()
            .check("user:alice", "view", "document:readme", true)
            .build();

        let db = MockDatabase::with_document("readme", Document::default());
        let service = DocumentService::new(mock, db);

        let doc = service.get_document("user:alice", "readme").await;
        assert!(doc.is_ok());
    }

    #[tokio::test]
    async fn test_get_document_forbidden() {
        let mock = MockClient::builder()
            .check("user:bob", "view", "document:secret", false)
            .build();

        let db = MockDatabase::with_document("secret", Document::default());
        let service = DocumentService::new(mock, db);

        let result = service.get_document("user:bob", "secret").await;
        assert!(matches!(result, Err(Error::Forbidden)));
    }
}
```

#### Generic Bounds for Flexibility

```rust
// Accept any authorization client
pub async fn protected_operation<C>(
    client: &C,
    user: &str,
) -> Result<(), Error>
where
    C: AuthorizationClient,
{
    client.check(user, "admin", "system:config").await?
        .then_some(())
        .ok_or(Error::Forbidden)
}

// Or use dynamic dispatch for runtime flexibility
pub async fn dynamic_operation(
    client: &dyn AuthorizationClient,
    user: &str,
) -> Result<(), Error> {
    // Same code works with any implementation
    client.check(user, "view", "resource:1").await?;
    Ok(())
}
```

---

## Sync API

For CLI tools, scripts, or blocking contexts that can't use async/await.

### Enable Blocking Feature

```toml
[dependencies]
inferadb = { version = "0.1", features = ["blocking"] }
```

### Blocking Client

```rust
use inferadb::blocking::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No async runtime needed
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials("client_id", "path/to/key.pem")
        .default_vault("vault_id")
        .build()?;  // No .await!

    // Synchronous check
    let allowed = client.check("user:alice", "view", "document:readme")?;

    println!("Access allowed: {}", allowed);
    Ok(())
}
```

### All Operations Available

```rust
use inferadb::blocking::Client;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()?;

// Authorization
let allowed = client.check("user:alice", "view", "doc:1")?;
let decision = client.check("user:alice", "edit", "doc:1").detailed()?;

// Relationships
client.write(Relationship::new("doc:1", "viewer", "user:alice"))?;
client.delete(Relationship::new("doc:1", "viewer", "user:alice"))?;

// Lookups (returns iterators instead of streams)
for resource in client.list_resources("user:alice", "view", "document")? {
    println!("Can view: {}", resource?);
}

// Control API
let orgs = client.control().organizations().list()?;
let vaults = client.control().vaults("org_id").list()?;
```

### Mixed Async/Sync Usage

```rust
// Get async client from sync client (shares connection pool)
let sync_client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .build()?;

// Use in async context
let async_client = sync_client.async_client();
let allowed = async_client.check("user:alice", "view", "doc:1").await?;

// Use sync client in spawn_blocking
let sync_clone = sync_client.clone();
let result = tokio::task::spawn_blocking(move || {
    sync_clone.check("user:bob", "view", "doc:2")
}).await??;
```

### Streaming in Sync Mode

Blocking streams return iterators:

```rust
// Returns impl Iterator<Item = Result<String, Error>>
let resources = client.list_resources("user:alice", "view", "document")?;

// Iterate synchronously
for resource in resources.take(100) {
    let resource = resource?;
    println!("Resource: {}", resource);
}

// Collect all (careful with large result sets)
let all: Vec<String> = client
    .list_resources("user:alice", "view", "document")?
    .collect::<Result<Vec<_>, _>>()?;
```

### When to Use Blocking

| Use Case                | Recommendation     |
| ----------------------- | ------------------ |
| CLI tools               | Blocking (simpler) |
| Short scripts           | Blocking           |
| Tests (simple)          | Blocking           |
| Web servers             | Async              |
| High concurrency        | Async              |
| Long-running services   | Async              |
| Within `spawn_blocking` | Blocking           |

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 7: COMMON PATTERNS & RECIPES
     ═══════════════════════════════════════════════════════════════════════════ -->

## Multi-Tenant SaaS

Pattern for applications serving multiple tenants with separate vaults.

### Per-Request Vault Selection

```rust
use axum::{extract::State, http::Request, middleware::Next, response::Response};

// Extract tenant from request
async fn tenant_middleware<B>(
    State(client): State<Client>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let tenant_id = request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    // Create tenant-scoped client
    let tenant_client = client.scoped_to_vault(tenant_vault_id(tenant_id));

    // Inject into request extensions
    request.extensions_mut().insert(tenant_client);

    next.run(request).await
}

// Use in handlers
async fn get_document(
    tenant_client: Extension<ScopedClient>,
    Path(doc_id): Path<String>,
) -> Result<Json<Document>, ApiError> {
    let allowed = tenant_client
        .check(&current_user(), "view", &format!("document:{}", doc_id))
        .await?;

    if !allowed {
        return Err(ApiError::Forbidden);
    }

    // Fetch and return document...
}
```

### Tenant Vault Mapping

```rust
// Cache tenant -> vault mapping
struct TenantVaultResolver {
    client: Client,
    cache: Cache<String, String>,
}

impl TenantVaultResolver {
    async fn resolve(&self, tenant_id: &str) -> Result<String, Error> {
        if let Some(vault_id) = self.cache.get(tenant_id) {
            return Ok(vault_id);
        }

        // Look up vault for tenant via Control API
        let org = self.client.control()
            .organizations()
            .get_by_slug(tenant_id)
            .await?;

        let vault = self.client.control()
            .vaults(&org.id)
            .get_default()
            .await?;

        self.cache.insert(tenant_id.to_string(), vault.id.clone());
        Ok(vault.id)
    }
}
```

---

## API Gateway Integration

Pattern for checking authorization on every API request.

### Axum Middleware

```rust
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

async fn authorization_middleware<B>(
    State(client): State<Client>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let user = extract_user(&request)?;
    let resource = path_to_resource(request.uri().path());
    let permission = method_to_permission(request.method());

    let allowed = client
        .check(&user, &permission, &resource)
        .with_context(Context::new()
            .insert("ip_address", extract_ip(&request))
            .insert("user_agent", extract_user_agent(&request)))
        .deadline(Duration::from_millis(50))  // Fail fast
        .await
        .unwrap_or(false);  // Deny on error

    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

// Helper functions
fn path_to_resource(path: &str) -> String {
    // /api/v1/documents/123 -> document:123
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["", "api", "v1", resource_type, id, ..] => {
            format!("{}:{}", resource_type.trim_end_matches('s'), id)
        }
        _ => format!("path:{}", path),
    }
}

fn method_to_permission(method: &Method) -> &'static str {
    match *method {
        Method::GET | Method::HEAD => "view",
        Method::POST => "create",
        Method::PUT | Method::PATCH => "edit",
        Method::DELETE => "delete",
        _ => "access",
    }
}
```

### Actix-Web Guard

```rust
use actix_web::{guard::Guard, http::Method, HttpRequest};

struct AuthorizationGuard {
    client: Client,
}

impl Guard for AuthorizationGuard {
    fn check(&self, req: &GuardContext<'_>) -> bool {
        let client = self.client.clone();
        let user = extract_user_sync(req);
        let resource = path_to_resource(req.head().uri.path());
        let permission = method_to_permission(req.head().method);

        // Run blocking check (guard is sync)
        client.blocking()
            .check(&user, &permission, &resource)
            .unwrap_or(false)
    }
}
```

---

## GraphQL & DataLoader

Efficient batch authorization for GraphQL resolvers.

### DataLoader Pattern

```rust
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;

struct AuthorizationLoader {
    client: Client,
    user: String,
}

#[async_trait]
impl Loader<AuthCheck> for AuthorizationLoader {
    type Value = bool;
    type Error = Error;

    async fn load(&self, checks: &[AuthCheck]) -> Result<HashMap<AuthCheck, bool>, Self::Error> {
        // Batch all checks in single request
        let results = self.client
            .check_batch(checks.iter().map(|c| (&self.user, &c.permission, &c.resource)))
            .collect()
            .await?;

        Ok(checks.iter()
            .zip(results.iter())
            .map(|(check, (_, decision))| (check.clone(), decision.allowed))
            .collect())
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct AuthCheck {
    permission: String,
    resource: String,
}

// Usage in resolver
#[Object]
impl Document {
    async fn can_edit(&self, ctx: &Context<'_>) -> Result<bool> {
        let loader = ctx.data::<DataLoader<AuthorizationLoader>>()?;
        loader.load_one(AuthCheck {
            permission: "edit".into(),
            resource: format!("document:{}", self.id),
        }).await.map(|r| r.unwrap_or(false))
    }
}
```

### Prefetching Related Permissions

```rust
#[Object]
impl Query {
    async fn documents(&self, ctx: &Context<'_>) -> Result<Vec<DocumentWithPermissions>> {
        let docs = fetch_documents().await?;
        let loader = ctx.data::<DataLoader<AuthorizationLoader>>()?;

        // Prefetch all permissions in parallel
        let checks: Vec<_> = docs.iter()
            .flat_map(|doc| ["view", "edit", "delete"].iter().map(move |p| AuthCheck {
                permission: p.to_string(),
                resource: format!("document:{}", doc.id),
            }))
            .collect();

        let permissions = loader.load_many(checks).await?;

        // Attach permissions to documents
        Ok(docs.into_iter().map(|doc| {
            DocumentWithPermissions {
                doc,
                can_view: permissions.get(&AuthCheck { permission: "view".into(), resource: format!("document:{}", doc.id) }).copied().unwrap_or(false),
                can_edit: permissions.get(&AuthCheck { permission: "edit".into(), resource: format!("document:{}", doc.id) }).copied().unwrap_or(false),
                can_delete: permissions.get(&AuthCheck { permission: "delete".into(), resource: format!("document:{}", doc.id) }).copied().unwrap_or(false),
            }
        }).collect())
    }
}
```

---

## Background Jobs

Pattern for long-running jobs that need authorization.

### Job with Embedded Credentials

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ExportJob {
    user_id: String,
    resource_ids: Vec<String>,
    // Store vault token for job execution
    vault_token: String,
}

async fn process_export_job(job: ExportJob) -> Result<(), JobError> {
    // Create client with job's token
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .bearer_token(&job.vault_token)
        .build()
        .await?;

    for resource_id in &job.resource_ids {
        // Re-check authorization (token may have been revoked)
        let allowed = client
            .check(&job.user_id, "export", resource_id)
            .await?;

        if !allowed {
            tracing::warn!("User {} no longer has export permission for {}", job.user_id, resource_id);
            continue;
        }

        export_resource(resource_id).await?;
    }

    Ok(())
}
```

### Service Account for Background Jobs

```rust
// Use service account with limited scopes
async fn setup_background_worker() -> Result<Client, Error> {
    Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(ClientCredentials {
            client_id: "background-worker".into(),
            private_key: Ed25519PrivateKey::from_env("WORKER_PRIVATE_KEY")?,
            certificate_id: None,
        })
        .default_vault("internal-vault")
        // Request only needed scopes
        .scopes([Scope::Check])  // Read-only
        .build()
        .await
}
```

---

## Audit Trail Enrichment

Pattern for enriching audit logs with authorization context.

### Middleware-Based Audit

```rust
use inferadb::middleware::{Middleware, Next, Request, Response};

struct AuditEnrichmentMiddleware {
    audit_service: Arc<AuditService>,
}

#[async_trait]
impl Middleware for AuditEnrichmentMiddleware {
    async fn handle(&self, request: Request, next: Next<'_>) -> Result<Response, Error> {
        let start = Instant::now();

        // Capture request details
        let subject = request.subject().map(|s| s.to_string());
        let resource = request.resource().map(|r| r.to_string());
        let permission = request.permission().map(|p| p.to_string());

        // Execute the request
        let response = next.run(request).await;

        // Log the decision
        if let (Some(subject), Some(resource), Some(permission)) = (subject, resource, permission) {
            let decision = response.as_ref()
                .ok()
                .and_then(|r| r.decision())
                .unwrap_or(false);

            self.audit_service.record(AuditEntry {
                timestamp: Utc::now(),
                subject,
                resource,
                permission,
                decision,
                latency: start.elapsed(),
                request_id: response.as_ref().ok().and_then(|r| r.request_id().map(|s| s.to_string())),
                error: response.as_ref().err().map(|e| e.to_string()),
            }).await;
        }

        response
    }
}
```

### Decision Reason Logging

```rust
async fn check_with_audit(
    client: &Client,
    subject: &str,
    permission: &str,
    resource: &str,
    audit: &AuditService,
) -> Result<bool, Error> {
    let decision = client
        .check(subject, permission, resource)
        .trace(true)  // Get detailed trace
        .detailed()
        .await?;

    // Log with full context
    audit.record(AuditEntry {
        subject: subject.to_string(),
        resource: resource.to_string(),
        permission: permission.to_string(),
        allowed: decision.allowed,
        reason: decision.reason.clone(),
        trace: decision.trace.as_ref().map(|t| {
            t.steps.iter()
                .map(|s| format!("{} -> {} ({})", s.relation, s.subject, s.result))
                .collect::<Vec<_>>()
                .join(" -> ")
        }),
    }).await;

    Ok(decision.allowed)
}
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 8: IMPLEMENTATION
     ═══════════════════════════════════════════════════════════════════════════ -->

## Type System

### Compile-Time Safety

The SDK uses Rust's type system to prevent common authorization mistakes at compile time.

#### `#[must_use]` on Authorization Results

All authorization checks return types marked with `#[must_use]`, preventing accidentally ignored results:

```rust
// ❌ COMPILE WARNING: unused result that must be used
client.check("user:alice", "view", "doc:1").await?;
proceed_with_access();  // BUG: didn't check the result!

// Warning: unused `bool` that must be used
// Note: authorization decisions should always be checked
```

```rust
// ✅ CORRECT: Result is checked
let allowed = client.check("user:alice", "view", "doc:1").await?;
if allowed {
    proceed_with_access();
}
```

#### The `Authorized<T>` Wrapper Pattern

For stronger guarantees, wrap protected data in an `Authorized<T>` type:

```rust
use inferadb::Authorized;

/// Data that can only be accessed after authorization
pub struct Authorized<T> {
    inner: T,
    decision: Decision,
}

impl<T> Authorized<T> {
    /// Only constructible via authorization check
    pub(crate) fn new(inner: T, decision: Decision) -> Self {
        debug_assert!(decision.allowed);
        Self { inner, decision }
    }

    /// Access the protected data
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Get the authorization decision
    pub fn decision(&self) -> &Decision {
        &self.decision
    }
}

// Usage in your code:
async fn get_document(
    client: &Client,
    user: &str,
    doc_id: &str,
) -> Result<Authorized<Document>, Error> {
    let doc = fetch_document(doc_id).await?;

    // Returns Authorized<Document> only if allowed
    client
        .authorize(user, "view", &format!("document:{}", doc_id))
        .protect(doc)
        .await
}

// Caller MUST have Authorized<T> to proceed
async fn render_document(doc: Authorized<Document>) -> Html {
    let document = doc.into_inner();  // Guaranteed authorized
    render(document)
}
```

#### Deny-By-Default Helpers

```rust
use inferadb::AuthorizedResult;

// Extension trait for cleaner error handling
impl Client {
    /// Check and return Forbidden error if denied
    pub async fn require(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<(), Error> {
        if !self.check(subject, permission, resource).await? {
            return Err(Error::Forbidden {
                message: format!(
                    "{} does not have {} on {}",
                    subject, permission, resource
                ),
            });
        }
        Ok(())
    }
}

// Usage - fails fast if not authorized
async fn delete_document(client: &Client, user: &str, doc_id: &str) -> Result<(), Error> {
    client.require(user, "delete", &format!("document:{}", doc_id)).await?;

    // Only reached if authorized
    actually_delete_document(doc_id).await
}
```

#### Type-State Pattern for Operations

Prevent invalid operation sequences at compile time:

```rust
use inferadb::builder::{Unchecked, Checked};

pub struct Operation<State> {
    subject: String,
    permission: String,
    resource: String,
    _state: PhantomData<State>,
}

impl Operation<Unchecked> {
    pub async fn check(self, client: &Client) -> Result<Operation<Checked>, Error> {
        let allowed = client.check(&self.subject, &self.permission, &self.resource).await?;
        if !allowed {
            return Err(Error::Forbidden { ... });
        }
        Ok(Operation {
            subject: self.subject,
            permission: self.permission,
            resource: self.resource,
            _state: PhantomData,
        })
    }
}

impl Operation<Checked> {
    /// Only available after authorization check
    pub async fn execute(self) -> Result<(), Error> {
        // Safe to proceed
        perform_operation().await
    }
}

// Usage:
let op = Operation::new("user:alice", "delete", "doc:1");
// op.execute().await;  // ❌ COMPILE ERROR: no method `execute` on Unchecked

let checked = op.check(&client).await?;
checked.execute().await?;  // ✅ Only compiles after check
```

### Core Types

```rust
/// A subject in the authorization model
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subject {
    pub type_name: String,
    pub id: String,
    pub relation: Option<String>,  // For usersets like group:eng#member
}

impl Subject {
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            type_name: "user".into(),
            id: id.into(),
            relation: None,
        }
    }

    pub fn group_member(group_id: impl Into<String>) -> Self {
        Self {
            type_name: "group".into(),
            id: group_id.into(),
            relation: Some("member".into()),
        }
    }
}

// Parse from string
let subject: Subject = "user:alice".parse()?;
let userset: Subject = "group:eng#member".parse()?;

// Display as string
assert_eq!(subject.to_string(), "user:alice");
assert_eq!(userset.to_string(), "group:eng#member");
```

### Resource Type

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource {
    pub type_name: String,
    pub id: String,
}

impl Resource {
    pub fn new(type_name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            id: id.into(),
        }
    }
}

// Parse from string
let resource: Resource = "document:readme".parse()?;

// Display as string
assert_eq!(resource.to_string(), "document:readme");
```

### Relationship Type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub resource: Resource,
    pub relation: String,
    pub subject: Subject,
}

impl Relationship {
    pub fn new(
        resource: impl Into<Resource>,
        relation: impl Into<String>,
        subject: impl Into<Subject>,
    ) -> Self {
        Self {
            resource: resource.into(),
            relation: relation.into(),
            subject: subject.into(),
        }
    }
}

// Convenient construction
let rel = Relationship::new("document:readme", "viewer", "user:alice");

// Or from tuple syntax
let rel: Relationship = ("document:readme", "viewer", "user:alice").into();
```

### Decision Type

```rust
#[derive(Debug, Clone)]
pub struct Decision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub trace: Option<DecisionTrace>,
    pub evaluated_at: DateTime<Utc>,
    pub latency: Duration,
}

#[derive(Debug, Clone)]
pub struct DecisionTrace {
    pub steps: Vec<TraceStep>,
    pub relationships_evaluated: usize,
}

#[derive(Debug, Clone)]
pub struct TraceStep {
    pub resource: String,
    pub relation: String,
    pub subject: String,
    pub result: bool,
    pub via: Option<String>,  // If through another relation
}
```

---

## Protocol Support

> **TL;DR**: Use defaults (gRPC + REST) unless you have specific constraints. gRPC is faster, REST works everywhere.

### Protocol Decision Tree

```text
Which protocol should I use?
│
├─► Are you in a browser/WASM environment?
│   └─► YES: Use REST only (gRPC not supported in browsers)
│
├─► Does your firewall block HTTP/2 or gRPC?
│   └─► YES: Use REST only
│
├─► Do you need real-time watch streams?
│   ├─► YES, with backpressure: Use gRPC
│   └─► YES, simple updates: Either works (SSE for REST)
│
├─► Are you optimizing for binary size?
│   └─► YES: Use REST only (~4MB saved)
│
├─► Do you need absolute minimum latency?
│   └─► YES: Use gRPC (persistent HTTP/2 connections)
│
└─► Otherwise: Use defaults (gRPC + REST fallback)
```

### Feature Flags

```toml
# Cargo.toml
[dependencies]
inferadb = { version = "0.1", features = ["grpc", "rest"] }

# Or specific protocol only
inferadb = { version = "0.1", default-features = false, features = ["grpc"] }
inferadb = { version = "0.1", default-features = false, features = ["rest"] }
```

### Protocol Selection

```rust
// Prefer gRPC, fall back to REST
let client = Client::builder()
    .url("https://api.inferadb.com")
    .prefer_grpc()  // Default
    .build()
    .await?;

// gRPC only (fails if unavailable)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .grpc_only()
    .build()
    .await?;

// REST only (useful for restricted networks)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .rest_only()
    .build()
    .await?;
```

### Streaming Behavior

| Operation              | gRPC                       | REST                    |
| ---------------------- | -------------------------- | ----------------------- |
| `check()`              | Unary                      | POST                    |
| `check_batch()`        | Bidirectional stream       | SSE stream              |
| `list_resources()`     | Server stream              | SSE stream              |
| `list_subjects()`      | Server stream              | SSE stream              |
| `list_relationships()` | Server stream              | SSE stream              |
| `expand()`             | Server stream              | SSE stream              |
| `watch()`              | Server stream (continuous) | SSE stream (continuous) |
| `write_batch()`        | Client stream              | POST                    |
| `delete_batch()`       | Client stream              | POST                    |

---

## Feature Flags

```toml
[features]
default = ["grpc", "rest", "rustls"]

# Protocol support
grpc = ["tonic", "prost"]
rest = ["reqwest"]

# TLS backends
rustls = ["reqwest/rustls-tls", "tonic/tls-roots"]
native-tls = ["reqwest/native-tls", "tonic/tls"]

# Optional features
tracing = ["tracing", "tracing-futures"]
metrics = ["metrics", "metrics-exporter-prometheus"]
test-utils = ["inferadb-test"]
opentelemetry = ["opentelemetry", "tracing-opentelemetry"]
blocking = ["tokio/rt"]

# Derive macros
derive = ["inferadb-macros"]

# Serialization
serde = ["dep:serde", "chrono/serde", "uuid/serde"]
```

### Minimal Build

```toml
# REST only, minimal dependencies
inferadb = {
    version = "0.1",
    default-features = false,
    features = ["rest", "rustls"]
}
```

### Serde Serialization

Enable `serde` support for caching decisions externally or logging:

```toml
[dependencies]
inferadb = { version = "0.1", features = ["serde"] }
```

```rust
use serde::{Serialize, Deserialize};

// All public types become serializable
let decision = client
    .check("user:alice", "view", "doc:1")
    .detailed()
    .await?;

// Serialize for external caching
let json = serde_json::to_string(&decision)?;
redis.set(&cache_key, &json).await?;

// Deserialize from cache
let cached: Decision = serde_json::from_str(&cached_json)?;

// Log decisions as structured JSON
tracing::info!(
    decision = %serde_json::to_string(&decision)?,
    "Authorization decision"
);
```

#### Serializable Types

With the `serde` feature enabled:

| Type           | Serialize | Deserialize | Notes                    |
| -------------- | --------- | ----------- | ------------------------ |
| `Decision`     | ✅        | ✅          | Full decision with trace |
| `Relationship` | ✅        | ✅          | For import/export        |
| `Subject`      | ✅        | ✅          |                          |
| `Resource`     | ✅        | ✅          |                          |
| `Error`        | ✅        | ❌          | For structured logging   |
| `Client`       | ❌        | ❌          | Contains connections     |
| `WatchChange`  | ✅        | ✅          | For event sourcing       |

### Feature Interaction Matrix

Not all feature combinations are valid. Use this matrix to understand feature interactions:

#### Protocol Features

| Combination | Valid | Notes |
|-------------|-------|-------|
| `grpc` + `rest` | ✅ | Default - SDK prefers gRPC, falls back to REST |
| `grpc` only | ✅ | gRPC exclusive - fails if gRPC unavailable |
| `rest` only | ✅ | REST exclusive - smaller binary, broader compatibility |
| Neither | ❌ | **Compile error** - at least one protocol required |

#### TLS Features

| Combination | Valid | Notes |
|-------------|-------|-------|
| `rustls` | ✅ | Recommended - pure Rust, no system dependencies |
| `native-tls` | ✅ | Uses system TLS (OpenSSL/Schannel/SecureTransport) |
| `rustls` + `native-tls` | ⚠️ | Compiles but wasteful - `rustls` takes precedence |
| Neither | ⚠️ | Only works with `http://` URLs (insecure) |

#### Observability Features

| Combination | Valid | Notes |
|-------------|-------|-------|
| `tracing` | ✅ | Emits tracing spans to any subscriber |
| `metrics` | ✅ | Emits metrics to any metrics backend |
| `opentelemetry` | ✅ | Adds OTLP integration (implies `tracing`) |
| `tracing` + `opentelemetry` | ✅ | Full distributed tracing with OTLP export |
| `metrics` + `opentelemetry` | ✅ | Metrics via OpenTelemetry |

#### Special Combinations

| Combination | Valid | Notes |
|-------------|-------|-------|
| `blocking` + `grpc` | ✅ | Blocking client with gRPC transport |
| `blocking` alone | ✅ | Uses REST (simpler for blocking) |
| `wasm` + `grpc` | ❌ | **gRPC not supported in browsers** |
| `wasm` + `native-tls` | ❌ | **No system TLS in WASM** |
| `wasm` + `blocking` | ❌ | **No blocking in browsers** |
| `derive` + anything | ✅ | Macros work with all configurations |
| `test-utils` + anything | ✅ | Testing utilities always compatible |

#### Recommended Feature Sets

```toml
# Production web service (maximum performance)
inferadb = { version = "0.1", features = ["grpc", "rest", "rustls", "tracing", "metrics"] }

# Production with OpenTelemetry
inferadb = { version = "0.1", features = ["grpc", "rest", "rustls", "opentelemetry"] }

# CLI tool
inferadb = { version = "0.1", features = ["rest", "rustls", "blocking"] }

# Minimal REST-only (smallest binary)
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }

# Browser/WASM
inferadb = { version = "0.1", default-features = false, features = ["wasm"] }

# Type-safe with macros
inferadb = { version = "0.1", features = ["derive", "serde"] }

# Integration testing
inferadb = { version = "0.1", features = ["test-utils"] }
```

#### Feature Detection at Runtime

```rust
// Check which features are enabled
#[cfg(feature = "grpc")]
println!("gRPC support enabled");

#[cfg(feature = "rest")]
println!("REST support enabled");

// SDK also exposes this programmatically
let features = inferadb::enabled_features();
println!("Enabled: {:?}", features);  // ["grpc", "rest", "rustls", "tracing"]
```

---

## Safety Guarantees

### Panic Safety

The SDK is designed to never panic under normal operation.

#### Panic-Free Guarantees

| Operation | Panics? | Notes |
|-----------|---------|-------|
| All public methods | ❌ No | Return `Result<T, Error>` |
| Internal parsing | ❌ No | Invalid input → Error variant |
| OOM conditions | ⚠️ Maybe | Depends on global allocator |
| Arithmetic | ❌ No | Uses checked/saturating ops |
| Index access | ❌ No | Uses `.get()` not `[]` |
| `unwrap()`/`expect()` | ❌ No | Never used on fallible ops |

```rust
// All operations return Results - no hidden panics
let result = client.check("user:alice", "view", "doc:1").await;
// ^^ Returns Result<bool, Error>, never panics

// Even with invalid input
let result = client.check("", "", "").await;
// ^^ Returns Err(Error::InvalidInput), doesn't panic
```

#### Panic Hooks for Debugging

If you suspect a panic (shouldn't happen), enable panic hooks:

```rust
// In tests or debugging
std::panic::set_hook(Box::new(|info| {
    eprintln!("Unexpected panic in InferaDB SDK: {}", info);
    // Report to error tracking
}));
```

### Unsafe Code Usage

The SDK minimizes `unsafe` code and documents all usages.

#### Unsafe Audit

| Location | Reason | Safety Justification |
|----------|--------|----------------------|
| `tonic` (dep) | FFI for gRPC | Well-audited, widely used |
| `hyper` (dep) | Performance-critical HTTP | Well-audited, widely used |
| `ring` (dep) | Cryptographic operations | Audited crypto library |
| SDK itself | **None** | Pure safe Rust |

```rust
// You can verify no unsafe in SDK source
// cargo geiger (or similar tools)
```

#### Safety Features

```rust
// The SDK provides additional safety features
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Sanitize inputs before sending (defense in depth)
    .sanitize_inputs(true)
    // Validate responses match expected schema
    .validate_responses(true)
    .build()
    .await?;
```

### Concurrency Safety

All SDK types are designed for concurrent use.

| Type | `Send` | `Sync` | Notes |
|------|--------|--------|-------|
| `Client` | ✅ | ✅ | Safe to share across threads |
| `Error` | ✅ | ✅ | Can be sent between threads |
| `Decision` | ✅ | ✅ | Immutable after creation |
| `WatchStream` | ✅ | ❌ | Single consumer only |
| `MockClient` | ✅ | ✅ | Safe for parallel tests |

---

## Performance

### Request Coalescing

Automatically deduplicate identical in-flight requests:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    // Enable request coalescing
    .coalesce_requests(true)
    .build()
    .await?;

// These concurrent identical checks result in ONE network call
let (r1, r2, r3) = tokio::join!(
    client.check("user:alice", "view", "doc:1"),
    client.check("user:alice", "view", "doc:1"),  // Coalesced!
    client.check("user:alice", "view", "doc:1"),  // Coalesced!
);

// All three return the same result from single request
assert_eq!(r1?, r2?);
assert_eq!(r2?, r3?);
```

#### Coalescing Configuration

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .coalesce_requests(CoalesceConfig {
        // How long to wait for identical requests
        window: Duration::from_millis(5),
        // Maximum requests to coalesce into one
        max_batch_size: 100,
        // Only coalesce read operations (check, list)
        write_operations: false,
    })
    .build()
    .await?;
```

#### When Coalescing Helps

| Scenario | Benefit |
|----------|---------|
| GraphQL DataLoader | Multiple resolvers checking same permission |
| Middleware chains | Multiple middleware checking same auth |
| Parallel handlers | Same user hitting multiple endpoints |
| Cache stampede | Many requests on cache miss |

### Expected Latency

| Operation             | P50  | P99   | Notes                      |
| --------------------- | ---- | ----- | -------------------------- |
| `check()`             | 2ms  | 10ms  | Single authorization check |
| `check_batch(100)`    | 15ms | 50ms  | 100 checks in parallel     |
| `list_resources(100)` | 20ms | 80ms  | First 100 results          |
| `write()`             | 5ms  | 20ms  | Single relationship        |
| `write_batch(1000)`   | 50ms | 200ms | 1000 relationships         |
| Token refresh         | 50ms | 150ms | Background, non-blocking   |

_Measured against InferaDB cloud with sub-10ms network latency._

### Optimizing Check Latency

```rust
// 1. Use caching for repeated checks
let client = Client::builder()
    .cache(CacheConfig { ttl: Duration::from_secs(60), .. })
    .build()
    .await?;

// 2. Batch multiple checks
let results = client.check_batch([
    ("user:alice", "view", "doc:1"),
    ("user:alice", "edit", "doc:1"),
    ("user:alice", "delete", "doc:1"),
]).collect().await?;

// 3. Use gRPC for lowest latency
let client = Client::builder()
    .grpc_only()  // Skip REST fallback
    .build()
    .await?;

// 4. Set aggressive deadlines for fast-fail
let allowed = client
    .check("user:alice", "view", "doc:1")
    .deadline(Duration::from_millis(50))
    .await?;
```

### Connection Pool Tuning

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)

    // HTTP pool for REST
    .max_connections(100)           // Total connections
    .max_idle_per_host(10)          // Idle connections per host
    .idle_timeout(Duration::from_secs(90))

    // gRPC settings
    .grpc_concurrency_limit(100)    // Max concurrent streams
    .grpc_keep_alive(Duration::from_secs(60))

    .build()
    .await?;
```

### Memory Footprint

| Component           | Memory | Notes                       |
| ------------------- | ------ | --------------------------- |
| Base client         | ~2 MB  | Connection pools, TLS state |
| Per connection      | ~50 KB | HTTP/2 or gRPC stream       |
| Cache (10k entries) | ~5 MB  | Depends on key/value size   |
| Token cache         | ~1 KB  | Per vault token             |

### Benchmarking

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn check_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = rt.block_on(async {
        Client::builder()
            .url("http://localhost:8080")
            .build()
            .await
            .unwrap()
    });

    c.bench_function("check_single", |b| {
        b.to_async(&rt).iter(|| async {
            client.check("user:alice", "view", "doc:readme").await
        })
    });

    c.bench_function("check_batch_100", |b| {
        let checks: Vec<_> = (0..100)
            .map(|i| ("user:alice", "view", format!("doc:{}", i)))
            .collect();

        b.to_async(&rt).iter(|| async {
            client.check_batch(&checks).collect::<Vec<_>>().await
        })
    });
}

criterion_group!(benches, check_benchmark);
criterion_main!(benches);
```

### Compile Time Optimization

```toml
# Cargo.toml - Minimize compile time
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest"] }

# Profile for faster dev builds
[profile.dev]
opt-level = 0
debug = true

# Profile for benchmarks
[profile.bench]
opt-level = 3
lto = "thin"
```

---

## Release Strategy

### Version Compatibility

- SDK version tracks API version (e.g., SDK 1.x supports API v1)
- Patch versions for bug fixes
- Minor versions for new features (backward compatible)
- Major versions for breaking changes

### MSRV (Minimum Supported Rust Version)

- Target: Rust 1.70+ (for async traits stabilization path)
- Test on stable, beta, and nightly

### Platform Support

| Platform         | Support Level      |
| ---------------- | ------------------ |
| Linux (x86_64)   | Tier 1             |
| Linux (aarch64)  | Tier 1             |
| macOS (x86_64)   | Tier 1             |
| macOS (aarch64)  | Tier 1             |
| Windows (x86_64) | Tier 2             |
| WASM             | Tier 3 (REST only) |
| `no_std`         | Not supported      |

### `no_std` and Embedded Support

The SDK currently requires `std` and is not designed for `no_std` environments.

#### Why `no_std` Is Not Supported

| Dependency | Requires `std` | Reason |
|------------|----------------|--------|
| `tokio` | Yes | Async runtime with OS threads |
| `reqwest` | Yes | HTTP client with TLS |
| `tonic` | Yes | gRPC with HTTP/2 |
| `rustls` | Partial | Can be `no_std` but we use std features |
| `serde_json` | Yes (default) | Heap allocation for JSON parsing |

#### Embedded Use Cases

For embedded or constrained environments, consider these alternatives:

##### Option 1: Edge Proxy Pattern

Run authorization at the edge, not on the constrained device:

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Embedded       │     │  Edge Gateway   │     │   InferaDB      │
│  Device         │────►│  (Full SDK)     │────►│   Cloud         │
│  (no_std)       │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
     Sends simple           Performs              Returns
     request with           authorization         decision
     device ID              check
```

##### Option 2: Pre-computed Decisions

Cache authorization decisions on the device:

```rust
// On your backend with full SDK
let decisions = client
    .check_batch(all_device_permissions)
    .collect()
    .await?;

let device_policy = DevicePolicy {
    device_id: "device:123",
    decisions: decisions.into_iter().collect(),
    valid_until: Utc::now() + Duration::hours(24),
};

// Send to device (small payload, no SDK needed)
send_to_device(&device_policy).await?;
```

##### Option 3: Minimal Types Crate

For sharing types without the full SDK:

```toml
# Cargo.toml for embedded project
[dependencies]
inferadb-types = { version = "0.1", default-features = false }  # Core types only
```

```rust
// inferadb-types can be no_std compatible
#![no_std]
use inferadb_types::{Subject, Resource, Decision};

// Parse pre-computed decisions
let decision: Decision = postcard::from_bytes(&bytes)?;
if decision.allowed {
    allow_operation();
}
```

#### Future `no_std` Considerations

We may explore `no_std` support in the future for:

- **`inferadb-types`**: Core types without networking (feasible)
- **`inferadb-offline`**: Offline policy evaluation (planned)
- **Full client**: Unlikely due to networking requirements

If you need `no_std` support, please [open an issue](https://github.com/inferadb/rust-sdk/issues) describing your use case.

### WASM / Browser Support

The SDK supports WebAssembly for browser-based authorization checks with some limitations.

#### Installation for WASM

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["wasm"] }
```

#### Browser Usage

```rust
use inferadb::wasm::Client;
use wasm_bindgen_futures::spawn_local;

// Create client for browser
let client = Client::builder()
    .url("https://api.inferadb.com")
    .bearer_token(&get_token_from_auth_provider())
    .build()?;

// Check authorization (async in browser)
spawn_local(async move {
    let allowed = client.check("user:alice", "view", "document:readme").await?;
    if allowed {
        show_document();
    } else {
        show_access_denied();
    }
});
```

#### JavaScript/TypeScript Interop

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn check_permission(
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, JsValue> {
    let client = get_cached_client()?;
    client.check(subject, permission, resource)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

```typescript
// Usage from TypeScript
import init, { check_permission } from "inferadb-wasm";

await init();
const allowed = await check_permission("user:alice", "view", "document:readme");
```

#### WASM Limitations

| Feature            | WASM Support   | Notes                          |
| ------------------ | -------------- | ------------------------------ |
| REST API           | ✅ Full        | Via `fetch` API                |
| gRPC               | ❌ None        | No HTTP/2 in browsers          |
| Streaming (SSE)    | ✅ Full        | Via `EventSource`              |
| Watch              | ✅ Full        | SSE-based                      |
| Client credentials | ❌ None        | Private keys unsafe in browser |
| Bearer token       | ✅ Full        | Recommended for browser        |
| Caching            | ✅ Memory only | No Redis/external cache        |
| File system        | ❌ None        | No PEM file loading            |

#### Security Considerations for Browser

```rust
// NEVER embed private keys in browser code
// ❌ BAD: This exposes your private key
let client = Client::builder()
    .client_credentials("id", include_str!("key.pem"))  // NEVER!
    .build()?;

// ✅ GOOD: Use short-lived tokens from your backend
let client = Client::builder()
    .bearer_token(&token_from_backend_api)
    .build()?;
```

#### Bundle Size Optimization

```toml
# Minimize WASM bundle size
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
panic = "abort"      # No unwinding

[dependencies]
inferadb = {
    version = "0.1",
    default-features = false,
    features = ["wasm", "rustls-webpki"]  # Minimal TLS
}
```

Expected bundle sizes:

- Minimal (check only): ~150 KB gzipped
- Full SDK: ~300 KB gzipped

### Binary Size by Feature Set

Understanding binary size impact helps choose the right feature combination.

#### Native Binary Sizes (x86_64-unknown-linux-gnu, release)

| Feature Set | Binary Size | Stripped | Notes |
|-------------|-------------|----------|-------|
| Default (`grpc` + `rest` + `rustls`) | ~8.5 MB | ~5.2 MB | Full functionality |
| `grpc` + `rustls` only | ~6.8 MB | ~4.1 MB | No REST fallback |
| `rest` + `rustls` only | ~4.2 MB | ~2.5 MB | Smallest networked |
| + `tracing` | +200 KB | +120 KB | Minimal overhead |
| + `metrics` | +180 KB | +100 KB | Minimal overhead |
| + `opentelemetry` | +1.2 MB | +800 KB | OTLP adds weight |
| + `derive` (macros) | +0 KB | +0 KB | Compile-time only |
| + `serde` | +300 KB | +180 KB | Serialization |
| + `blocking` | +50 KB | +30 KB | Tokio runtime |

#### Optimization Strategies

```toml
# Cargo.toml - Optimize for size
[profile.release]
opt-level = "z"      # Optimize for size (vs "3" for speed)
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
panic = "abort"      # No unwinding (smaller binary)
strip = true         # Strip symbols

[profile.release.package.inferadb]
opt-level = "z"      # Ensure SDK is also size-optimized
```

#### Size Reduction Tips

```rust
// 1. Disable unused protocols
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }

// 2. Skip OpenTelemetry if using simpler tracing
inferadb = { version = "0.1", features = ["tracing"] }  // Not "opentelemetry"

// 3. For size-critical deployments, consider REST-only
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }
```

#### Compile Time Impact

| Feature Set | Clean Build | Incremental |
|-------------|-------------|-------------|
| Default | ~45s | ~3s |
| `rest` only | ~25s | ~2s |
| + `derive` | +5s | +0.5s |
| + `opentelemetry` | +15s | +1s |

### Backwards Compatibility

#### API Stability Guarantees

| API Surface                            | Stability   | Policy                                |
| -------------------------------------- | ----------- | ------------------------------------- |
| Public types (`Client`, `Error`, etc.) | Stable      | No breaking changes in minor versions |
| Builder methods                        | Stable      | New methods additive only             |
| Trait implementations                  | Stable      | Won't remove `Send`, `Sync`, `Clone`  |
| Feature flags                          | Semi-stable | New flags may be added                |
| Internal modules (`__internal`)        | Unstable    | May change without notice             |
| Error messages                         | Unstable    | Text may change                       |

#### Deprecation Policy

1. **Deprecation notice**: Feature marked with `#[deprecated]` for at least one minor version
2. **Migration guide**: Documentation provided for transitioning
3. **Removal**: Only in major version bumps
4. **Timeline**: Minimum 6 months between deprecation and removal

```rust
// Deprecated APIs emit compile-time warnings
#[deprecated(since = "0.3.0", note = "Use Client::builder() instead")]
pub fn new(url: &str) -> Result<Client, Error> { ... }

// Usage triggers warning:
let client = Client::new("https://api.inferadb.com")?;
// warning: use of deprecated function `Client::new`: Use Client::builder() instead
```

#### Version Support Matrix

| SDK Version | API Version | Rust Version | Support Status      |
| ----------- | ----------- | ------------ | ------------------- |
| 1.x         | v1          | 1.70+        | Active              |
| 0.x         | v1          | 1.65+        | Security fixes only |

### Release Checklist

1. Update `CHANGELOG.md`
2. Bump version in all `Cargo.toml` files
3. Run full test suite (`cargo nextest run --workspace`)
4. Run clippy (`cargo clippy --workspace --all-targets -- -D warnings`)
5. Run rustfmt (`cargo +nightly fmt --all`)
6. Build docs (`cargo doc --workspace --no-deps`)
7. Tag release
8. Publish to crates.io

### Changelog Format

We follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog

## [Unreleased]

## [0.3.0] - 2024-03-15

### Added
- `check_batch_v2()` with improved streaming performance
- Support for custom JWT claims (#123)
- `Client::from_env()` convenience constructor

### Changed
- **BREAKING**: `Error` now implements `std::error::Error` differently
- Default timeout increased from 10s to 30s
- Minimum Rust version bumped to 1.70

### Deprecated
- `Client::new()` - use `Client::builder()` instead

### Removed
- **BREAKING**: Removed `legacy_auth` feature flag

### Fixed
- Token refresh race condition under high concurrency (#456)
- Memory leak in long-running watch streams (#789)

### Security
- Updated `ring` dependency to fix CVE-2024-XXXX
```

#### Change Type Indicators

| Prefix | Meaning | Action Required |
|--------|---------|-----------------|
| None | Safe to upgrade | Review new features |
| **BREAKING** | API changed incompatibly | Code changes needed |
| (deprecated) | Will be removed | Plan migration |
| (experimental) | May change | Don't depend on in production |

#### Subscribing to Changes

```bash
# Watch releases on GitHub
gh repo watch inferadb/rust-sdk --events releases

# Or subscribe via RSS
# https://github.com/inferadb/rust-sdk/releases.atom
```

---

---

<!-- ═══════════════════════════════════════════════════════════════════════════
     PART 9: REFERENCE
     ═══════════════════════════════════════════════════════════════════════════ -->

## Error Recovery Patterns

Structured patterns for handling SDK errors gracefully.

### Error Classification

```rust
use inferadb::{Error, ErrorKind};

fn classify_error(err: &Error) -> ErrorAction {
    match err.kind() {
        // Retryable errors - automatic retry with backoff
        ErrorKind::Network | ErrorKind::Timeout | ErrorKind::ServiceUnavailable => {
            ErrorAction::Retry
        }

        // Rate limited - wait and retry
        ErrorKind::RateLimited => {
            let retry_after = err.retry_after().unwrap_or(Duration::from_secs(1));
            ErrorAction::RetryAfter(retry_after)
        }

        // Auth errors - may need re-authentication
        ErrorKind::Unauthorized => ErrorAction::Reauthenticate,

        // Client errors - don't retry, fix the request
        ErrorKind::InvalidInput | ErrorKind::NotFound | ErrorKind::Forbidden => {
            ErrorAction::Fail
        }

        // Server errors - log and maybe retry
        ErrorKind::Internal => ErrorAction::LogAndRetry,
    }
}

enum ErrorAction {
    Retry,
    RetryAfter(Duration),
    Reauthenticate,
    Fail,
    LogAndRetry,
}
```

### Custom Recovery Logic

```rust
use inferadb::Error;

async fn check_with_recovery(
    client: &Client,
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match client.check(subject, permission, resource).await {
            Ok(allowed) => return Ok(allowed),

            Err(e) if e.is_retryable() && attempts < max_attempts => {
                attempts += 1;
                let backoff = Duration::from_millis(100 * 2_u64.pow(attempts));

                tracing::warn!(
                    error = %e,
                    attempt = attempts,
                    backoff_ms = backoff.as_millis(),
                    "Retrying after error"
                );

                tokio::time::sleep(backoff).await;
            }

            Err(e) if e.kind() == ErrorKind::RateLimited => {
                let retry_after = e.retry_after().unwrap_or(Duration::from_secs(1));
                tracing::warn!(
                    retry_after_ms = retry_after.as_millis(),
                    "Rate limited, waiting"
                );
                tokio::time::sleep(retry_after).await;
                // Don't count against max_attempts for rate limits
            }

            Err(e) => return Err(e),
        }
    }
}
```

### Circuit Breaker Recovery

```rust
async fn handle_circuit_open(client: &Client, subject: &str, permission: &str, resource: &str) -> bool {
    match client.check(subject, permission, resource).await {
        Ok(allowed) => allowed,

        Err(e) if e.kind() == ErrorKind::CircuitOpen => {
            tracing::warn!("Circuit breaker open, using fallback");

            // Option 1: Fail closed (deny)
            // return false;

            // Option 2: Use cached decision if available
            if let Some(cached) = get_cached_decision(subject, permission, resource) {
                return cached;
            }

            // Option 3: Apply emergency policy
            emergency_policy(subject, permission, resource)
        }

        Err(e) => {
            tracing::error!(error = %e, "Authorization check failed");
            false  // Fail closed
        }
    }
}
```

### Graceful Degradation Patterns

```rust
/// Authorization with multiple fallback levels
async fn check_with_fallbacks(
    client: &Client,
    cache: &Cache,
    subject: &str,
    permission: &str,
    resource: &str,
) -> AuthDecision {
    // Level 1: Try real-time check
    match client.check(subject, permission, resource).await {
        Ok(allowed) => {
            cache.set(subject, permission, resource, allowed);
            return AuthDecision::Live(allowed);
        }
        Err(e) if !e.is_transient() => {
            // Permanent error - don't fallback
            return AuthDecision::Error(e);
        }
        Err(_) => { /* continue to fallbacks */ }
    }

    // Level 2: Use local cache
    if let Some(cached) = cache.get(subject, permission, resource) {
        tracing::info!("Using cached decision");
        return AuthDecision::Cached(cached);
    }

    // Level 3: Apply static policy
    let static_decision = static_policy(subject, permission, resource);
    tracing::warn!("Using static fallback policy");
    AuthDecision::Fallback(static_decision)
}

#[derive(Debug)]
enum AuthDecision {
    Live(bool),
    Cached(bool),
    Fallback(bool),
    Error(Error),
}
```

---

## Structured Concurrency

Best practices for using the SDK with Tokio's structured concurrency primitives.

### Using with JoinSet

```rust
use tokio::task::JoinSet;

async fn check_many_resources(
    client: &Client,
    user: &str,
    resources: Vec<String>,
) -> Result<HashMap<String, bool>, Error> {
    let mut set = JoinSet::new();

    for resource in resources {
        let client = client.clone();
        let user = user.to_string();
        let resource = resource.clone();

        set.spawn(async move {
            let allowed = client.check(&user, "view", &resource).await?;
            Ok::<_, Error>((resource, allowed))
        });
    }

    let mut results = HashMap::new();
    while let Some(result) = set.join_next().await {
        let (resource, allowed) = result??;
        results.insert(resource, allowed);
    }

    Ok(results)
}
```

### Cancellation Safety

The SDK is cancellation-safe - dropping a future mid-execution won't corrupt state:

```rust
use tokio::time::timeout;

// Safe to timeout/cancel
let result = timeout(
    Duration::from_millis(100),
    client.check("user:alice", "view", "doc:1")
).await;

match result {
    Ok(Ok(allowed)) => println!("Allowed: {}", allowed),
    Ok(Err(e)) => println!("Check error: {}", e),
    Err(_) => println!("Timed out - no corruption, safe to retry"),
}
```

### Select! Patterns

```rust
use tokio::select;

async fn check_with_cancellation(
    client: &Client,
    cancel: &mut tokio::sync::oneshot::Receiver<()>,
) -> Option<bool> {
    select! {
        result = client.check("user:alice", "view", "doc:1") => {
            Some(result.unwrap_or(false))
        }
        _ = cancel => {
            tracing::info!("Check cancelled");
            None
        }
    }
}
```

### Semaphore for Concurrency Limits

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

struct RateLimitedClient {
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl RateLimitedClient {
    pub fn new(client: Client, max_concurrent: usize) -> Self {
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error> {
        let _permit = self.semaphore.acquire().await?;
        self.client.check(subject, permission, resource).await
    }
}

// Limit to 10 concurrent authorization checks
let limited_client = RateLimitedClient::new(client, 10);
```

### Batch Processing with Buffering

```rust
use futures::StreamExt;
use tokio::sync::mpsc;

async fn process_batch_stream(
    client: Client,
    mut rx: mpsc::Receiver<(String, String, String)>,
) {
    // Buffer up to 100 checks before sending batch
    let mut buffer = Vec::with_capacity(100);

    loop {
        // Collect checks for up to 10ms or until buffer is full
        let deadline = tokio::time::sleep(Duration::from_millis(10));
        tokio::pin!(deadline);

        loop {
            select! {
                Some((subject, permission, resource)) = rx.recv() => {
                    buffer.push((subject, permission, resource));
                    if buffer.len() >= 100 {
                        break;
                    }
                }
                _ = &mut deadline => break,
                else => return,  // Channel closed
            }
        }

        if !buffer.is_empty() {
            // Send batch
            let checks: Vec<_> = buffer.iter()
                .map(|(s, p, r)| (s.as_str(), p.as_str(), r.as_str()))
                .collect();

            match client.check_batch(checks).collect().await {
                Ok(results) => {
                    for ((subject, permission, resource), allowed) in
                        buffer.drain(..).zip(results.iter())
                    {
                        tracing::debug!(
                            subject, permission, resource,
                            allowed = allowed.1.allowed,
                            "Batch check result"
                        );
                    }
                }
                Err(e) => tracing::error!(error = %e, "Batch check failed"),
            }
            buffer.clear();
        }
    }
}
```

---

## Troubleshooting

### Common Errors

#### `Error::Unauthorized`

**Symptom**: All requests fail with 401 Unauthorized.

**Causes & Solutions**:

```rust
// 1. Token expired and refresh failed
//    Check: Is refresh token still valid (30 day TTL)?
//    Solution: Re-authenticate or check credentials

// 2. Wrong certificate for client
//    Check: Does certificate_id match an active certificate?
let creds = ClientCredentials {
    client_id: "my-client",
    private_key: key,
    certificate_id: Some("correct-kid"),  // Verify this matches
};

// 3. Clock skew (JWT issued in the future)
//    Check: Is system clock synchronized?
//    Solution: Run `ntpdate` or check NTP settings
```

#### `Error::Forbidden`

**Symptom**: Authenticated but operations fail with 403.

**Causes & Solutions**:

```rust
// 1. Insufficient scopes
//    Check: Does token have required scopes?
let token_info = client.token_info().await?;
println!("Scopes: {:?}", token_info.scopes);
// Required scopes: check -> inferadb.check, write -> inferadb.write

// 2. Wrong vault
//    Check: Is the vault correct for this operation?
let result = client
    .with_vault("correct-vault-id")  // Override vault
    .check("user:alice", "view", "doc:1")
    .await?;

// 3. User not member of organization
//    Check: Is user in organization that owns the vault?
```

#### `Error::Network`

**Symptom**: Intermittent connection failures.

**Causes & Solutions**:

```rust
// 1. DNS resolution failing
//    Solution: Check DNS settings, try IP directly for testing

// 2. TLS handshake issues
//    Solution: Verify TLS configuration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .danger_accept_invalid_certs(true)  // DEV ONLY - diagnose TLS issues
    .build()
    .await?;

// 3. Connection pool exhausted
//    Solution: Increase pool size or check for leaks
let client = Client::builder()
    .max_connections(200)  // Increase from default 100
    .build()
    .await?;
```

#### `Error::RateLimited`

**Symptom**: Requests fail with 429 Too Many Requests.

**Solutions**:

```rust
// 1. Enable automatic retry with backoff
let client = Client::builder()
    .retry(RetryConfig {
        max_attempts: 5,
        retryable_status_codes: vec![429],
        ..Default::default()
    })
    .build()
    .await?;

// 2. Batch requests to reduce call count
let results = client.check_batch(checks).await?;  // 1 request vs N

// 3. Enable caching
let client = Client::builder()
    .cache(CacheConfig { ttl: Duration::from_secs(60), .. })
    .build()
    .await?;
```

### Debug Mode

```rust
// Enable verbose logging
std::env::set_var("RUST_LOG", "inferadb=trace");
tracing_subscriber::fmt::init();

// Or per-request debugging
let decision = client
    .check("user:alice", "view", "doc:1")
    .trace(true)
    .debug(true)  // Log request/response
    .detailed()
    .await?;

println!("Request ID: {:?}", decision.request_id);
println!("Trace: {:?}", decision.trace);
```

### Trace ID Correlation

Correlate client-side errors with server-side logs for debugging:

```rust
// All errors include request ID for server correlation
match client.check("user:alice", "view", "doc:1").await {
    Ok(allowed) => { /* ... */ }
    Err(e) => {
        // Get the request ID for correlation
        let request_id = e.request_id().unwrap_or("unknown");
        let trace_id = e.trace_id();  // OpenTelemetry trace ID if available

        tracing::error!(
            request_id = %request_id,
            trace_id = ?trace_id,
            error = %e,
            "Authorization check failed"
        );

        // Include in error response for support
        return Err(ApiError::new(e.to_string())
            .with_header("X-Request-ID", request_id)
            .with_header("X-Trace-ID", trace_id.unwrap_or_default()));
    }
}
```

#### Finding Server Logs

Use the request ID to find matching server logs:

```bash
# Search InferaDB server logs
kubectl logs -l app=inferadb-engine | grep "req_abc123"

# Or in your log aggregator
# Datadog: @request_id:req_abc123
# Elasticsearch: request_id:"req_abc123"
```

#### Error Context Chain

Errors preserve full context for debugging:

```rust
let err = client.check("user:alice", "view", "doc:1").await.unwrap_err();

// Display shows user-friendly message
println!("Error: {}", err);
// Output: "Authorization check failed: connection refused"

// Debug shows full context
println!("Debug: {:?}", err);
// Output: Error {
//     kind: Network(ConnectionRefused),
//     request_id: Some("req_abc123"),
//     trace_id: Some("4bf92f3577b34da6a3ce929d0e0e4736"),
//     operation: "check",
//     subject: "user:alice",
//     resource: "doc:1",
//     permission: "view",
//     latency: 50ms,
//     retry_count: 3,
//     cause: Some(hyper::Error(Connect, ...))
// }

// Structured logging with all context
tracing::error!(
    error.kind = %err.kind(),
    error.request_id = ?err.request_id(),
    error.trace_id = ?err.trace_id(),
    error.operation = %err.operation(),
    error.retryable = %err.is_retryable(),
    "Operation failed"
);
```

### Health Check

```rust
// Quick connectivity test
match client.health().await {
    Ok(health) => {
        println!("Status: {:?}", health.status);
        println!("Latency: {:?}", health.latency);
    }
    Err(e) => {
        println!("Health check failed: {}", e);
        println!("Request ID: {:?}", e.request_id());
    }
}
```

---

## Migration Guide

### From 0.1 to 0.2 (Example)

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

### From SpiceDB

```rust
// SpiceDB
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

// InferaDB - Much simpler!
let allowed = client.check("user:alice", "view", "document:readme").await?;
```

### From OpenFGA

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

// InferaDB - Streaming handles pagination
let objects: Vec<String> = client
    .list_resources("user:alice", "viewer", "document")
    .collect()
    .await?;
```

---

## Security Considerations

### Private Key Storage

```rust
// DO: Load from secure secret manager
let key = Ed25519PrivateKey::from_env("INFERADB_PRIVATE_KEY")?;

// DO: Use file with restricted permissions (chmod 600)
let key = Ed25519PrivateKey::from_pem_file("/secrets/inferadb.pem")?;

// DON'T: Hardcode keys
let key = Ed25519PrivateKey::from_pem("-----BEGIN PRIVATE KEY-----...")?;  // BAD!

// DON'T: Log keys
tracing::debug!("Key: {:?}", key);  // NEVER DO THIS!
```

### TLS Verification

```rust
// Production: Always verify TLS (default)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .build()
    .await?;

// Development only: Disable TLS verification
let client = Client::builder()
    .url("https://localhost:8080")
    .danger_accept_invalid_certs(true)  // NEVER in production!
    .build()
    .await?;
```

### Credential Rotation

```rust
// Certificates have finite lifetime - rotate before expiry
let certs = client.control()
    .clients("org_id")
    .certificates("client_id")
    .list()
    .await?;

for cert in certs {
    if cert.expires_at < Utc::now() + Duration::days(30) {
        tracing::warn!("Certificate {} expires in < 30 days", cert.id);
        // Create new certificate
        let new_cert = client.control()
            .clients("org_id")
            .certificates("client_id")
            .create()
            .await?;
        // Deploy new private key to secrets manager
        // Update client configuration
        // Revoke old certificate after transition
    }
}
```

### Audit Logging

```rust
// Enable audit logging for security-sensitive operations
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .with_middleware(AuditMiddleware::new()
        .log_checks(true)
        .log_writes(true)
        .log_deletes(true))
    .build()
    .await?;
```

### Network Security

```rust
// Use mTLS for additional security (if supported)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(creds)
    .client_certificate(ClientCertificate {
        cert: include_bytes!("../certs/client.crt"),
        key: include_bytes!("../certs/client.key"),
    })
    .build()
    .await?;
```

---

## Runnable Examples

The SDK ships with runnable examples in the `examples/` directory.

### Available Examples

```bash
# List all examples
cargo run --example

# Run specific examples:
cargo run --example basic_check          # Simple authorization check
cargo run --example batch_check          # Batch multiple checks
cargo run --example watch_changes        # Real-time relationship updates
cargo run --example middleware_axum      # Axum middleware integration
cargo run --example middleware_actix     # Actix-web guard integration
cargo run --example graphql_dataloader   # async-graphql DataLoader
cargo run --example multi_tenant         # Multi-vault SaaS pattern
cargo run --example blocking_cli         # Sync client for CLI
cargo run --example custom_types         # Type-safe macros
cargo run --example opentelemetry        # Full OTLP setup
```

### Example: Basic Check

```rust
// examples/basic_check.rs
use inferadb::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure from environment
    let client = Client::from_env().await?;

    // Simple check
    let allowed = client
        .check("user:alice", "view", "document:readme")
        .await?;

    println!("Access allowed: {}", allowed);
    Ok(())
}
```

Run with:

```bash
export INFERADB_URL=https://api.inferadb.com
export INFERADB_CLIENT_ID=my_client
export INFERADB_PRIVATE_KEY_PATH=./private_key.pem
export INFERADB_VAULT_ID=my_vault

cargo run --example basic_check
```

### Example: Watch Changes

```rust
// examples/watch_changes.rs
use inferadb::{Client, WatchFilter};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env().await?;

    // Watch for changes to document permissions
    let mut stream = client
        .watch()
        .filter(WatchFilter::resource_type("document"))
        .run()
        .await?;

    println!("Watching for changes...");

    while let Some(change) = stream.next().await {
        let change = change?;
        println!(
            "[{}] {} {} {} -> {}",
            change.timestamp,
            change.operation,
            change.relationship.resource,
            change.relationship.relation,
            change.relationship.subject
        );
    }

    Ok(())
}
```

### Running Examples Against Local InferaDB

```bash
# Start local InferaDB (using docker-compose)
docker-compose up -d

# Run example against local instance
INFERADB_URL=http://localhost:8080 \
INFERADB_VAULT_ID=test-vault \
cargo run --example basic_check
```

---

## Contributing

### Development Setup

```bash
# Clone the SDK repository
git clone https://github.com/inferadb/rust-sdk
cd rust-sdk

# Install dependencies
cargo build --workspace

# Run tests
cargo nextest run --workspace

# Run specific test
cargo test test_check_authorization

# Run with local InferaDB
INFERADB_URL=http://localhost:8080 cargo test --features integration
```

### Code Style

```bash
# Format code
cargo +nightly fmt --all

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Check documentation
cargo doc --workspace --no-deps
```

### Testing Guidelines

```rust
// Unit tests: Use mocks
#[tokio::test]
async fn test_check_returns_true_for_allowed() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .build();

    assert!(mock.check("user:alice", "view", "doc:1").await.unwrap());
}

// Integration tests: Use real client with test fixtures
#[tokio::test]
#[ignore]  // Run with --ignored flag
async fn integration_test_check() {
    let client = test_client().await;
    let vault = TestVault::create(&client).await.unwrap();

    vault.write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());
}
```

### Pull Request Checklist

- [ ] Tests pass (`cargo nextest run --workspace`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo +nightly fmt --all`)
- [ ] Documentation updated (`cargo doc --no-deps`)
- [ ] CHANGELOG.md updated
- [ ] Version bumped if needed

### Reporting Issues

File issues at: <https://github.com/inferadb/rust-sdk/issues>

Include:

- SDK version (`cargo pkgid inferadb`)
- Rust version (`rustc --version`)
- Minimal reproduction code
- Error message with request ID if available

---

## Production Readiness Checklist

Before deploying to production, verify your InferaDB integration:

### Security

- [ ] **TLS Verification Enabled**: Not using `danger_accept_invalid_certs(true)`
- [ ] **Private Keys Secured**: Keys stored in secrets manager, not in code/env files
- [ ] **Key Rotation Plan**: Certificate expiry monitoring and rotation process
- [ ] **Minimal Scopes**: Client credentials request only needed scopes
- [ ] **No Browser Private Keys**: Using bearer tokens, not client credentials in browser

### Resilience

- [ ] **Retry Configured**: Custom `RetryConfig` appropriate for your SLAs
- [ ] **Circuit Breaker**: `CircuitBreakerConfig` prevents cascade failures
- [ ] **Fallback Policy**: Explicit fail-open or fail-closed decision
- [ ] **Timeouts Set**: Request and connection timeouts appropriate for your needs
- [ ] **Graceful Shutdown**: Client shutdown integrated with service lifecycle

### Performance

- [ ] **Connection Pool Sized**: Pool size matches expected concurrency
- [ ] **Caching Strategy**: Cache TTL and invalidation approach defined
- [ ] **Batch Operations**: Using `check_batch()` instead of loops where possible
- [ ] **Protocol Selected**: gRPC for performance, REST if network restricted

### Observability

- [ ] **Logging Configured**: Log level set appropriately (not trace in prod)
- [ ] **Metrics Enabled**: `metrics(true)` with Prometheus/OTel export
- [ ] **Tracing Enabled**: `tracing(true)` with trace context propagation
- [ ] **Alerts Configured**: Rate limit warnings, circuit breaker events monitored
- [ ] **Request IDs Logged**: Correlation IDs captured for debugging

### Testing

- [ ] **Unit Tests**: Using `MockClient` for isolated testing
- [ ] **Integration Tests**: Using `TestContainer` or staging environment
- [ ] **Load Tests**: Verified performance under expected load
- [ ] **Failure Tests**: Tested behavior when InferaDB unavailable

### Operations

- [ ] **Health Checks**: `/health` endpoint includes InferaDB status
- [ ] **Runbook**: Documented troubleshooting steps for common errors
- [ ] **On-Call**: Team knows how to investigate authorization issues
- [ ] **Rollback Plan**: Can disable authorization checks if critical bug

---

## Performance Tuning Guide

### Connection Pool Sizing

Formula for HTTP connection pool:

```rust
// Rule of thumb: 2x concurrent requests expected
let expected_concurrent = 50;  // Peak concurrent authorization checks
let pool_size = expected_concurrent * 2;  // Buffer for spikes

let client = Client::builder()
    .max_connections(pool_size)
    .build()
    .await?;
```

For gRPC, streams are multiplexed:

```rust
// Fewer connections needed for gRPC
let client = Client::builder()
    .grpc_concurrency_limit(100)  // Streams per connection
    .build()
    .await?;
```

### Cache TTL Selection

| Consistency Need     | Recommended TTL | Invalidation |
| -------------------- | --------------- | ------------ |
| Strong (financial)   | 0 (no cache)    | N/A          |
| Near-real-time       | 5-10 seconds    | Watch-based  |
| Eventual (most apps) | 30-60 seconds   | TTL expiry   |
| Relaxed              | 5+ minutes      | TTL expiry   |

```rust
// For most applications
let client = Client::builder()
    .cache(CacheConfig {
        ttl: Duration::from_secs(60),
        negative_ttl: Duration::from_secs(30),  // Cache denials shorter
        max_entries: 10_000,
    })
    .build()
    .await?;
```

### Batching Strategy

When to batch vs individual checks:

```rust
// Individual checks: When you need immediate result
let allowed = client.check("user:alice", "view", "doc:1").await?;

// Batch checks: When checking multiple in one operation
let results = client.check_batch([
    ("user:alice", "view", "doc:1"),
    ("user:alice", "edit", "doc:1"),
    ("user:alice", "delete", "doc:1"),
]).collect().await?;

// Use all/any for guard clauses
let can_modify = client.check_batch([
    ("user:alice", "edit", "doc:1"),
    ("user:alice", "admin", "org:acme"),
]).any().await?;
```

### Memory Optimization

```rust
// Minimize memory footprint
let client = Client::builder()
    // Smaller cache if memory constrained
    .cache(CacheConfig {
        max_entries: 1_000,  // Reduce from 10_000
        ..Default::default()
    })
    // Limit concurrent streams
    .grpc_concurrency_limit(50)  // Reduce from 100
    // Aggressive idle timeout
    .idle_timeout(Duration::from_secs(30))  // Reduce from 90s
    .build()
    .await?;
```

### Latency Optimization

```rust
// Minimize p99 latency
let client = Client::builder()
    // Prefer gRPC (lower latency)
    .grpc_only()
    // Enable caching
    .cache(CacheConfig::default())
    // Aggressive timeouts
    .timeout(Duration::from_millis(100))
    // Fast retries
    .retry(RetryConfig {
        max_attempts: 2,  // Fail faster
        initial_backoff: Duration::from_millis(10),
        ..Default::default()
    })
    .build()
    .await?;
```

---

## Authorization Anti-Patterns

Common mistakes to avoid when implementing authorization.

### Anti-Pattern: Check in a Loop

```rust
// ❌ BAD: N network calls
for doc in documents {
    let allowed = client.check(&user, "view", &doc.id).await?;
    if allowed {
        results.push(doc);
    }
}

// ✅ GOOD: 1 network call
let checks: Vec<_> = documents.iter()
    .map(|doc| (&user, "view", &doc.id))
    .collect();

let allowed_set: HashSet<_> = client
    .check_batch(checks)
    .collect()
    .await?
    .into_iter()
    .filter(|(_, decision)| decision.allowed)
    .map(|(idx, _)| idx)
    .collect();

let results: Vec<_> = documents
    .into_iter()
    .enumerate()
    .filter(|(idx, _)| allowed_set.contains(idx))
    .map(|(_, doc)| doc)
    .collect();
```

### Anti-Pattern: Cache Without Invalidation Strategy

```rust
// ❌ BAD: Long TTL without invalidation
let client = Client::builder()
    .cache(CacheConfig {
        ttl: Duration::from_secs(3600),  // 1 hour - too long!
    })
    .build()
    .await?;

// ✅ GOOD: Reasonable TTL with invalidation
let client = Client::builder()
    .cache(CacheConfig {
        ttl: Duration::from_secs(60),
        invalidation: WatchInvalidation::new(),  // Real-time invalidation
    })
    .build()
    .await?;
```

### Anti-Pattern: Ignoring #[must_use]

```rust
// ❌ BAD: Ignoring the authorization result
client.check("user:alice", "view", "doc:1").await?;  // SECURITY BUG!
serve_document(doc_id);

// ✅ GOOD: Always check the result
let allowed = client.check("user:alice", "view", "doc:1").await?;
if !allowed {
    return Err(Error::Forbidden);
}
serve_document(doc_id);

// ✅ BETTER: Use require() for clarity
client.require("user:alice", "view", "doc:1").await?;
serve_document(doc_id);
```

### Anti-Pattern: Fail-Open Without Understanding

```rust
// ❌ BAD: Fail-open without consideration
let client = Client::builder()
    .fallback(FallbackPolicy::Allow)  // DANGER: Everyone gets access if InferaDB is down!
    .build()
    .await?;

// ✅ GOOD: Explicit, limited fail-open
let client = Client::builder()
    .fallback(FallbackPolicy::Custom(|ctx| {
        // Only allow reads, deny writes
        match ctx.permission.as_str() {
            "view" | "list" => FallbackDecision::Allow,
            _ => FallbackDecision::Deny,
        }
    }))
    .build()
    .await?;
```

### Anti-Pattern: Checking After Fetching

```rust
// ❌ BAD: Fetch first, then check (leaks existence)
let doc = db.fetch_document(doc_id).await?;  // Attacker learns doc exists
let allowed = client.check(&user, "view", doc_id).await?;
if !allowed {
    return Err(Error::NotFound);  // Too late - timing attack possible
}

// ✅ GOOD: Check before fetch
let allowed = client.check(&user, "view", doc_id).await?;
if !allowed {
    return Err(Error::NotFound);  // Same error for "not allowed" and "doesn't exist"
}
let doc = db.fetch_document(doc_id).await?;
```

### Anti-Pattern: Hardcoded Fallback Permissions

```rust
// ❌ BAD: Hardcoded admin bypass
if user.is_admin {
    return true;  // Bypass all authorization!
}
let allowed = client.check(&user, permission, resource).await?;

// ✅ GOOD: Admin is just another relation
// In your schema:
// permissions { admin_bypass: is_superadmin }
let allowed = client.check(&user, permission, resource).await?;
```

### Anti-Pattern: Not Batching Related Checks

```rust
// ❌ BAD: Separate calls for related permissions
let can_view = client.check(&user, "view", &doc_id).await?;
let can_edit = client.check(&user, "edit", &doc_id).await?;
let can_delete = client.check(&user, "delete", &doc_id).await?;

// ✅ GOOD: Single batch call
let [can_view, can_edit, can_delete] = client
    .check_batch([
        (&user, "view", &doc_id),
        (&user, "edit", &doc_id),
        (&user, "delete", &doc_id),
    ])
    .collect::<Vec<_>>()
    .await?
    .try_into()
    .unwrap();
```

---

## API Ergonomics

Advanced patterns for cleaner authorization code.

### Capability-Based Client

Get compile-time guarantees about available operations based on token scopes:

```rust
use inferadb::capabilities::{ReadOnly, ReadWrite, Admin};

// Type-safe client based on credentials
let reader: Client<ReadOnly> = Client::builder()
    .url("https://api.inferadb.com")
    .bearer_token(read_only_token)
    .build()
    .await?;

// Only read operations available at compile time
reader.check("user:alice", "view", "doc:1").await?;  // ✅ Compiles
// reader.write(relationship).await?;                  // ❌ Compile error!

// Full access client
let admin: Client<Admin> = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(admin_creds)
    .build()
    .await?;

admin.check("user:alice", "view", "doc:1").await?;  // ✅
admin.write(relationship).await?;                    // ✅
admin.control().schemas("org", "vault").deploy(schema).await?;  // ✅
```

#### Capability Traits

```rust
/// Marker trait for read operations
pub trait CanRead {}

/// Marker trait for write operations
pub trait CanWrite: CanRead {}

/// Marker trait for admin operations
pub trait CanAdmin: CanWrite {}

// Capability markers
pub struct ReadOnly;
impl CanRead for ReadOnly {}

pub struct ReadWrite;
impl CanRead for ReadWrite {}
impl CanWrite for ReadWrite {}

pub struct Admin;
impl CanRead for Admin {}
impl CanWrite for Admin {}
impl CanAdmin for Admin {}
```

#### Capability Narrowing

```rust
// Start with full access
let admin: Client<Admin> = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(admin_creds)
    .build()
    .await?;

// Narrow to read-only for passing to untrusted code
let reader: Client<ReadOnly> = admin.as_reader();
untrusted_module.process(&reader);  // Can only read

// Narrow to read-write
let writer: Client<ReadWrite> = admin.as_writer();
```

### Relationship Builder DSL

Fluent DSL for building relationships:

```rust
use inferadb::dsl::*;

// Simple relationship
let rel = relationship!("document:readme" <- viewer <- "user:alice");

// Equivalent to:
let rel = Relationship::new("document:readme", "viewer", "user:alice");
```

#### Fluent Builder

```rust
use inferadb::RelationshipBuilder;

// Single relationship
let rel = RelationshipBuilder::new()
    .resource("document", "readme")
    .relation("viewer")
    .subject("user", "alice")
    .build();

// With subject relation (for groups)
let rel = RelationshipBuilder::new()
    .resource("document", "readme")
    .relation("viewer")
    .subject_set("group", "engineering", "member")
    .build();

// Batch relationships
let rels = RelationshipBuilder::batch()
    .resource("folder", "projects")
    .relation("viewer")
    .subjects([
        subject!("user:alice"),
        subject!("user:bob"),
        subject!("group:engineering#member"),
    ])
    .build();  // Returns Vec<Relationship>
```

#### Tuple String Parsing

```rust
// Parse from string format
let rel: Relationship = "document:readme#viewer@user:alice".parse()?;
let rel: Relationship = "document:readme#viewer@group:eng#member".parse()?;

// Display as tuple string
let rel = Relationship::new("document:readme", "viewer", "user:alice");
assert_eq!(rel.to_string(), "document:readme#viewer@user:alice");
```

#### Relationship Predicates

```rust
// Build check predicates fluently
let allowed = client
    .check(Check::new()
        .subject("user:alice")
        .permission("view")
        .resource("document:readme"))
    .await?;

// Or use tuple syntax
let allowed = client
    .check(("user:alice", "view", "document:readme"))
    .await?;
```

### Fluent Error Handling

```rust
use inferadb::ext::ResultExt;

// Chain context onto errors
let allowed = client
    .check("user:alice", "view", "doc:1")
    .await
    .with_context("checking document access")?;

// Convert to HTTP response
let allowed = client
    .check("user:alice", "view", "doc:1")
    .await
    .or_forbidden()?;  // Returns 403 on error or denial
```

### Decision Result Helpers

```rust
use inferadb::Decision;

impl Decision {
    /// Return Ok(()) if allowed, Err(Forbidden) otherwise
    pub fn require(self) -> Result<(), Error> {
        if self.allowed {
            Ok(())
        } else {
            Err(Error::Forbidden {
                message: self.reason.unwrap_or_default(),
            })
        }
    }

    /// Return Ok(()) if allowed, custom error otherwise
    pub fn require_or<E>(self, err: E) -> Result<(), E> {
        if self.allowed { Ok(()) } else { Err(err) }
    }

    /// Return Ok(()) if allowed, computed error otherwise
    pub fn require_or_else<E, F: FnOnce() -> E>(self, f: F) -> Result<(), E> {
        if self.allowed { Ok(()) } else { Err(f()) }
    }
}

// Usage
client.check(&user, "delete", &doc_id)
    .await?
    .require()?;  // Throws Forbidden if not allowed
```

### Type-Safe Permission Checking

```rust
use inferadb::Permission;

// Define permissions as types
trait DocumentPermission: Permission {
    const NAME: &'static str;
}

struct View;
impl Permission for View { const NAME: &'static str = "view"; }

struct Edit;
impl Permission for Edit { const NAME: &'static str = "edit"; }

// Extension trait for typed checks
impl Client {
    async fn check_permission<P: Permission>(
        &self,
        subject: impl AsRef<str>,
        resource: impl AsRef<str>,
    ) -> Result<bool, Error> {
        self.check(subject.as_ref(), P::NAME, resource.as_ref()).await
    }
}

// Usage - permission is type-checked
let allowed = client.check_permission::<View>(&user, &doc).await?;
```

### Builder Extensions

```rust
// Extend the check builder with domain-specific helpers
trait CheckBuilderExt {
    fn for_current_user(self) -> Self;
    fn with_request_context(self, req: &HttpRequest) -> Self;
}

impl CheckBuilderExt for CheckBuilder<'_> {
    fn for_current_user(self) -> Self {
        let user = current_user();  // From thread-local or context
        self.subject(&user)
    }

    fn with_request_context(self, req: &HttpRequest) -> Self {
        self.with_context(Context::new()
            .insert("ip_address", req.peer_addr())
            .insert("user_agent", req.headers().get("User-Agent")))
    }
}

// Usage
let allowed = client
    .check("view", "doc:1")
    .for_current_user()
    .with_request_context(&request)
    .await?;
```

### Macro for Common Patterns

```rust
// Define a macro for common authorization patterns
macro_rules! require_permission {
    ($client:expr, $user:expr, $perm:expr, $resource:expr) => {
        if !$client.check($user, $perm, $resource).await? {
            return Err(Error::Forbidden {
                message: format!("{} cannot {} on {}", $user, $perm, $resource),
            });
        }
    };
}

// Usage
async fn delete_document(client: &Client, user: &str, doc_id: &str) -> Result<()> {
    require_permission!(client, user, "delete", format!("document:{}", doc_id));
    actually_delete(doc_id).await
}
```

---

---

## Sources

### Competitor SDKs Analyzed

- [SpiceDB Clients](https://authzed.com/products/spicedb-clients) - Official SpiceDB client ecosystem
- [spicedb-rust (community)](https://crates.io/crates/spicedb-rust) - Community Rust client by Lur1an
- [openfga-client](https://github.com/vakamo-labs/openfga-client) - Type-safe OpenFGA Rust client
- [openfga-rs](https://github.com/liamwh/openfga-rs) - OpenFGA SDK from protobufs
- [Oso Rust SDK](https://docs.rs/oso) - Oso authorization library
- [Cedar](https://www.cedarpolicy.com/) - AWS Cedar policy language and SDK
- [Open Policy Agent](https://www.openpolicyagent.org/) - OPA/Rego policy engine

### Comparison with Cedar and OPA/Rego

For users evaluating policy engines, here's how InferaDB compares:

| Feature | InferaDB | Cedar | OPA/Rego |
|---------|----------|-------|----------|
| **Primary Focus** | ReBAC + ABAC | ABAC | General policy |
| **Policy Language** | IPL | Cedar | Rego |
| **Relationship Graphs** | ✅ Native | ❌ | ❌ (manual) |
| **Rust SDK Quality** | ✅ First-class | ✅ First-class | ⚠️ WASM bindings |
| **Server Mode** | ✅ Distributed | ❌ Library only | ✅ Centralized |
| **Streaming Updates** | ✅ Watch API | ❌ | ❌ |
| **Multi-Tenant** | ✅ Vault isolation | ❌ | ❌ |
| **Schema Types** | ✅ Typed entities | ✅ Typed | ❌ Untyped |

#### Cedar Comparison

Cedar excels at attribute-based access control (ABAC) with a focus on AWS integration:

```cedar
// Cedar policy
permit(
    principal == User::"alice",
    action == Action::"view",
    resource
) when {
    resource.classification == "public"
};
```

```rust
// InferaDB IPL - combines ReBAC with ABAC
entity Document {
    permissions {
        view: viewer when {
            resource.classification == "public" or
            principal.has_clearance(resource.classification)
        }
    }
}
```

**Choose InferaDB over Cedar when:**

- You need relationship-based access (teams, folders, sharing)
- You want a managed distributed service
- You need real-time permission updates
- Multi-tenancy is a requirement

**Choose Cedar when:**

- Pure attribute-based policies suffice
- You need deep AWS integration
- You want embeddable library (no network)

#### OPA/Rego Comparison

OPA is a general-purpose policy engine using Rego:

```rego
# Rego policy
allow {
    input.action == "view"
    input.user == data.documents[input.resource].viewers[_]
}
```

```rust
// InferaDB - relationships are first-class
let allowed = client.check("user:alice", "view", "document:readme").await?;
```

**Choose InferaDB over OPA when:**

- Authorization is your primary use case
- You need relationship traversal (groups, hierarchies)
- You want type-safe Rust SDK
- Latency is critical (InferaDB is optimized for authz)

**Choose OPA when:**

- You need general policy beyond authorization
- Rego's flexibility is required
- You have existing Rego policies

### Standards & Specifications

- [RFC 7523](https://datatracker.ietf.org/doc/html/rfc7523) - JWT Bearer client authentication
- [RFC 7517](https://datatracker.ietf.org/doc/html/rfc7517) - JSON Web Key (JWK)
- [AuthZEN](https://openid.net/wg/authzen/) - Authorization API interoperability

### InferaDB Documentation

- [Engine CLAUDE.md](../engine/CLAUDE.md) - Engine architecture and patterns
- [Control Authentication](../control/docs/authentication.md) - Authentication flows
- [IPL Specification](../engine/concept.ipl) - Policy language reference
