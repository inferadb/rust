# InferaDB Rust SDK Design Document

Internal design document for SDK implementers. For user-facing documentation, see [README.md](README.md).

---

## Related Documentation

| Document                                                                       | Audience     | Purpose                                        |
| ------------------------------------------------------------------------------ | ------------ | ---------------------------------------------- |
| [README.md](README.md)                                                         | SDK Users    | Quick start, installation, basic usage         |
| [CONTRIBUTING.md](CONTRIBUTING.md)                                             | Contributors | Development setup, PR guidelines               |
| [CHANGELOG.md](CHANGELOG.md)                                                   | All          | Version history, breaking changes              |
| [MIGRATION.md](MIGRATION.md)                                                   | SDK Users    | Version upgrades, migration from other systems |
| [docs/troubleshooting.md](docs/troubleshooting.md)                             | SDK Users    | Common issues and solutions                    |
| [docs/guides/production-checklist.md](docs/guides/production-checklist.md)     | Operators    | Deployment readiness checklist                 |
| [docs/guides/performance-tuning.md](docs/guides/performance-tuning.md)         | Operators    | Optimization strategies                        |
| [docs/guides/integration-patterns.md](docs/guides/integration-patterns.md)     | SDK Users    | Framework integration examples                 |
| [docs/guides/testing.md](docs/guides/testing.md)                               | SDK Users    | Testing patterns and utilities                 |
| [docs/guides/caching.md](docs/guides/caching.md)                               | SDK Users    | Caching strategies and invalidation            |
| [docs/internal/competitive-analysis.md](docs/internal/competitive-analysis.md) | Internal     | Competitive positioning                        |

---

## Table of Contents

### Getting Started

- [5-Minute Quickstart](#5-minute-quickstart)

### Part 1: Vision & Architecture

- [Design Philosophy](#design-philosophy)
- [Architecture Overview](#architecture-overview)
- [Crate Structure](#crate-structure)

### Part 2: Client Design

- [Client Builder](#client-builder)
- [Async-First Design](#async-first-design)
- [Typestate Builder Pattern](#typestate-builder-pattern)
- [Authentication](#authentication)
- [Connection Management](#connection-management)
  - [Client Cloning Semantics](#client-cloning-semantics)
- [Health Check & Lifecycle](#health-check--lifecycle)
- [Vault Scoping](#vault-scoping)
  - [Sub-Client Types](#sub-client-types)
- [Middleware and Interceptors](#middleware-and-interceptors)

### Part 3: Type System & Safety

- [Type-Safe Relationships](#type-safe-relationships)
- [Zero-Copy APIs](#zero-copy-apis)
- [Async Trait Objects & DI](#async-trait-objects--di)

### Part 4: Engine API Design

- [Authorization Checks](#authorization-checks)
  - [The Hero Pattern: require()](#the-hero-pattern-require)
  - [Convenience Helpers](#convenience-helpers)
  - [Batch Checks](#batch-checks)
- [Structured Decision Traces](#structured-decision-traces)
- [Explain Permission](#explain-permission)
- [Simulate (What-If)](#simulate-what-if)
- [Relationship Management](#relationship-management)
  - [Relationship History](#relationship-history)
  - [Relationship Validation](#relationship-validation)
- [Request ID & Idempotency](#request-id--idempotency)
- [Lookup Operations](#lookup-operations)
- [Streaming & Watch](#streaming--watch)
- [Vault Statistics](#vault-statistics)
- [Bulk Operations](#bulk-operations)
- [Caching](#caching)
  - [Consistency Tokens and Cache Interaction](#consistency-tokens-and-cache-interaction)
  - [Read-After-Write Recipe](#read-after-write-recipe)
  - [Cache Invalidation via Watch](#cache-invalidation-via-watch)

### Part 5: Control API Design

- [Control API Overview](#control-api-overview)
- [Account Management](#account-management)
- [Organization Management](#organization-management)
  - [Organization Members](#organization-members)
  - [Organization Invitations](#organization-invitations)
  - [Organization Roles](#organization-roles)
- [Team Management](#team-management)
- [Vault Management](#vault-management)
  - [Vault Roles](#vault-roles)
  - [Vault Tokens](#vault-tokens)
- [Client Management](#client-management)
- [Schema Management](#schema-management)
  - [Schema Versioning](#schema-versioning)
  - [Schema Lifecycle](#schema-lifecycle)
  - [Canary Deployments](#canary-deployments)
  - [Pre-flight Checks](#pre-flight-checks)
- [Audit Logs](#audit-logs)
- [JWKS Operations](#jwks-operations)

### Part 6: Developer Experience

- [Authentication Flows](#authentication-flows)
  - [OAuth PKCE Flow](#oauth-pkce-flow)
  - [Token Management](#token-management)
  - [Registration](#registration)
- [Error Handling](#error-handling)
  - [Protocol Status → ErrorKind Mapping](#protocol-status--errorkind-mapping)
  - [check() vs require() Error Semantics](#check-vs-require-error-semantics)
  - [Retry Recommendations by ErrorKind](#retry-recommendations-by-errorkind)
- [Retry & Resilience](#retry--resilience)
- [Graceful Degradation](#graceful-degradation)
- [Observability](#observability)
  - [Health Checks](#health-checks)
  - [Ping & Latency](#ping--latency)
  - [Diagnostics](#diagnostics)
- [W3C Trace Context Propagation](#w3c-trace-context-propagation)
- [Testing Support](#testing-support)
  - [MockClient: The Hero Testing Pattern](#mockclient-the-hero-testing-pattern)
  - [Decision Trace Snapshot Testing](#decision-trace-snapshot-testing)
  - [Simulation + Snapshot for What-If Testing](#simulation--snapshot-for-what-if-testing)

### Part 7: Implementation Details

- [Protocol Support](#protocol-support)
- [Feature Flags](#feature-flags)
- [WASM / Browser Usage](#wasm--browser-usage)
- [Stability Policy](#stability-policy)
- [Safety Guarantees](#safety-guarantees)
- [Security Considerations](#security-considerations)
- [Release Strategy](#release-strategy)

---

## 5-Minute Quickstart

Get up and running with InferaDB in minutes.

### Installation

```toml
# Cargo.toml
[dependencies]
inferadb = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use inferadb::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // 1. Create client with explicit configuration
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(ClientCredentialsConfig {
            client_id: "your_client_id".into(),
            private_key: Ed25519PrivateKey::from_pem_file("path/to/private-key.pem")?,
            certificate_id: None,
        })
        .build()
        .await?;

    // 2. Get vault context (organization-first hierarchy)
    let vault = client
        .organization("org_8675309...")
        .vault("vlt_01JFQGK...");

    // 3. Write a relationship
    vault.relationships()
        .write(Relationship::new("document:readme", "viewer", "user:alice"))
        .await?;

    // 4. Check permission (returns bool)
    let allowed = vault
        .check("user:alice", "view", "document:readme")
        .await?;

    println!("Alice can view: {}", allowed);  // true

    // 5. Use require() for HTTP handlers (fails fast on denial)
    vault.check("user:alice", "view", "document:readme")
        .require()
        .await?;  // Returns Ok(()) or Err(AccessDenied)

    println!("Access granted!");
    Ok(())
}
```

### Next Steps

- **[Authorization Checks](#authorization-checks)**: Learn `check()`, `require()`, `explain()`
- **[Relationship Management](#relationship-management)**: CRUD operations for the graph
- **[Testing Support](#testing-support)**: MockClient, InMemoryClient, TestVault
- **[Error Handling](#error-handling)**: Error types and retry strategies

---

## Design Philosophy

### Core Principles

1. **Zero-friction authentication**: SDK self-manages tokens, refresh cycles, and credential rotation. Developers provide credentials once and forget about auth.

2. **Unified service URL**: Single endpoint routes to both Engine and Control APIs transparently. No separate clients or configuration.

3. **Type-safe by default**: Leverage Rust's type system to prevent invalid states. Relationship tuples, permissions, and resources are typed at compile time.

4. **Async-first**: All I/O operations are async. `build_sync()` is the only sync method, provided for early-boot initialization. See [Async-First Design](#async-first-design).

5. **Streaming-first**: All list operations support streaming for memory efficiency. Batch operations stream results as they complete.

6. **Protocol flexibility**: Support both gRPC (high performance) and REST (universal compatibility) with feature flags.

7. **Observability built-in**: First-class tracing, metrics, and structured logging without configuration.

8. **Testing as a feature**: Mock clients, simulation mode, and test utilities are first-class SDK features.

### Design Invariants

These invariants are **guaranteed behaviors** that users can rely on. They inform implementation decisions and should be verified by tests:

| Invariant                             | Description                                                                                                                                                                |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Denial is not an error**            | `check()` returns `Ok(false)` for denied access. Only `require()` converts denial to `Err(AccessDenied)`. This keeps authorization decisions separate from error handling. |
| **Fail-closed by default**            | All error handling defaults to `FailureMode::FailClosed`. Fail-open (`FailOpen`) must be explicitly requested and always logs at WARN level.                               |
| **Transport fallback is transparent** | Falling back from gRPC to REST never changes authorization semantics—only availability characteristics. Both protocols return identical results.                           |
| **Results preserve input order**      | Batch operations (`check_batch`, `write_batch`) return results in the same order as input items, even when parallelized internally.                                        |
| **Streams are lazy**                  | Query streams don't fetch data until consumed. Creating a stream has no side effects.                                                                                      |
| **Writes are acknowledged**           | Write operations return only after server acknowledgment. `WriteResult.consistency_token()` is always valid for read-after-write.                                          |
| **Cache never changes semantics**     | Cached results are identical to fresh results. Stale cache entries may reduce availability but never change authorization outcomes.                                        |
| **Errors include request IDs**        | All errors that reach the server include a `request_id()` for debugging and support escalation.                                                                            |

### Competitive Differentiation

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

### API Ordering Rationale (Quick Reference)

The SDK uses context-appropriate parameter orderings. Keep this reference handy:

| API                                                    | Order                           | Mental Model                            |
| ------------------------------------------------------ | ------------------------------- | --------------------------------------- |
| `check(sub, perm, res)`                                | Subject → Permission → Resource | "Can **who** do **what** to **which**?" |
| `Relationship::new(res, rel, sub)`                     | Resource → Relation → Subject   | Graph edge: `res -[rel]→ sub`           |
| `resources().accessible_by(sub).with_permission(perm)` | Subject → Permission            | "What can **who** **do**?"              |
| `subjects().with_permission(perm).on_resource(res)`    | Permission → Resource           | "Who can **do** to **which**?"          |
| `relationships().list().resource(res).relation(rel)`   | Resource → Relation             | Filter by edge origin                   |

**Why different orderings?**

1. **`check()` uses "question" order**: Matches natural language "Can Alice view the document?" → `check("user:alice", "view", "doc:readme")`

2. **`Relationship` uses "graph edge" order**: Matches ReBAC graph mental model where edges flow from resource to subject: `document:readme -[viewer]→ user:alice`

3. **Query builders use "filter chain" order**: Each method narrows the result set, so order follows the question being asked

**Mnemonic**:

- **Checks**: "**Who** can do **what** to **which**?" → Subject, Permission, Resource
- **Relationships**: "**Which** has **what** with **whom**?" → Resource, Relation, Subject
- **Queries**: Start with what you're looking for, then add filters

**Conversion helpers**:

```rust
// When you have the data in a different order, use named parameters
let rel = Relationship::builder()
    .resource("document:readme")
    .relation("viewer")
    .subject("user:alice")
    .build();

// Or use the from_check helper to convert check parameters to relationship
let rel = Relationship::from_check_params("user:alice", "viewer", "document:readme");
// Result: Relationship { resource: "document:readme", relation: "viewer", subject: "user:alice" }
```

---

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                              InferaDB Rust SDK                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                              Client                                   │  │
│  │  .organization(id) → OrganizationClient                               │  │
│  │  .account() → AccountClient                                           │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  │                                          │
│  ┌───────────────────────────────┴───────────────────────────────────────┐  │
│  │                       AuthManager (internal)                          │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────────────┐  │  │
│  │  │ ClientAssertion │  │  TokenCache     │  │  RefreshScheduler     │  │  │
│  │  │ (Ed25519 JWT)   │  │  (vault-scoped) │  │  (background task)    │  │  │
│  │  └─────────────────┘  └─────────────────┘  └───────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                       OrganizationClient                              │  │
│  │  (via .organization(id))                                              │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐  │  │
│  │  │ .vault(id) → VaultClient (authorization + management)           │  │  │
│  │  │ .vaults() → VaultsClient (list/create vaults)                   │  │  │
│  │  │ .members() → MembersClient                                      │  │  │
│  │  │ .teams() → TeamsClient                                          │  │  │
│  │  │ .invitations() → InvitationsClient                              │  │  │
│  │  │ .clients() → ClientsClient (API clients for M2M auth)           │  │  │
│  │  │ .audit_logs() → AuditLogsClient                                 │  │  │
│  │  │ .get() / .update() / .delete()                                  │  │  │
│  │  └─────────────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                            VaultClient                                │  │
│  │  (via org.vault(id))                                                  │  │
│  │  ┌──────────────────────────┐  ┌────────────────────────────────────┐ │  │
│  │  │ Authorization (Access)   │  │ Management (Control)               │ │  │
│  │  │ ────────────────────────-│  │ ──────────────────────────────     │ │  │
│  │  │ check() / check_batch()  │  │ schemas() → SchemaClient           │ │  │
│  │  │ expand()                 │  │ tokens() → TokensClient            │ │  │
│  │  │ resources() → queries    │  │ roles() → RolesClient              │ │  │
│  │  │ subjects() → queries     │  │ get() / update() / delete()        │ │  │
│  │  │ relationships() → CRUD   │  │                                    │ │  │
│  │  │ watch() / simulate()     │  │                                    │ │  │
│  │  │ export() / import()      │  │                                    │ │  │
│  │  └──────────────────────────┘  └────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        Transport Layer                                │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────────────┐  │  │
│  │  │ GrpcTransport   │  │ HttpTransport   │  │ MockTransport         │  │  │
│  │  │ (tonic)         │  │ (reqwest)       │  │ (testing)             │  │  │
│  │  └─────────────────┘  └─────────────────┘  └───────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Organization-First Hierarchy

The SDK uses an organization-first context hierarchy. Organizations are the top-level resource that own vaults:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

// Organization-first: all vault operations flow through org context
let org = client.organization("org_8675309...");
let vault = org.vault("vlt_01JFQGK...");

// Authorization operations
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Management operations on same vault context
let schema = vault.schemas().get_active().await?;
```

This design:

- **Reflects ownership**: Vaults belong to organizations
- **Unifies access patterns**: Same `VaultClient` type for both authorization and management
- **Enables context propagation**: Organization context flows to all child operations
- **Simplifies multi-org scenarios**: Natural scoping for SaaS applications

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
├── Cargo.toml                   # Workspace manifest
├── inferadb/                    # Main SDK crate (re-exports everything)
│   ├── src/
│   │   ├── lib.rs               # Public API surface
│   │   └── prelude.rs           # Common imports
│   └── Cargo.toml
├── inferadb-client/             # Core client implementation
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
├── inferadb-types/              # Shared types
│   ├── src/
│   │   ├── lib.rs
│   │   ├── relationship.rs      # Relationship, Subject, Resource
│   │   ├── decision.rs          # Decision, Trace
│   │   ├── vault.rs             # Vault, VaultRole
│   │   ├── organization.rs      # Organization, Member
│   │   └── error.rs             # Error types
│   └── Cargo.toml
├── inferadb-macros/             # Procedural macros
│   ├── src/
│   │   ├── lib.rs
│   │   ├── resource.rs          # #[derive(Resource)]
│   │   └── relation.rs          # #[derive(Relation)]
│   └── Cargo.toml
└── inferadb-test/               # Testing utilities
    ├── src/
    │   ├── lib.rs
    │   ├── mock.rs              # MockClient
    │   ├── fixtures.rs          # Test data builders
    │   └── assertions.rs        # Custom assertions
    └── Cargo.toml
```

### Prelude

The prelude provides a single import for common SDK types:

```rust
// Import everything you need with one line
use inferadb::prelude::*;

// Equivalent to:
use inferadb::{
    // Core client types
    Client,
    OrganizationClient,
    VaultClient,

    // Traits (for dependency injection)
    AuthorizationClient,

    // Configuration
    ClientCredentialsConfig,
    BearerCredentialsConfig,
    RetryConfig,
    OperationRetry,
    RetryBudget,
    CacheConfig,
    RefreshConfig,
    Transport,
    TransportStrategy,
    FallbackTrigger,

    // Relationships and authorization
    Relationship,
    RelationshipFilter,
    Decision,
    Context,
    ConsistencyToken,

    // Results and errors
    Error,
    ErrorKind,
    AccessDenied,

    // Keys
    Ed25519PrivateKey,
};
```

### Tiered Preludes

For projects that need finer control over imports, the SDK offers tiered preludes:

**Core Prelude** (`inferadb::prelude::core`) - Minimal, essential types only:

```rust
use inferadb::prelude::core::*;

// Includes only:
// - Client, OrganizationClient, VaultClient
// - AuthorizationClient trait
// - Relationship, Decision, Context
// - Error, ErrorKind, AccessDenied
// - ConsistencyToken
```

**Full Prelude** (`inferadb::prelude`) - All commonly used types (default):

```rust
use inferadb::prelude::*;

// Includes core plus:
// - Configuration types (RetryConfig, CacheConfig, etc.)
// - Credential types
// - Transport types
// - Ed25519PrivateKey
```

**Extended Prelude** (`inferadb::prelude::extended`) - Everything including optional features:

```rust
use inferadb::prelude::extended::*;

// Includes full plus:
// - derive::* (Resource, Subject macros)
// - testing::* (MockClient, InMemoryClient, TestVault)
// - integrations::* (framework extractors, middleware)
```

**When to Use Each**:

| Prelude    | Use Case                        | Binary Size Impact |
| ---------- | ------------------------------- | ------------------ |
| `core`     | Minimal footprint, library code | Smallest           |
| Default    | Most applications               | Standard           |
| `extended` | Feature-rich apps, test code    | Largest            |

```rust
// Library code - minimal dependencies
mod authz {
    use inferadb::prelude::core::*;

    pub async fn check_access(vault: &impl AuthorizationClient, ...) { }
}

// Application code - full features
mod main {
    use inferadb::prelude::*;

    // Full configuration available
    let client = Client::builder()
        .retry(RetryConfig::default())
        .build()
        .await?;
}

// Test code - testing utilities
#[cfg(test)]
mod tests {
    use inferadb::prelude::extended::*;

    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .build();
}
```

---

## Client Builder

### Design Goals

1. **Ergonomic defaults** - Minimize required configuration
2. **Type-safe construction** - Missing required fields don't compile; semantic validation at `build()`
3. **Lazy connection** - Don't block on network during build

### Builder Pattern

```rust
use inferadb::prelude::*;

// Minimal setup with client credentials
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: None,
    })
    .build()
    .await?;

// Organization-first: all operations flow through org → vault context
let allowed = client
    .organization("org_8675309...")
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "document:readme")
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

    // Connection pool
    .pool_size(20)
    .idle_timeout(Duration::from_secs(60))

    // Retry behavior (mechanics + policy)
    .retry(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10)))

    // Caching
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_secs(300))
        .schema_ttl(Duration::from_secs(3600)))

    // Transport layer
    .transport(Transport::Grpc)  // or Transport::Http, Transport::Mock

    // Build (validates and creates client)
    .build()
    .await?;
```

### Configuration Defaults

All config types implement `Default` with sensible production values:

```rust
/// Unified retry configuration covering both mechanics and policy
#[derive(Debug, Clone)]
pub struct RetryConfig {
    // Retry mechanics (how to retry)
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
    pub jitter: f64,

    // Retry budget (prevents retry storms under load)
    pub budget: Option<RetryBudget>,

    // Retry policy (when to retry, by operation category)
    pub reads: OperationRetry,
    pub idempotent_writes: OperationRetry,
    pub non_idempotent_writes: OperationRetry,
}

/// Retry budget to prevent retry storms under high load
#[derive(Debug, Clone)]
pub struct RetryBudget {
    /// Time window for tracking retry ratio
    pub ttl: Duration,
    /// Minimum retries per second allowed regardless of ratio
    pub min_retries_per_second: u32,
    /// Maximum ratio of retries to successful requests (0.0-1.0)
    pub retry_ratio: f64,
}

impl RetryBudget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn min_retries_per_second(mut self, n: u32) -> Self {
        self.min_retries_per_second = n;
        self
    }

    pub fn retry_ratio(mut self, ratio: f64) -> Self {
        self.retry_ratio = ratio;
        self
    }
}

impl Default for RetryBudget {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(10),
            min_retries_per_second: 10,
            retry_ratio: 0.1,  // Max 10% retries
        }
    }
}

/// Per-operation-category retry settings
#[derive(Debug, Clone)]
pub struct OperationRetry {
    /// Override max_retries for this category (None = use default)
    pub max_retries: Option<u32>,
    /// Whether to retry on transient errors (timeouts, 5xx, rate limits)
    pub on_transient: bool,
    /// Whether to retry on connection errors (TCP failures, TLS handshake)
    pub on_connection: bool,
}

impl Default for OperationRetry {
    /// Default is fully enabled (retry on both transient and connection errors)
    fn default() -> Self {
        Self::enabled()
    }
}

impl OperationRetry {
    /// Retry on all retriable errors (transient + connection)
    pub fn enabled() -> Self {
        Self { max_retries: None, on_transient: true, on_connection: true }
    }

    /// Never retry this operation category
    pub fn disabled() -> Self {
        Self { max_retries: Some(0), on_transient: false, on_connection: false }
    }

    /// Only retry on connection errors (safe for non-idempotent writes).
    ///
    /// Connection errors occur before the request reaches the server,
    /// so retrying is safe even for non-idempotent operations.
    pub fn connection_only() -> Self {
        Self { max_retries: None, on_transient: false, on_connection: true }
    }

    /// Set maximum retry attempts for this operation category
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = Some(n);
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            // Mechanics
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: 0.1,
            // Budget: disabled by default (opt-in)
            budget: None,
            // Policy: safe defaults
            reads: OperationRetry::enabled(),
            idempotent_writes: OperationRetry::enabled(),
            non_idempotent_writes: OperationRetry::connection_only(),
        }
    }
}

impl RetryConfig {
    // Builder methods for mechanics
    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn initial_backoff(mut self, d: Duration) -> Self {
        self.initial_backoff = d;
        self
    }

    pub fn max_backoff(mut self, d: Duration) -> Self {
        self.max_backoff = d;
        self
    }

    pub fn backoff_multiplier(mut self, m: f64) -> Self {
        self.backoff_multiplier = m;
        self
    }

    pub fn jitter(mut self, j: f64) -> Self {
        self.jitter = j;
        self
    }

    /// Set retry budget to prevent retry storms under high load
    pub fn retry_budget(mut self, budget: RetryBudget) -> Self {
        self.budget = Some(budget);
        self
    }

    // Builder methods for policy (per-operation-category)
    pub fn reads(mut self, policy: OperationRetry) -> Self {
        self.reads = policy;
        self
    }

    pub fn idempotent_writes(mut self, policy: OperationRetry) -> Self {
        self.idempotent_writes = policy;
        self
    }

    pub fn non_idempotent_writes(mut self, policy: OperationRetry) -> Self {
        self.non_idempotent_writes = policy;
        self
    }
}

/// Cache configuration with production defaults
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// TTL for permission check results
    pub permission_ttl: Duration,
    /// TTL for relationship lookups
    pub relationship_ttl: Duration,
    /// TTL for schema metadata
    pub schema_ttl: Duration,
    /// TTL for denial results (shorter to catch permission grants faster)
    pub negative_ttl: Duration,
    /// Maximum number of cached entries
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            permission_ttl: Duration::from_secs(30),
            relationship_ttl: Duration::from_secs(300),
            schema_ttl: Duration::from_secs(3600),
            negative_ttl: Duration::from_secs(10),
            max_entries: 10_000,
        }
    }
}

impl CacheConfig {
    /// Disable caching entirely - all requests hit the server
    pub fn disabled() -> Self {
        Self {
            permission_ttl: Duration::ZERO,
            relationship_ttl: Duration::ZERO,
            schema_ttl: Duration::ZERO,
            negative_ttl: Duration::ZERO,
            max_entries: 0,
        }
    }

    pub fn permission_ttl(mut self, ttl: Duration) -> Self {
        self.permission_ttl = ttl;
        self
    }

    pub fn relationship_ttl(mut self, ttl: Duration) -> Self {
        self.relationship_ttl = ttl;
        self
    }

    pub fn schema_ttl(mut self, ttl: Duration) -> Self {
        self.schema_ttl = ttl;
        self
    }

    pub fn negative_ttl(mut self, ttl: Duration) -> Self {
        self.negative_ttl = ttl;
        self
    }

    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n;
        self
    }
}

/// Strategy for invalidating cached entries
#[derive(Debug, Clone, Default)]
pub enum CacheInvalidation {
    /// TTL-based expiration only (default)
    /// Entries expire based on their TTL; no active invalidation.
    #[default]
    TtlOnly,

    /// Use watch stream for real-time invalidation
    /// Client subscribes to relationship changes and evicts affected entries.
    Watch,

    /// Invalidate on consistency token mismatch
    /// Entries are evicted when a newer consistency token is observed.
    ConsistencyToken,

    /// Custom invalidation via callback
    /// Allows integration with external cache invalidation systems.
    Custom(Box<dyn CacheInvalidator + Send + Sync>),
}

/// Trait for custom cache invalidation strategies
pub trait CacheInvalidator: std::fmt::Debug {
    /// Called when a cache entry should be checked for invalidation
    fn should_invalidate(&self, key: &str, cached_at: DateTime<Utc>) -> bool;

    /// Called when relationships change (if subscribed to changes)
    fn on_relationship_change(&self, event: &WatchEvent);
}

/// Token refresh configuration with production defaults
#[derive(Debug, Clone)]
pub struct RefreshConfig {
    /// Refresh when this fraction of token lifetime has elapsed (0.0-1.0)
    pub threshold_ratio: f64,
    /// Minimum time before expiry to trigger refresh (fallback)
    pub min_remaining: Duration,
    /// Grace period: allow requests with expiring token while refresh in-flight
    pub grace_period: Duration,
    /// Maximum retries for refresh attempts
    pub max_retries: u32,
    /// Backoff between refresh retries
    pub retry_backoff: Duration,
    /// Whether to retry on auth failures (401/403) - usually false
    pub retry_on_auth_failure: bool,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            threshold_ratio: 0.8,                     // Refresh at 80% of lifetime
            min_remaining: Duration::from_secs(300),  // Or when <5 min remaining
            grace_period: Duration::from_secs(10),    // 10s grace during refresh
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
            retry_on_auth_failure: false,             // Don't retry 401/403
        }
    }
}
```

**Defaults Table**:

| Config Type     | Key Defaults                                                         |
| --------------- | -------------------------------------------------------------------- |
| `RetryConfig`   | 3 retries, 100ms initial, 10s max, 2x backoff, 10% jitter            |
| `CacheConfig`   | 30s permissions, 5m relationships, 1h schema, 10k entries            |
| `RefreshConfig` | 80% threshold, 5m min remaining, 10s grace, 3 retries, no auth retry |

**Customization Pattern**:

```rust
// Start with defaults, customize specific values
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .retry(RetryConfig {
        max_retries: 5,  // Override just this
        ..Default::default()
    })
    .cache(CacheConfig {
        permission_ttl: Duration::from_secs(60),  // Longer cache
        ..Default::default()
    })
    .build()
    .await?;
```

### Builder Validation Model

**Design Decision**: All builder methods return `Self` (infallible). Validation is deferred to `build()`.

**Rationale**:

- Cleaner ergonomics - no `?` on every method call
- Typestate already catches missing required fields at compile time
- Consistent with established patterns (`reqwest::ClientBuilder`, `tonic::Channel`)
- Validation errors collected and returned from single point (`build()`)

```rust
// Builder methods are infallible - return Self
let client = Client::builder()
    .url("https://api.inferadb.com")     // Stores value, validates later
    .pool_size(20)                        // Stores value, validates later
    .connect_timeout(Duration::ZERO)      // Will fail at build()
    .credentials(creds)
    .build()                              // All validation happens here
    .await?;                              // BuildError if any validation failed
```

**Validation in build()**:

```rust
impl ClientBuilder<HasUrl, HasAuth> {
    pub async fn build(self) -> Result<Client, BuildError> {
        // Validate URL format
        let url = Url::parse(&self.url)
            .map_err(|e| BuildError::InvalidUrl(e))?;

        // Validate pool size
        if self.pool_size == 0 {
            return Err(BuildError::InvalidConfig("pool_size must be > 0"));
        }

        // Validate timeouts
        if self.connect_timeout == Duration::ZERO {
            return Err(BuildError::InvalidConfig("connect_timeout must be > 0"));
        }

        // ... create client
    }
}
```

**BuildError provides actionable context**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("invalid configuration: {0}")]
    InvalidConfig(&'static str),

    #[error("authentication error: {0}")]
    AuthError(#[from] AuthError),

    #[error("connection failed: {0}")]
    ConnectionFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Error type for authentication failures during client operations
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Token request failed (e.g., invalid credentials, server error)
    #[error("token request failed: {0}")]
    TokenRequestFailed(String),

    /// Token response was invalid or unparseable
    #[error("invalid token response: {0}")]
    InvalidTokenResponse(String),

    /// Token has expired and refresh failed
    #[error("token expired and refresh failed: {0}")]
    TokenExpired(String),

    /// Credentials were rejected by the server (401/403)
    #[error("credentials rejected: {0}")]
    CredentialsRejected(String),

    /// JWT signing failed
    #[error("failed to sign JWT: {0}")]
    SigningFailed(String),

    /// Private key is invalid or unsupported
    #[error("invalid private key: {0}")]
    InvalidKey(#[from] KeyError),
}
```

### Connection Lifecycle

```text
Client::builder()
    │
    ▼
┌─────────────────┐
│ Validate Config │  Fail fast on invalid configuration
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Create AuthMgr  │  Initialize credential handling
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Create Transport│  Set up connection pool (lazy)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ .build().await  │  Optionally validate connectivity
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Client Ready    │  First request triggers connection
└─────────────────┘
```

### Lazy vs Eager Connection

```rust
// Lazy (default) - Don't connect until first request
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()  // Returns immediately
    .await?;

// Eager - Validate connection during build
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .eager_connect(true)  // Validates connectivity
    .build()  // Fails if server unreachable
    .await?;
```

### Sync vs Async Build Variants

```rust
// Async build (default) - for async contexts
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

// Sync build - for main() or blocking contexts
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build_sync()?;  // Blocks, validates synchronously

// Const configuration - embed settings at compile time, validate at runtime
const CONFIG: ClientConfig = ClientConfig::const_builder()
    .url("https://api.inferadb.com")
    .pool_size(20)
    .connect_timeout_secs(10)
    .build_const();  // Const-evaluable; semantic validation still at Client::from_config().build()

let client = Client::from_config(CONFIG)
    .credentials(creds)
    .build()
    .await?;
```

---

## Async-First Design

This is an **async-first SDK**. All I/O methods are async; there are no `check_sync()`, `write_sync()`, or similar blocking variants.

### Why Async-Only

| Reason                  | Explanation                                                                         |
| ----------------------- | ----------------------------------------------------------------------------------- |
| **Ecosystem alignment** | Rust async ecosystem (tokio, hyper, tonic) is mature and standard for network I/O   |
| **Performance**         | Async enables concurrent requests, connection pooling, and efficient resource usage |
| **Simplicity**          | One API surface to document, test, and maintain                                     |
| **No hidden blocking**  | Sync wrappers hide `block_on()` calls that can panic in async contexts              |

### The Exception: `build_sync()`

`build_sync()` is the **only** synchronous method, provided for early-boot initialization before an async runtime is available:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Before async runtime - use build_sync()
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(creds)
        .build_sync()?;  // OK: no runtime yet

    // Now start async runtime with initialized client
    tokio::runtime::Runtime::new()?.block_on(async {
        let vault = client.organization("org_123").vault("vlt_456");
        vault.check("user:alice", "view", "doc:1").await?;
        Ok(())
    })
}
```

### Using the SDK in Blocking Contexts

If you need to call the SDK from synchronous code, use one of these patterns:

**Pattern 1: Dedicated runtime** (recommended for libraries)

```rust
use tokio::runtime::Runtime;

pub struct BlockingAuthzClient {
    client: Client,
    runtime: Runtime,
}

impl BlockingAuthzClient {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            runtime: Runtime::new().expect("Failed to create runtime"),
        }
    }

    pub fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error> {
        let vault = self.client.organization("org_123").vault("vlt_456");
        self.runtime.block_on(vault.check(subject, permission, resource))
    }
}
```

**Pattern 2: `spawn_blocking`** (when already in async context)

```rust
// From sync code that's called within an async context
let result = tokio::task::spawn_blocking(move || {
    tokio::runtime::Handle::current().block_on(async {
        vault.check("user:alice", "view", "doc:1").await
    })
}).await??;
```

**Pattern 3: Handle from sync callback** - When you have a handle to an existing runtime:

```rust
fn sync_callback(handle: &tokio::runtime::Handle, vault: &VaultClient) -> Result<bool, Error> {
    handle.block_on(vault.check("user:alice", "view", "doc:1"))
}
```

### Why No Sync Feature Flag?

We considered a `sync` feature that would provide blocking wrappers, but decided against it:

| Approach                            | Problem                                                 |
| ----------------------------------- | ------------------------------------------------------- |
| `#[cfg(feature = "sync")]` wrappers | Hidden `block_on()` panics if called from async context |
| Separate `inferadb-sync` crate      | Maintenance burden, API drift, user confusion           |
| Codegen both variants               | Complexity, larger binary, documentation burden         |

The patterns above give full control to users who need blocking behavior, without the SDK making assumptions about runtime context.

---

## Typestate Builder Pattern

Use phantom types to enforce required field presence at compile time. Semantic validation (valid URLs, non-zero timeouts, pool sizes) happens at `build()` time.

### Typestate Design Goals

1. **Compile-time enforcement** - Missing required fields don't compile (presence, not validity)
2. **Clear error messages** - Type errors indicate what's missing
3. **IDE support** - Autocomplete shows only valid next steps
4. **Semantic validation at build** - Value correctness checked when `build()` is called

### Type States

```rust
// Marker types for builder state
mod state {
    pub struct NoUrl;
    pub struct HasUrl;
    pub struct NoAuth;
    pub struct HasAuth;
}

#[must_use = "ClientBuilder does nothing until .build() is called"]
pub struct ClientBuilder<Url, Auth> {
    url: Option<String>,
    auth: Option<AuthConfig>,
    // ... other fields
    _marker: PhantomData<(Url, Auth)>,
}
```

### State Transitions

```rust
impl ClientBuilder<NoUrl, NoAuth> {
    pub fn new() -> Self { /* ... */ }
}

impl<Auth> ClientBuilder<NoUrl, Auth> {
    // Setting URL transitions NoUrl -> HasUrl
    // Note: Validation deferred to build() - method is infallible
    pub fn url(self, url: impl Into<String>) -> ClientBuilder<HasUrl, Auth> {
        ClientBuilder {
            url: Some(url.into()),
            auth: self.auth,
            _marker: PhantomData,
        }
    }
}

impl<Url> ClientBuilder<Url, NoAuth> {
    // Setting credentials transitions NoAuth -> HasAuth
    pub fn credentials(self, creds: impl Into<Credentials>)
        -> ClientBuilder<Url, HasAuth>
    {
        ClientBuilder {
            auth: Some(creds.into()),
            ..self
        }
    }
}

// build() is ONLY available when all required states are satisfied
impl ClientBuilder<HasUrl, HasAuth> {
    pub async fn build(self) -> Result<Client, BuildError> {
        // All required fields guaranteed present by type system
        // Validation happens here, not in setter methods

        // Validate URL format
        let url = Url::parse(&self.url.unwrap())
            .map_err(BuildError::InvalidUrl)?;

        // Validate other configuration...

        Ok(Client { /* ... */ })
    }

    pub fn build_sync(self) -> Result<Client, BuildError> {
        // Blocking variant - same validation
    }
}
```

### Compile-Time Errors

```rust
// ✅ Compiles - all required fields set
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;  // Only place where errors can occur

// ❌ Compile error: build() not available without auth
let client = Client::builder()
    .url("https://api.inferadb.com")
    .build()  // Error: method `build` not found for `ClientBuilder<HasUrl, NoAuth>`
    .await?;

// ❌ Compile error: build() not available without URL
let client = Client::builder()
    .credentials(creds)
    .build()  // Error: method `build` not found for `ClientBuilder<NoUrl, HasAuth>`
    .await?;
```

### Optional Fields

Optional configuration doesn't affect type state:

```rust
impl<Url, Auth> ClientBuilder<Url, Auth> {
    // These don't change type state - always available
    pub fn pool_size(mut self, size: usize) -> Self { /* ... */ }
    pub fn connect_timeout(mut self, timeout: Duration) -> Self { /* ... */ }
    pub fn retries(mut self, config: RetryConfig) -> Self { /* ... */ }
}
```

### Validation Model

The builder uses a two-tier validation approach:

| Level                   | When              | What's Checked                      | Mechanism                    |
| ----------------------- | ----------------- | ----------------------------------- | ---------------------------- |
| **Compile-time**        | Type checking     | Required field presence (URL, auth) | Typestate phantom types      |
| **Runtime (`build()`)** | At `build()` call | Semantic correctness                | `Result<Client, BuildError>` |

**Compile-time (typestate enforced):**

- URL must be provided (presence, not format)
- Auth configuration must be provided (presence, not validity)

**Runtime (`build()` validates):**

- URL format is valid and parseable
- Timeouts are non-zero
- Pool sizes are within bounds (1-1000)
- Retry attempts are reasonable (0-10)
- Auth credentials are properly formatted

**Design rationale:** We chose not to encode semantic invariants (like `NonZeroU32` for pool sizes) into the type system because:

1. It adds API friction for minimal benefit
2. Error messages at `build()` are clear and actionable
3. Most validation requires runtime context (URL resolution, key file access)

---

## Authentication

### Design Decision: OAuth 2.0 JWT Bearer

We use RFC 7523 (JWT Bearer Client Authentication) for service-to-service auth:

**Rationale:**

- Stateless token validation (no token introspection needed)
- Ed25519 signatures (fast, small, secure)
- Self-service key rotation via certificate management
- Industry standard (works with existing infrastructure)

### Authentication Flow

```text
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│ SDK Client  │         │  Token API  │         │ Engine API  │
└──────┬──────┘         └──────┬──────┘         └──────┬──────┘
       │                       │                       │
       │ 1. Create JWT assertion                       │
       │ (client_id + private_key)                     │
       │──────────────────────►│                       │
       │                       │                       │
       │ 2. Exchange for access token                  │
       │◄──────────────────────│                       │
       │ {access_token, refresh_token, expires_in}     │
       │                       │                       │
       │ 3. API call with Bearer token                 │
       │───────────────────────────────────────────────►
       │                       │                       │
       │ 4. Response                                   │
       │◄──────────────────────────────────────────────│
       │                       │                       │
       │ [Background: Refresh before expiry]           │
       │──────────────────────►│                       │
       │◄──────────────────────│                       │
```

### Client Assertion JWT

```rust
// JWT Claims
{
  "iss": "client_id",           // Client identifier
  "sub": "client_id",           // Subject (same as issuer for client creds)
  "aud": "https://api.inferadb.com/control/v1/auth/token",
  "iat": 1699000000,            // Issued at
  "exp": 1699000300,            // Expires (5 min max)
  "jti": "unique-request-id"    // Prevents replay
}

// Signature: EdDSA with Ed25519 private key
```

### Token Refresh Strategy

```rust
// Proactive refresh (default)
// Refresh when token is 80% through its lifetime
let client = Client::builder()
    .refresh(RefreshConfig::default().threshold_ratio(0.8))  // Refresh at 80% of lifetime
    .build()
    .await?;

// Background refresh task
// SDK spawns a task to refresh tokens before expiry
// No request ever blocks on token refresh
```

### Token Refresh Semantics

Detailed behavior of the token refresh system:

**Cold Start Behavior**:

```rust
// On first API call after client creation:
// 1. SDK has no token yet (cold start)
// 2. First request triggers token acquisition
// 3. Request waits for token (blocking on auth)
// 4. Subsequent requests use cached token

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;  // Does NOT acquire token (lazy)

// First call triggers token acquisition
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
let result = vault.check("user:alice", "view", "doc:1").await?;
//           ^^^^^^ This blocks waiting for token

// Optional: Pre-warm token during startup
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .eager_connect(true)  // Pre-acquires token during build()
    .build()
    .await?;  // Blocks until token is acquired
```

**Singleflight Token Refresh**:

When multiple requests arrive during a cold start or refresh, only one refresh operation executes:

```rust
// Internal implementation uses singleflight pattern
struct TokenManager {
    token: RwLock<Option<CachedToken>>,
    refresh_in_flight: Mutex<Option<Shared<BoxFuture<'static, Result<Token, Error>>>>>,
}

impl TokenManager {
    async fn get_token(&self) -> Result<String, Error> {
        // Fast path: valid cached token
        if let Some(token) = self.get_cached_if_valid() {
            return Ok(token);
        }

        // Slow path: need refresh - use singleflight
        self.refresh_with_singleflight().await
    }

    async fn refresh_with_singleflight(&self) -> Result<String, Error> {
        let mut in_flight = self.refresh_in_flight.lock().await;

        // If refresh already in progress, join it
        if let Some(fut) = in_flight.as_ref() {
            return fut.clone().await;
        }

        // Start new refresh, share future with waiters
        let fut = self.do_refresh().boxed().shared();
        *in_flight = Some(fut.clone());

        let result = fut.await;

        // Clear in-flight state
        *in_flight = None;

        result
    }
}
```

**Refresh Failure Handling**:

```rust
// Scenario 1: Background refresh fails, but token still valid
// - SDK logs warning, continues using current token
// - Next background refresh retries

// Scenario 2: Background refresh fails, token expired
// - Next API call triggers synchronous refresh attempt
// - If sync refresh fails, API call returns AuthError

// Scenario 3: Credentials invalid (key revoked, client deleted)
// - Refresh returns 401/403
// - SDK propagates error, does not retry with same credentials

// Configure retry behavior for token refresh
let client = Client::builder()
    .refresh(RefreshConfig {
        // When to trigger refresh
        threshold_ratio: 0.9,                       // Refresh at 90% of lifetime
        min_remaining: Duration::from_secs(60),     // Or when <1 min remaining

        // Grace period during refresh
        grace_period: Duration::from_secs(10),

        // Retry settings for refresh attempts
        max_retries: 3,
        retry_backoff: Duration::from_millis(100),
        retry_on_auth_failure: false,               // Don't retry 401/403
    })
    .build()
    .await?;
```

**Token Refresh Timeline**:

```text
Token acquired at T=0, expires at T=300s, threshold=0.8

T=0       T=240     T=270     T=290     T=300
│         │         │         │         │
▼         ▼         ▼         ▼         ▼
[───valid──┼──refresh─┼──grace──┼─expired─]
           │ window   │ period  │
           │          │         │
           │          │         └─ Requests fail if no new token
           │          └─ Requests use old token while refresh in-flight
           └─ Background refresh starts (80% of lifetime)
```

**Observability**:

```rust
// Token refresh events are traced
// Spans: inferadb.token.refresh, inferadb.token.acquire
// Metrics:
//   - inferadb_token_refresh_total{status="success|failure"}
//   - inferadb_token_refresh_duration_seconds
//   - inferadb_token_expiry_seconds (gauge, time until expiry)
```

### Credential Types

The `.credentials()` method accepts any type that implements `Into<Credentials>`, supporting both static credentials and dynamic providers:

```rust
/// Unified credentials source for authentication
pub enum Credentials {
    /// Static client credentials (service-to-service authentication)
    Client(ClientCredentialsConfig),
    /// Static bearer token (user-initiated requests)
    Bearer(BearerCredentialsConfig),
    /// API key authentication (simpler, for WASM/browser environments)
    ApiKey(String),
    /// Dynamic provider for key rotation, secrets managers, HSMs
    Provider(Arc<dyn CredentialsProvider>),
}

impl Credentials {
    /// Create credentials from an API key (for WASM/browser environments)
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(key.into())
    }

    /// Create credentials from a bearer token
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(BearerCredentialsConfig { token: token.into() })
    }
}

/// Client credentials using Ed25519 private key
pub struct ClientCredentialsConfig {
    pub client_id: String,
    pub private_key: Ed25519PrivateKey,
    pub certificate_id: Option<String>,
}

/// Bearer token credentials
pub struct BearerCredentialsConfig {
    pub token: String,
}

// Convenience: From implementations for ergonomic usage
impl From<ClientCredentialsConfig> for Credentials { ... }
impl From<BearerCredentialsConfig> for Credentials { ... }
impl<T: CredentialsProvider> From<T> for Credentials { ... }
```

**Usage Examples**:

```rust
// Client credentials from file (service-to-service)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: None,  // Auto-detect from JWKS
    })
    .build()
    .await?;

// Client credentials from bytes
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem(include_bytes!("key.pem"))?,
        certificate_id: Some("kid-123".into()),  // Specific key ID
    })
    .build()
    .await?;

// Bearer token (user-initiated requests)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9...".into(),
    })
    .build()
    .await?;
```

### Custom JWT Claims

For advanced use cases, inject custom claims into client assertions:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .custom_claims(|claims| {
        claims.insert("environment", "production");
        claims.insert("region", "us-east-1");
        claims.insert("deployment_id", "deploy-abc123");
    })
    .build()
    .await?;
```

### CredentialsProvider Abstraction

For dynamic credential management (key rotation, secrets managers, HSMs), implement the `CredentialsProvider` trait and pass it to `.credentials()`:

```rust
/// Trait for dynamic credential resolution.
/// Implement this for custom key management integrations.
#[async_trait]
pub trait CredentialsProvider: Send + Sync + 'static {
    /// Returns the current credentials for signing.
    /// Called before each token refresh, allowing for key rotation.
    async fn get_credentials(&self) -> Result<ClientCredentialsConfig, CredentialsError>;

    /// Optional: Called when credentials fail authentication.
    /// Allows providers to invalidate cached credentials.
    async fn on_auth_failure(&self, _error: &AuthError) {
        // Default: no-op
    }
}

/// Error type for credential providers
#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    /// Credentials not found (e.g., missing environment variable, secret not in vault)
    #[error("credentials not found: {0}")]
    NotFound(String),

    /// Failed to load or parse credentials
    #[error("failed to load credentials: {0}")]
    LoadError(String),

    /// Credentials provider is unavailable (e.g., network error to secrets manager)
    #[error("credentials provider unavailable: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Credentials have been revoked or are invalid
    #[error("credentials revoked or invalid")]
    Revoked,

    /// Rate limited by credentials provider
    #[error("credentials provider rate limited, retry after {0:?}")]
    RateLimited(Option<Duration>),

    /// Provider-specific error (e.g., AWS SDK error, Vault API error)
    #[error("credentials provider error: {0}")]
    ProviderError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Credentials format is invalid or unparseable
    #[error("invalid credentials format: {0}")]
    InvalidFormat(String),
}
```

**Built-in Implementations**:

```rust
// Static credentials (default) - credentials never change
impl CredentialsProvider for ClientCredentialsConfig {
    async fn get_credentials(&self) -> Result<ClientCredentialsConfig, CredentialsError> {
        Ok(self.clone())
    }
}
```

**AWS Secrets Manager Integration**:

```rust
/// Expected JSON structure for credentials stored in AWS Secrets Manager.
/// Customize this to match your secret format.
#[derive(Debug, Deserialize)]
struct SecretsPayload {
    client_id: String,
    private_key: String,  // PEM-encoded Ed25519 private key
    #[serde(default)]
    certificate_id: Option<String>,
}

pub struct AwsSecretsProvider {
    client: aws_sdk_secretsmanager::Client,
    secret_id: String,
    cache: RwLock<Option<(ClientCredentialsConfig, Instant)>>,
    cache_ttl: Duration,
}

#[async_trait]
impl CredentialsProvider for AwsSecretsProvider {
    async fn get_credentials(&self) -> Result<ClientCredentialsConfig, CredentialsError> {
        // Check cache first
        if let Some((creds, fetched_at)) = &*self.cache.read().await {
            if fetched_at.elapsed() < self.cache_ttl {
                return Ok(creds.clone());
            }
        }

        // Fetch from Secrets Manager
        let secret = self.client
            .get_secret_value()
            .secret_id(&self.secret_id)
            .send()
            .await
            .map_err(|e| CredentialsError::ProviderError(e.into()))?;

        let secret_string = secret.secret_string()
            .ok_or(CredentialsError::InvalidFormat("missing secret_string".into()))?;

        let parsed: SecretsPayload = serde_json::from_str(secret_string)
            .map_err(|e| CredentialsError::InvalidFormat(e.to_string()))?;

        let creds = ClientCredentialsConfig {
            client_id: parsed.client_id,
            private_key: Ed25519PrivateKey::from_pem(parsed.private_key.as_bytes())?,
            certificate_id: parsed.certificate_id,
        };

        // Update cache
        *self.cache.write().await = Some((creds.clone(), Instant::now()));

        Ok(creds)
    }

    async fn on_auth_failure(&self, _error: &AuthError) {
        // Invalidate cache on auth failure to force re-fetch
        *self.cache.write().await = None;
    }
}
```

**Usage with Client Builder**:

```rust
// Static client credentials
let client = Client::builder()
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: None,
    })
    .build()
    .await?;

// Dynamic credentials via AWS Secrets Manager
let client = Client::builder()
    .credentials(AwsSecretsProvider::new(
        secrets_client,
        "inferadb/production/credentials",
    ))
    .build()
    .await?;

// Bearer token
let client = Client::builder()
    .credentials(BearerCredentialsConfig {
        token: user_token.into(),
    })
    .build()
    .await?;
```

**Key Rotation Without Restart**:

```rust
// With CredentialsProvider, key rotation happens automatically:
// 1. Deploy new key to secrets manager
// 2. On next token refresh, SDK calls get_credentials()
// 3. New key is used for signing
// 4. Old key remains valid until its tokens expire

// For zero-downtime rotation:
// 1. Add new key to secrets (don't remove old yet)
// 2. SDK picks up new key on refresh
// 3. Wait for old tokens to expire (typically 5-15 min)
// 4. Remove old key from secrets
```

---

## Connection Management

### Connection Pooling

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    // Pool configuration
    .pool_size(20)                          // Max connections per host
    .idle_timeout(Duration::from_secs(60))  // Close idle connections
    .build()
    .await?;
```

**Pool Configuration Options**:

| Option              | Default     | Description                                     |
| ------------------- | ----------- | ----------------------------------------------- |
| `pool_size`         | 10          | Maximum number of connections per host          |
| `idle_timeout`      | 90 seconds  | Time before idle connections are closed         |
| `max_idle_per_host` | `pool_size` | Maximum idle connections to retain per host     |
| `pool_timeout`      | 30 seconds  | Max time to wait for a connection from the pool |
| `http2_only`        | false       | Force HTTP/2 (required for gRPC)                |
| `http2_keepalive`   | 20 seconds  | HTTP/2 keepalive ping interval                  |

```rust
// Full pool configuration example
let client = Client::builder()
    .url("https://api.inferadb.com")
    .pool_size(50)                               // High-throughput application
    .idle_timeout(Duration::from_secs(120))      // Keep connections longer
    .max_idle_per_host(20)                       // Don't keep all 50 idle
    .pool_timeout(Duration::from_secs(10))       // Fail fast if pool exhausted
    .http2_only(true)                            // Use HTTP/2 multiplexing
    .http2_keepalive(Duration::from_secs(30))    // Keep HTTP/2 connections alive
    .build()
    .await?;
```

**Pool Sizing Guidance**:

| Scenario                      | pool_size | Notes                                 |
| ----------------------------- | --------- | ------------------------------------- |
| Low traffic (< 10 req/s)      | 5-10      | Default is usually sufficient         |
| Medium traffic (10-100 req/s) | 20-50     | Balance between latency and resources |
| High traffic (> 100 req/s)    | 50-100    | HTTP/2 multiplexing helps here        |
| Burst-heavy                   | 100+      | Size for peak, idle_timeout cleans up |

### Connection Lifecycle Diagram

```text
┌──────────────────────────────────────────────────────────────────────┐
│                         Connection Pool                               │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  Request arrives                                                      │
│       │                                                               │
│       ▼                                                               │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐ │
│  │ Check pool for  │ YES │ Validate conn   │ OK  │ Return to       │ │
│  │ idle connection │────►│ (health check)  │────►│ caller          │ │
│  └────────┬────────┘     └────────┬────────┘     └─────────────────┘ │
│           │ NO                    │ FAILED                           │
│           ▼                       ▼                                  │
│  ┌─────────────────┐     ┌─────────────────┐                        │
│  │ Pool at max?    │     │ Discard & retry │                        │
│  └────────┬────────┘     └─────────────────┘                        │
│      YES  │  NO                                                      │
│       ▼   ▼                                                          │
│  ┌─────────────────┐     ┌─────────────────┐                        │
│  │ Wait for conn   │     │ Create new      │                        │
│  │ (with timeout)  │     │ connection      │                        │
│  └─────────────────┘     └────────┬────────┘                        │
│                                   │                                  │
│                                   ▼                                  │
│                          ┌─────────────────┐                        │
│                          │ TLS handshake   │                        │
│                          │ (if HTTPS)      │                        │
│                          └────────┬────────┘                        │
│                                   │                                  │
│                                   ▼                                  │
│                          ┌─────────────────┐                        │
│                          │ Return to pool  │                        │
│                          │ after request   │                        │
│                          └─────────────────┘                        │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Keep-Alive and Timeouts

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    // TCP keep-alive
    .tcp_keepalive(Duration::from_secs(60))
    // HTTP/2 keep-alive ping
    .http2_keepalive_interval(Duration::from_secs(30))
    .http2_keepalive_timeout(Duration::from_secs(10))
    // Connection timeouts
    .connect_timeout(Duration::from_secs(10))
    .request_timeout(Duration::from_secs(30))
    .build()
    .await?;
```

### TLS Configuration

Configure TLS for security requirements, including custom CA certificates and mutual TLS (mTLS).

**Custom CA Certificates**:

```rust
// Add custom CA certificates (for private PKI or self-signed certs)
let client = Client::builder()
    .url("https://inferadb.internal.corp")
    .tls(TlsConfig::new()
        .add_root_certificate(Certificate::from_pem(include_bytes!("corp-ca.pem"))?))
    .build()
    .await?;

// Load from system trust store plus custom CA
let client = Client::builder()
    .url("https://inferadb.internal.corp")
    .tls(TlsConfig::new()
        .with_native_roots()  // Include system CAs
        .add_root_certificate(Certificate::from_pem(corp_ca_bytes)?))
    .build()
    .await?;
```

**Mutual TLS (mTLS)**:

```rust
// Client certificate authentication (for zero-trust environments)
let client = Client::builder()
    .url("https://inferadb.secure.corp")
    .tls(TlsConfig::new()
        .client_identity(Identity::from_pem(
            include_bytes!("client-cert.pem"),
            include_bytes!("client-key.pem"),
        )?))
    .build()
    .await?;

// mTLS with custom CA
let client = Client::builder()
    .url("https://inferadb.secure.corp")
    .tls(TlsConfig::new()
        .add_root_certificate(Certificate::from_pem(ca_cert)?)
        .client_identity(Identity::from_pem(client_cert, client_key)?))
    .build()
    .await?;
```

**TLS Configuration Types**:

````rust
/// TLS configuration for secure connections
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Additional root certificates to trust
    root_certificates: Vec<Certificate>,
    /// Client identity for mTLS
    client_identity: Option<Identity>,
    /// Include system/native root certificates
    use_native_roots: bool,
    /// Minimum TLS version (default: 1.2)
    min_version: TlsVersion,
    /// Enable certificate hostname verification (default: true)
    verify_hostname: bool,
    /// Enable server certificate verification (default: true)
    /// WARNING: Disabling this is insecure and should only be used for development
    verify_server: bool,
    /// ALPN protocols (default: ["h2", "http/1.1"])
    alpn_protocols: Vec<String>,
}

impl TlsConfig {
    pub fn new() -> Self {
        Self {
            root_certificates: Vec::new(),
            client_identity: None,
            use_native_roots: true,
            min_version: TlsVersion::Tls12,
            verify_hostname: true,
            verify_server: true,
            alpn_protocols: vec!["h2".into(), "http/1.1".into()],
        }
    }

    /// Create an insecure TLS config that skips server certificate verification.
    /// WARNING: Only use for local development with self-signed certificates.
    #[cfg(feature = "insecure")]
    pub fn insecure() -> Self {
        Self {
            root_certificates: Vec::new(),
            client_identity: None,
            use_native_roots: false,
            min_version: TlsVersion::Tls12,
            verify_hostname: false,
            verify_server: false,
            alpn_protocols: vec!["h2".into(), "http/1.1".into()],
        }
    }

    /// Add a root certificate to trust
    pub fn add_root_certificate(mut self, cert: Certificate) -> Self {
        self.root_certificates.push(cert);
        self
    }

    /// Set client identity for mTLS
    pub fn client_identity(mut self, identity: Identity) -> Self {
        self.client_identity = Some(identity);
        self
    }

    /// Include system root certificates (default: true)
    pub fn with_native_roots(mut self) -> Self {
        self.use_native_roots = true;
        self
    }

    /// Use only explicitly added certificates
    pub fn without_native_roots(mut self) -> Self {
        self.use_native_roots = false;
        self
    }

    /// Set minimum TLS version
    pub fn min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// TLS version requirement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

/// A PEM-encoded certificate
#[derive(Debug, Clone)]
pub struct Certificate(Vec<u8>);

impl Certificate {
    pub fn from_pem(pem: &[u8]) -> Result<Self, Error> {
        // Validate PEM format
        Ok(Self(pem.to_vec()))
    }

    pub fn from_der(der: &[u8]) -> Result<Self, Error> {
        Ok(Self(der.to_vec()))
    }
}

/// A client identity (certificate + private key) for mTLS
#[derive(Debug, Clone)]
pub struct Identity {
    cert: Vec<u8>,
    key: Vec<u8>,
}

impl Identity {
    pub fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            cert: cert_pem.to_vec(),
            key: key_pem.to_vec(),
        })
    }

    /// Load identity from a PKCS#12/PFX bundle.
    ///
    /// **Note**: PKCS#12 support is planned for a future release.
    /// For now, convert PKCS#12 to PEM format using OpenSSL:
    /// ```bash
    /// openssl pkcs12 -in cert.p12 -out cert.pem -nodes
    /// ```
    pub fn from_pkcs12(_pkcs12: &[u8], _password: &str) -> Result<Self, Error> {
        Err(Error::new(
            ErrorKind::ConfigurationError,
            "PKCS#12 support not yet available. Convert to PEM format using: \
             openssl pkcs12 -in cert.p12 -out cert.pem -nodes"
        ))
    }
}
````

### Proxy Configuration

Configure HTTP/HTTPS/SOCKS proxy for corporate network environments.

```rust
// HTTP proxy
let client = Client::builder()
    .url("https://api.inferadb.com")
    .proxy(Proxy::http("http://proxy.corp:8080")?)
    .build()
    .await?;

// HTTPS proxy
let client = Client::builder()
    .url("https://api.inferadb.com")
    .proxy(Proxy::https("http://proxy.corp:8080")?)
    .build()
    .await?;

// SOCKS5 proxy
let client = Client::builder()
    .url("https://api.inferadb.com")
    .proxy(Proxy::socks5("socks5://proxy.corp:1080")?)
    .build()
    .await?;

// Proxy with authentication
let client = Client::builder()
    .url("https://api.inferadb.com")
    .proxy(Proxy::https("http://proxy.corp:8080")?
        .basic_auth("username", "password"))
    .build()
    .await?;

// No proxy for specific hosts
let client = Client::builder()
    .url("https://api.inferadb.com")
    .proxy(Proxy::https("http://proxy.corp:8080")?
        .no_proxy("localhost,127.0.0.1,.internal.corp"))
    .build()
    .await?;
```

**Proxy Types**:

```rust
/// Proxy configuration
#[derive(Debug, Clone)]
pub struct Proxy {
    /// Proxy URL
    url: Url,
    /// Proxy type
    kind: ProxyKind,
    /// Authentication credentials
    auth: Option<ProxyAuth>,
    /// Hosts to bypass proxy
    no_proxy: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProxyKind {
    Http,
    Https,
    Socks5,
}

impl Proxy {
    /// HTTP proxy (for HTTP connections)
    pub fn http(url: &str) -> Result<Self, Error>;

    /// HTTPS proxy (for HTTPS connections via CONNECT)
    pub fn https(url: &str) -> Result<Self, Error>;

    /// SOCKS5 proxy
    pub fn socks5(url: &str) -> Result<Self, Error>;

    /// Add basic authentication
    pub fn basic_auth(mut self, username: &str, password: &str) -> Self;

    /// Set hosts to bypass proxy (comma-separated)
    pub fn no_proxy(mut self, hosts: &str) -> Self;
}
```

### Client Cloning Semantics

The `Client` is designed to be cheaply cloned and shared across threads:

```rust
// Client uses Arc internally - cloning is O(1)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

// Cheap clone - shares connection pool, auth manager, config
let client2 = client.clone();  // Increments Arc refcount only

// Safe to move across threads
tokio::spawn({
    let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
    async move {
        vault.check("user:alice", "view", "doc:1").await
    }
});

// Safe to share via Arc (but Clone is already cheap)
let shared: Arc<Client> = Arc::new(client);
```

#### Internal Structure

```rust
/// Client is a thin wrapper around Arc<Inner>
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    config: ClientConfig,
    transport: Transport,          // Connection pool (shared)
    auth_manager: AuthManager,     // Token cache (shared)
    cache: Option<DecisionCache>,  // Decision cache (shared)
}
```

#### Clone Behavior Summary

| Component       | Cloning Behavior       |
| --------------- | ---------------------- |
| Connection pool | Shared (Arc)           |
| Auth tokens     | Shared (`Arc<RwLock>`) |
| Decision cache  | Shared (Arc)           |
| Config          | Shared (Arc)           |
| Metrics         | Shared (Arc)           |

#### Thread Safety

```rust
// Client is Send + Sync
fn assert_send_sync<T: Send + Sync>() {}
assert_send_sync::<Client>();

// Safe patterns
static CLIENT: OnceLock<Client> = OnceLock::new();

// In Axum
#[derive(Clone)]
struct AppState {
    client: Client,  // Clone is cheap
}

// In Actix
let client = web::Data::new(client);  // Wraps in Arc unnecessarily but harmless
```

---

## Health Check & Lifecycle

Production applications require health checking and graceful shutdown.

### Health Check API

```rust
// Simple health check - verifies connectivity
let healthy = client.health_check().await?;

// Detailed health with component status
let health = client.health().await?;
println!("Status: {:?}", health.status);  // Healthy, Degraded, Unhealthy
println!("Latency: {:?}", health.latency);
println!("Components: {:?}", health.components);

// Blocking wait for readiness (for startup orchestration)
client.wait_ready(Duration::from_secs(30)).await?;

// With custom readiness criteria
client.wait_ready_with(ReadinessConfig {
    timeout: Duration::from_secs(30),
    check_interval: Duration::from_millis(100),
    require_auth: true,      // Verify token exchange works
    require_vault: true,     // Verify vault is accessible
}).await?;
```

### Health Response Structure

```rust
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub latency: Duration,
    pub components: HashMap<String, ComponentHealth>,
    pub timestamp: DateTime<Utc>,
}

impl HealthResponse {
    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,  // Partial functionality
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency: Option<Duration>,
    pub last_check: DateTime<Utc>,
}
```

### Graceful Shutdown

```rust
use tokio::signal;

// Create client with shutdown handle
let (client, shutdown_handle) = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build_with_shutdown()
    .await?;

// In shutdown handler
tokio::select! {
    _ = signal::ctrl_c() => {
        println!("Shutting down...");

        // Graceful shutdown - drains in-flight requests
        shutdown_handle.shutdown().await;

        // Or with timeout
        shutdown_handle.shutdown_timeout(Duration::from_secs(30)).await;
    }
    _ = run_server(client) => {}
}
```

### Shutdown Semantics

```rust
impl ShutdownHandle {
    /// Initiate graceful shutdown
    /// - Stops accepting new requests
    /// - Waits for in-flight requests to complete
    /// - Closes connections cleanly
    pub async fn shutdown(self) {
        self.inner.shutdown().await
    }

    /// Shutdown with deadline
    /// - After timeout, forcefully closes connections
    pub async fn shutdown_timeout(self, timeout: Duration) {
        tokio::select! {
            _ = self.inner.shutdown() => {}
            _ = tokio::time::sleep(timeout) => {
                self.inner.force_shutdown();
            }
        }
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.inner.is_shutting_down()
    }
}
```

**Shutdown Phases**:

| Phase              | Client Behavior                                          | In-Flight Requests     |
| ------------------ | -------------------------------------------------------- | ---------------------- |
| Normal             | Accepts new requests, processes normally                 | Continue               |
| Shutdown initiated | Rejects new requests with `Error { kind: ShuttingDown }` | Continue to completion |
| Draining           | No new requests, waiting for in-flight                   | Complete or timeout    |
| Force shutdown     | Closes all connections immediately                       | Cancelled with error   |
| Complete           | All resources released                                   | N/A                    |

**Drop Behavior**:

If a `Client` is dropped without explicit shutdown, in-flight requests may be cancelled abruptly. For production applications, always use explicit shutdown:

```rust
// ❌ Bad: abrupt drop may cancel in-flight requests
{
    let client = Client::builder().build().await?;
    // ... use client ...
}  // Dropped here - connections may close abruptly

// ✅ Good: explicit shutdown with timeout
let (client, shutdown) = Client::builder().build_with_shutdown().await?;
// ... use client ...
shutdown.shutdown_timeout(Duration::from_secs(30)).await;
```

**Shutdown Notification**:

Check if the client is shutting down to avoid starting new work:

```rust
async fn process_request(client: &Client, vault: &VaultClient) -> Result<(), Error> {
    if client.is_shutting_down() {
        return Err(Error::shutting_down());
    }

    // Proceed with request
    vault.check("user:alice", "view", "doc:1").await
}
```

### Integration with Kubernetes

```rust
// Liveness probe endpoint
async fn liveness(client: &Client) -> impl IntoResponse {
    // Simple connectivity check
    match client.health_check().await {
        Ok(true) => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

// Readiness probe endpoint
async fn readiness(client: &Client) -> impl IntoResponse {
    match client.health().await {
        Ok(health) if health.status == HealthStatus::Healthy => StatusCode::OK,
        Ok(health) if health.status == HealthStatus::Degraded => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
```

---

## Vault Scoping

All authorization operations require explicit vault specification to prevent accidental cross-vault operations. The SDK uses an organization-first hierarchy where vaults are accessed through their owning organization.

### Quick Start

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

// Organization-first: get org context, then vault
let org = client.organization("org_8675309...");
let vault = org.vault("vlt_01JFQGK...");

// Authorization operations
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Management operations on same vault
let schema = vault.schemas().get_active().await?;
```

### API Hierarchy

The SDK uses a unified hierarchy where operations flow through organizations and vaults:

| Method             | Returns              | Use Case                                  |
| ------------------ | -------------------- | ----------------------------------------- |
| `organization(id)` | `OrganizationClient` | Organization context for all child ops    |
| `org.vault(id)`    | `VaultClient`        | Unified vault: authorization + management |
| `org.vaults()`     | `VaultsClient`       | List/create vaults in organization        |
| `org.members()`    | `MembersClient`      | Organization membership management        |
| `account()`        | `AccountClient`      | Current user account operations           |

The `VaultClient` type provides both authorization and management operations:

| Category      | Methods                                                                           |
| ------------- | --------------------------------------------------------------------------------- |
| Authorization | `check()`, `check_batch()`, `expand()`                                            |
| Resources     | `resources().accessible_by().with_permission()` (extensible for resource queries) |
| Subjects      | `subjects().with_permission().on_resource()` (extensible for subject queries)     |
| Relationships | `relationships().list()`, `.write()`, `.delete()` (full CRUD)                     |
| Streaming     | `watch()`, `simulate()`                                                           |
| Management    | `schemas()`, `tokens()`, `roles()`, `get()`, `update()`, `delete()`               |

### Single Operation

```rust
// Inline for one-off operations
let allowed = client
    .organization("org_8675309...")
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "doc:1")
    .await?;
```

### Multiple Operations (Same Vault)

```rust
// Store vault for multiple operations
let org = client.organization("org_8675309...");
let production = org.vault("vlt_01JFQGK...");

// Authorization operations
production.check("user:alice", "view", "doc:1").await?;
production.relationships().write(Relationship::new("doc:1", "viewer", "user:bob")).await?;

// Management operations on same vault
let schema = production.schemas().get_active().await?;
production.schemas().push(new_schema).await?;

// Different vault for staging (same org)
let staging = org.vault("vlt_02STAGING...");
staging.check("user:alice", "view", "doc:1").await?;
```

### Multiple Organizations

```rust
// Work with multiple organizations
let acme = client.organization("org_acme...");
let globex = client.organization("org_globex...");

// Each org has isolated vaults
let acme_prod = acme.vault("vlt_acme_prod...");
let globex_prod = globex.vault("vlt_globex_prod...");

// Operations are scoped to their respective org/vault
acme_prod.check("user:alice", "view", "doc:1").await?;
globex_prod.check("user:bob", "view", "doc:1").await?;
```

### Vault Design

`VaultClient` is owned and cheaply cloneable (uses `Arc` internally, like `Client`):

```rust
/// A unified client scoped to a specific vault.
/// Provides both authorization and management operations.
/// Cheaply cloneable - can be stored, passed to tasks, shared across threads.
#[derive(Clone)]
pub struct VaultClient {
    inner: Client,
    org_id: String,
    vault_id: String,
}

impl OrganizationClient {
    /// Create a vault-scoped client for authorization and management
    pub fn vault(&self, vault_id: impl Into<String>) -> VaultClient {
        VaultClient {
            inner: self.inner.clone(),  // Cheap Arc clone
            org_id: self.org_id.clone(),
            vault_id: vault_id.into(),
        }
    }
}

impl VaultClient {
    // Accessors for logging, debugging, and context propagation
    /// Returns the organization ID this client is scoped to.
    pub fn organization_id(&self) -> &str { &self.org_id }

    /// Returns the vault ID this client is scoped to.
    pub fn vault_id(&self) -> &str { &self.vault_id }

    // Authorization operations
    /// Standard check - returns 'static builder (Arc clone internally)
    pub fn check(&self, subject: &str, permission: &str, resource: &str) -> CheckRequest<'static>;

    /// Borrowed check - zero allocation, requires lifetime management (expert API)
    pub fn check_borrowed<'a>(&'a self, subject: &'a str, permission: &'a str, resource: &'a str) -> CheckRequest<'a>;

    /// Owned check - takes ownership of strings for 'static lifetime
    pub async fn check_owned(&self, subject: String, permission: String, resource: String) -> Result<bool, Error>;

    /// Batch check - returns stream of results
    pub fn check_batch(&self, checks: impl IntoIterator<Item = Check>) -> CheckBatchStream;
    pub fn expand(&self, resource: &str, relation: &str) -> ExpandBuilder;
    pub fn watch(&self) -> WatchBuilder;
    pub fn simulate(&self) -> SimulateBuilder;

    // Resource, subject, and relationship queries (extensible sub-clients)
    pub fn resources(&self) -> ResourcesClient<'_>;
    pub fn subjects(&self) -> SubjectsClient<'_>;
    pub fn relationships(&self) -> RelationshipsClient<'_>;

    // Management operations
    pub fn schemas(&self) -> SchemaClient;
    pub fn tokens(&self) -> TokensClient;
    pub fn roles(&self) -> RolesClient;
    pub async fn get(&self) -> Result<VaultInfo, Error>;
    pub async fn update(&self, update: UpdateVault) -> Result<VaultInfo, Error>;
    pub async fn delete(&self) -> Result<(), Error>;
}

// Accessor usage example:
// let vault = client.organization("org_123").vault("vlt_456");
// log::info!("Checking in {}/{}", vault.organization_id(), vault.vault_id());
```

### Sub-Client Types

The sub-client pattern provides namespaced, extensible APIs for related operations. Sub-clients borrow the parent `VaultClient` and provide focused method sets.

#### ResourcesClient

Query resources that a subject can access with a given permission.

````rust
/// Sub-client for resource queries.
/// Obtained via `vault.resources()`.
pub struct ResourcesClient<'a> {
    vault: &'a VaultClient,
}

impl<'a> ResourcesClient<'a> {
    /// Start a query for resources accessible by a subject.
    ///
    /// Returns a builder that must be further configured with `.with_permission()`.
    ///
    /// # Example
    /// ```rust
    /// // Find all documents Alice can view
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .resource_type("document")
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn accessible_by(&self, subject: impl Into<Cow<'a, str>>) -> ResourcesQueryBuilder<'a>;

    // Future extensibility:
    // pub fn count(&self) -> ResourcesCountBuilder<'a>;
    // pub fn exists(&self, resource: &str) -> ResourcesExistsBuilder<'a>;
}

/// Builder for resource queries - requires subject and permission.
pub struct ResourcesQueryBuilder<'a> {
    vault: &'a VaultClient,
    subject: Cow<'a, str>,
}

impl<'a> ResourcesQueryBuilder<'a> {
    /// Specify the permission to check (required).
    #[must_use]
    pub fn with_permission(self, permission: impl Into<Cow<'a, str>>) -> ResourcesListBuilder<'a> {
        ResourcesListBuilder {
            vault: self.vault,
            subject: self.subject,
            permission: permission.into(),
            resource_type: None,
            consistency: None,
            page_size: None,
        }
    }
}

/// Builder for resource list queries (after subject and permission are set).
pub struct ResourcesListBuilder<'a> {
    vault: &'a VaultClient,
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource_type: Option<Cow<'a, str>>,
    consistency: Option<ConsistencyToken>,
    page_size: Option<u32>,
}

impl<'a> ResourcesListBuilder<'a> {
    /// Filter by resource type (e.g., "document", "folder").
    #[must_use]
    pub fn resource_type(mut self, resource_type: impl Into<Cow<'a, str>>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Ensure read consistency with a previously obtained token.
    #[must_use]
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Set page size for pagination.
    #[must_use]
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Limit results to first N items.
    #[must_use]
    pub fn take(self, n: usize) -> ResourcesListTake<'a> {
        ResourcesListTake { inner: self, limit: n }
    }

    /// Execute as a stream (preferred for large result sets).
    pub fn stream(self) -> impl Stream<Item = Result<String, Error>> + 'a;

    /// Collect all results into a Vec.
    /// Use with caution for large result sets.
    pub async fn collect(self) -> Result<Vec<String>, Error>;

    /// Get a specific page of results.
    pub async fn page(self, page: usize) -> Result<Page<String>, Error>;

    /// Get paginated results with cursor.
    pub async fn cursor(self, cursor: Option<&str>) -> Result<CursorPage<String>, Error>;
}

/// Builder wrapper that limits results to first N items.
/// Created by calling `.take(n)` on a ResourcesListBuilder.
pub struct ResourcesListTake<'a> {
    inner: ResourcesListBuilder<'a>,
    limit: usize,
}

impl<'a> ResourcesListTake<'a> {
    /// Execute as a stream, stopping after limit items.
    pub fn stream(self) -> impl Stream<Item = Result<String, Error>> + 'a {
        self.inner.stream().take(self.limit)
    }

    /// Collect limited results into a Vec.
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        self.stream().try_collect().await
    }
}
````

#### SubjectsClient

Query subjects that have a given permission on a resource.

````rust
/// Sub-client for subject queries.
/// Obtained via `vault.subjects()`.
pub struct SubjectsClient<'a> {
    vault: &'a VaultClient,
}

impl<'a> SubjectsClient<'a> {
    /// Start a query for subjects with a given permission.
    ///
    /// Returns a builder that must be further configured with `.on_resource()`.
    ///
    /// # Example
    /// ```rust
    /// // Find all users who can edit this document
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .subject_type("user")
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn with_permission(&self, permission: impl Into<Cow<'a, str>>) -> SubjectsQueryBuilder<'a>;

    // Future extensibility:
    // pub fn count(&self) -> SubjectsCountBuilder<'a>;
    // pub fn exists(&self, subject: &str) -> SubjectsExistsBuilder<'a>;
}

/// Builder for subject queries - requires permission and resource.
pub struct SubjectsQueryBuilder<'a> {
    vault: &'a VaultClient,
    permission: Cow<'a, str>,
}

impl<'a> SubjectsQueryBuilder<'a> {
    /// Specify the resource to check (required).
    #[must_use]
    pub fn on_resource(self, resource: impl Into<Cow<'a, str>>) -> SubjectsListBuilder<'a> {
        SubjectsListBuilder {
            vault: self.vault,
            permission: self.permission,
            resource: resource.into(),
            subject_type: None,
            consistency: None,
            page_size: None,
        }
    }
}

/// Builder for subject list queries (after permission and resource are set).
pub struct SubjectsListBuilder<'a> {
    vault: &'a VaultClient,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
    subject_type: Option<Cow<'a, str>>,
    consistency: Option<ConsistencyToken>,
    page_size: Option<u32>,
}

impl<'a> SubjectsListBuilder<'a> {
    /// Filter by subject type (e.g., "user", "group", "service").
    #[must_use]
    pub fn subject_type(mut self, subject_type: impl Into<Cow<'a, str>>) -> Self {
        self.subject_type = Some(subject_type.into());
        self
    }

    /// Ensure read consistency with a previously obtained token.
    #[must_use]
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Set page size for pagination.
    #[must_use]
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Limit results to first N items.
    #[must_use]
    pub fn take(self, n: usize) -> SubjectsListTake<'a> {
        SubjectsListTake { inner: self, limit: n }
    }

    /// Execute as a stream (preferred for large result sets).
    pub fn stream(self) -> impl Stream<Item = Result<String, Error>> + 'a;

    /// Collect all results into a Vec.
    pub async fn collect(self) -> Result<Vec<String>, Error>;

    /// Get a specific page of results.
    pub async fn page(self, page: usize) -> Result<Page<String>, Error>;

    /// Get paginated results with cursor.
    pub async fn cursor(self, cursor: Option<&str>) -> Result<CursorPage<String>, Error>;
}

/// Builder wrapper that limits results to first N items.
/// Created by calling `.take(n)` on a SubjectsListBuilder.
pub struct SubjectsListTake<'a> {
    inner: SubjectsListBuilder<'a>,
    limit: usize,
}

impl<'a> SubjectsListTake<'a> {
    /// Execute as a stream, stopping after limit items.
    pub fn stream(self) -> impl Stream<Item = Result<String, Error>> + 'a {
        self.inner.stream().take(self.limit)
    }

    /// Collect limited results into a Vec.
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        self.stream().try_collect().await
    }
}
````

#### RelationshipsClient

Full CRUD operations for relationships, plus querying.

````rust
/// Sub-client for relationship operations.
/// Obtained via `vault.relationships()`.
///
/// Provides read, write, and delete operations for relationships.
pub struct RelationshipsClient<'a> {
    vault: &'a VaultClient,
}

impl<'a> RelationshipsClient<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Read Operations
    // ─────────────────────────────────────────────────────────────────────────

    /// List relationships matching the given filters.
    ///
    /// # Example
    /// ```rust
    /// // List all relationships for a resource
    /// let rels = vault.relationships()
    ///     .list()
    ///     .resource("document:readme")
    ///     .collect()
    ///     .await?;
    ///
    /// // List all relationships for a subject
    /// let rels = vault.relationships()
    ///     .list()
    ///     .subject("user:alice")
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn list(&self) -> RelationshipsListBuilder<'a>;

    // ─────────────────────────────────────────────────────────────────────────
    // Write Operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Write a single relationship.
    ///
    /// # Example
    /// ```rust
    /// vault.relationships()
    ///     .write(Relationship::new("document:readme", "viewer", "user:alice"))
    ///     .await?;
    /// ```
    pub fn write(&self, relationship: impl Into<Relationship<'a>>) -> WriteRelationshipBuilder<'a>;

    /// Write multiple relationships atomically.
    ///
    /// All relationships are written in a single transaction - either all
    /// succeed or all fail.
    ///
    /// # Example
    /// ```rust
    /// vault.relationships()
    ///     .write_batch([
    ///         Relationship::new("folder:docs", "viewer", "group:eng"),
    ///         Relationship::new("document:readme", "parent", "folder:docs"),
    ///     ])
    ///     .await?;
    /// ```
    pub fn write_batch(&self, relationships: impl IntoIterator<Item = impl Into<Relationship<'a>>>) -> WriteBatchBuilder<'a>;

    /// Write multiple relationships as a stream (non-atomic, high-throughput).
    ///
    /// Unlike `write_batch()`, this streams relationships to the server
    /// and returns results as they complete. Failures don't roll back
    /// successful writes.
    ///
    /// # Example
    /// ```rust
    /// let mut results = vault.relationships()
    ///     .write_batch_streaming(large_relationship_set)
    ///     .stream();
    ///
    /// while let Some(result) = results.next().await {
    ///     match result {
    ///         Ok(token) => println!("Written, token: {:?}", token),
    ///         Err(e) => eprintln!("Failed: {}", e),
    ///     }
    /// }
    /// ```
    pub fn write_batch_streaming(&self, relationships: impl IntoIterator<Item = impl Into<Relationship<'a>>>) -> WriteStreamBuilder<'a>;

    // ─────────────────────────────────────────────────────────────────────────
    // Delete Operations
    // ─────────────────────────────────────────────────────────────────────────

    /// Delete a single relationship.
    ///
    /// Deletes are idempotent - deleting a non-existent relationship succeeds.
    ///
    /// # Example
    /// ```rust
    /// vault.relationships()
    ///     .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    ///     .await?;
    /// ```
    pub fn delete(&self, relationship: impl Into<Relationship<'a>>) -> DeleteRelationshipBuilder<'a>;

    /// Delete multiple relationships atomically.
    ///
    /// # Example
    /// ```rust
    /// vault.relationships()
    ///     .delete_batch([
    ///         Relationship::new("doc:1", "viewer", "user:alice"),
    ///         Relationship::new("doc:2", "viewer", "user:alice"),
    ///     ])
    ///     .await?;
    /// ```
    pub fn delete_batch(&self, relationships: impl IntoIterator<Item = impl Into<Relationship<'a>>>) -> DeleteBatchBuilder<'a>;

    /// Delete relationships matching a query.
    ///
    /// For bulk deletion without fetching relationships first.
    ///
    /// # Example
    /// ```rust
    /// // Remove all access for a departing user
    /// vault.relationships()
    ///     .delete_where()
    ///     .subject("user:departed")
    ///     .execute()
    ///     .await?;
    /// ```
    pub fn delete_where(&self) -> DeleteWhereBuilder<'a>;
}
````

#### RelationshipsListBuilder

```rust
/// Builder for relationship list queries.
pub struct RelationshipsListBuilder<'a> {
    vault: &'a VaultClient,
    subject: Option<Cow<'a, str>>,
    subject_type: Option<Cow<'a, str>>,
    resource: Option<Cow<'a, str>>,
    resource_type: Option<Cow<'a, str>>,
    relation: Option<Cow<'a, str>>,
    consistency: Option<ConsistencyToken>,
    page_size: Option<u32>,
}

impl<'a> RelationshipsListBuilder<'a> {
    /// Filter by subject (exact match).
    #[must_use]
    pub fn subject(mut self, subject: impl Into<Cow<'a, str>>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Filter by subject type prefix.
    #[must_use]
    pub fn subject_type(mut self, subject_type: impl Into<Cow<'a, str>>) -> Self {
        self.subject_type = Some(subject_type.into());
        self
    }

    /// Filter by resource (exact match).
    #[must_use]
    pub fn resource(mut self, resource: impl Into<Cow<'a, str>>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Filter by resource type prefix.
    #[must_use]
    pub fn resource_type(mut self, resource_type: impl Into<Cow<'a, str>>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Filter by relation name.
    #[must_use]
    pub fn relation(mut self, relation: impl Into<Cow<'a, str>>) -> Self {
        self.relation = Some(relation.into());
        self
    }

    /// Ensure read consistency with a previously obtained token.
    #[must_use]
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Set page size for pagination.
    #[must_use]
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Limit results to first N items.
    #[must_use]
    pub fn take(self, n: usize) -> RelationshipsListTake<'a> {
        RelationshipsListTake { inner: self, limit: n }
    }

    /// Execute as a stream (preferred for large result sets).
    pub fn stream(self) -> impl Stream<Item = Result<Relationship<'static>, Error>> + 'a;

    /// Collect all results into a Vec.
    pub async fn collect(self) -> Result<Vec<Relationship<'static>>, Error>;

    /// Get paginated results with cursor.
    pub async fn cursor(self, cursor: Option<&str>) -> Result<CursorPage<Relationship<'static>>, Error>;

    /// Get paginated results with offset.
    pub async fn offset(self, offset: u64, limit: u32) -> Result<OffsetPage<Relationship<'static>>, Error>;
}

/// Builder wrapper that limits results to first N items.
/// Created by calling `.take(n)` on a RelationshipsListBuilder.
pub struct RelationshipsListTake<'a> {
    inner: RelationshipsListBuilder<'a>,
    limit: usize,
}

impl<'a> RelationshipsListTake<'a> {
    /// Execute as a stream, stopping after limit items.
    pub fn stream(self) -> impl Stream<Item = Result<Relationship<'static>, Error>> + 'a {
        self.inner.stream().take(self.limit)
    }

    /// Collect limited results into a Vec.
    pub async fn collect(self) -> Result<Vec<Relationship<'static>>, Error> {
        self.stream().try_collect().await
    }
}
```

#### Write Builders

```rust
/// Builder for single relationship write.
pub struct WriteRelationshipBuilder<'a> {
    vault: &'a VaultClient,
    relationship: Relationship<'a>,
    request_id: Option<Uuid>,
    unless_exists: bool,
    precondition: Option<Precondition>,
    dry_run: bool,
}

impl<'a> WriteRelationshipBuilder<'a> {
    /// Set a request ID for idempotency.
    /// The server will deduplicate requests with the same ID within the idempotency window.
    #[must_use]
    pub fn request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Only write if the relationship doesn't already exist.
    #[must_use]
    pub fn unless_exists(mut self) -> Self {
        self.unless_exists = true;
        self
    }

    /// Write only if a precondition is met.
    #[must_use]
    pub fn precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }

    /// Preview what would be written without committing.
    #[must_use]
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    /// Override the client-level timeout for this operation.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl<'a> IntoFuture for WriteRelationshipBuilder<'a> {
    type Output = Result<WriteResult, Error>;
    type IntoFuture = impl Future<Output = Self::Output> + 'a;

    fn into_future(self) -> Self::IntoFuture {
        async move { self.vault.execute_write(self).await }
    }
}

/// Result of a write operation.
#[derive(Debug, Clone)]
pub struct WriteResult {
    /// Consistency token for read-after-write guarantees.
    pub consistency_token: ConsistencyToken,
    /// Whether the relationship was created (false if it already existed).
    pub created: bool,
    /// Request ID (if provided or auto-generated).
    pub request_id: Option<Uuid>,
}

impl WriteResult {
    /// Get the consistency token for subsequent reads.
    pub fn consistency_token(&self) -> &ConsistencyToken {
        &self.consistency_token
    }
}

/// Precondition for conditional write/delete operations.
/// Enables optimistic concurrency control and atomic compare-and-swap patterns.
#[derive(Debug, Clone)]
pub enum Precondition {
    /// Relationship must not exist (for create-if-not-exists)
    NotExists {
        resource: String,
        relation: String,
        /// Use "*" to match any subject
        subject: String,
    },

    /// Relationship must exist (for update-if-exists or CAS)
    Exists {
        resource: String,
        relation: String,
        /// Use "*" to match any subject
        subject: String,
    },

    /// Consistency token must match (optimistic locking)
    TokenMatches(ConsistencyToken),

    /// Multiple preconditions that must all be satisfied
    All(Vec<Precondition>),

    /// At least one precondition must be satisfied
    Any(Vec<Precondition>),
}

impl Precondition {
    /// Create a "must not exist" precondition
    pub fn not_exists(resource: impl Into<String>, relation: impl Into<String>, subject: impl Into<String>) -> Self {
        Self::NotExists {
            resource: resource.into(),
            relation: relation.into(),
            subject: subject.into(),
        }
    }

    /// Create a "must exist" precondition
    pub fn exists(resource: impl Into<String>, relation: impl Into<String>, subject: impl Into<String>) -> Self {
        Self::Exists {
            resource: resource.into(),
            relation: relation.into(),
            subject: subject.into(),
        }
    }

    /// Create a consistency token precondition
    pub fn token_matches(token: ConsistencyToken) -> Self {
        Self::TokenMatches(token)
    }
}

/// Builder for batch relationship writes.
pub struct WriteBatchBuilder<'a> {
    vault: &'a VaultClient,
    relationships: Vec<Relationship<'a>>,
    request_id: Option<Uuid>,
    atomic: bool,
}

impl<'a> WriteBatchBuilder<'a> {
    /// Set a request ID for the entire batch.
    #[must_use]
    pub fn request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Make the batch atomic (all-or-nothing). This is the default.
    #[must_use]
    pub fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }

    /// Override timeout for this batch operation.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl<'a> IntoFuture for WriteBatchBuilder<'a> {
    type Output = Result<WriteBatchResult, Error>;
    type IntoFuture = impl Future<Output = Self::Output> + 'a;

    fn into_future(self) -> Self::IntoFuture {
        async move { self.vault.execute_write_batch(self).await }
    }
}

/// Result of a batch write operation.
#[derive(Debug, Clone)]
pub struct WriteBatchResult {
    /// Number of relationships written.
    pub written: u64,
    /// Consistency token for read-after-write guarantees.
    pub consistency_token: ConsistencyToken,
    /// Request ID (if provided or auto-generated).
    pub request_id: Option<Uuid>,
}
```

#### Delete Builders

```rust
/// Builder for single relationship delete.
pub struct DeleteRelationshipBuilder<'a> {
    vault: &'a VaultClient,
    relationship: Relationship<'a>,
    request_id: Option<Uuid>,
}

impl<'a> DeleteRelationshipBuilder<'a> {
    /// Set a request ID for idempotency tracking.
    #[must_use]
    pub fn request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Override timeout for this operation.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl<'a> IntoFuture for DeleteRelationshipBuilder<'a> {
    type Output = Result<DeleteResult, Error>;
    type IntoFuture = impl Future<Output = Self::Output> + 'a;

    fn into_future(self) -> Self::IntoFuture {
        async move { self.vault.execute_delete(self).await }
    }
}

/// Result of a delete operation.
#[derive(Debug, Clone)]
pub struct DeleteResult {
    /// Whether the relationship existed and was deleted.
    pub deleted: bool,
    /// Consistency token for read-after-write guarantees.
    pub consistency_token: ConsistencyToken,
}

/// Builder for batch relationship deletes.
pub struct DeleteBatchBuilder<'a> {
    vault: &'a VaultClient,
    relationships: Vec<Relationship<'a>>,
    request_id: Option<Uuid>,
}

impl<'a> DeleteBatchBuilder<'a> {
    /// Set a request ID for the entire batch.
    #[must_use]
    pub fn request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }
}

impl<'a> IntoFuture for DeleteBatchBuilder<'a> {
    type Output = Result<DeleteBatchResult, Error>;
    type IntoFuture = impl Future<Output = Self::Output> + 'a;

    fn into_future(self) -> Self::IntoFuture {
        async move { self.vault.execute_delete_batch(self).await }
    }
}

/// Result of a batch delete operation.
#[derive(Debug, Clone)]
pub struct DeleteBatchResult {
    /// Number of relationships deleted.
    pub deleted: u64,
    /// Consistency token for read-after-write guarantees.
    pub consistency_token: ConsistencyToken,
}

/// Preview of what a delete-where operation would affect.
/// Returned by `DeleteWhereBuilder::dry_run()`.
#[derive(Debug, Clone)]
pub struct DeletePreview {
    /// Number of relationships that would be deleted.
    pub count: u64,
    /// Sample of relationships that would be deleted (up to 100).
    pub sample: Vec<Relationship<'static>>,
    /// Whether the sample is exhaustive (count <= sample.len()).
    pub is_exhaustive: bool,
    /// Breakdown by entity type.
    pub by_entity_type: HashMap<String, u64>,
    /// Breakdown by relation.
    pub by_relation: HashMap<String, u64>,
}

/// Builder for delete-by-query operations.
/// See [Bulk Delete by Query](#bulk-delete-by-query) for detailed usage.
pub struct DeleteWhereBuilder<'a> {
    vault: &'a VaultClient,
    subject: Option<Cow<'a, str>>,
    subject_type: Option<Cow<'a, str>>,
    resource: Option<Cow<'a, str>>,
    resource_type: Option<Cow<'a, str>>,
    relation: Option<Cow<'a, str>>,
    confirm_threshold: Option<u64>,
}

impl<'a> DeleteWhereBuilder<'a> {
    /// Filter by subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<Cow<'a, str>>) -> Self;

    /// Filter by subject type.
    #[must_use]
    pub fn subject_type(mut self, subject_type: impl Into<Cow<'a, str>>) -> Self;

    /// Filter by resource.
    #[must_use]
    pub fn resource(mut self, resource: impl Into<Cow<'a, str>>) -> Self;

    /// Filter by resource type.
    #[must_use]
    pub fn resource_type(mut self, resource_type: impl Into<Cow<'a, str>>) -> Self;

    /// Filter by relation.
    #[must_use]
    pub fn relation(mut self, relation: impl Into<Cow<'a, str>>) -> Self;

    /// Require confirmation if deleting more than N relationships.
    #[must_use]
    pub fn confirm_above(mut self, threshold: u64) -> Self {
        self.confirm_threshold = Some(threshold);
        self
    }

    /// Preview what would be deleted without actually deleting.
    pub async fn dry_run(self) -> Result<DeletePreview, Error>;

    /// Execute the delete.
    pub async fn execute(self) -> Result<DeleteWhereResult, Error>;
}

/// Result of a delete-where operation.
#[derive(Debug, Clone)]
pub struct DeleteWhereResult {
    /// Number of relationships deleted.
    pub count: u64,
    /// Duration of the operation.
    pub duration: Duration,
    /// Consistency token.
    pub consistency_token: ConsistencyToken,
}

/// Builder for streaming write operations.
/// Unlike WriteBatchBuilder (atomic), this writes each relationship independently
/// and streams results as they complete.
pub struct WriteStreamBuilder<'a> {
    vault: &'a VaultClient,
    relationships: Vec<Relationship<'a>>,
    continue_on_error: bool,
}

impl<'a> WriteStreamBuilder<'a> {
    /// Continue processing remaining relationships if one fails (default: true)
    #[must_use]
    pub fn continue_on_error(mut self, continue_on_error: bool) -> Self {
        self.continue_on_error = continue_on_error;
        self
    }

    /// Get the result stream.
    /// Each item is a Result indicating success or failure for that relationship.
    pub fn stream(self) -> impl Stream<Item = Result<WriteResult, Error>> + 'a {
        // Implementation streams results as each write completes
    }

    /// Collect all results, stopping on first error.
    pub async fn collect(self) -> Result<Vec<WriteResult>, Error> {
        self.stream().try_collect().await
    }

    /// Collect all results, including errors.
    /// Returns Ok with a Vec of individual results.
    pub async fn collect_all(self) -> Vec<Result<WriteResult, Error>> {
        self.stream().collect().await
    }
}
```

#### Pagination Types

```rust
/// Cursor-based page of results.
#[derive(Debug, Clone)]
pub struct CursorPage<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Cursor for the next page (None if last page).
    pub next_cursor: Option<String>,
    /// Whether there are more pages.
    pub has_next: bool,
}

/// Offset-based page of results.
#[derive(Debug, Clone)]
pub struct OffsetPage<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Current offset.
    pub offset: u64,
    /// Total count (if available).
    pub total: Option<u64>,
    /// Whether there are more pages.
    pub has_next: bool,
}

// Convenience alias
pub type Page<T> = CursorPage<T>;
```

#### Sub-Client Design Rationale

**Why sub-clients instead of flat methods?**

| Approach                                                            | Pros                  | Cons                               |
| ------------------------------------------------------------------- | --------------------- | ---------------------------------- |
| Flat methods (`vault.list_resources()`)                             | Shorter call chains   | Pollutes namespace, hard to extend |
| Sub-clients (`vault.resources().accessible_by().with_permission()`) | Extensible, organized | Extra method call                  |

The sub-client pattern:

1. **Enables future extensibility** - Adding `resources().count()` or `resources().exists()` doesn't require changing `VaultClient`
2. **Groups related operations** - All relationship operations are under `relationships()`
3. **Reduces API surface** - `VaultClient` stays focused on core operations
4. **Follows Rust idioms** - Similar to `std::fs::File::metadata().permissions()`

**Lifetime design (`'a` parameter)**:

Sub-clients borrow the parent `VaultClient` rather than cloning it:

```rust
// Sub-client borrows vault, doesn't own it
let rels = vault.relationships();  // rels: RelationshipsClient<'_>

// This ensures the vault outlives any builders:
let builder = vault.relationships().list();  // builder borrows vault
drop(vault);  // Error: vault still borrowed by builder
builder.collect().await?;  // Would fail if vault dropped
```

This design is zero-cost (no `Arc` clone) and ensures safe lifetime management.

**Design Rationale:**

We chose a unified `VaultClient` type over separate `VaultAccess` and `VaultManagement` types:

- **Single context**: One type for all vault operations reduces cognitive load
- **Natural ownership**: Vaults flow from organizations, matching the actual data model
- **Cheap cloning**: `Arc` internally means clone is O(1) refcount increment
- **Consistent patterns**: Follows established Rust SDK conventions (`reqwest::Client`, AWS SDK)

**Usage Examples**:

```rust
// Inline use
client.organization("org_8675309...").vault("vlt_01JFQGK...").check("user:alice", "view", "doc:1").await?;

// Store for reuse
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
vault.check("user:alice", "view", "doc:1").await?;
vault.check("user:alice", "edit", "doc:1").await?;
vault.schemas().get_active().await?;  // Management on same vault

// Store in struct (no lifetime parameter needed)
struct MyService {
    vault: VaultClient,
}

// Pass to spawned task (VaultClient is 'static)
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
tokio::spawn(async move {
    vault.check("user:alice", "view", "doc:1").await
});
```

### Organization Client

The `OrganizationClient` type provides access to organization-level operations and child resources:

```rust
/// Client scoped to a specific organization
#[derive(Clone)]
pub struct OrganizationClient {
    inner: Client,
    org_id: String,
}

impl Client {
    /// Get an organization context
    pub fn organization(&self, org_id: impl Into<String>) -> OrganizationClient {
        OrganizationClient {
            inner: self.clone(),
            org_id: org_id.into(),
        }
    }
}

impl OrganizationClient {
    // Vault operations
    pub fn vault(&self, vault_id: impl Into<String>) -> VaultClient;
    pub fn vaults(&self) -> VaultsClient;  // List/create vaults

    // Organization management
    pub fn members(&self) -> MembersClient;
    pub fn teams(&self) -> TeamsClient;
    pub fn invitations(&self) -> InvitationsClient;
    pub fn audit_logs(&self) -> AuditLogsClient;

    // CRUD
    pub async fn get(&self) -> Result<OrganizationInfo, Error>;
    pub async fn update(&self, update: UpdateOrg) -> Result<OrganizationInfo, Error>;
    pub async fn delete(&self) -> Result<(), Error>;
}
```

### Multi-Organization SaaS Pattern

For multi-organization applications where each organization has their own vault:

```rust
// Extract organization and vault from request
async fn handle_request(
    client: &Client,
    org_id: &str,
    vault_id: &str,
    request: Request,
) -> Result<Response, Error> {
    // Organization-first: scope to org, then vault
    let vault = client.organization(org_id).vault(vault_id);

    let user = extract_user(&request);
    let resource = extract_resource(&request);

    // Use require() for clean fail-fast authorization
    vault.check(&user, "access", &resource)
        .require()
        .await?;  // Returns Err(AccessDenied) on denial

    // Process request...
}
```

### Vault Validation

```rust
// Validate vault exists before operations
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
vault.validate().await?;  // Fails if vault doesn't exist or no access

// Or check access without failing
let org = client.organization("org_8675309...");
if org.vault("vlt_01JFQGK...").get().await.is_ok() {
    // Safe to use vault
}
```

### Dependency Injection

For testing, inject vault behavior using the `AuthorizationClient` trait (see [Testing Support](#testing-support) for the full trait definition):

```rust
use inferadb::AuthorizationClient;

// Use the trait in your application for testability
struct MyService {
    authz: Arc<dyn AuthorizationClient>,
}

impl MyService {
    pub async fn can_access(&self, user: &str, resource: &str) -> Result<bool, Error> {
        self.authz.check(user, "access", resource).await
    }
}

// Production: inject real VaultClient
let vault = client.organization("org_...").vault("vlt_...");
let service = MyService { authz: Arc::new(vault) };

// Testing: inject MockClient
let mock = MockClient::builder()
    .check("user:alice", "access", "doc:1", true)
    .build();
let service = MyService { authz: Arc::new(mock) };
```

For management operations, use the concrete `VaultClient` type directly or define your own trait as needed.

---

## Middleware and Interceptors

### Design Pattern

Middleware wraps the transport layer, allowing cross-cutting concerns:

```text
┌─────────────────────────────────────────────────────────────────────┐
│                          Request Pipeline                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  vault.check(...)                                                    │
│       │                                                              │
│       ▼                                                              │
│  ┌─────────────────┐                                                │
│  │ Logging Layer   │  Log request/response                          │
│  └────────┬────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                │
│  │ Metrics Layer   │  Record latency, count                         │
│  └────────┬────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                │
│  │ Retry Layer     │  Handle transient failures                     │
│  └────────┬────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                │
│  │ Auth Layer      │  Inject Bearer token                           │
│  └────────┬────────┘                                                │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                │
│  │ Transport       │  HTTP/gRPC call                                │
│  └─────────────────┘                                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Middleware Types

```rust
use std::future::Future;
use std::pin::Pin;

/// Middleware trait for intercepting SDK requests.
/// Middleware wraps the transport layer, enabling cross-cutting concerns
/// like logging, metrics, custom headers, or request transformation.
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    /// Handle a request, optionally modifying it or the response.
    /// Call `next.call(req)` to continue the chain.
    async fn handle(&self, req: Request, next: Next<'_>) -> Result<Response, Error>;
}

/// An SDK request being processed through the middleware chain.
#[derive(Debug)]
pub struct Request {
    /// The operation being performed (e.g., "check", "write", "list_resources")
    operation: String,
    /// Request metadata
    metadata: RequestMetadata,
    /// The serialized request body
    body: Vec<u8>,
}

impl Request {
    /// Get the operation name.
    pub fn operation(&self) -> &str { &self.operation }

    /// Get request metadata (headers, trace context, etc.).
    pub fn metadata(&self) -> &RequestMetadata { &self.metadata }

    /// Get mutable access to metadata for adding custom headers.
    pub fn metadata_mut(&mut self) -> &mut RequestMetadata { &mut self.metadata }
}

/// Request metadata including headers and trace context.
#[derive(Debug, Clone, Default)]
pub struct RequestMetadata {
    /// Custom headers to include in the request
    pub headers: HashMap<String, String>,
    /// Trace context for distributed tracing
    pub trace_context: Option<TraceContext>,
    /// Request ID (auto-generated if not set)
    pub request_id: Option<String>,
}

/// The response from an SDK operation.
#[derive(Debug)]
pub struct Response {
    /// Response metadata
    metadata: ResponseMetadata,
    /// The serialized response body
    body: Vec<u8>,
}

impl Response {
    /// Check if the response indicates success.
    pub fn is_ok(&self) -> bool { self.metadata.status.is_success() }

    /// Get response metadata.
    pub fn metadata(&self) -> &ResponseMetadata { &self.metadata }
}

/// Response metadata including status and headers.
#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    /// Response status
    pub status: ResponseStatus,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Server-provided request ID
    pub request_id: Option<String>,
}

/// Response status.
#[derive(Debug, Clone, Copy)]
pub enum ResponseStatus {
    Success,
    Error(ErrorKind),
}

impl ResponseStatus {
    pub fn is_success(&self) -> bool { matches!(self, Self::Success) }
}

/// The next middleware or transport in the chain.
pub struct Next<'a> {
    inner: Box<dyn FnOnce(Request) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> + Send + 'a>,
}

impl<'a> Next<'a> {
    /// Call the next middleware or transport.
    pub async fn call(self, req: Request) -> Result<Response, Error> {
        (self.inner)(req).await
    }
}
```

### Adding Custom Middleware

```rust
use inferadb::middleware::{Middleware, Request, Response, Next};
use std::sync::Arc;

/// Example: Audit logging middleware
struct AuditLogger {
    logger: Arc<dyn Log + Send + Sync>,
}

impl AuditLogger {
    pub fn new(logger: impl Log + Send + Sync + 'static) -> Self {
        Self { logger: Arc::new(logger) }
    }
}

/// Simple logging trait (or use your preferred logging framework)
trait Log: Send + Sync {
    fn log(&self, entry: AuditEntry);
}

struct AuditEntry {
    operation: String,
    duration: Duration,
    success: bool,
}

#[async_trait]
impl Middleware for AuditLogger {
    async fn handle(&self, req: Request, next: Next<'_>) -> Result<Response, Error> {
        let start = Instant::now();
        let operation = req.operation().to_string();

        let response = next.call(req).await;

        self.logger.log(AuditEntry {
            operation,
            duration: start.elapsed(),
            success: response.as_ref().map(|r| r.is_ok()).unwrap_or(false),
        });

        response
    }
}

// Usage
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .middleware(AuditLogger::new(my_logger))
    .build()
    .await?;
```

---

## Type-Safe Relationships

Compile-time schema validation is our key differentiator vs SpiceDB/OpenFGA.

### The Problem with Stringly-Typed APIs

```rust
// Runtime errors only - typos compile fine
vault.check("user:alice", "veiw", "document:readme").await?;  // "veiw" typo
vault.relationships().write(Relationship::new("doc:1", "viwer", "user:alice")).await?;  // "viwer" typo
```

### Schema-Derived Types

With the `derive` feature, generate types from your IPL schema:

```rust
// schema.ipl
// entity User {}
// entity Document {
//     relations { viewer: User, editor: User, owner: User }
//     permissions { view: viewer | editor | owner, edit: editor | owner }
// }

use inferadb::derive::schema;

// Generate types at compile time from schema
schema!("schema.ipl");

// Now you have type-safe entities and relations
let doc = Document::new("readme");
let user = User::new("alice");

// ✅ Compiles - valid relation
doc.viewer().add(&user);

// ❌ Compile error - "viwer" doesn't exist
doc.viwer().add(&user);  // Error: no method named `viwer`

// ❌ Compile error - wrong subject type
let other_doc = Document::new("other");
doc.viewer().add(&other_doc);  // Error: expected User, found Document
```

### Derive Macro Implementation

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

// Generated by macro:
impl Document {
    pub fn viewer(&self) -> RelationBuilder<User> { /* ... */ }
    pub fn editor(&self) -> RelationBuilder<User> { /* ... */ }
    pub fn owner(&self) -> RelationBuilder<User> { /* ... */ }
}

// Type-safe checks
impl Document {
    pub fn can_view(&self, subject: &User) -> CheckBuilder { /* ... */ }
    pub fn can_edit(&self, subject: &User) -> CheckBuilder { /* ... */ }
}
```

### Type-Safe Check API

```rust
// Stringly-typed (still supported for dynamic use cases)
vault.check("user:alice", "view", "document:readme").await?;

// Type-safe (preferred)
let allowed = vault
    .check_typed(&user, Document::VIEW, &doc)
    .await?;

// Or using entity methods
let allowed = doc.can_view(&user).check(&vault).await?;
```

### Type-Safe Relationship Builder

```rust
// Stringly-typed
vault.relationships().write(Relationship::new("document:readme", "viewer", "user:alice")).await?;

// Type-safe builder
vault.relationships().write(
    doc.viewer().is(&user)
).await?;

// Batch with mixed types
vault.relationships().write_batch([
    doc.viewer().is(&alice),
    doc.editor().is(&bob),
    folder.parent().is(&root_folder),
]).await?;
```

### Schema Validation at Compile Time

```rust
// build.rs - validate schema during compilation
fn main() {
    inferadb_build::validate_schema("schema.ipl")
        .expect("Invalid schema");

    // Generate types
    inferadb_build::generate_types("schema.ipl", "src/generated.rs")
        .expect("Failed to generate types");
}
```

### Strong ID Types

Beyond schema-derived types, the SDK provides strongly-typed ID wrappers that prevent mixing up different identifier types:

```rust
use std::borrow::Cow;

/// Error type for parsing entity references and subject references
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    /// Missing colon separator in "type:id" format
    #[error("missing ':' separator in entity reference")]
    MissingColon,

    /// Empty entity type
    #[error("entity type cannot be empty")]
    EmptyType,

    /// Empty entity ID
    #[error("entity ID cannot be empty")]
    EmptyId,

    /// Invalid characters in entity type (must be alphanumeric + underscore)
    #[error("invalid characters in entity type: {0}")]
    InvalidTypeChars(String),

    /// Invalid characters in entity ID
    #[error("invalid characters in entity ID: {0}")]
    InvalidIdChars(String),

    /// Invalid userset format (for SubjectRef with #relation)
    #[error("invalid userset format: {0}")]
    InvalidUserset(String),
}

/// A reference to an entity (resource or subject) in the format "type:id"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityRef<'a> {
    entity_type: Cow<'a, str>,
    entity_id: Cow<'a, str>,
}

impl<'a> EntityRef<'a> {
    /// Parse from "type:id" format
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        let (entity_type, entity_id) = s.split_once(':')
            .ok_or(ParseError::MissingColon)?;
        Ok(Self {
            entity_type: Cow::Borrowed(entity_type),
            entity_id: Cow::Borrowed(entity_id),
        })
    }

    /// Create from components
    pub fn new(entity_type: impl Into<Cow<'a, str>>, entity_id: impl Into<Cow<'a, str>>) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn entity_type(&self) -> &str { &self.entity_type }
    pub fn entity_id(&self) -> &str { &self.entity_id }

    pub fn into_owned(self) -> EntityRef<'static> {
        EntityRef {
            entity_type: Cow::Owned(self.entity_type.into_owned()),
            entity_id: Cow::Owned(self.entity_id.into_owned()),
        }
    }
}

impl std::fmt::Display for EntityRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.entity_type, self.entity_id)
    }
}

/// A subject reference, which can be an entity or entity#relation (userset)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubjectRef<'a> {
    entity: EntityRef<'a>,
    relation: Option<Cow<'a, str>>,  // For usersets like "group:admins#member"
}

impl<'a> SubjectRef<'a> {
    /// Parse from "type:id" or "type:id#relation" format
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        if let Some((entity_part, relation)) = s.split_once('#') {
            Ok(Self {
                entity: EntityRef::parse(entity_part)?,
                relation: Some(Cow::Borrowed(relation)),
            })
        } else {
            Ok(Self {
                entity: EntityRef::parse(s)?,
                relation: None,
            })
        }
    }

    /// Create a userset subject (e.g., "group:admins#member")
    pub fn userset(entity: EntityRef<'a>, relation: impl Into<Cow<'a, str>>) -> Self {
        Self { entity, relation: Some(relation.into()) }
    }

    pub fn is_userset(&self) -> bool { self.relation.is_some() }
}

/// Strongly-typed organization identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(String);

impl OrganizationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for OrganizationId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<String> for OrganizationId {
    fn from(s: String) -> Self { Self(s) }
}

impl std::fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly-typed vault identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VaultId(String);

impl VaultId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for VaultId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

impl From<String> for VaultId {
    fn from(s: String) -> Self { Self(s) }
}
```

**Benefits of Strong ID Types**:

```rust
// Without strong types - easy to mix up parameters
fn check_permission(subject: &str, resource: &str, permission: &str) { /* ... */ }
check_permission("doc:readme", "user:alice", "view");  // Oops! Subject/resource swapped

// With strong types - compiler catches mistakes
fn check_permission(subject: SubjectRef<'_>, permission: &str, resource: EntityRef<'_>) { /* ... */ }
// check_permission(resource, "view", subject);  // ❌ Compile error: type mismatch

// Parsing validates format at boundaries
let subject = SubjectRef::parse("user:alice")?;  // ✅ Valid
let subject = SubjectRef::parse("invalid")?;     // ❌ ParseError::MissingColon
```

**Interop with String-Based APIs**:

```rust
// Strong types implement Display and Into<String>
let entity = EntityRef::new("document", "readme");
let as_string: String = entity.to_string();  // "document:readme"

// Vault methods accept both strong types and strings
vault.check(subject, "view", resource).await?;  // Strong types
vault.check("user:alice", "view", "doc:readme").await?;  // Strings (still works)

// Relationship accepts EntityRef for resource/subject
let rel = Relationship::new(
    EntityRef::new("document", "readme"),
    "viewer",
    SubjectRef::parse("user:alice")?,
);
```

### Display Trait Implementations

All core types implement `Display` for ergonomic logging, debugging, and error messages:

| Type               | Display Output Example                          | Notes                           |
| ------------------ | ----------------------------------------------- | ------------------------------- |
| `EntityRef`        | `"document:readme"`                             | `type:id` format                |
| `SubjectRef`       | `"user:alice"` or `"group:admins#member"`       | Includes subrelation if present |
| `Relationship`     | `"document:readme#viewer@user:alice"`           | Compact graph edge format       |
| `Decision`         | `"allow"` or `"deny"`                           | Lowercase boolean string        |
| `Error`            | `"rate limited: Too many requests (req_abc)"`   | Kind + message + request_id     |
| `ErrorKind`        | `"rate limited"`                                | Lowercase human-readable        |
| `AccessDenied`     | `"access denied: user:alice cannot view doc:1"` | Actionable message              |
| `ConsistencyToken` | `"ct_01JFQG..."`                                | Truncated for readability       |
| `OrganizationId`   | `"org_8675309..."`                              | Full ID                         |
| `VaultId`          | `"vlt_01JFQG..."`                               | Full ID                         |

**Usage in Logging**:

```rust
// Display makes logging natural
tracing::info!("Checking permission: {} {} {}", subject, permission, resource);

// Format strings work intuitively
let msg = format!("Created relationship: {}", relationship);

// Error messages are user-friendly
eprintln!("Authorization failed: {}", access_denied);
```

**Relationship Formats**:

```rust
let rel = Relationship::new("document:readme", "viewer", "user:alice");

// Display format (compact, for logging)
println!("{}", rel);  // "document:readme#viewer@user:alice"

// Debug format (full structure, for debugging)
println!("{:?}", rel);  // Relationship { resource: "document:readme", ... }

// Alternative format (via method)
println!("{}", rel.as_tuple());  // "(document:readme, viewer, user:alice)"
```

---

## Zero-Copy APIs

Support borrowed data for high-volume paths where allocation matters.

### Design Decision: `'static` Builders by Default

**Goal**: Minimize lifetime friction for the common case while preserving zero-copy performance for hot paths.

**Approach**: The primary builder APIs (`check()`, `relationships()`, etc.) return `'static` builders that internally clone an `Arc<ClientInner>`. This means:

- ✅ Builders can be stored in structs, moved across tasks, and composed freely
- ✅ No lifetime parameters needed on handler functions
- ✅ Ergonomic async/await with no borrow-checker friction
- ⚠️ One `Arc::clone()` per operation (typically <10ns)

**For hot paths** where even `Arc::clone()` overhead matters, use the `_borrowed` variants documented below. These are "expert mode" APIs for maximum performance:

```rust
// Standard API - 'static, easy to use
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Expert API - borrowed, zero allocation, requires lifetime management
let allowed = vault.check_borrowed(&subject, &permission, &resource).await?;
```

**Rationale**: Most authorization checks are not in tight loops. The common case should "just work" without lifetime annotations. Developers who profile and identify authorization as a bottleneck can opt into the borrowed APIs.

### Borrowed Relationships

```rust
// Owned (default) - allocates strings
let rel = Relationship::new("document:readme", "viewer", "user:alice");

// Borrowed - zero allocation
let rel = Relationship::borrowed("document:readme", "viewer", "user:alice");

// From existing strings without copying
let resource = "document:readme";
let relation = "viewer";
let subject = "user:alice";
let rel = Relationship::from_refs(resource, relation, subject);
```

### Relationship Type with Cow

````rust
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct Relationship<'a> {
    pub resource: Cow<'a, str>,
    pub relation: Cow<'a, str>,
    pub subject: Cow<'a, str>,
}

/// Convenience alias for owned relationships
pub type OwnedRelationship = Relationship<'static>;

impl<'a> Relationship<'a> {
    /// Ergonomic constructor - creates owned relationship from any string-like types.
    /// This is the most common constructor for typical use cases.
    pub fn new(
        resource: impl Into<String>,
        relation: impl Into<String>,
        subject: impl Into<String>,
    ) -> Relationship<'static> {
        Relationship {
            resource: Cow::Owned(resource.into()),
            relation: Cow::Owned(relation.into()),
            subject: Cow::Owned(subject.into()),
        }
    }

    /// Zero-copy constructor for string literals and borrowed data.
    /// Use this in hot paths where allocation overhead matters.
    pub fn borrowed(
        resource: &'a str,
        relation: &'a str,
        subject: &'a str,
    ) -> Self {
        Self {
            resource: Cow::Borrowed(resource),
            relation: Cow::Borrowed(relation),
            subject: Cow::Borrowed(subject),
        }
    }

    /// Explicit owned constructor (same as new(), more explicit naming)
    pub fn owned(
        resource: impl Into<String>,
        relation: impl Into<String>,
        subject: impl Into<String>,
    ) -> Relationship<'static> {
        Self::new(resource, relation, subject)
    }

    /// Convert to owned for storage/sending across threads
    pub fn into_owned(self) -> Relationship<'static> {
        Relationship {
            resource: Cow::Owned(self.resource.into_owned()),
            relation: Cow::Owned(self.relation.into_owned()),
            subject: Cow::Owned(self.subject.into_owned()),
        }
    }

    /// Create a relationship builder for named-parameter construction.
    /// Useful when you have values in a different order than the constructor.
    pub fn builder() -> RelationshipBuilder {
        RelationshipBuilder::new()
    }

    /// Create a relationship from check() parameter order.
    /// Converts (subject, relation, resource) → Relationship(resource, relation, subject).
    ///
    /// # Example
    /// ```rust
    /// // Convert check parameters to relationship
    /// let rel = Relationship::from_check_params("user:alice", "viewer", "document:readme");
    /// assert_eq!(rel.resource, "document:readme");
    /// assert_eq!(rel.relation, "viewer");
    /// assert_eq!(rel.subject, "user:alice");
    /// ```
    pub fn from_check_params(
        subject: impl Into<String>,
        relation: impl Into<String>,
        resource: impl Into<String>,
    ) -> Relationship<'static> {
        Relationship {
            resource: Cow::Owned(resource.into()),
            relation: Cow::Owned(relation.into()),
            subject: Cow::Owned(subject.into()),
        }
    }
}

/// Builder for constructing relationships with named parameters.
/// Useful when values come in a different order than `Relationship::new()`.
#[derive(Debug, Default)]
pub struct RelationshipBuilder {
    resource: Option<String>,
    relation: Option<String>,
    subject: Option<String>,
}

impl RelationshipBuilder {
    /// Create a new relationship builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the resource (e.g., "document:readme").
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the relation (e.g., "viewer", "editor").
    pub fn relation(mut self, relation: impl Into<String>) -> Self {
        self.relation = Some(relation.into());
        self
    }

    /// Set the subject (e.g., "user:alice").
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Build the relationship. Panics if any field is missing.
    /// For fallible construction, use `try_build()`.
    pub fn build(self) -> Relationship<'static> {
        Relationship {
            resource: Cow::Owned(self.resource.expect("resource is required")),
            relation: Cow::Owned(self.relation.expect("relation is required")),
            subject: Cow::Owned(self.subject.expect("subject is required")),
        }
    }

    /// Build the relationship, returning an error if any field is missing.
    pub fn try_build(self) -> Result<Relationship<'static>, &'static str> {
        Ok(Relationship {
            resource: Cow::Owned(self.resource.ok_or("resource is required")?),
            relation: Cow::Owned(self.relation.ok_or("relation is required")?),
            subject: Cow::Owned(self.subject.ok_or("subject is required")?),
        })
    }
}
````

### Parameter Order: Why Relationship and check() Differ

The `Relationship` struct and `check()` method use different parameter orders. This is intentional and aligns with common usage patterns:

| API                   | Order                             | Rationale                                            |
| --------------------- | --------------------------------- | ---------------------------------------------------- |
| `Relationship::new()` | `(resource, relation, subject)`   | Matches graph edge: `resource --relation--> subject` |
| `check()`             | `(subject, permission, resource)` | Matches question: "Can subject do X to resource?"    |

**Relationship Order - Graph Edge Mental Model**:

```rust
// Think: "document:readme has viewer user:alice"
// Or:    "document:readme --viewer--> user:alice"
Relationship::new("document:readme", "viewer", "user:alice")
//                 ^resource          ^relation  ^subject
```

**check() Order - Question Mental Model**:

```rust
// Think: "Can Alice view the readme?"
// Or:    "Does Alice have view permission on readme?"
vault.check("user:alice", "view", "document:readme").await?
//          ^subject      ^perm   ^resource
```

**Consistency with Builders**:

Builder methods follow the same patterns:

```rust
// Relationship filtering - resource-first (like Relationship::new)
vault.relationships()
    .resource("document:readme")  // First filter by resource
    .relation("viewer")           // Then by relation
    .subject("user:alice")        // Finally by subject
    .collect().await?;

// Query accessibility - subject-first (like check())
vault.resources()
    .accessible_by("user:alice")  // Who is asking?
    .with_permission("view")      // What do they want to do?
    .resource_type("document")    // What type of resource?
    .collect().await?;
```

**Mnemonic**: "Who can do what to which?" for `check()`, "Which has what with whom?" for `Relationship`.

### When to Use Cow vs Into<String>

The SDK uses `Cow<'a, str>` internally for zero-copy efficiency, but exposes ergonomic `impl Into<String>` constructors by default. Here's when to use each:

| Scenario                     | Use                        | Reason                                   |
| ---------------------------- | -------------------------- | ---------------------------------------- |
| **Typical application code** | `Relationship::new()`      | Convenience outweighs micro-optimization |
| **String literals in code**  | `Relationship::borrowed()` | Zero allocation for static strings       |
| **Hot path (>10K ops/sec)**  | `Relationship::borrowed()` | Avoid allocation overhead                |
| **Building from user input** | `Relationship::new()`      | Input is already owned                   |
| **Storing in collections**   | `OwnedRelationship`        | Needs `'static` lifetime                 |
| **Sending across threads**   | `.into_owned()`            | Needs `'static` lifetime                 |
| **Builder method chains**    | `impl Into<Cow<'a, str>>`  | Accepts both borrowed and owned          |

**Examples**:

```rust
// ✅ Typical app code - use new() for simplicity
vault.relationships()
    .write(Relationship::new(doc_id, "viewer", user_id))
    .await?;

// ✅ Static strings - use borrowed() to avoid allocation
const ADMIN_RELATION: &str = "admin";
vault.relationships()
    .write(Relationship::borrowed("org:default", ADMIN_RELATION, "user:root"))
    .await?;

// ✅ Hot path batch operations - use borrowed where possible
for (subject, resource) in large_batch {
    let rel = Relationship::borrowed(resource.as_str(), "viewer", subject.as_str());
    batch.push(rel.into_owned());  // Convert when storing
}

// ✅ Builder methods use Into<Cow<'a, str>> - accepts both
vault.resources()
    .accessible_by("user:alice")        // &str - borrowed
    .with_permission(perm_string)        // String - owned
    .resource_type(Cow::Borrowed("doc")) // Explicit Cow
    .collect()
    .await?;
```

**Internal API Design Note**:

Builder methods that filter queries use `impl Into<Cow<'a, str>>` to accept both borrowed and owned data efficiently:

```rust
impl<'a> ResourcesListBuilder<'a> {
    // Accepts &str, String, and Cow<'a, str>
    pub fn resource_type(mut self, resource_type: impl Into<Cow<'a, str>>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }
}
```

This is more efficient than `impl Into<String>` when the caller has a `&str`, because it avoids allocation.

### Zero-Copy Batch Checks

```rust
// Borrowed batch - no allocation per check
let checks: Vec<(&str, &str, &str)> = vec![
    ("user:alice", "view", "doc:1"),
    ("user:alice", "edit", "doc:1"),
    ("user:bob", "view", "doc:1"),
];

let results = client
    .check_batch_borrowed(&checks)
    .collect()
    .await?;

// Compare to owned version (allocates)
let checks: Vec<CheckRequest> = vec![
    CheckRequest::new("user:alice", "view", "doc:1"),
    // ...
];
```

### When to Use Borrowed APIs

| Scenario                   | Recommended API                                   |
| -------------------------- | ------------------------------------------------- |
| Static strings/literals    | `Relationship::borrowed()`                        |
| Hot path, many checks      | `check_batch_borrowed()`                          |
| Data from parsed input     | `Relationship::borrowed()` if input outlives call |
| Cross-thread/async storage | `Relationship::owned()` or `.into_owned()`        |
| Unknown lifetime           | `Relationship::owned()`                           |

### Lifetime Troubleshooting

Common lifetime issues and solutions when working with `Cow<'a, str>` APIs:

**Problem: "borrowed value does not live long enough"**

```rust
// ❌ This won't compile - builder outlives the borrowed string
async fn check_user(vault: &VaultClient, user_id: String) -> Result<bool, Error> {
    let user_ref = format!("user:{}", user_id);
    let builder = vault.check(&user_ref, "view", "doc:1");
    // ... do other work ...
    builder.await  // Error: user_ref doesn't live long enough
}

// ✅ Solution 1: Don't store the builder, await immediately
async fn check_user(vault: &VaultClient, user_id: String) -> Result<bool, Error> {
    let user_ref = format!("user:{}", user_id);
    vault.check(&user_ref, "view", "doc:1").await
}

// ✅ Solution 2: Use owned data for deferred execution
async fn check_user(vault: &VaultClient, user_id: String) -> Result<bool, Error> {
    let user_ref = format!("user:{}", user_id);
    vault.check_owned(user_ref, "view".to_string(), "doc:1".to_string()).await
}
```

**Problem: "cannot return value referencing local variable"**

```rust
// ❌ This won't compile - returning a reference to local data
fn build_relationships(ids: &[String]) -> Vec<Relationship<'_>> {
    ids.iter()
        .map(|id| Relationship::borrowed("doc:1", "viewer", id.as_str()))
        .collect()  // Error: can't return borrowed data
}

// ✅ Solution: Use into_owned() before returning
fn build_relationships(ids: &[String]) -> Vec<OwnedRelationship> {
    ids.iter()
        .map(|id| Relationship::borrowed("doc:1", "viewer", id.as_str()).into_owned())
        .collect()
}

// ✅ Or use Relationship::new() which is always owned
fn build_relationships(ids: &[String]) -> Vec<OwnedRelationship> {
    ids.iter()
        .map(|id| Relationship::new("doc:1", "viewer", id))
        .collect()
}
```

**Problem: Moving builder across await points**

```rust
// ❌ Complex case: builder with borrowed data across await
async fn conditional_check(vault: &VaultClient, condition: bool) -> Result<bool, Error> {
    let subject = get_subject().await?;  // Returns String
    let builder = vault.resources()
        .accessible_by(&subject)  // Borrows subject
        .with_permission("view");

    if condition {
        do_something_else().await;  // subject must live past this await
    }

    builder.collect().await  // Error if subject dropped
}

// ✅ Solution: Structure code to not need the builder to outlive the borrow
async fn conditional_check(vault: &VaultClient, condition: bool) -> Result<bool, Error> {
    let subject = get_subject().await?;

    if condition {
        do_something_else().await;
    }

    // Create builder after all awaits that don't need it
    vault.resources()
        .accessible_by(&subject)
        .with_permission("view")
        .collect()
        .await
}
```

**Pattern: Builder Lifetime Boundaries**

The key insight is that builders with borrowed data must be awaited before the borrowed data goes out of scope. Structure your code so that:

1. Borrowed data is created
2. Builder is created from borrowed data
3. Builder is awaited (consuming the borrow)
4. Original data can now be dropped

```rust
// ✅ Correct pattern
async fn process(vault: &VaultClient) -> Result<(), Error> {
    let data = fetch_data().await?;  // 1. Create data

    vault.check(&data.user, "view", &data.resource)  // 2. Create builder
        .await?;  // 3. Await (borrow ends here)

    // 4. data can be dropped or modified now
    Ok(())
}
```

---

## Async Trait Objects & DI

Enable dependency injection and testing with trait objects.

### API Design: Concrete Types + Trait Abstraction

The SDK provides two patterns for authorization:

1. **Concrete types** (`VaultClient`) - Full builder API with all options
2. **Trait abstraction** (`AuthorizationClient`) - Simplified API for DI and testing

**When to Use Each**:

| Situation                         | Recommendation                 | Why                                |
| --------------------------------- | ------------------------------ | ---------------------------------- |
| Application code with VaultClient | Use `VaultClient` directly     | Full access to builder methods     |
| Application code needing DI       | Use `&dyn AuthorizationClient` | Swap implementations at runtime    |
| Library code accepting authz      | Use `impl AuthorizationClient` | Monomorphized, works with any impl |
| Testing with mocks                | Use `MockClient` via trait     | Same interface as production       |

**Performance Note**: The `AuthorizationClient` trait uses `async_trait` for object safety. For hot paths where vtable dispatch overhead matters (rare), use `VaultClient` directly and pass via generics:

```rust
// Hot path - direct VaultClient, no vtable
async fn authorize_hot_path(vault: &VaultClient, check: &AuthzCheck) -> Result<bool, Error> {
    vault.check(&check.subject, &check.permission, &check.resource).await
}

// Normal path - trait object, flexible
async fn authorize_normal(authz: &dyn AuthorizationClient, check: &AuthzCheck) -> Result<bool, Error> {
    authz.check(&check.subject, &check.permission, &check.resource).await
}
```

### The AuthorizationClient Trait (Object-Safe)

The SDK provides two API styles for the same operations:

| API Style   | Method                       | Returns                | Use Case                |
| ----------- | ---------------------------- | ---------------------- | ----------------------- |
| **Builder** | `vault.check(s, p, r)`       | `CheckRequest` builder | Fluent API with options |
| **Trait**   | `authz.check(s, p, r).await` | `Result<bool, Error>`  | DI and testing          |

The builder API (`VaultClient::check()`) returns a `CheckRequest` that supports `.with_context()`, `.timeout()`, `.trace()`, etc. The trait API (`AuthorizationClient::check()`) provides a simple async method for dependency injection.

**VaultClient implements both**: The builder methods are the primary API, but VaultClient also implements `AuthorizationClient` which internally awaits the builder with default options.

```rust
use async_trait::async_trait;

/// Object-safe authorization trait for dependency injection.
/// Implemented by VaultClient, MockClient, InMemoryClient.
/// Use this when you need `dyn AuthorizationClient`.
///
/// See [Testing Support](#testing-support) for the full trait definition
/// including all methods (check, write, delete, list, expand, simulate, watch).
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    // Core authorization (simplified signatures for object safety)
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    async fn check_batch(&self, checks: Vec<(&str, &str, &str)>) -> Result<Vec<bool>, Error>;

    // Relationship management
    async fn write(&self, relationship: Relationship) -> Result<(), Error>;
    async fn write_batch(&self, relationships: Vec<Relationship>) -> Result<(), Error>;
    async fn delete(&self, relationship: Relationship) -> Result<(), Error>;
    async fn delete_batch(&self, relationships: Vec<Relationship>) -> Result<(), Error>;

    // ... additional methods: resources_list, subjects_list, relationships_list, expand, simulate, watch
    // See Testing Support section for complete definition
}

// VaultClient bridges builder API to trait API
impl AuthorizationClient for VaultClient {
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error> {
        // Delegate to builder API with default options
        VaultClient::check(self, subject, permission, resource).await
    }
    // ... other methods delegate similarly
}
```

### Choosing Between API Styles

```rust
// For maximum features: Use VaultClient directly
// Access to builder methods, streaming, context, etc.
async fn process_request(
    vault: &VaultClient,
    request: Request,
) -> Result<Response, Error> {
    vault.check(&request.user, "access", &request.resource)
        .with_context(Context::from_request(&request))
        .trace(true)
        .await?;
    // ...
}

// For flexibility/DI: Use trait objects
async fn authorize_request(
    authz: &dyn AuthorizationClient,
    user: &str,
    action: &str,
    resource: &str,
) -> Result<bool, Error> {
    // vtable dispatch - slight overhead, but enables runtime polymorphism
    // Trait API uses default options (no context, no trace, default timeout)
    authz.check(user, action, resource).await
}

// For application state: Store as trait object
type SharedAuthz = Arc<dyn AuthorizationClient>;

struct AppState {
    authz: SharedAuthz,
}
```

### Trait Object Usage

```rust
// Accept any implementation
async fn authorize_request(
    authz: &dyn AuthorizationClient,
    user: &str,
    action: &str,
    resource: &str,
) -> Result<bool, Error> {
    authz.check(user, action, resource).await
}

// Or with Arc for shared ownership
type SharedAuthz = Arc<dyn AuthorizationClient>;

struct AppState {
    authz: SharedAuthz,
}
```

### Dependency Injection Pattern

```rust
// Production - VaultClient implements AuthorizationClient
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");
let app = App::new(Arc::new(vault) as SharedAuthz);

// Testing
let mock = MockClient::builder()
    .check("user:alice", "view", "doc:1", true)
    .build();

let app = App::new(Arc::new(mock) as SharedAuthz);
```

### Generic Bounds

```rust
// Function generic over authorization implementation
async fn process_request<A: AuthorizationClient>(
    authz: &A,
    request: Request,
) -> Result<Response, Error> {
    let allowed = authz.check(&request.user, "access", &request.resource).await?;
    // ...
}

// Struct generic over authorization
struct RequestHandler<A: AuthorizationClient> {
    authz: A,
}

impl<A: AuthorizationClient> RequestHandler<A> {
    pub async fn handle(&self, req: Request) -> Result<Response, Error> {
        // ...
    }
}
```

### Testing with Trait Objects

```rust
#[tokio::test]
async fn test_authorization_flow() {
    // Create mock with stubbed results (see Testing Support section for full API)
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:bob", "view", "doc:1", false)
        .verify_on_drop(true)  // Verify all expectations were consumed
        .build();

    // Test through trait object
    let authz: &dyn AuthorizationClient = &mock;

    assert!(authz.check("user:alice", "view", "doc:1").await.unwrap());
    assert!(!authz.check("user:bob", "view", "doc:1").await.unwrap());

    // Verification happens automatically on drop when verify_on_drop(true)
}
```

---

## Authorization Checks

All authorization operations require vault scoping via `client.organization(...).vault(...)`.

### The Hero Pattern: `require()`

For HTTP handlers and most authorization scenarios, use `require()` as your primary pattern:

```rust
// The recommended pattern for HTTP handlers
async fn get_document(
    vault: &VaultClient,
    user_id: &str,
    doc_id: &str,
) -> Result<Document, AppError> {
    // Guard clause - fail fast on denial
    vault.check(user_id, "view", doc_id)
        .require()
        .await?;  // Returns Err(AccessDenied) on denial → 403 in HTTP handlers

    // Authorized - proceed with operation
    let doc = fetch_document(doc_id).await?;
    Ok(doc)
}
```

**Why `require()` is the hero pattern:**

- Fail-fast semantics match HTTP authorization flow
- `?` operator provides clean early-return
- `AccessDenied` error integrates with Axum/Actix error handlers
- Reads naturally: "require view permission on doc"

### Core API

```rust
// Get vault-scoped client for authorization operations
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");

// Simple check - returns bool (use when you need the boolean value)
let allowed = vault.check("user:alice", "view", "document:readme").await?;

// The hero pattern - early-return on denial
vault.check("user:alice", "view", "document:readme")
    .require()
    .await?;

// With ABAC context
vault.check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.50")
        .insert("mfa_verified", true))
    .require()
    .await?;
```

### Detailed Decisions (Debugging & Audit)

When you need the full decision trace for debugging or audit logs:

```rust
let decision = vault
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .detailed()
    .await?;

println!("Allowed: {}, Reason: {:?}", decision.allowed, decision.reason);
for step in &decision.trace {
    println!("  {} -> {}", step.rule, step.result);
}
```

### CheckRequest Builder with IntoFuture

The `check()` method returns a builder that implements `IntoFuture`, enabling both simple one-liner usage and optional configuration:

````rust
/// ABAC context for attribute-based authorization checks.
///
/// Provides additional attributes that can be evaluated by policy conditions,
/// such as IP addresses, time of day, MFA status, or custom application data.
#[derive(Debug, Clone, Default)]
pub struct Context {
    attributes: HashMap<String, ContextValue>,
}

/// Values that can be stored in an ABAC context.
#[derive(Debug, Clone)]
pub enum ContextValue {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    List(Vec<ContextValue>),
    Map(HashMap<String, ContextValue>),
}

impl Context {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    /// Insert an attribute into the context.
    /// Supports strings, booleans, integers, and floats.
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<ContextValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Get an attribute value by key.
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.attributes.get(key)
    }

    /// Check if context contains a key.
    pub fn contains(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get the number of attributes.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Check if context is empty.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

// Convenient From implementations for ContextValue
impl From<&str> for ContextValue {
    fn from(s: &str) -> Self {
        ContextValue::String(s.to_owned())
    }
}

impl From<String> for ContextValue {
    fn from(s: String) -> Self {
        ContextValue::String(s)
    }
}

impl From<bool> for ContextValue {
    fn from(b: bool) -> Self {
        ContextValue::Bool(b)
    }
}

impl From<i64> for ContextValue {
    fn from(i: i64) -> Self {
        ContextValue::Int(i)
    }
}

impl From<i32> for ContextValue {
    fn from(i: i32) -> Self {
        ContextValue::Int(i as i64)
    }
}

impl From<f64> for ContextValue {
    fn from(f: f64) -> Self {
        ContextValue::Float(f)
    }
}

/// Builder for authorization check requests.
/// Implements IntoFuture for ergonomic await syntax.
#[must_use = "CheckRequest does nothing until awaited"]
pub struct CheckRequest<'a> {
    vault: &'a VaultClient,
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
    context: Option<Context>,
    trace: bool,
    consistency: Option<ConsistencyToken>,
    timeout: Option<Duration>,
}

impl<'a> CheckRequest<'a> {
    /// Add ABAC context to the check
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Enable decision trace for debugging
    pub fn trace(mut self, enabled: bool) -> Self {
        self.trace = enabled;
        self
    }

    /// Request detailed decision (not just bool)
    pub fn detailed(self) -> DetailedCheckRequest<'a> {
        DetailedCheckRequest { inner: self }
    }

    /// Ensure read-after-write consistency with a token
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Override the client-level timeout for this operation
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Transform into a require request that returns `Result<(), AccessDenied>`.
    /// Use this for guard clauses in HTTP handlers for clean early-return on denial.
    ///
    /// # Example
    /// ```rust
    /// // In an HTTP handler - returns 403 on denial
    /// vault.check(user_id, "view", doc_id)
    ///     .require()
    ///     .await?;  // Err(AccessDenied) → 403 Forbidden
    /// ```
    pub fn require(self) -> RequireCheckRequest<'a> {
        RequireCheckRequest { inner: self }
    }
}

// IntoFuture enables direct .await on the builder
impl<'a> IntoFuture for CheckRequest<'a> {
    type Output = Result<bool, Error>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            self.vault.execute_check(self).await
        }
    }
}

/// Request that returns full decision details instead of just a boolean.
/// Created via `CheckRequest::detailed()`.
#[must_use = "DetailedCheckRequest does nothing until awaited"]
pub struct DetailedCheckRequest<'a> {
    inner: CheckRequest<'a>,
}

impl<'a> DetailedCheckRequest<'a> {
    /// Ensure read-after-write consistency with a token
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.inner.consistency = Some(token);
        self
    }

    /// Override the client-level timeout for this operation
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = Some(timeout);
        self
    }
}

impl<'a> IntoFuture for DetailedCheckRequest<'a> {
    type Output = Result<Decision, Error>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            self.inner.vault.execute_detailed_check(self.inner).await
        }
    }
}

/// Request that fails with `AccessDenied` on denial instead of returning `false`.
/// Created via `CheckRequest::require()`. Ideal for HTTP handler guard clauses.
#[must_use = "RequireCheckRequest does nothing until awaited"]
pub struct RequireCheckRequest<'a> {
    inner: CheckRequest<'a>,
}

impl<'a> RequireCheckRequest<'a> {
    /// Add ABAC context to the check
    pub fn with_context(mut self, context: Context) -> Self {
        self.inner.context = Some(context);
        self
    }

    /// Enable decision trace for debugging
    pub fn trace(mut self, enabled: bool) -> Self {
        self.inner.trace = enabled;
        self
    }

    /// Ensure read-after-write consistency with a token
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.inner.consistency = Some(token);
        self
    }

    /// Override the client-level timeout for this operation
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner.timeout = Some(timeout);
        self
    }
}

impl<'a> IntoFuture for RequireCheckRequest<'a> {
    type Output = Result<(), AccessDenied>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let allowed = self.inner.vault.execute_check(self.inner).await
                .map_err(|e| AccessDenied::from_error(e))?;

            if allowed {
                Ok(())
            } else {
                Err(AccessDenied::new(
                    self.inner.subject.into_owned(),
                    self.inner.permission.into_owned(),
                    self.inner.resource.into_owned(),
                ))
            }
        }
    }
}
````

**Usage patterns enabled by IntoFuture**:

```rust
// Get vault-scoped client for authorization operations
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");

// Pattern 1: Simple one-liner (IntoFuture converts builder to future)
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Pattern 2: Builder with options (still uses IntoFuture)
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .with_context(ctx)
    .trace(true)
    .await?;

// Pattern 3: Store builder for later execution
let check = vault.check("user:alice", "view", "doc:1");
// ... do other work ...
let allowed = check.await?;  // Execute when ready
```

### Type-Transforming Methods

Some methods transform the builder into a different type with a different return value. These are **terminal transformations** - you cannot chain further options after calling them:

| Method        | Transforms To          | Returns                    | Notes                    |
| ------------- | ---------------------- | -------------------------- | ------------------------ |
| `.detailed()` | `DetailedCheckRequest` | `Decision`                 | Full decision with trace |
| `.require()`  | `RequireCheckRequest`  | `Result<(), AccessDenied>` | Early-return pattern     |

```rust
// ✅ Correct: Options before transformation
let decision = vault
    .check("user:alice", "view", "doc:1")
    .with_context(ctx)   // Option on CheckRequest
    .trace(true)          // Option on CheckRequest
    .detailed()           // Transform to DetailedCheckRequest
    .await?;              // Returns Decision

// ❌ Won't compile: Can't add options after transformation
vault
    .check("user:alice", "view", "doc:1")
    .detailed()           // Now DetailedCheckRequest
    .with_context(ctx)    // Error: method doesn't exist on DetailedCheckRequest
    .await?;
```

**Why IntoFuture over async methods**:

| Approach                        | Pros                     | Cons                         |
| ------------------------------- | ------------------------ | ---------------------------- |
| `async fn check()`              | Simple                   | Can't add options after call |
| `fn check() -> impl Future`     | Options via builder      | Verbose `.execute().await`   |
| `fn check() -> impl IntoFuture` | Options + clean `.await` | Requires Rust 1.64+          |

### Result Handling

**Primary pattern: `require()`** (see [The Hero Pattern](#the-hero-pattern-require) above)

For custom error handling, use the boolean result directly:

```rust
let allowed = vault.check("user:alice", "view", "doc:1").await?;
if !allowed {
    return Err(AppError::AccessDenied {
        user: user_id.clone(),
        resource: doc_id.clone(),
    });
}
```

### Convenience Helpers

These patterns are useful in specific scenarios but are secondary to the hero pattern.

| Helper                | Use When                         | Returns                    |
| --------------------- | -------------------------------- | -------------------------- |
| `then(closure)`       | Conditionally execute on success | `Result<Option<T>, Error>` |
| `filter_authorized()` | Filter collections by permission | `Result<Vec<T>, Error>`    |

#### Then Pattern

When you want to combine auth check with an operation in a functional style:

```rust
// Execute action only if authorized:
let document = vault.check("user:alice", "view", "doc:1")
    .then(|| fetch_document(doc_id))
    .await?;  // Returns Option<Document>

match document {
    Some(doc) => Ok(Json(doc)),
    None => Err(StatusCode::FORBIDDEN),
}
```

#### Filter Authorized

When you need to filter a collection down to authorized items:

```rust
// Filter a collection to only authorized items:
let accessible_docs = vault
    .filter_authorized("user:alice", "view", &documents, |doc| format!("document:{}", doc.id))
    .await?;
```

### AccessDenied Error Type

The `AccessDenied` error returned by `require()` integrates with common web frameworks.

**Note**: `AccessDenied` (authorization decision "deny") is distinct from `ErrorKind::Forbidden` (lacking permission for a management API operation). See [Error Semantics](#check-vs-require-error-semantics) for details.

```rust
/// Error returned when authorization is denied (subject lacks permission)
#[derive(Debug, Clone)]
pub struct AccessDenied {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub request_id: Option<String>,
}

impl std::fmt::Display for AccessDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "access denied: {} cannot {} on {}",
            self.subject, self.permission, self.resource
        )
    }
}

impl std::error::Error for AccessDenied {}

impl AccessDenied {
    /// Create a new AccessDenied error
    pub fn new(
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            request_id: None,
        }
    }

    /// Create from a general Error (for network/server errors during auth check)
    pub fn from_error(error: Error) -> Self {
        Self {
            subject: String::new(),
            permission: String::new(),
            resource: String::new(),
            request_id: error.request_id().map(String::from),
        }
    }

    /// Set the request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Convert to HTTP status code
    pub fn status_code(&self) -> u16 { 403 }

    /// Convert to a user-safe message
    pub fn user_message(&self) -> &'static str {
        "You don't have permission to perform this action"
    }
}

// Integrates with common error types
impl From<AccessDenied> for axum::http::StatusCode {
    fn from(_: AccessDenied) -> Self { Self::FORBIDDEN }
}

impl From<AccessDenied> for actix_web::error::Error {
    fn from(e: AccessDenied) -> Self {
        actix_web::error::ErrorForbidden(e.user_message())
    }
}
```

### Batch Checks

**Primary pattern: `check_batch().collect()`** for most batch authorization scenarios.

```rust
// Primary pattern - collect all results
let results = vault
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
        ("user:bob", "view", "doc:1"),
    ])
    .collect()
    .await?;

for (check, allowed) in results {
    println!("{} {} {} = {}", check.0, check.1, check.2, allowed);
}
```

**Streaming pattern** - for large batches where you want to process results incrementally:

```rust
let mut stream = vault
    .check_batch(checks)
    .stream();

while let Some(result) = stream.next().await {
    let (check, allowed) = result?;
    handle_result(check, allowed);
}
```

**Check and CheckBatchStream Types**:

```rust
/// A single authorization check request
#[derive(Debug, Clone)]
pub struct Check<'a> {
    /// The subject requesting access
    pub subject: Cow<'a, str>,
    /// The permission being checked
    pub permission: Cow<'a, str>,
    /// The resource being accessed
    pub resource: Cow<'a, str>,
    /// Optional consistency token
    pub consistency: Option<ConsistencyToken>,
}

impl<'a> Check<'a> {
    /// Create a new check from tuple-like syntax
    pub fn new(
        subject: impl Into<Cow<'a, str>>,
        permission: impl Into<Cow<'a, str>>,
        resource: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            consistency: None,
        }
    }
}

/// Convenience: allow tuple conversion for ergonomic batch syntax
impl<'a> From<(&'a str, &'a str, &'a str)> for Check<'a> {
    fn from((s, p, r): (&'a str, &'a str, &'a str)) -> Self {
        Self::new(s, p, r)
    }
}

/// Stream of batch check results
pub struct CheckBatchStream<'a> {
    // Internal implementation
    inner: Pin<Box<dyn Stream<Item = Result<(Check<'a>, bool), Error>> + Send + 'a>>,
}

impl<'a> CheckBatchStream<'a> {
    /// Collect all results into a Vec
    pub async fn collect(self) -> Result<Vec<(Check<'a>, bool)>, Error> {
        self.inner.try_collect().await
    }

    /// Get the underlying stream for custom processing
    pub fn stream(self) -> impl Stream<Item = Result<(Check<'a>, bool), Error>> + 'a {
        self.inner
    }
}

/// Authorization check representation used in simulation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCheck {
    /// The subject that was checked
    pub subject: String,
    /// The permission that was checked
    pub permission: String,
    /// The resource that was checked
    pub resource: String,
}
```

### Batch Size Constraints

Understanding batch limits prevents surprises in production:

| Operation                                             | Max Batch Size | Recommended   | Notes                                    |
| ----------------------------------------------------- | -------------- | ------------- | ---------------------------------------- |
| `check_batch()`                                       | 1,000          | 100-500       | Larger batches increase latency variance |
| `relationships().write_batch()`                       | 10,000         | 1,000-5,000   | Transactional - all or nothing           |
| `relationships().delete_batch()`                      | 10,000         | 1,000-5,000   | Transactional - all or nothing           |
| `resources/subjects/relationships().list().collect()` | Unlimited      | Use streaming | Memory-bound by client                   |

```rust
// Exceeding limits returns an error
let checks: Vec<_> = (0..2000).map(|i| ("user:alice", "view", format!("doc:{}", i))).collect();
let result = vault.check_batch(&checks).collect().await;
// Err(Error { kind: InvalidInput, message: "Batch size 2000 exceeds limit of 1000" })

// Chunk large batches automatically
use futures::stream::{self, StreamExt, TryStreamExt};

async fn chunked_batch_check(
    vault: &VaultClient,
    checks: Vec<(&str, &str, &str)>,
) -> Result<Vec<bool>, Error> {
    const CHUNK_SIZE: usize = 500;

    stream::iter(checks.chunks(CHUNK_SIZE))
        .map(|chunk| vault.check_batch(chunk).collect())
        .buffer_unordered(4)  // 4 concurrent batches
        .try_concat()
        .await
}
```

### Batch Operation Semantics

Understanding the difference between atomic and streaming batch behavior:

| Operation                                 | Semantics     | Failure Behavior                                 |
| ----------------------------------------- | ------------- | ------------------------------------------------ |
| `check_batch()`                           | **Streaming** | Individual check failures don't affect others    |
| `relationships().write_batch()`           | **Atomic**    | All or nothing - partial failure rolls back      |
| `relationships().delete_batch()`          | **Atomic**    | All or nothing - partial failure rolls back      |
| `relationships().write_batch_streaming()` | **Streaming** | Each write independent, partial success possible |

**Batch Ordering and Parallelization Guarantees**:

| Property                | Guarantee                                                            |
| ----------------------- | -------------------------------------------------------------------- |
| **Result ordering**     | Results are returned in the same order as input items                |
| **SDK parallelization** | SDK may execute checks in parallel (up to `batch_parallelism` limit) |
| **Partial failure**     | Streaming: per-item `Result`; Atomic: all-or-nothing `Result`        |
| **Short-circuit**       | Streaming batches never short-circuit; all items are evaluated       |

```rust
// Results preserve input order even with parallel execution
let checks = [
    ("user:alice", "view", "doc:1"),
    ("user:bob", "view", "doc:2"),
    ("user:charlie", "view", "doc:3"),
];

let results = vault
    .check_batch(checks)
    .collect()
    .await?;

// Results preserve input order - each tuple contains the original check and result
// results[0] = (Check { subject: "user:alice", ... }, allowed)
// results[1] = (Check { subject: "user:bob", ... }, allowed)
// results[2] = (Check { subject: "user:charlie", ... }, allowed)
assert_eq!(results.len(), checks.len());

// Access just the booleans if needed
let allowed: Vec<bool> = results.iter().map(|(_, allowed)| *allowed).collect();
```

**Atomic Batches (write_batch, delete_batch)**:

```rust
// Atomic: Either all writes succeed, or none do
let result = vault.relationships()
    .write_batch([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:1", "viewer", "user:bob"),
        Relationship::new("doc:1", "viewer", "user:charlie"),
    ])
    .await;

match result {
    Ok(_) => {
        // All three relationships created
    }
    Err(e) => {
        // None of the relationships created
        // Transaction was rolled back
    }
}
```

**Streaming Batches (check_batch, write_batch_streaming)**:

```rust
// Streaming: Each operation is independent
let mut stream = vault
    .check_batch([
        ("user:alice", "view", "doc:1"),
        ("user:bob", "view", "doc:1"),      // This might fail individually
        ("user:charlie", "view", "doc:1"),
    ])
    .stream();

while let Some(result) = stream.next().await {
    match result {
        Ok((check, allowed)) => {
            println!("{}: {}", check.subject, allowed);
        }
        Err(e) => {
            // One check failed, but stream continues
            println!("Check failed: {}", e);
        }
    }
}

// For writes that tolerate partial success:
let mut stream = vault.relationships()
    .write_batch_streaming([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:1", "viewer", "user:bob"),
    ])
    .stream();

let mut successes = 0;
let mut failures = 0;

while let Some(result) = stream.next().await {
    match result {
        Ok(_) => successes += 1,
        Err(_) => failures += 1,
    }
}
```

**Choosing Atomic vs Streaming**:

| Use Case                               | Recommended                                |
| -------------------------------------- | ------------------------------------------ |
| Adding user to multiple resources      | `relationships().write_batch()` (atomic)   |
| Bulk import (tolerate partial failure) | `relationships().write_batch_streaming()`  |
| Authorization checks                   | `check_batch()` (streaming)                |
| Remove user from all resources         | `relationships().delete_batch()` (atomic)  |
| Cleanup/migration                      | `relationships().delete_batch_streaming()` |

### Expand Operation

```rust
// Expand shows why a permission would be granted/denied
let expansion = vault
    .expand("user:alice", "edit", "document:readme")
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

**ExpandBuilder and ExpansionNode Types**:

```rust
/// Builder for expand operations
pub struct ExpandBuilder<'a> {
    vault: &'a VaultClient,
    resource: Cow<'a, str>,
    relation: Cow<'a, str>,
    max_depth: Option<u32>,
    include_intermediate: bool,
}

impl<'a> ExpandBuilder<'a> {
    /// Limit expansion depth (default: no limit)
    #[must_use]
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Include intermediate nodes in the result (default: false)
    #[must_use]
    pub fn include_intermediate(mut self, include: bool) -> Self {
        self.include_intermediate = include;
        self
    }
}

impl<'a> IntoFuture for ExpandBuilder<'a> {
    type Output = Result<ExpansionNode, Error>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move { /* ... */ }
    }
}

/// A node in the permission expansion tree
#[derive(Debug, Clone)]
pub struct ExpansionNode {
    /// Type of operation at this node
    pub operation: NodeOperation,

    /// Human-readable description of this expansion step
    pub description: String,

    /// Whether this path grants access
    pub grants_access: bool,

    /// Child nodes (for compound operations like union/intersection)
    pub children: Vec<ExpansionNode>,

    /// The relationship that led to this node (if applicable)
    pub relationship: Option<OwnedRelationship>,

    /// Source location in schema
    pub source: Option<SchemaLocation>,
}
```

---

## Structured Decision Traces

Decision traces must be queryable, not just printable.

### DecisionNode Tree Structure

```rust
#[derive(Debug, Clone)]
pub struct Decision {
    pub allowed: bool,
    pub reason: DecisionReason,
    pub trace: Option<DecisionNode>,
    pub metadata: DecisionMetadata,
}

#[derive(Debug, Clone)]
pub struct DecisionNode {
    /// Unique identifier for this node
    pub id: NodeId,

    /// Type of operation at this node
    pub operation: NodeOperation,

    /// Human-readable description
    pub description: String,

    /// Result of this node's evaluation
    pub result: NodeResult,

    /// Child nodes (for compound operations)
    pub children: Vec<DecisionNode>,

    /// Performance metrics for this node
    pub metrics: NodeMetrics,

    /// Source location in schema (for debugging)
    pub source: Option<SchemaLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOperation {
    /// Direct relationship lookup
    Direct,
    /// Computed permission (union/intersection)
    Computed,
    /// Union of multiple paths
    Union,
    /// Intersection of multiple paths
    Intersection,
    /// Exclusion (NOT)
    Exclusion,
    /// ABAC condition evaluation
    Condition,
    /// Recursive/transitive lookup
    Recursive,
}

#[derive(Debug, Clone)]
pub struct NodeResult {
    pub satisfied: bool,
    pub cached: bool,
    pub short_circuited: bool,
}

#[derive(Debug, Clone)]
pub struct NodeMetrics {
    pub duration: Duration,
    pub db_queries: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub relationships_traversed: u32,
}

/// Why the authorization decision was made
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// Direct relationship exists between subject and resource
    DirectRelationship(OwnedRelationship),
    /// Permission granted through computed permission path
    ComputedPermission { path: Vec<String> },
    /// Permission granted through group/role membership
    GroupMembership { group: String, relation: String },
    /// Permission denied - no valid path found
    NoValidPath,
    /// Permission denied - excluded by explicit denial rule
    ExplicitDenial { rule: String },
    /// ABAC condition evaluated
    ConditionResult { condition: String, satisfied: bool },
}

/// Metadata about the decision execution
#[derive(Debug, Clone)]
pub struct DecisionMetadata {
    /// Unique request identifier
    pub request_id: String,
    /// Total evaluation time
    pub duration: Duration,
    /// Whether the result was served from cache
    pub cached: bool,
    /// Consistency token used for the evaluation
    pub consistency_token: Option<ConsistencyToken>,
    /// Schema version used for evaluation
    pub schema_version: u64,
    /// Timestamp of the evaluation
    pub evaluated_at: DateTime<Utc>,
}

/// Unique identifier for a decision tree node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new node ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Location in the schema where a permission rule is defined
#[derive(Debug, Clone)]
pub struct SchemaLocation {
    /// Entity type name (e.g., "document", "folder")
    pub entity_type: String,
    /// Relation or permission name
    pub relation: String,
    /// Line number in schema file (if available)
    pub line: Option<u32>,
    /// Column number in schema file (if available)
    pub column: Option<u32>,
}
```

### Querying Decision Trees

```rust
let decision = vault
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .await?;

// Find all paths that contributed to the decision
let contributing_paths = decision.trace
    .as_ref()
    .map(|t| t.find_satisfied_paths())
    .unwrap_or_default();

for path in contributing_paths {
    println!("Access granted via: {}", path.description());
}

// Find slow nodes for debugging
let slow_nodes = decision.trace
    .as_ref()
    .map(|t| t.find_nodes_slower_than(Duration::from_millis(10)))
    .unwrap_or_default();

for node in slow_nodes {
    println!("Slow operation: {:?} took {:?}", node.operation, node.metrics.duration);
}

// Get cache efficiency
let stats = decision.trace
    .as_ref()
    .map(|t| t.aggregate_metrics());
if let Some(stats) = stats {
    println!("Cache hit rate: {:.1}%",
        stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64 * 100.0
    );
}
```

### DecisionNode Query Methods

```rust
impl DecisionNode {
    /// Find all leaf nodes (actual relationship lookups)
    pub fn find_leaves(&self) -> Vec<&DecisionNode>;

    /// Find all nodes matching a predicate
    pub fn find_nodes<F>(&self, predicate: F) -> Vec<&DecisionNode>
    where
        F: Fn(&DecisionNode) -> bool;

    /// Find all paths from root to satisfied leaves
    pub fn find_satisfied_paths(&self) -> Vec<DecisionPath>;

    /// Find nodes slower than threshold
    pub fn find_nodes_slower_than(&self, threshold: Duration) -> Vec<&DecisionNode>;

    /// Aggregate metrics across all nodes
    pub fn aggregate_metrics(&self) -> AggregateMetrics;

    /// Convert to JSON for external analysis
    pub fn to_json(&self) -> serde_json::Value;

    /// Render as human-readable tree
    pub fn render_tree(&self) -> String;

    /// Get depth of the tree
    pub fn depth(&self) -> usize;
}
```

### Decision Serialization

```rust
// Serialize for logging/analysis
let decision = vault
    .check("user:alice", "edit", "document:readme")
    .trace(true)
    .await?;

// JSON for external tools
let json = serde_json::to_string_pretty(&decision)?;

// Structured logging
tracing::info!(
    allowed = decision.allowed,
    reason = ?decision.reason,
    duration_ms = decision.metadata.duration.as_millis(),
    cache_hit = decision.metadata.cached,
    trace = %decision.trace.as_ref().map(|t| t.render_tree()).unwrap_or_default(),
    "Authorization decision"
);
```

---

## Explain Permission

Deep permission explanation goes beyond simple allow/deny to provide full reasoning about why access was granted or denied.

### ExplainPermission API

```rust
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");

let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .await?;

// Check the result
if explanation.allowed {
    println!("Access granted via: {:?}", explanation.resolution_path);
} else {
    println!("Access denied. Reasons:");
    for reason in &explanation.denial_reasons {
        println!("  - {}", reason);
    }
}
```

### Explanation Response Type

```rust
/// Comprehensive permission explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionExplanation {
    /// Whether the permission was granted
    pub allowed: bool,

    /// The subject being checked
    pub subject: String,

    /// The permission being checked
    pub permission: String,

    /// The resource being accessed
    pub resource: String,

    /// The path that granted access (if allowed)
    pub resolution_path: Option<Vec<PathNode>>,

    /// Alternative paths that could grant access
    pub alternative_paths: Vec<Vec<PathNode>>,

    /// Reasons why access was denied (if denied)
    pub denial_reasons: Vec<DenialReason>,

    /// Schema context for the permission
    pub schema_context: SchemaContext,

    /// Attribute conditions that were evaluated
    pub attribute_conditions: Vec<EvaluatedCondition>,

    /// Suggestions for granting/revoking access
    pub suggestions: Vec<AccessSuggestion>,

    /// Query execution metadata
    pub metadata: ExplanationMetadata,
}

/// A node in the access path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    /// The entity being traversed
    pub entity: String,

    /// The relation traversed
    pub relation: String,

    /// How this node was reached
    pub via: PathVia,

    /// Whether this node contributed to the result
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathVia {
    /// Direct relationship exists
    Direct,
    /// Through group membership
    GroupMembership { group: String },
    /// Through relation inheritance
    Inheritance { from_relation: String },
    /// Through parent resource
    ParentResource { parent: String },
    /// Through computed permission
    Computed { expression: String },
}

/// Reason for access denial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenialReason {
    pub kind: DenialKind,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DenialKind {
    /// No direct relationship
    NoDirectRelationship,
    /// No group membership grants access
    NoGroupAccess,
    /// No inherited access from parent
    NoParentAccess,
    /// Attribute condition not satisfied
    ConditionNotMet { condition: String },
    /// Permission not defined in schema
    PermissionNotDefined,
}

/// Suggestion for modifying access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessSuggestion {
    pub action: SuggestionAction,
    pub description: String,
    /// SDK code to execute this suggestion
    pub code_example: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionAction {
    AddRelationship { relationship: String },
    AddToGroup { group: String },
    ModifyAttribute { attribute: String, value: String },
}

/// Metadata about the explanation query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationMetadata {
    /// Unique request identifier
    pub request_id: String,
    /// Total evaluation time
    pub duration: Duration,
    /// Number of relationships traversed
    pub relationships_traversed: u32,
    /// Maximum depth reached during traversal
    pub max_depth_reached: u32,
    /// Schema version used for evaluation
    pub schema_version: u64,
    /// Timestamp of the evaluation
    pub evaluated_at: DateTime<Utc>,
}

/// Schema context for the permission being explained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaContext {
    /// The entity type definition
    pub entity_type: String,
    /// The relation/permission definition as it appears in schema
    pub definition: String,
    /// Dependencies (other relations this permission depends on)
    pub dependencies: Vec<String>,
    /// Schema version
    pub schema_version: u64,
}

/// An ABAC condition that was evaluated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedCondition {
    /// The condition expression
    pub expression: String,
    /// Whether the condition was satisfied
    pub satisfied: bool,
    /// The attributes that were evaluated
    pub evaluated_attributes: Vec<EvaluatedAttribute>,
    /// Error if condition evaluation failed
    pub error: Option<String>,
}

/// An attribute value used in condition evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedAttribute {
    /// Attribute name
    pub name: String,
    /// Attribute value (as JSON)
    pub value: serde_json::Value,
    /// Source of the attribute value
    pub source: AttributeSource,
}

/// Where an attribute value came from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeSource {
    /// Stored on the resource
    Resource,
    /// Stored on the subject
    Subject,
    /// Stored on the relationship
    Relationship,
    /// Provided in the request context
    Context,
    /// Default value from schema
    Default,
}
```

### Explanation Options

```rust
// Get all possible access paths (not just the first match)
let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .all_paths(true)
    .await?;

// Limit traversal depth for performance
let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .max_depth(5)
    .await?;

// Include schema definition in response
let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .include_schema(true)
    .await?;
```

### Rendering Explanations

```rust
impl PermissionExplanation {
    /// Render as human-readable text
    pub fn render_text(&self) -> String;

    /// Render as ASCII tree diagram
    pub fn render_tree(&self) -> String;

    /// Render as Mermaid diagram
    pub fn render_mermaid(&self) -> String;

    /// Render as DOT (Graphviz)
    pub fn render_dot(&self) -> String;
}

// Example usage
let explanation = vault
    .explain_permission("user:alice", "view", "document:readme")
    .await?;

println!("{}", explanation.render_tree());
// Output:
// user:alice
//   └─ direct ─────────────────────────────────────┐
//      └─ viewer on document:readme ✓              │
//                                                  ▼
//                                     document:readme [view ✓]
```

---

## Simulate (What-If)

Test authorization decisions with ephemeral (hypothetical) relationships without persisting changes.

### Basic Simulation

```rust
// Check what would happen if we add a relationship
let result = vault
    .simulate()
    .check("user:alice", "view", "document:secret")
    .with_relationship(Relationship::new("user:alice", "viewer", "document:secret"))
    .await?;

assert!(result.allowed);
```

### Complex Scenarios

```rust
// Simulate multiple relationships
let result = vault
    .simulate()
    .check("user:alice", "edit", "document:report")
    .with_relationships([
        Relationship::new("user:alice", "member", "group:editors"),
        Relationship::new("group:editors", "editor", "document:report"),
    ])
    .await?;

// Simulate relationship removal
let result = vault
    .simulate()
    .check("user:bob", "view", "document:readme")
    .without_relationship(Relationship::new("user:bob", "viewer", "document:readme"))
    .await?;

// Combined add and remove
let result = vault
    .simulate()
    .check("user:alice", "view", "document:readme")
    .with_relationship(Relationship::new("user:alice", "viewer", "document:readme"))
    .without_relationship(Relationship::new("group:all", "viewer", "document:readme"))
    .await?;
```

### Batch Simulation

```rust
// Simulate multiple checks with the same hypothetical state
let results = vault
    .simulate()
    .with_relationships([
        Relationship::new("user:alice", "viewer", "document:secret"),
        Relationship::new("user:alice", "editor", "document:draft"),
    ])
    .check_batch([
        ("user:alice", "view", "document:secret"),
        ("user:alice", "edit", "document:secret"),
        ("user:alice", "view", "document:draft"),
        ("user:alice", "edit", "document:draft"),
    ])
    .collect()
    .await?;

// Results: [true, false, true, true]
```

**SimulateBuilder Type**:

```rust
/// Builder for what-if simulation queries
pub struct SimulateBuilder<'a> {
    vault: &'a VaultClient,
    added_relationships: Vec<Relationship<'a>>,
    removed_relationships: Vec<Relationship<'a>>,
    compare_to_current: bool,
    include_trace: bool,
}

impl<'a> SimulateBuilder<'a> {
    /// Add a hypothetical relationship for simulation
    #[must_use]
    pub fn with_relationship(mut self, rel: Relationship<'a>) -> Self {
        self.added_relationships.push(rel);
        self
    }

    /// Add multiple hypothetical relationships for simulation
    #[must_use]
    pub fn with_relationships(mut self, rels: impl IntoIterator<Item = Relationship<'a>>) -> Self {
        self.added_relationships.extend(rels);
        self
    }

    /// Remove a relationship for simulation (as if it doesn't exist)
    #[must_use]
    pub fn without_relationship(mut self, rel: Relationship<'a>) -> Self {
        self.removed_relationships.push(rel);
        self
    }

    /// Also run the check against current state to see if result differs
    #[must_use]
    pub fn compare_to_current(mut self, compare: bool) -> Self {
        self.compare_to_current = compare;
        self
    }

    /// Include full decision trace in the result
    #[must_use]
    pub fn with_trace(mut self) -> Self {
        self.include_trace = true;
        self
    }

    /// Run a single simulated check
    pub fn check(
        self,
        subject: impl Into<Cow<'a, str>>,
        permission: impl Into<Cow<'a, str>>,
        resource: impl Into<Cow<'a, str>>,
    ) -> SimulateCheckBuilder<'a> {
        SimulateCheckBuilder { /* ... */ }
    }

    /// Run multiple simulated checks with the same hypothetical state
    pub fn check_batch(
        self,
        checks: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
    ) -> SimulateBatchBuilder<'a> {
        SimulateBatchBuilder { /* ... */ }
    }
}

/// Builder for a single simulated check (implements IntoFuture)
pub struct SimulateCheckBuilder<'a> { /* ... */ }

impl<'a> IntoFuture for SimulateCheckBuilder<'a> {
    type Output = Result<SimulationResult, Error>;
    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move { /* ... */ }
    }
}

/// Builder for batch simulated checks
pub struct SimulateBatchBuilder<'a> { /* ... */ }

impl<'a> SimulateBatchBuilder<'a> {
    /// Collect all results into a Vec
    pub async fn collect(self) -> Result<Vec<SimulationResult>, Error> {
        /* ... */
    }

    /// Stream results as they complete
    pub fn stream(self) -> impl Stream<Item = Result<SimulationResult, Error>> {
        /* ... */
    }
}
```

### Simulation Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// The authorization result
    pub allowed: bool,

    /// The check that was performed
    pub check: AuthorizationCheck,

    /// Relationships that were added for simulation
    pub added_relationships: Vec<OwnedRelationship>,

    /// Relationships that were removed for simulation
    pub removed_relationships: Vec<OwnedRelationship>,

    /// Full decision trace (if requested)
    pub trace: Option<DecisionNode>,

    /// Whether the result differs from current state
    pub differs_from_current: bool,
}

// Check if simulation changes the outcome
let result = vault
    .simulate()
    .check("user:alice", "view", "document:readme")
    .with_relationship(Relationship::new("user:alice", "viewer", "document:readme"))
    .compare_to_current(true)  // Also check without the hypothetical
    .await?;

if result.differs_from_current {
    println!("Adding this relationship would change access from deny to allow");
}
```

---

## Relationship Management

Relationship operations are accessed via the `relationships()` sub-client, providing a consistent and extensible API.

### Write Operations

```rust
// Single write
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Batch write
vault.relationships()
    .write_batch([
        Relationship::new("folder:docs", "viewer", "group:engineering#member"),
        Relationship::new("document:readme", "parent", "folder:docs"),
    ])
    .await?;

// Conditional write (only if doesn't exist)
vault.relationships()
    .write(Relationship::new("document:readme", "viewer", "user:bob"))
    .unless_exists()
    .await?;
```

### Delete Operations

```rust
// Delete specific relationship
vault.relationships()
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Batch delete (atomic)
vault.relationships()
    .delete_batch([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:2", "viewer", "user:alice"),
    ])
    .await?;
```

### Bulk Delete by Query

Delete relationships matching complex filters without fetching them first:

```rust
// Delete all relationships for a subject
let deleted = vault.relationships()
    .delete_where()
    .subject("user:alice")
    .execute()
    .await?;
println!("Deleted {} relationships", deleted.count);

// Delete all relationships for a resource
let deleted = vault.relationships()
    .delete_where()
    .resource("document:readme")
    .execute()
    .await?;

// Delete specific relation type from a resource
let deleted = vault.relationships()
    .delete_where()
    .resource("document:readme")
    .relation("viewer")
    .execute()
    .await?;

// Delete with multiple filters (AND logic)
let deleted = vault.relationships()
    .delete_where()
    .resource_type("document")          // All documents
    .relation("viewer")                  // Viewer relation only
    .subject("user:departed_employee")   // Specific user
    .execute()
    .await?;

// Dry run - see what would be deleted without deleting
let preview = vault.relationships()
    .delete_where()
    .subject("user:alice")
    .dry_run()
    .await?;

println!("Would delete {} relationships:", preview.count);
for rel in preview.relationships.iter().take(10) {
    println!("  {} --{}-> {}", rel.resource, rel.relation, rel.subject);
}

// Delete with confirmation (for large deletes)
let deleted = vault.relationships()
    .delete_where()
    .subject("user:alice")
    .confirm_above(100)  // Require confirmation if >100 relationships
    .execute()
    .await?;
```

See [DeleteWhereBuilder](#delete-builders) for the full type definition.

**Safety Considerations**:

| Filter Combination           | Allowed | Notes                                     |
| ---------------------------- | ------- | ----------------------------------------- |
| No filters                   | ❌      | Compile error - too dangerous             |
| Subject only                 | ✅      | Common: remove user from everything       |
| Resource only                | ✅      | Common: clear all permissions on resource |
| Relation only                | ❌      | Too broad - requires resource or subject  |
| Subject + resource           | ✅      | Remove user from specific resource        |
| Subject type + resource type | ⚠️      | Requires confirmation above threshold     |

**No Matches Behavior**:

When `delete_where()` finds no relationships matching the filters, it succeeds with `count: 0`. This is intentional for idempotency:

```rust
// First call - deletes Alice's access
let result = vault.relationships()
    .delete_where()
    .subject("user:alice")
    .resource("document:readme")
    .execute()
    .await?;

println!("Deleted: {}", result.count);  // e.g., 3

// Second call - same filters, no matches
let result = vault.relationships()
    .delete_where()
    .subject("user:alice")
    .resource("document:readme")
    .execute()
    .await?;

println!("Deleted: {}", result.count);  // 0 - not an error

// Non-existent subject - also succeeds with count: 0
let result = vault.relationships()
    .delete_where()
    .subject("user:nonexistent")
    .execute()
    .await?;

assert_eq!(result.count, 0);  // No error, just zero matches
```

This behavior ensures:

- Idempotent cleanup scripts can be safely re-run
- User offboarding workflows don't fail if already completed
- Defensive deletion doesn't require existence checks

### Relationship Type Design

The `Relationship` type uses `Cow<'a, str>` for zero-copy efficiency while supporting both borrowed and owned data. See [Zero-Copy APIs](#zero-copy-apis) for the full type definition.

**API Convenience**: For ergonomic use, `Relationship::new()` accepts anything implementing `Into<String>`:

```rust
// These all work - new() uses Into<String> for convenience
Relationship::new("document:readme", "viewer", "user:alice");
Relationship::new(doc_id, "viewer", user_id);  // String variables
Relationship::new(format!("doc:{}", id), "viewer", subject);

// For hot paths, use borrowed() to avoid allocation
Relationship::borrowed("document:readme", "viewer", "user:alice");
```

**Type Alias for Owned Data**: When you need `'static` lifetime (e.g., storing in collections, sending across threads), use `OwnedRelationship` (see [Zero-Copy APIs](#zero-copy-apis)):

```rust
// Use OwnedRelationship when storing or sending across threads
let rel: OwnedRelationship = Relationship::owned("doc:1", "viewer", "user:alice");
```

### Relationship History

Query the change history of relationships for auditing and debugging:

```rust
// Get history for a specific relationship
let history = vault
    .relationships()
    .history("user:alice", "viewer", "document:readme")
    .await?;

for event in history {
    println!(
        "{}: {} by {}",
        event.timestamp,
        event.action,
        event.actor.unwrap_or("system".into())
    );
}
```

**History Query Builder**:

```rust
// Query history with filters
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
    let event = event?;
    process_event(event);
}
```

**History Event Type**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipHistoryEvent {
    /// When the change occurred
    pub timestamp: DateTime<Utc>,

    /// The action performed
    pub action: HistoryAction,

    /// The relationship that was changed
    pub relationship: Relationship<'static>,

    /// Who made the change (if audit logging enabled)
    pub actor: Option<String>,

    /// Request ID of the original operation
    pub request_id: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HistoryAction {
    Created,
    Deleted,
}
```

### Relationship Validation

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

**Validate Against Specific Schema Version**:

```rust
// Validate against a schema version (not the active one)
let result = vault
    .relationships()
    .validate(Relationship::new("user:alice", "viewer", "document:readme"))
    .against_schema(schema_id)
    .await?;
```

**Batch Validation**:

```rust
// Validate multiple relationships efficiently
let results = vault
    .relationships()
    .validate_batch([
        Relationship::new("user:alice", "viewer", "document:readme"),
        Relationship::new("user:bob", "invalid_relation", "document:readme"),
        Relationship::new("group:eng", "member", "user:charlie"),
    ])
    .collect()
    .await?;

for (rel, result) in relationships.iter().zip(results) {
    if !result.valid {
        println!("{} -> {}: {}", rel.subject, rel.resource, result.error.unwrap());
    }
}
```

**Validation Result Type**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the relationship is valid
    pub valid: bool,

    /// Error message if invalid
    pub error: Option<String>,

    /// The schema entity this relationship targets
    pub entity: Option<String>,

    /// Valid relations for this entity (for suggestions)
    pub available_relations: Vec<RelationInfo>,

    /// Suggestions for fixing invalid relationships
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationInfo {
    pub name: String,
    pub accepted_types: Vec<String>,
}
```

**Dry-Run Writes**:

```rust
// Preview what a write would do without committing
let preview = vault.relationships()
    .write(Relationship::new("user:alice", "viewer", "document:readme"))
    .dry_run(true)
    .await?;

println!("Would create: {}", preview.would_create);
println!("Validation: {:?}", preview.validation);
```

---

## Request ID & Idempotency

Safe retries for mutations require request ID support.

### Request ID for Mutations

```rust
use uuid::Uuid;

// Explicit request ID for idempotent retries
let request_id = Uuid::new_v4();

vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)
    .await?;

// Safe to retry with same request ID - server deduplicates
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)  // Same ID = same operation
    .await?;
```

### Auto-Generated Request IDs

```rust
// SDK can auto-generate request IDs
let client = Client::builder()
    .url("https://api.inferadb.com")
    .auto_request_id(true)  // Generate UUID for each mutation
    .build()
    .await?;

// Each write gets unique request ID automatically
vault.relationships().write(Relationship::new("doc:1", "viewer", "user:alice")).await?;
```

### Request ID in Responses

```rust
// All responses include request ID for debugging
let result = vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await;

match result {
    Ok(response) => {
        println!("Request ID: {}", response.request_id);
    }
    Err(e) => {
        // Errors also include request ID
        if let Some(request_id) = e.request_id() {
            eprintln!("Failed request: {}", request_id);
        }
    }
}
```

### Batch Operations with Request IDs

```rust
// Batch with single request ID (atomic)
vault.relationships()
    .write_batch([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:1", "editor", "user:bob"),
    ])
    .request_id(Uuid::new_v4())
    .await?;

// Or individual IDs per operation
vault.relationships()
    .write_batch_with_ids([
        (Uuid::new_v4(), Relationship::new("doc:1", "viewer", "user:alice")),
        (Uuid::new_v4(), Relationship::new("doc:1", "editor", "user:bob")),
    ])
    .await?;
```

### Idempotency Window

```rust
// Server maintains idempotency window (default: 24 hours)
// Requests with same ID within window return cached result

// First call - executes
vault.relationships().write(rel).request_id(id).await?;  // Executes, returns Ok

// Second call within window - returns cached result
vault.relationships().write(rel).request_id(id).await?;  // Returns cached Ok

// After window expires - executes again
// (may fail if relationship already exists)
```

### Request ID Lifecycle

Understanding how request IDs work across retries is critical for safe operations.

#### The Golden Rule: Same ID = Same Operation

When retrying, **always reuse the same request ID**. The server deduplicates based on the ID:

```rust
async fn write_with_retry(
    vault: &VaultClient,
    relationship: Relationship<'_>,
    max_retries: usize,
) -> Result<WriteResult, Error> {
    // Generate ID ONCE, before any attempts
    let request_id = Uuid::new_v4();

    let mut last_error = None;

    for attempt in 0..=max_retries {
        let result = vault.relationships()
            .write(relationship.clone())
            .request_id(request_id)  // ← Same ID for ALL attempts
            .await;

        match result {
            Ok(response) => return Ok(response),
            Err(e) if e.is_retriable() => {
                last_error = Some(e);
                tokio::time::sleep(backoff(attempt)).await;
                // Continue with same request_id
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap())
}
```

#### Common Mistake: New ID Per Retry

```rust
// ❌ WRONG: New ID per attempt can cause duplicates
async fn write_with_retry_wrong(vault: &VaultClient, relationship: Relationship<'_>) {
    for _ in 0..3 {
        let request_id = Uuid::new_v4();  // ❌ New ID each time!

        match vault.relationships()
            .write(relationship.clone())
            .request_id(request_id)
            .await
        {
            Ok(_) => return,
            Err(_) => continue,  // Retrying with different ID could create duplicates
        }
    }
}

// ✅ CORRECT: Generate ID once, reuse across retries
async fn write_with_retry_correct(vault: &VaultClient, relationship: Relationship<'_>) {
    let request_id = Uuid::new_v4();  // ✅ Generate ONCE

    for _ in 0..3 {
        match vault.relationships()
            .write(relationship.clone())
            .request_id(request_id)  // ✅ Reuse same ID
            .await
        {
            Ok(_) => return,
            Err(_) => continue,  // Safe - server deduplicates
        }
    }
}
```

#### Crash Recovery Pattern

For writes that must survive process crashes, persist the request ID:

```rust
/// Durable write that can recover from crashes
async fn durable_write(
    vault: &VaultClient,
    db: &Database,
    relationship: Relationship<'_>,
) -> Result<(), Error> {
    // Check if we have a pending request ID for this operation
    let operation_key = format!("{}:{}:{}",
        relationship.resource,
        relationship.relation,
        relationship.subject
    );

    let request_id = match db.get_pending_request(&operation_key).await? {
        Some(id) => {
            // Resuming after crash - reuse the same ID
            id
        }
        None => {
            // New operation - generate and persist ID
            let id = Uuid::new_v4();
            db.save_pending_request(&operation_key, id).await?;
            id
        }
    };

    // Perform the write
    vault.relationships()
        .write(relationship)
        .request_id(request_id)
        .await?;

    // Success - clear the pending ID
    db.clear_pending_request(&operation_key).await?;

    Ok(())
}
```

#### Request ID vs Relationship Existence

Request IDs and `unless_exists()` serve different purposes:

| Mechanism         | Purpose                                   | Scope                        |
| ----------------- | ----------------------------------------- | ---------------------------- |
| `request_id()`    | Deduplicate retries of the same operation | Per request_id, time-bounded |
| `unless_exists()` | Skip if relationship already exists       | Per relationship, permanent  |

```rust
// Scenario: Retry-safe creation, don't overwrite existing
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)   // Safe retries
    .unless_exists()          // Don't fail if already exists
    .await?;
```

### Conditional Writes

```rust
// Write only if not exists (idempotent by nature)
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .unless_exists()
    .await?;

// Write with precondition
vault.relationships()
    .write(Relationship::new("doc:1", "owner", "user:alice"))
    .precondition(Precondition::not_exists("doc:1", "owner", "*"))
    .await?;

// Atomic compare-and-swap
vault.relationships()
    .write(Relationship::new("doc:1", "owner", "user:bob"))
    .precondition(Precondition::exists("doc:1", "owner", "user:alice"))
    .await?;
```

### Consistency Tokens (Revision Semantics)

For read-after-write consistency across distributed nodes, use consistency tokens:

```rust
/// Opaque token representing a point-in-time snapshot of the authorization graph.
/// Obtained from write operations, used to ensure subsequent reads see the write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyToken(String);

impl ConsistencyToken {
    /// Create from raw string (e.g., from external storage)
    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the raw token for external storage
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

**Write-then-Read Pattern**:

```rust
// Write returns a consistency token
let write_result = vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

let token = write_result.consistency_token();

// Immediately read with consistency guarantee
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .at_least_as_fresh_as(token.clone())  // Ensures read sees the write
    .await?;

assert!(allowed);  // Guaranteed to see the write we just made
```

**Propagating Tokens Across Services**:

```rust
// Service A: Writes relationship, returns token to client
async fn add_viewer(vault: &VaultClient, doc: &str, user: &str) -> Result<ConsistencyToken, Error> {
    let result = vault.relationships()
        .write(Relationship::new(doc, "viewer", user))
        .await?;
    Ok(result.consistency_token())
}

// Service B: Uses token from Service A to ensure consistency
async fn check_access(
    vault: &VaultClient,
    user: &str,
    doc: &str,
    token: Option<ConsistencyToken>,  // Passed from Service A
) -> Result<bool, Error> {
    let mut check = vault.check(user, "view", doc);

    if let Some(token) = token {
        check = check.at_least_as_fresh_as(token);
    }

    check.await
}
```

**Consistency Levels**:

```rust
/// Consistency level for reads
pub enum Consistency {
    /// Eventually consistent - fastest, may not see recent writes
    Eventual,

    /// Bounded staleness - reads within N seconds of latest
    BoundedStaleness(Duration),

    /// At least as fresh as the given token
    AtLeastAsFreshAs(ConsistencyToken),

    /// Fully consistent - always reads latest (highest latency)
    Strong,
}

// Configure per-request
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .consistency(Consistency::BoundedStaleness(Duration::from_secs(5)))
    .await?;

// Or set default for client
let client = Client::builder()
    .default_consistency(Consistency::Strong)
    .build()
    .await?;
```

**Token Lifecycle**:

```text
Write ───► Token Created ───► Token Valid ───► Token Expired
                │                   │                │
                │                   │                └─ Reads with expired token
                │                   │                   fall back to eventual
                │                   │
                │                   └─ Reads guaranteed to see
                │                      at least this write
                │
                └─ Opaque string, can be serialized/stored
```

---

## Lookup Operations

### List Resources

```rust
// Find all documents Alice can view
let resources = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;

// Paginated results
let page = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .page_size(100)
    .page(1)
    .await?;
```

### List Subjects

```rust
// Find all users who can edit this document
let subjects = vault
    .subjects()
    .with_permission("edit")
    .on_resource("document:readme")
    .subject_type("user")
    .collect()
    .await?;
```

### List Relationships

```rust
// List all relationships for a resource
let relationships = vault
    .relationships().list()
    .resource("document:readme")
    .collect()
    .await?;

// Filter by relation
let viewers = vault
    .relationships().list()
    .resource("document:readme")
    .relation("viewer")
    .collect()
    .await?;

// Filter by subject
let alice_rels = vault
    .relationships().list()
    .subject("user:alice")
    .collect()
    .await?;
```

### Pagination

For explicit control over pagination, use cursor-based pagination:

```rust
// Cursor-based pagination (recommended for large datasets)
let mut cursor: Option<String> = None;
let mut all_resources = Vec::new();

loop {
    let page = vault
        .resources()
        .accessible_by("user:alice")
        .with_permission("view")
        .limit(100)
        .cursor(cursor.as_deref())
        .await?;

    all_resources.extend(page.items);

    match page.next_cursor {
        Some(next) => cursor = Some(next),
        None => break,  // No more pages
    }
}

// Offset-based pagination (for UI pagination)
let page: OffsetPage<_> = vault
    .relationships().list()
    .resource("document:readme")
    .offset(50, 25)  // Skip 50, fetch 25
    .await?;

println!("Showing {}-{} of {:?}",
    page.offset + 1,
    page.offset + page.items.len() as u64,
    page.total);
```

**Pagination Types**:

See [Sub-Client Types](#sub-client-types) for the full definitions of `CursorPage<T>` and `OffsetPage<T>`. The `Page<T>` type alias defaults to cursor-based pagination:

```rust
// Convenience alias - see Sub-Client Types for full definitions
pub type Page<T> = CursorPage<T>;
```

**Pagination vs Streaming**:

| Use Case                       | Approach   | Method                 |
| ------------------------------ | ---------- | ---------------------- |
| Display paginated UI           | Pagination | `.limit(25).offset(0)` |
| Process all items sequentially | Streaming  | `.stream()`            |
| Export/sync all data           | Streaming  | `.stream()`            |
| Random access to specific page | Pagination | `.limit(n).cursor(c)`  |
| Count total before fetching    | Pagination | `.count_only().await?` |
| Memory-constrained processing  | Streaming  | `.stream()`            |

---

## Streaming & Watch

True streaming without hidden buffering - `.collect()` is opt-in, not the default path.

### API Principle: Single vs Multi-Value Returns

The SDK uses different patterns based on return cardinality:

| Return Type         | Pattern                              | Example                                                                                      |
| ------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------- |
| **Single value**    | `IntoFuture` (direct `.await`)       | `vault.check(...).await?` → `bool`                                                           |
| **Multiple values** | Explicit `.stream()` or `.collect()` | `vault.resources().accessible_by(...).with_permission(...).collect().await?` → `Vec<String>` |

**Why this distinction?**

- Single-value operations are always bounded and predictable
- Multi-value operations could return thousands of items - you must explicitly choose streaming vs collecting

```rust
// Single value: use IntoFuture, await directly
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Multiple values: must choose .stream() or .collect()
let docs = vault.resources().accessible_by("user:alice").with_permission("view").collect().await?;
// OR
let mut stream = vault.resources().accessible_by("user:alice").with_permission("view").stream();
```

### Explicit Stream API

```rust
use futures::{Stream, StreamExt, TryStreamExt};

// Returns impl Stream - NO buffering, NO collect
let stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .resource_type("document")
    .stream();  // Explicit: returns Stream, not collected Vec

// Process items as they arrive
pin_mut!(stream);
while let Some(resource) = stream.try_next().await? {
    process_resource(resource);
}

// Or with stream combinators
let processed: Vec<_> = stream
    .map(|r| r.map(transform))
    .try_collect()
    .await?;
```

### Stream vs Collect

```rust
// ❌ Anti-pattern: Defeats streaming, buffers everything
let resources = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .collect()  // Loads ALL into memory
    .await?;

// ✅ True streaming: Process without buffering
let mut stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .stream();

while let Some(resource) = stream.try_next().await? {
    // Process one at a time, constant memory
}

// ✅ Bounded collection: When you need a Vec but want limits
let resources = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .take(100)  // Limit before collecting
    .collect()
    .await?;
```

### Stream Types

```rust
// All list operations return Stream types
pub trait ListResourcesExt {
    /// Returns a Stream of resources (preferred)
    fn stream(self) -> impl Stream<Item = Result<String, Error>>;

    /// Convenience: collect all into Vec (use with caution)
    async fn collect(self) -> Result<Vec<String>, Error>;

    /// Paginated: returns page at a time
    async fn page(self, page: usize, page_size: usize) -> Result<Page<String>, Error>;
}

// Batch checks also stream
let stream = vault
    .check_batch(checks)
    .stream();  // Stream<Item = Result<(Check, bool), Error>>
```

### Streaming Guarantees & Transport Differences

Understanding streaming behavior across transports:

| Guarantee              | gRPC                          | REST (SSE)                        |
| ---------------------- | ----------------------------- | --------------------------------- |
| Backpressure           | Native (HTTP/2 flow control)  | Client-side only (drop or buffer) |
| Ordering               | Guaranteed                    | Guaranteed                        |
| Reconnection           | Manual (SDK provides helpers) | Automatic (SSE built-in)          |
| Resume from checkpoint | Server-side cursor            | Revision token parameter          |
| Max message size       | 4MB default (configurable)    | No limit (chunked)                |
| Binary data            | Efficient (protobuf)          | Base64 encoded                    |

**gRPC Streaming Semantics**:

```rust
// gRPC streams maintain a persistent HTTP/2 connection
// Server can push items as they become available
// Backpressure: If consumer is slow, server sees flow control signals

let mut stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .stream();  // Opens gRPC server stream

// Each .next() pulls one item from the stream
// If stream buffer is empty, awaits server
// If stream buffer is full, server is backpressured
while let Some(resource) = stream.try_next().await? {
    // Processing speed affects server delivery rate
}
```

**REST/SSE Streaming Semantics**:

```rust
// REST streams use Server-Sent Events (SSE)
// Server pushes events; client has no backpressure mechanism
// SDK buffers incoming events to prevent data loss

let mut stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .stream();

// Events buffer while consumer processes
// If buffer fills, oldest events may be dropped (depends on config)
while let Some(resource) = stream.try_next().await? {
    // Processing should be fast to avoid buffer growth
}
```

**Stream Error Handling**:

```rust
// Streams surface errors as items, not panics
let mut stream = vault.resources().accessible_by("user:alice").with_permission("view").stream();

while let Some(result) = stream.next().await {
    match result {
        Ok(resource) => process(resource),
        Err(e) if e.is_retriable() => {
            // Network hiccup - stream may continue
            tracing::warn!("Retriable error: {}", e);
        }
        Err(e) => {
            // Fatal error - stream is terminated
            return Err(e);
        }
    }
}

// After error, stream is fused (returns None forever)
// Must create new stream to retry
```

**Transport-Specific Configuration**:

```rust
// Configure streaming behavior per transport
let client = Client::builder()
    .grpc(GrpcConfig {
        max_message_size: 16 * 1024 * 1024,  // 16MB
        stream_window_size: 1024 * 1024,      // 1MB flow control window
        keep_alive_interval: Duration::from_secs(30),
    })
    .rest(RestConfig {
        sse_buffer_size: 1000,  // Max events to buffer
        sse_retry_interval: Duration::from_secs(3),
    })
    .build()
    .await?;
```

### Watch for Changes

```rust
use futures::StreamExt;

// Watch all changes
let mut stream = vault
    .watch()
    .run()
    .await?;

while let Some(change) = stream.next().await {
    let change = change?;
    println!("{:?}: {} -[{}]-> {}",
        change.operation,
        change.relationship.subject,
        change.relationship.relation,
        change.relationship.resource
    );
}

// Filtered watch
let mut stream = vault
    .watch()
    .filter(WatchFilter::resource_type("document"))
    .filter(WatchFilter::operations([Operation::Create]))
    .from_revision(12345)  // Resume from checkpoint
    .run()
    .await?;
```

### Watch Filter Options

Comprehensive filtering to receive only relevant changes:

```rust
// Filter by resource type
let stream = vault.watch()
    .filter(WatchFilter::resource_type("document"))
    .run()
    .await?;

// Filter by subject type
let stream = vault.watch()
    .filter(WatchFilter::subject_type("user"))
    .run()
    .await?;

// Filter by relation
let stream = vault.watch()
    .filter(WatchFilter::relation("viewer"))
    .run()
    .await?;

// Filter by operation type
let stream = vault.watch()
    .filter(WatchFilter::operations([Operation::Create, Operation::Delete]))
    .run()
    .await?;

// Filter by specific resource
let stream = vault.watch()
    .filter(WatchFilter::resource("document:readme"))
    .run()
    .await?;

// Filter by specific subject
let stream = vault.watch()
    .filter(WatchFilter::subject("user:alice"))
    .run()
    .await?;

// Combine multiple filters (AND logic)
let stream = vault.watch()
    .filter(WatchFilter::resource_type("document"))
    .filter(WatchFilter::relation("viewer"))
    .filter(WatchFilter::operations([Operation::Create]))
    .run()
    .await?;
```

**WatchFilter Types**:

```rust
pub enum WatchFilter {
    /// Filter by resource type (e.g., "document", "folder")
    ResourceType(String),

    /// Filter by subject type (e.g., "user", "group")
    SubjectType(String),

    /// Filter by specific resource ID
    Resource(String),

    /// Filter by specific subject ID
    Subject(String),

    /// Filter by relation name
    Relation(String),

    /// Filter by operation type
    Operations(Vec<Operation>),

    /// Custom filter expression (server-evaluated)
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Create,
    Delete,
}
```

### Watch Reconnection Behavior

The watch stream automatically handles disconnections with configurable retry:

```rust
// Default: auto-reconnect with exponential backoff
let stream = vault.watch()
    .resumable()
    .run()
    .await?;

// Custom reconnection configuration
let stream = vault.watch()
    .resumable()
    .reconnect(ReconnectConfig {
        max_retries: Some(10),           // None = infinite
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        jitter: 0.1,
    })
    .run()
    .await?;

// Disable auto-reconnect (fail on disconnect)
let stream = vault.watch()
    .no_reconnect()
    .run()
    .await?;
```

**Reconnection Semantics**:

| Event                | Behavior with `.resumable()`          | Without `.resumable()`          |
| -------------------- | ------------------------------------- | ------------------------------- |
| Network timeout      | Reconnect from last revision          | Stream ends with error          |
| Server restart       | Reconnect from last revision          | Stream ends with error          |
| Auth token expired   | Attempt token refresh, then reconnect | Stream ends with `Unauthorized` |
| Server returns error | Depends on error type                 | Stream ends with error          |
| Max retries exceeded | Stream ends with error                | N/A                             |

**Resumption with Checkpoints**:

```rust
// The stream tracks its position internally
let mut stream = vault.watch()
    .resumable()
    .run()
    .await?;

// After processing, you can save the checkpoint for crash recovery
while let Some(change) = stream.next().await {
    let change = change?;

    // Process the change
    process_change(&change).await?;

    // Save checkpoint to database for crash recovery
    save_checkpoint(change.revision).await?;
}

// On restart, resume from checkpoint
let checkpoint = load_checkpoint().await?;
let stream = vault.watch()
    .from_revision(checkpoint)
    .resumable()
    .run()
    .await?;
```

### Watch Event Types

```rust
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// The operation that occurred
    pub operation: Operation,

    /// The relationship that changed
    pub relationship: OwnedRelationship,

    /// Server revision number (for resumption)
    pub revision: u64,

    /// Timestamp of the change
    pub timestamp: DateTime<Utc>,

    /// Actor who made the change (if audit logging enabled)
    pub actor: Option<String>,

    /// Request ID of the original operation
    pub request_id: Option<String>,
}

impl WatchEvent {
    /// Check if this is a creation event
    pub fn is_create(&self) -> bool {
        self.operation == Operation::Create
    }

    /// Check if this is a deletion event
    pub fn is_delete(&self) -> bool {
        self.operation == Operation::Delete
    }
}

/// Builder for watch streams
pub struct WatchBuilder<'a> {
    vault: &'a VaultClient,
    filters: Vec<WatchFilter>,
    from_revision: Option<u64>,
    resumable: bool,
    reconnect_config: Option<ReconnectConfig>,
}

impl<'a> WatchBuilder<'a> {
    /// Add a filter to narrow which changes are received
    #[must_use]
    pub fn filter(mut self, filter: WatchFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Resume from a specific revision (for crash recovery)
    #[must_use]
    pub fn from_revision(mut self, revision: u64) -> Self {
        self.from_revision = Some(revision);
        self
    }

    /// Enable automatic reconnection on disconnect
    #[must_use]
    pub fn resumable(mut self) -> Self {
        self.resumable = true;
        self
    }

    /// Disable automatic reconnection
    #[must_use]
    pub fn no_reconnect(mut self) -> Self {
        self.resumable = false;
        self
    }

    /// Configure custom reconnection behavior
    #[must_use]
    pub fn reconnect(mut self, config: ReconnectConfig) -> Self {
        self.reconnect_config = Some(config);
        self.resumable = true;
        self
    }

    /// Start the watch stream
    pub async fn run(self) -> Result<impl Stream<Item = Result<WatchEvent, Error>>, Error> {
        // ...
    }
}

/// Configuration for stream reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (None = infinite)
    pub max_retries: Option<u32>,

    /// Initial backoff duration
    pub initial_backoff: Duration,

    /// Maximum backoff duration
    pub max_backoff: Duration,

    /// Backoff multiplier (e.g., 2.0 for exponential)
    pub backoff_multiplier: f64,

    /// Random jitter factor (0.0 - 1.0)
    pub jitter: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: None,  // Infinite retries
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: 0.1,
        }
    }
}
```

### Resumable Streams

```rust
// Automatic reconnection with resume
let stream = vault
    .watch()
    .resumable()  // Auto-handles disconnects
    .run()
    .await?;
```

### Backpressure Handling

```rust
// Stream respects backpressure - server won't overwhelm client
let mut stream = vault
    .resources()
    .accessible_by("user:alice")
    .with_permission("view")
    .stream();

// Slow consumer is fine - server pauses
while let Some(resource) = stream.try_next().await? {
    slow_operation(resource).await;  // Won't cause unbounded buffering
}

// Or with explicit buffer limits
use futures::stream::StreamExt;

let buffered = stream
    .buffered(10)  // Max 10 in-flight
    .try_for_each(|r| async { process(r) })
    .await?;
```

### Cancellation Semantics

Understanding how to cancel in-flight operations and clean up streams is critical for well-behaved applications.

#### Drop-Based Cancellation

All SDK futures and streams support Drop-based cancellation. When a future or stream is dropped before completion, the underlying operation is cancelled:

```rust
use tokio::time::timeout;

// Timeout cancels the future if it takes too long
let result = timeout(Duration::from_secs(1), vault.check("user:alice", "view", "doc:1")).await;

match result {
    Ok(Ok(allowed)) => println!("Allowed: {}", allowed),
    Ok(Err(e)) => println!("SDK error: {}", e),
    Err(_) => println!("Operation timed out and was cancelled"),
}
```

#### Stream Cancellation

Streams are cancelled when dropped. This is safe and expected:

```rust
let mut stream = vault.watch().stream();

// Process for 60 seconds, then stop
let deadline = Instant::now() + Duration::from_secs(60);

loop {
    tokio::select! {
        Some(event) = stream.next() => {
            handle_event(event?);
        }
        _ = tokio::time::sleep_until(deadline) => {
            // Stream is dropped here, cleanly closing the connection
            break;
        }
    }
}
// stream dropped - server connection gracefully closed
```

#### Explicit Cancellation with AbortHandle

For more control, use `AbortHandle`:

```rust
use futures::future::AbortHandle;

// Create abortable check
let (check_future, abort_handle) = futures::future::abortable(
    vault.check("user:alice", "view", "doc:1")
);

// Spawn the check
let handle = tokio::spawn(check_future);

// Cancel from elsewhere
abort_handle.abort();

// Handle will return Err(Aborted)
match handle.await {
    Ok(Ok(Ok(allowed))) => println!("Completed: {}", allowed),
    Ok(Ok(Err(e))) => println!("SDK error: {}", e),
    Ok(Err(_)) => println!("Cancelled"),
    Err(e) => println!("Task panicked: {}", e),
}
```

#### Watch Stream Graceful Shutdown

The `watch()` stream supports explicit shutdown for graceful termination:

```rust
let (stream, shutdown) = vault.watch()
    .with_shutdown()
    .stream();

// In shutdown handler
shutdown.trigger();  // Gracefully closes the stream

// Stream will complete on next iteration
while let Some(event) = stream.next().await {
    if matches!(event, Err(Error::Shutdown)) {
        println!("Watch stream shut down gracefully");
        break;
    }
    handle_event(event?);
}
```

#### Cancellation Safety

| Operation                                 | Cancellation Safe? | Notes                                             |
| ----------------------------------------- | ------------------ | ------------------------------------------------- |
| `check().await`                           | ✅ Yes             | Unary request, no side effects                    |
| `check_batch().collect().await`           | ✅ Yes             | Partial results discarded                         |
| `relationships().write().await`           | ⚠️ Caution         | May complete on server even if client cancels     |
| `relationships().write_batch().await`     | ⚠️ Caution         | Atomic - all or nothing, but may commit on server |
| `watch().stream()`                        | ✅ Yes             | Drop closes connection cleanly                    |
| `resources().accessible_by(...).stream()` | ✅ Yes             | Drop closes connection cleanly                    |

**Important**: Write operations may complete on the server even if the client cancels. Always use idempotency keys for writes that might be cancelled:

```rust
let request_id = Uuid::new_v4();

// Even if this times out and we retry, the request_id prevents duplicates
let result = timeout(
    Duration::from_secs(5),
    vault.relationships()
        .write(Relationship::new("doc:1", "viewer", "user:alice"))
        .request_id(request_id)
).await;

match result {
    Ok(Ok(_)) => println!("Write confirmed"),
    Ok(Err(e)) => println!("Write failed: {}", e),
    Err(_) => {
        // Timed out - but write may have succeeded on server
        // Safe to retry with same request_id
        println!("Timed out, will retry with same request_id");
    }
}
```

---

## Caching

### Cache Behavior Summary

| Property               | Behavior                                         |
| ---------------------- | ------------------------------------------------ |
| **Scope**              | Per-client, in-memory                            |
| **Safety**             | Best-effort optimization; always safe to disable |
| **Consistency tokens** | Bypass cache, force fresh read from server       |
| **Watch invalidation** | Real-time cache invalidation via server events   |

### Local Decision Cache

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .cache(CacheConfig::default()
        .max_entries(10_000)
        .permission_ttl(Duration::from_secs(60))
        .negative_ttl(Duration::from_secs(10)))  // Cache denials shorter
    .build()
    .await?;

// Disable caching entirely (always hits server)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .cache(CacheConfig::disabled())
    .build()
    .await?;
```

### Consistency Tokens and Cache Interaction

**Critical**: When you use `.at_least_as_fresh_as(token)`, the cache is **bypassed** for that request. This ensures read-after-write consistency.

```rust
// Write returns a consistency token
let write_result = vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

let token = write_result.consistency_token();

// This check BYPASSES the cache and hits the server
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .at_least_as_fresh_as(token)  // Forces cache bypass
    .await?;

// Subsequent checks WITHOUT token may still hit stale cache
let maybe_stale = vault
    .check("user:alice", "view", "doc:1")
    .await?;  // May return cached (potentially stale) result
```

**Cache + Consistency Decision Matrix**:

| Request Type                                | Cache Behavior      | Server Hit     | Use When                       |
| ------------------------------------------- | ------------------- | -------------- | ------------------------------ |
| `check().await`                             | Cache hit if fresh  | Only on miss   | Normal authorization checks    |
| `check().at_least_as_fresh_as(token).await` | **Bypassed**        | Always         | Immediately after write        |
| `check().consistency(Strong).await`         | **Bypassed**        | Always         | When staleness is unacceptable |
| `check().consistency(Eventual).await`       | Cache hit if exists | On miss/expiry | Maximum performance            |

### Read-After-Write Recipe

For guaranteed consistency after writes:

```rust
/// Safe pattern: write, then immediately verify with consistency token
async fn grant_and_verify(
    vault: &VaultClient,
    user: &str,
    doc: &str,
) -> Result<(), Error> {
    // Step 1: Write the relationship
    let result = vault.relationships()
        .write(Relationship::new(doc, "viewer", user))
        .await?;

    // Step 2: Verify using consistency token (bypasses cache)
    let allowed = vault
        .check(user, "view", doc)
        .at_least_as_fresh_as(result.consistency_token())
        .require()
        .await?;

    // Guaranteed to see our write
    Ok(())
}

/// Alternative: Propagate token to caller for their reads
async fn grant_access(
    vault: &VaultClient,
    user: &str,
    doc: &str,
) -> Result<ConsistencyToken, Error> {
    let result = vault.relationships()
        .write(Relationship::new(doc, "viewer", user))
        .await?;

    // Caller can use this token for consistent reads
    Ok(result.consistency_token())
}
```

### Cache Invalidation via Watch

For real-time cache invalidation without consistency tokens:

```rust
// Combined caching + watch for real-time consistency
let client = Client::builder()
    .url("https://api.inferadb.com")
    .cache(CacheConfig::default())
    .cache_invalidation(CacheInvalidation::Watch)  // Use watch stream
    .build()
    .await?;
```

**How watch invalidation works**:

1. Client subscribes to relationship change events from server
2. When relationships change, server pushes invalidation events
3. Client evicts affected cache entries immediately
4. Subsequent reads fetch fresh data

**Trade-offs**:

| Approach           | Latency | Consistency                  | Complexity                   |
| ------------------ | ------- | ---------------------------- | ---------------------------- |
| TTL-based          | Low     | Eventual (bounded staleness) | Simple                       |
| Consistency tokens | Medium  | Strong (for token holder)    | Moderate                     |
| Watch invalidation | Low     | Near real-time               | Higher (connection overhead) |

### When to Use Each Approach

| Scenario                                | Recommended Approach                    |
| --------------------------------------- | --------------------------------------- |
| High-throughput reads, staleness OK     | TTL-based caching (default)             |
| Read immediately after your own write   | Consistency token                       |
| Multi-service, writer notifies readers  | Pass consistency token between services |
| Real-time permission revocation         | Watch invalidation                      |
| Maximum consistency, latency acceptable | `Consistency::Strong` (no caching)      |
| Debugging/testing                       | `CacheConfig::disabled()`               |

---

## Vault Statistics

Understanding vault usage patterns with comprehensive statistics.

### Basic Statistics

```rust
// Get vault statistics (follows organization-first hierarchy)
let vault = client.organization("org_123").vault("vlt_456");
let stats = vault.stats().await?;

println!("Total relationships: {}", stats.total_relationships);
println!("Entity types: {:?}", stats.entity_type_counts);
println!("Relation distribution: {:?}", stats.relation_counts);
println!("Last modified: {:?}", stats.last_modified);
```

### Statistics by Type

```rust
// Detailed breakdown by entity type
let stats = vault
    .stats()
    .group_by(GroupBy::EntityType)
    .await?;

for (entity_type, count) in &stats.by_entity_type {
    println!("{}: {} relationships", entity_type, count);
}

// Breakdown by relation
let stats = vault
    .stats()
    .group_by(GroupBy::Relation)
    .await?;

for (relation, count) in &stats.by_relation {
    println!("{}: {} relationships", relation, count);
}
```

### Historical Trends

```rust
// Get trends over time
let trends = vault
    .stats()
    .trends(TrendPeriod::Days(7))
    .resolution(Resolution::Hourly)
    .await?;

for point in &trends.data_points {
    println!("{}: {} total, +{} / -{}",
        point.timestamp,
        point.total_relationships,
        point.relationships_added,
        point.relationships_removed
    );
}
```

### Statistics Types

```rust
/// Vault statistics summary
#[derive(Debug, Clone)]
pub struct VaultStats {
    /// Total number of relationships in the vault
    pub total_relationships: u64,

    /// Relationships grouped by subject entity type
    pub by_subject_type: HashMap<String, u64>,

    /// Relationships grouped by resource entity type
    pub by_resource_type: HashMap<String, u64>,

    /// Relationships grouped by relation name
    pub by_relation: HashMap<String, u64>,

    /// Active schema version
    pub schema_version: String,

    /// Timestamp of last relationship modification
    pub last_modified: Option<DateTime<Utc>>,

    /// Vault creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Historical trend data
#[derive(Debug, Clone)]
pub struct VaultTrends {
    /// Time period covered
    pub period: TrendPeriod,

    /// Data resolution
    pub resolution: Resolution,

    /// Individual data points
    pub data_points: Vec<TrendDataPoint>,

    /// Summary statistics
    pub summary: TrendSummary,
}

#[derive(Debug, Clone)]
pub struct TrendDataPoint {
    pub timestamp: DateTime<Utc>,
    pub total_relationships: u64,
    pub relationships_added: u64,
    pub relationships_removed: u64,
    pub unique_subjects: u64,
    pub unique_resources: u64,
}

#[derive(Debug, Clone)]
pub struct TrendSummary {
    pub net_change: i64,
    pub peak_relationships: u64,
    pub peak_timestamp: DateTime<Utc>,
    pub average_daily_writes: f64,
}

/// Time period for trend analysis
#[derive(Debug, Clone, Copy)]
pub enum TrendPeriod {
    Hours(u32),
    Days(u32),
    Weeks(u32),
    Months(u32),
}

/// Data point resolution
#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    Minute,
    Hourly,
    Daily,
    Weekly,
}

/// Grouping options for statistics breakdown
#[derive(Debug, Clone, Copy)]
pub enum GroupBy {
    /// Group by entity type (e.g., "document", "user", "folder")
    EntityType,
    /// Group by relation name (e.g., "viewer", "owner", "member")
    Relation,
    /// Group by subject type
    SubjectType,
    /// Group by both entity type and relation
    EntityTypeAndRelation,
}
```

---

## Bulk Operations

High-performance export and import operations for vault data. These operations are vault-scoped, accessed through the organization-first hierarchy.

### Export Relationships

```rust
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...");

// Stream export for memory efficiency
let mut stream = vault
    .export()
    .stream();

while let Some(batch) = stream.try_next().await? {
    for relationship in batch.relationships {
        process(relationship);
    }
}

// Collect all (use with caution for large vaults)
let export = vault
    .export()
    .collect()
    .await?;

println!("Exported {} relationships", export.relationships.len());
```

### Filtered Export

```rust
// Export specific entity types
let export = vault
    .export()
    .resource_types(&["document", "folder"])
    .subject_types(&["user", "group"])
    .collect()
    .await?;

// Export with time filter
let export = vault
    .export()
    .changed_since(timestamp)
    .collect()
    .await?;
```

### Export with Schema

```rust
// Include schema in export
let export = vault
    .export()
    .include_schema(true)
    .collect()
    .await?;

println!("Schema version: {}", export.schema.as_ref().unwrap().version);

// Include metadata (timestamps, actors)
let export = vault
    .export()
    .include_metadata(true)
    .collect()
    .await?;

for rel in &export.relationships {
    if let Some(meta) = &rel.metadata {
        println!("{} created by {} at {}",
            rel.relationship,
            meta.created_by,
            meta.created_at
        );
    }
}
```

### Export to File

```rust
// Direct export to file (streaming, no memory buffering)
vault
    .export()
    .to_file("backup.json")
    .format(ExportFormat::JsonLines)
    .await?;

// With compression
vault
    .export()
    .to_file("backup.json.gz")
    .format(ExportFormat::JsonLines)
    .compress(Compression::Gzip)
    .await?;
```

### Import Relationships

```rust
// Import from file
let result = vault
    .import()
    .from_file("backup.json")
    .await?;

println!("Imported: {} created, {} updated, {} skipped",
    result.created,
    result.updated,
    result.skipped
);

// Import from stream
let relationships = vec![
    Relationship::new("user:alice", "viewer", "document:readme"),
    Relationship::new("user:bob", "editor", "document:readme"),
];

let result = vault
    .import()
    .relationships(relationships)
    .await?;
```

### Import Modes

```rust
/// How to handle existing relationships during import
#[derive(Debug, Clone, Copy, Default)]
pub enum ImportMode {
    /// Skip existing relationships, only add new ones
    #[default]
    Merge,

    /// Update existing relationships, add new ones
    Upsert,

    /// Replace all relationships (dangerous - deletes existing)
    Replace,
}

// Merge mode (default) - skip conflicts
let result = vault
    .import()
    .from_file("backup.json")
    .mode(ImportMode::Merge)
    .await?;

// Upsert mode - update existing
let result = vault
    .import()
    .from_file("backup.json")
    .mode(ImportMode::Upsert)
    .await?;

// Replace mode - full replacement (use with caution!)
let result = vault
    .import()
    .from_file("backup.json")
    .mode(ImportMode::Replace)
    .confirm_replace(true)  // Required safety flag
    .await?;
```

### Conflict Resolution

```rust
/// How to handle conflicts during import
#[derive(Debug, Clone, Copy, Default)]
pub enum ConflictResolution {
    /// Skip conflicting relationships
    #[default]
    Skip,

    /// Overwrite with imported data
    Overwrite,

    /// Fail the entire import on first conflict
    Fail,
}

let result = vault
    .import()
    .from_file("backup.json")
    .on_conflict(ConflictResolution::Skip)
    .await?;

// With detailed conflict reporting
let result = vault
    .import()
    .from_file("backup.json")
    .on_conflict(ConflictResolution::Skip)
    .report_conflicts(true)
    .await?;

for conflict in &result.conflicts {
    println!("Conflict: {} ({})",
        conflict.relationship,
        conflict.reason
    );
}
```

### Atomic Import

```rust
// All-or-nothing import (transactional)
let result = vault
    .import()
    .from_file("backup.json")
    .atomic(true)
    .await?;

// If any relationship fails, entire import is rolled back
```

### Async Import (Background Job)

```rust
// Start import as background job
let job = vault
    .import()
    .from_file("large-backup.json")
    .start_async()
    .await?;

println!("Import job started: {}", job.id);

// Check job status
loop {
    let status = vault.import_status(&job.id).await?;

    match status.state {
        JobState::Pending => println!("Waiting to start..."),
        JobState::Running => {
            println!("Progress: {}/{} ({:.1}%)",
                status.processed,
                status.total,
                status.progress_percent()
            );
        }
        JobState::Completed => {
            println!("Import completed: {} imported", status.processed);
            break;
        }
        JobState::Failed => {
            println!("Import failed: {}", status.error.unwrap());
            break;
        }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}

// Cancel a running job
vault.cancel_import(&job.id).await?;
```

### Import/Export Types

```rust
/// Export result containing relationships and optional metadata
#[derive(Debug, Clone)]
pub struct ExportData {
    /// Exported relationships
    pub relationships: Vec<ExportedRelationship>,

    /// Schema (if include_schema was true)
    pub schema: Option<Schema>,

    /// Export metadata
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone)]
pub struct ExportedRelationship {
    pub relationship: Relationship,
    pub metadata: Option<RelationshipMetadata>,
}

#[derive(Debug, Clone)]
pub struct RelationshipMetadata {
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_at: Option<DateTime<Utc>>,
    pub modified_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportMetadata {
    pub vault_id: String,
    pub exported_at: DateTime<Utc>,
    pub total_relationships: u64,
    pub export_duration: Duration,
}

/// Import result with detailed statistics
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Number of relationships created
    pub created: u64,

    /// Number of relationships updated
    pub updated: u64,

    /// Number of relationships skipped
    pub skipped: u64,

    /// Conflicts encountered (if report_conflicts was true)
    pub conflicts: Vec<ImportConflict>,

    /// Validation errors
    pub errors: Vec<ImportError>,

    /// Import duration
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ImportConflict {
    pub relationship: Relationship,
    pub reason: ConflictReason,
    pub existing: Option<Relationship>,
}

#[derive(Debug, Clone)]
pub enum ConflictReason {
    AlreadyExists,
    DifferentRelation,
    SchemaViolation(String),
}

/// Background job status
#[derive(Debug, Clone)]
pub struct ImportJobStatus {
    pub id: String,
    pub state: JobState,
    pub total: u64,
    pub processed: u64,
    pub created: u64,
    pub skipped: u64,
    pub errors: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl ImportJobStatus {
    pub fn progress_percent(&self) -> f64 {
        if self.total == 0 { 0.0 }
        else { (self.processed as f64 / self.total as f64) * 100.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Export format options
#[derive(Debug, Clone, Copy, Default)]
pub enum ExportFormat {
    /// JSON array of relationships
    #[default]
    Json,

    /// Newline-delimited JSON (one relationship per line)
    JsonLines,

    /// CSV format
    Csv,
}

/// Compression options for export
#[derive(Debug, Clone, Copy)]
pub enum Compression {
    None,
    Gzip,
    Zstd,
}
```

---

## Control API Overview

Management operations flow through the organization-first hierarchy, with vault management unified into the `VaultClient` type alongside authorization operations.

### Common Operations

```rust
// Organization-first: get org context
let org = client.organization("org_8675309...");

// Most common operations
let info = org.get().await?;                              // Get org info
let vault = org.vaults().create(CreateVault { name: "prod", .. }).await?;  // Create vault
let schema = org.vault(&vault.id).schemas().get_active().await?;  // Get active schema

// Account operations (current user)
let account = client.account().get().await?;
```

### API Hierarchy Convention

The SDK follows a consistent singular/plural resource pattern:

```text
client
    .organization(&id)       // Singular: scoped to specific org
        .vault(&id)          // Singular: unified vault (auth + management)
        .vaults()            // Plural: collection operations (list, create)
        .members()           // Plural: collection operations
    .account()               // Current user (no ID needed)
```

**Standard Pattern**: Plural for collections, singular when scoped to a specific instance:

```rust
// ✅ Correct: Singular for specific instances, plural for collections
let org = client.organization(&org_id);
org.get().await?;                                     // Get specific org
org.members().list().await?;                          // List members of this org
org.vaults().create(CreateVault { .. }).await?;       // Create vault in this org
org.vault(&vault_id).schemas().list().await?;         // Schemas of specific vault

// ❌ Avoid: Plural when working with specific instance
client.organizations(&org_id).get().await?;  // Should be .organization(&id)
```

### Complete Hierarchy

```rust
// Account (current user) - no ID needed
client.account().get().await?;
client.account().emails().list().await?;
client.account().sessions().list().await?;

// Organization context
let org = client.organization(&org_id);

// Organization operations
org.get().await?;
org.update(UpdateOrg { .. }).await?;
org.delete().await?;

// Organization members and teams
org.members().list().await?;
org.members().invite(invite).await?;
org.invitations().list().await?;
org.teams().list().await?;
org.teams().create(CreateTeam { .. }).await?;

// Vaults (scoped to organization)
org.vaults().list().await?;
org.vaults().create(CreateVault { .. }).await?;

// Unified VaultClient: authorization + management
let vault = org.vault(&vault_id);
vault.get().await?;                          // Get vault info
vault.update(UpdateVault { .. }).await?;     // Update vault
vault.delete().await?;                       // Delete vault

// Vault schemas
vault.schemas().list().await?;
vault.schemas().get_active().await?;
vault.schemas().push(content).await?;
vault.schemas().activate(&version).await?;

// Vault tokens and roles
vault.tokens().list().await?;
vault.tokens().create(CreateToken { .. }).await?;
vault.roles().list().await?;

// Vault authorization (same unified VaultClient type)
vault.check("user:alice", "view", "doc:1").await?;
vault.relationships().write(Relationship::new("doc:1", "viewer", "user:bob")).await?;

// Audit logs (scoped to organization)
org.audit_logs().list().await?;
org.audit_logs().actor(&user_id).list().await?;

// API Clients (scoped to organization, for service-to-service auth)
org.clients().list().await?;
org.clients().create(CreateClient { .. }).await?;
org.client(&client_id).get().await?;
org.client(&client_id).certificates().list().await?;
```

---

## Account Management

Manage the authenticated user's account, emails, and sessions.

### Get Account Details

```rust
// Get current user information
let account = client.account().get().await?;

println!("User ID: {}", account.id);
println!("Name: {}", account.name);
println!("Email: {}", account.primary_email);
println!("Created: {}", account.created_at);
```

### Update Account

```rust
// Update account details
let updated = client
    .account()
    .update(UpdateAccount {
        name: Some("New Name".into()),
        ..Default::default()
    })
    .await?;

println!("Updated: {}", updated.name);
```

### Email Management

```rust
// List all emails associated with account
let emails = client.account().emails().list().await?;

for email in &emails {
    println!("{} (primary: {}, verified: {})",
        email.address,
        email.is_primary,
        email.verified
    );
}

// Add a new email
let email = client
    .account()
    .emails()
    .add("new@example.com")
    .await?;

println!("Verification sent to: {}", email.address);

// Verify email with token
client
    .account()
    .emails()
    .verify(&email.id, verification_token)
    .await?;

// Set as primary
client
    .account()
    .emails()
    .set_primary(&email.id)
    .await?;

// Remove email
client
    .account()
    .emails()
    .remove(&email.id)
    .await?;
```

### Session Management

```rust
// List active sessions
let sessions = client.account().sessions().list().await?;

for session in &sessions {
    println!("Session: {} ({})",
        session.id,
        if session.is_current { "current" } else { &session.user_agent }
    );
    println!("  Last active: {}", session.last_active_at);
    println!("  IP: {}", session.ip_address);
}

// Revoke a specific session
client
    .account()
    .sessions()
    .revoke(&session_id)
    .await?;

// Revoke all other sessions (security measure)
let revoked = client
    .account()
    .sessions()
    .revoke_others()
    .await?;

println!("Revoked {} sessions", revoked.count);
```

### Password Management

```rust
// Request password reset
client
    .account()
    .password()
    .request_reset("user@example.com")
    .await?;

// Complete password reset with token
client
    .account()
    .password()
    .reset(reset_token, "new_password")
    .await?;

// Change password (when logged in)
client
    .account()
    .password()
    .change("current_password", "new_password")
    .await?;
```

### Delete Account

```rust
// Delete account (requires confirmation)
client
    .account()
    .delete()
    .confirm("DELETE")  // Safety confirmation
    .await?;
```

### Account Types

```rust
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub primary_email: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub email_verified: bool,
    pub mfa_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateAccount {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Email {
    pub id: String,
    pub address: String,
    pub is_primary: bool,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub is_current: bool,
    pub user_agent: String,
    pub ip_address: String,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}
```

---

## Organization Management

Comprehensive organization lifecycle and membership management.

### Organization CRUD

```rust
// List organizations (user's memberships)
let orgs = client.organizations().list().await?;

for org in &orgs {
    println!("{}: {} ({})", org.id, org.name, org.role);
}

// Create organization
let org = client
    .organizations()
    .create(CreateOrganization {
        name: "Acme Corp".into(),
        slug: Some("acme".into()),
        ..Default::default()
    })
    .await?;

// Get organization details
let org = client.organization(&org_id).get().await?;

// Update organization
let org = client
    .organization(&org_id)
    .update(UpdateOrganization {
        name: Some("Acme Corporation".into()),
        ..Default::default()
    })
    .await?;

// Delete organization (requires owner role)
client
    .organization(&org_id)
    .delete()
    .confirm("DELETE ACME")  // Safety confirmation
    .await?;
```

### Organization Lifecycle

```rust
// Suspend organization (admin only)
client
    .organization(&org_id)
    .suspend()
    .reason("Billing issue")
    .await?;

// Resume suspended organization
client
    .organization(&org_id)
    .resume()
    .await?;

// Leave organization (self-removal)
client
    .organization(&org_id)
    .leave()
    .await?;
```

### Organization Members

```rust
// List members
let members = client
    .organization(&org_id)
    .members()
    .list()
    .await?;

for member in &members {
    println!("{}: {} ({})", member.user_id, member.name, member.role);
}

// Update member role
client
    .organization(&org_id)
    .member(&user_id)
    .update_role(OrganizationRole::Admin)
    .await?;

// Remove member
client
    .organization(&org_id)
    .member(&user_id)
    .remove()
    .await?;
```

### Organization Invitations

```rust
// List pending invitations
let invitations = client
    .organization(&org_id)
    .invitations()
    .list()
    .await?;

// Create invitation
let invitation = client
    .organization(&org_id)
    .invitations()
    .create(CreateInvitation {
        email: "newuser@example.com".into(),
        role: OrganizationRole::Member,
        message: Some("Welcome to our team!".into()),
        expires_in: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
    })
    .await?;

println!("Invitation sent: {}", invitation.id);

// Resend invitation
client
    .organization(&org_id)
    .invitation(&invitation.id)
    .resend()
    .await?;

// Delete (cancel) invitation
client
    .organization(&org_id)
    .invitation(&invitation.id)
    .delete()
    .await?;

// Accept invitation (from invitee's perspective)
client
    .invitation(invitation_token)
    .accept()
    .await?;

// Decline invitation
client
    .invitation(invitation_token)
    .decline()
    .await?;
```

### Organization Roles

```rust
/// Built-in organization roles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganizationRole {
    /// Full control over organization
    Owner,

    /// Manage members, teams, and vaults
    Admin,

    /// Standard member access
    Member,

    /// View-only access
    Viewer,
}

// List role assignments
let assignments = client
    .organization(&org_id)
    .roles()
    .list()
    .await?;

// Grant role
client
    .organization(&org_id)
    .roles()
    .grant(&user_id, OrganizationRole::Admin)
    .await?;

// Update role
client
    .organization(&org_id)
    .role(&user_id)
    .update(OrganizationRole::Member)
    .await?;

// Revoke role (remove from org)
client
    .organization(&org_id)
    .role(&user_id)
    .revoke()
    .await?;
```

### Organization Types

```rust
#[derive(Debug, Clone)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub role: OrganizationRole,  // Current user's role
    pub member_count: u32,
    pub vault_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub suspended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateOrganization {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateOrganization {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrganizationMember {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: OrganizationRole,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Invitation {
    pub id: String,
    pub email: String,
    pub role: OrganizationRole,
    pub status: InvitationStatus,
    pub invited_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
}
```

---

## Team Management

Organize members into teams with granular vault access.

### Team CRUD

```rust
let org = client.organization(&org_id);

// List teams in organization
let teams = org.teams().list().await?;

for team in &teams {
    println!("{}: {} ({} members)",
        team.id,
        team.name,
        team.member_count
    );
}

// Create team
let team = org.teams()
    .create(CreateTeam {
        name: "Engineering".into(),
        description: Some("Engineering team".into()),
        ..Default::default()
    })
    .await?;

// Get team details
let team = org.team(&team_id).get().await?;

// Update team
let team = org.team(&team_id)
    .update(UpdateTeam {
        name: Some("Platform Engineering".into()),
        ..Default::default()
    })
    .await?;

// Delete team
org.team(&team_id).delete().await?;
```

### Team Members

```rust
let org = client.organization(&org_id);

// List team members
let members = org.team(&team_id).members().list().await?;

// Add member to team
org.team(&team_id)
    .members()
    .add(&user_id, TeamRole::Member)
    .await?;

// Update member's team role
org.team(&team_id)
    .member(&user_id)
    .update_role(TeamRole::Lead)
    .await?;

// Remove member from team
org.team(&team_id)
    .member(&user_id)
    .remove()
    .await?;
```

### Team Vault Grants

```rust
let org = client.organization(&org_id);

// List vault grants for team
let grants = org.team(&team_id).grants().list().await?;

for grant in &grants {
    println!("Vault {}: {} access", grant.vault_id, grant.role);
}

// Grant vault access to team
let grant = org.team(&team_id)
    .grants()
    .create(CreateVaultGrant {
        vault_id: vault_id.into(),
        role: VaultRole::ReadWrite,
    })
    .await?;

// Update grant
org.team(&team_id)
    .grant(&grant.id)
    .update(VaultRole::Admin)
    .await?;

// Revoke grant
org.team(&team_id)
    .grant(&grant.id)
    .delete()
    .await?;
```

### Team Types

```rust
#[derive(Debug, Clone)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
    pub vault_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateTeam {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTeam {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TeamMember {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamRole {
    /// Can manage team members and settings
    Lead,

    /// Standard team member
    Member,
}

#[derive(Debug, Clone)]
pub struct VaultGrant {
    pub id: String,
    pub vault_id: String,
    pub vault_name: String,
    pub role: VaultRole,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
}

#[derive(Debug, Clone, Default)]
pub struct CreateVaultGrant {
    pub vault_id: String,
    pub role: VaultRole,
}
```

---

## Vault Management

Manage authorization vaults with role-based access control. Vault operations are accessed through the organization context.

### Vault CRUD

```rust
let org = client.organization(&org_id);

// List vaults in organization
let vaults = org.vaults().list().await?;

for vault in &vaults {
    println!("{}: {} ({})", vault.id, vault.name, vault.role);
}

// Create vault
let vault_info = org.vaults()
    .create(CreateVault {
        name: "production".into(),
        description: Some("Production authorization data".into()),
        ..Default::default()
    })
    .await?;

// Get vault details (via unified VaultClient type)
let vault = org.vault(&vault_id);
let info = vault.get().await?;

// Update vault
let info = vault
    .update(UpdateVault {
        name: Some("prod-main".into()),
        ..Default::default()
    })
    .await?;

// Delete vault
vault
    .delete()
    .confirm("DELETE PRODUCTION")
    .await?;
```

### Vault Roles

```rust
/// Vault access roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VaultRole {
    /// Full control over vault
    Admin,

    /// Read and write relationships
    #[default]
    ReadWrite,

    /// Read-only access
    ReadOnly,
}

let vault = client.organization(&org_id).vault(&vault_id);

// List user role assignments
let assignments = vault.roles().list().await?;

// Grant role to user
vault.roles().grant(&user_id, VaultRole::ReadWrite).await?;

// Update role
vault.role(&assignment_id).update(VaultRole::Admin).await?;

// Revoke role
vault.role(&assignment_id).revoke().await?;
```

### Team Vault Roles

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// List team role assignments
let team_assignments = vault.team_roles().list().await?;

// Grant role to team
vault.team_roles().grant(&team_id, VaultRole::ReadWrite).await?;

// Update team role
vault.team_role(&assignment_id).update(VaultRole::ReadOnly).await?;

// Revoke team role
vault.team_role(&assignment_id).revoke().await?;
```

### Vault Tokens

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// List vault API tokens
let tokens = vault.tokens().list().await?;

for token in &tokens {
    println!("{}: {} (expires: {:?})",
        token.id,
        token.name,
        token.expires_at
    );
}

// Generate new token
let token = vault.tokens()
    .generate(GenerateToken {
        name: "api-service".into(),
        role: VaultRole::ReadWrite,
        expires_in: Some(Duration::from_secs(90 * 24 * 60 * 60)), // 90 days
        scopes: Some(vec!["read", "write"]),
    })
    .await?;

// IMPORTANT: Token secret is only returned once
println!("Token: {}", token.secret);

// Revoke specific token
vault.token(&token.id).revoke().await?;

// Revoke all tokens (emergency)
vault.tokens().revoke_all().confirm(true).await?;
```

### Vault Types

```rust
#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub organization_id: String,
    pub role: VaultRole,  // Current user's role
    pub schema_version: Option<String>,
    pub relationship_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateVault {
    pub name: String,
    pub organization_id: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateVault {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VaultRoleAssignment {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub role: VaultRole,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,
}

#[derive(Debug, Clone)]
pub struct TeamVaultRoleAssignment {
    pub id: String,
    pub team_id: String,
    pub team_name: String,
    pub role: VaultRole,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,
}

#[derive(Debug, Clone)]
pub struct VaultToken {
    pub id: String,
    pub name: String,
    pub role: VaultRole,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct GeneratedToken {
    pub id: String,
    pub name: String,
    /// The token secret - only returned once at creation
    pub secret: String,
    pub role: VaultRole,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct GenerateToken {
    pub name: String,
    pub role: VaultRole,
    pub expires_in: Option<Duration>,
    pub scopes: Option<Vec<String>>,
}
```

---

## Client Management

Manage API clients for machine-to-machine authentication. API clients are organization-scoped.

### Client CRUD

```rust
let org = client.organization("org_8675309...");

// List clients
let clients = org.clients().list().await?;

for c in &clients {
    println!("{}: {} (active: {})", c.id, c.name, c.active);
}

// Create client
let api_client = org
    .clients()
    .create(CreateClient {
        name: "backend-service".into(),
        description: Some("Main backend service".into()),
        ..Default::default()
    })
    .await?;

// Get client details
let api_client = org
    .client(&client_id)
    .get()
    .await?;

// Update client
let api_client = org
    .client(&client_id)
    .update(UpdateClient {
        name: Some("backend-api".into()),
        ..Default::default()
    })
    .await?;

// Delete client
org
    .client(&client_id)
    .delete()
    .await?;
```

### Client Lifecycle

```rust
// Deactivate client (emergency disable)
org
    .client(&client_id)
    .deactivate()
    .reason("Security incident")
    .await?;

// Reactivate client
org
    .client(&client_id)
    .activate()
    .await?;
```

### Certificate Management

```rust
// List certificates for client
let certs = org
    .client(&client_id)
    .certificates()
    .list()
    .await?;

for cert in &certs {
    println!("{}: {} (expires: {})",
        cert.id,
        cert.fingerprint,
        cert.expires_at
    );
}

// Add certificate (for key rotation)
let cert = org
    .client(&client_id)
    .certificates()
    .create(CreateCertificate {
        name: "primary-key-2024".into(),
        public_key: public_key_pem.into(),
    })
    .await?;

// Get certificate details
let cert = org
    .client(&client_id)
    .certificate(&cert_id)
    .get()
    .await?;

// Revoke certificate
org
    .client(&client_id)
    .certificate(&cert_id)
    .revoke()
    .reason("Key rotation")
    .await?;

// Delete certificate (after revocation)
org
    .client(&client_id)
    .certificate(&cert_id)
    .delete()
    .await?;
```

### Client Types

```rust
#[derive(Debug, Clone)]
pub struct ApiClient {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub certificate_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct CreateClient {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateClient {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A public key certificate for client authentication (JWKS).
/// Not to be confused with `Certificate` in TLS configuration.
#[derive(Debug, Clone)]
pub struct ClientCertificate {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateClientCertificate {
    pub name: String,
    pub public_key: String,
}
```

---

## Schema Management

Schema operations are accessed through the unified `VaultClient` type.

### Basic Usage

```rust
// Get vault context (organization-first)
let vault = client.organization(&org_id).vault(&vault_id);

// Get active schema for a vault
let schema = vault.schemas().get_active().await?;

// Push new schema (validates automatically)
let version = vault.schemas()
    .push(schema_content)
    .message("Added team support")
    .await?;

// Activate the new version
vault.schemas().activate(&version.id).await?;
```

### Schema Introspection

```rust
// Get active schema
let vault = client.organization(&org_id).vault(&vault_id);
let schema = vault.schemas().get_active().await?;

println!("Entities:");
for entity in &schema.entities {
    println!("  {}", entity.name);
    for rel in &entity.relations {
        println!("    {} -> {:?}", rel.name, rel.types);
    }
    for perm in &entity.permissions {
        println!("    {} = {}", perm.name, perm.expression);
    }
}

// Validate relationships against schema
let validation = client
    .validate_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

if !validation.valid {
    println!("Invalid: {}", validation.error);
}
```

### Schema Evolution

During rolling deployments, clients may run different schema versions:

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// Get schema with version info
let schema = vault.schemas().get_active().await?;
println!("Schema version: {}", schema.version);
println!("Compatible since: {}", schema.compatible_since);

// Check feature availability
if schema.supports_permission("document", "archive") {
    vault.check(user, "archive", doc).await?;
} else {
    // Fallback for older schema
    vault.check(user, "edit", doc).await?;
}

// SDK can also report capabilities
let caps = vault.capabilities().await?;
if caps.supports("batch_check") {
    vault.check_batch(checks).await?;
} else {
    for check in checks {
        vault.check(check).await?;
    }
}
```

### Schema Versioning

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// List all schema versions
let versions = vault.schemas().list().await?;

for version in &versions {
    println!("{}: {} (active: {})",
        version.id,
        version.message.as_deref().unwrap_or("no message"),
        version.active
    );
}

// Include inactive versions
let all_versions = vault.schemas()
    .list()
    .include_inactive(true)
    .await?;

// Get specific version
let schema = vault.schemas().get(&schema_id).await?;

// Get currently active version
let active = vault.schemas().get_active().await?;

// Compare two versions
let diff = vault.schemas()
    .diff(&from_schema_id, &to_schema_id)
    .await?;

println!("Added entities: {:?}", diff.added_entities);
println!("Removed entities: {:?}", diff.removed_entities);
println!("Modified entities: {:?}", diff.modified_entities);
println!("Breaking changes: {}", diff.has_breaking_changes);
```

### Schema Lifecycle

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// Push new schema version (without activating)
let version = vault.schemas()
    .push(schema_content)
    .message("Added team support")
    .await?;

println!("Pushed version: {}", version.id);

// Activate a version
vault.schemas().activate(&version.id).await?;

// Rollback to specific version
vault.schemas()
    .rollback_to(&previous_version_id)
    .await?;

// Rollback to previous version
vault.schemas()
    .rollback_to_previous()
    .await?;

// Copy schema to another vault
let target_vault = client.organization(&org_id).vault(&target_vault_id);
vault.schemas()
    .copy_to(&target_vault)
    .schema(&schema_id)  // or .active() for active schema
    .activate(true)       // activate in target vault
    .await?;
```

### Canary Deployments

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// Activate with canary deployment
vault.schemas()
    .activate(&version.id)
    .canary(CanaryConfig {
        percentage: 10,  // Route 10% of traffic to new schema
        duration: Some(Duration::from_secs(30 * 60)),  // 30 min observation
    })
    .await?;

// Check canary status
let status = vault.schemas().canary_status().await?;

println!("Canary percentage: {}%", status.percentage);
println!("Canary errors: {}", status.canary_metrics.error_count);
println!("Baseline errors: {}", status.baseline_metrics.error_count);

if status.has_anomalies {
    println!("Anomalies detected: {:?}", status.anomalies);
}

// Gradually increase canary percentage
vault.schemas().canary_adjust(25).await?;  // Increase to 25%

// Promote canary to 100%
vault.schemas().canary_promote().await?;

// Rollback canary (revert to baseline)
vault.schemas().canary_rollback().await?;
```

### Pre-flight Checks

```rust
let vault = client.organization(&org_id).vault(&vault_id);

// Run pre-flight checks before activating
let preflight = vault.schemas()
    .preflight(schema_content)
    .await?;

// Check validation results
if !preflight.validation.valid {
    for error in &preflight.validation.errors {
        println!("Syntax error: {} at line {}", error.message, error.line);
    }
    return Err("Schema has syntax errors".into());
}

// Check for breaking changes
if preflight.compatibility.has_breaking_changes {
    println!("Breaking changes detected:");
    for change in &preflight.compatibility.breaking_changes {
        println!("  - {}: {}", change.entity, change.description);
    }
}

// Check relationship impact
println!("Relationships affected: {}", preflight.impact.affected_relationships);
println!("Relationships invalidated: {}", preflight.impact.invalidated_relationships);

// Check test results (if schema includes tests)
if let Some(tests) = &preflight.test_results {
    println!("Tests: {} passed, {} failed", tests.passed, tests.failed);
    for failure in &tests.failures {
        println!("  FAIL: {} - {}", failure.name, failure.message);
    }
}

// Review recommendations
for rec in &preflight.recommendations {
    match rec.severity {
        Severity::Error => println!("ERROR: {}", rec.message),
        Severity::Warning => println!("WARNING: {}", rec.message),
        Severity::Info => println!("INFO: {}", rec.message),
    }
}
```

### Safe Schema Deployment Workflow

Recommended workflow for production schema changes:

```rust
use inferadb::schema::{DeploymentPlan, SafetyLevel};

let vault = client.organization(&org_id).vault(&vault_id);

// Step 1: Validate and analyze the new schema
let analysis = vault.schemas()
    .analyze(new_schema)
    .compare_to_active()  // Compare against current active schema
    .include_impact_analysis()  // Check relationship compatibility
    .await?;

// Step 2: Review safety assessment
match analysis.safety_level {
    SafetyLevel::Safe => {
        println!("✓ Schema change is safe to deploy");
    }
    SafetyLevel::RequiresReview => {
        println!("⚠ Schema change requires manual review:");
        for issue in &analysis.review_items {
            println!("  - {}", issue);
        }
    }
    SafetyLevel::Dangerous => {
        println!("✗ Schema change has dangerous implications:");
        for issue in &analysis.dangerous_items {
            println!("  - {}", issue);
        }
        return Err("Aborting: schema change too risky".into());
    }
}

// Step 3: Simulate impact on existing permissions
let simulation = vault.schemas()
    .simulate_with_schema(new_schema)
    .sample_checks(1000)  // Test 1000 random permission checks
    .await?;

println!("Permission changes:");
println!("  Newly allowed: {}", simulation.newly_allowed);
println!("  Newly denied: {}", simulation.newly_denied);
println!("  Unchanged: {}", simulation.unchanged);

// Review specific changes
for change in simulation.changes.iter().take(10) {
    println!("  {} {} {} on {}: {} → {}",
        change.subject, change.permission, change.resource,
        if change.was_allowed { "was allowed" } else { "was denied" },
        if change.now_allowed { "now allowed" } else { "now denied" }
    );
}

// Step 4: Create deployment plan
let plan = vault.schemas()
    .plan_deployment(new_schema)
    .with_canary(10, Duration::from_secs(30 * 60))  // 10% for 30 min
    .with_auto_rollback_on_error_rate(0.01)  // Rollback if >1% errors
    .await?;

println!("Deployment plan:");
println!("  Phase 1: Deploy to {}% of traffic", plan.canary_percentage);
println!("  Observation period: {:?}", plan.observation_duration);
println!("  Auto-rollback threshold: {}% error rate", plan.rollback_threshold * 100.0);

// Step 5: Execute deployment with confirmation
vault.schemas()
    .execute_plan(&plan)
    .require_confirmation("I understand this will affect production traffic")
    .await?;
```

**Deployment Safety Types**:

```rust
/// Safety assessment for schema changes
#[derive(Debug, Clone)]
pub struct SchemaAnalysis {
    /// Overall safety level
    pub safety_level: SafetyLevel,
    /// Issues requiring human review
    pub review_items: Vec<String>,
    /// Dangerous changes that should block deployment
    pub dangerous_items: Vec<String>,
    /// Breaking changes detected
    pub breaking_changes: Vec<BreakingChange>,
    /// Number of existing relationships affected
    pub affected_relationship_count: u64,
    /// Number of relationships that would become invalid
    pub invalid_relationship_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyLevel {
    /// No breaking changes, safe for automatic deployment
    Safe,
    /// Minor changes that should be reviewed
    RequiresReview,
    /// Potentially dangerous changes, requires explicit confirmation
    Dangerous,
}

/// Simulation results for schema changes
#[derive(Debug, Clone)]
pub struct SchemaSimulation {
    /// Checks that would change from denied to allowed
    pub newly_allowed: u64,
    /// Checks that would change from allowed to denied
    pub newly_denied: u64,
    /// Checks that remain unchanged
    pub unchanged: u64,
    /// Detailed list of permission changes (sampled)
    pub changes: Vec<PermissionChange>,
}

#[derive(Debug, Clone)]
pub struct PermissionChange {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub was_allowed: bool,
    pub now_allowed: bool,
}

/// Deployment plan for controlled rollout
#[derive(Debug, Clone)]
pub struct DeploymentPlan {
    pub schema_version_id: String,
    pub canary_percentage: u8,
    pub observation_duration: Duration,
    pub rollback_threshold: f64,
    pub auto_promote: bool,
}
```

### Schema Types

```rust
#[derive(Debug, Clone)]
pub struct SchemaVersion {
    pub id: String,
    pub vault_id: String,
    pub active: bool,
    pub message: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SchemaDiff {
    pub added_entities: Vec<String>,
    pub removed_entities: Vec<String>,
    pub modified_entities: Vec<EntityDiff>,
    pub has_breaking_changes: bool,
    pub breaking_changes: Vec<BreakingChange>,
}

#[derive(Debug, Clone)]
pub struct EntityDiff {
    pub name: String,
    pub added_relations: Vec<String>,
    pub removed_relations: Vec<String>,
    pub modified_relations: Vec<RelationDiff>,
    pub added_permissions: Vec<String>,
    pub removed_permissions: Vec<String>,
    pub modified_permissions: Vec<PermissionDiff>,
}

#[derive(Debug, Clone)]
pub struct BreakingChange {
    pub entity: String,
    pub change_type: BreakingChangeType,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub enum BreakingChangeType {
    EntityRemoved,
    RelationRemoved,
    PermissionRemoved,
    TypeNarrowed,
}

#[derive(Debug, Clone)]
pub struct CanaryStatus {
    pub active: bool,
    pub percentage: u8,
    pub started_at: DateTime<Utc>,
    pub baseline_version: String,
    pub canary_version: String,
    pub canary_metrics: SchemaMetrics,
    pub baseline_metrics: SchemaMetrics,
    pub has_anomalies: bool,
    pub anomalies: Vec<Anomaly>,
}

#[derive(Debug, Clone)]
pub struct SchemaMetrics {
    pub request_count: u64,
    pub error_count: u64,
    pub error_rate: f64,
    pub p50_latency: Duration,
    pub p99_latency: Duration,
}

#[derive(Debug, Clone)]
pub struct PreflightResult {
    pub validation: ValidationResult,
    pub compatibility: CompatibilityResult,
    pub impact: ImpactSummary,
    pub test_results: Option<TestResults>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone)]
pub struct CompatibilityResult {
    pub has_breaking_changes: bool,
    pub breaking_changes: Vec<BreakingChange>,
    pub compatible_with_current: bool,
}

#[derive(Debug, Clone)]
pub struct ImpactSummary {
    pub affected_relationships: u64,
    pub invalidated_relationships: u64,
    pub affected_entities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestResults {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
```

---

## Audit Logs

Query and analyze audit logs for compliance and debugging.

### Query Audit Logs

```rust
let org = client.organization(&org_id);

// List recent audit events
let events = org.audit_logs().list().await?;

for event in &events {
    println!("[{}] {} performed {} on {}",
        event.timestamp,
        event.actor,
        event.action,
        event.resource
    );
}

// Filter by actor
let user_events = org.audit_logs().actor(&user_id).list().await?;

// Filter by action
let create_events = org.audit_logs().action("vault.create").list().await?;

// Filter by resource
let vault_events = org.audit_logs()
    .resource_type("vault")
    .resource_id(&vault_id)
    .list()
    .await?;

// Time range filter
let recent_events = org.audit_logs()
    .from(start_time)
    .to(end_time)
    .list()
    .await?;

// Combine filters
let filtered = org.audit_logs()
    .actor(&user_id)
    .action("relationship.write")
    .from(start_time)
    .list()
    .await?;
```

### Stream Audit Logs

```rust
let org = client.organization(&org_id);

// Stream for large result sets
let mut stream = org.audit_logs().from(start_time).stream();

while let Some(event) = stream.try_next().await? {
    process_audit_event(event);
}

// Export to file
org.audit_logs()
    .from(start_time)
    .to(end_time)
    .export_to("audit-export.json")
    .await?;
```

### Audit Log Types

```rust
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: AuditActor,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub organization_id: String,
    pub vault_id: Option<String>,
    pub metadata: AuditMetadata,
    pub outcome: AuditOutcome,
}

#[derive(Debug, Clone)]
pub struct AuditActor {
    pub id: String,
    pub actor_type: ActorType,
    pub name: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorType {
    User,
    ApiClient,
    System,
}

#[derive(Debug, Clone)]
pub struct AuditMetadata {
    pub request_id: String,
    pub changes: Option<serde_json::Value>,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
}

/// Common audit actions
pub mod actions {
    // Organization
    pub const ORG_CREATE: &str = "organization.create";
    pub const ORG_UPDATE: &str = "organization.update";
    pub const ORG_DELETE: &str = "organization.delete";
    pub const ORG_SUSPEND: &str = "organization.suspend";

    // Vault
    pub const VAULT_CREATE: &str = "vault.create";
    pub const VAULT_UPDATE: &str = "vault.update";
    pub const VAULT_DELETE: &str = "vault.delete";

    // Schema
    pub const SCHEMA_PUSH: &str = "schema.push";
    pub const SCHEMA_ACTIVATE: &str = "schema.activate";
    pub const SCHEMA_ROLLBACK: &str = "schema.rollback";

    // Relationships
    pub const RELATIONSHIP_WRITE: &str = "relationship.write";
    pub const RELATIONSHIP_DELETE: &str = "relationship.delete";

    // Members
    pub const MEMBER_INVITE: &str = "member.invite";
    pub const MEMBER_REMOVE: &str = "member.remove";
    pub const MEMBER_ROLE_UPDATE: &str = "member.role_update";
}
```

---

## JWKS Operations

Retrieve JSON Web Key Sets for token verification.

### Service JWKS

```rust
// Get service-level JWKS
let jwks = client.jwks().service().await?;

println!("Keys:");
for key in &jwks.keys {
    println!("  {}: {} ({})",
        key.kid,
        key.kty,
        key.alg.as_deref().unwrap_or("unspecified")
    );
}

// Get JWKS URL for service
let url = client.jwks().service_url();
println!("JWKS URL: {}", url);
```

### Organization JWKS

```rust
// Get organization-specific JWKS
let jwks = client.jwks().organization(&org_id).await?;

// Get JWKS URL for organization
let url = client.jwks().organization_url(&org_id);
```

### Token Verification

```rust
// Fetch JWKS and verify token locally
let jwks = client.jwks().service().await?;

// Use with a JWT library like jsonwebtoken
use jsonwebtoken::{decode, DecodingKey, Validation};

let key = jwks.find_key(&kid)?;
let decoding_key = DecodingKey::from_jwk(key)?;
let token_data = decode::<Claims>(token, &decoding_key, &Validation::default())?;
```

### JWKS Types

```rust
#[derive(Debug, Clone)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

impl Jwks {
    /// Find key by key ID
    pub fn find_key(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|k| k.kid == kid)
    }
}

#[derive(Debug, Clone)]
pub struct Jwk {
    pub kty: String,
    pub kid: String,
    pub alg: Option<String>,
    pub use_: Option<String>,

    // RSA keys
    pub n: Option<String>,
    pub e: Option<String>,

    // EC keys
    pub crv: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,

    // OKP keys (EdDSA)
    pub crv_okp: Option<String>,
    pub x_okp: Option<String>,
}
```

---

## Authentication Flows

Interactive and programmatic authentication flows for CLI and application use.

### OAuth PKCE Flow

Browser-based authentication for CLI and desktop applications.

```rust
use inferadb::auth::{OAuthPkce, OAuthConfig};

// Configure OAuth
let config = OAuthConfig {
    client_id: "cli-client".into(),
    redirect_uri: "http://localhost:8080/callback".into(),
    scopes: vec!["openid", "profile", "offline_access"],
};

// Start OAuth PKCE flow
let oauth = OAuthPkce::new(config);
let (auth_url, state) = oauth.start_flow()?;

println!("Open this URL in your browser:");
println!("{}", auth_url);

// Wait for callback (OAuth state includes PKCE verifier)
// Your app should handle the redirect and extract the code
let code = wait_for_callback(&state).await?;

// Complete the flow
let tokens = oauth.complete_flow(code, state).await?;

println!("Access token: {}", tokens.access_token);
println!("Expires in: {:?}", tokens.expires_in);

// Build client with tokens
let client = Client::builder()
    .url("https://api.inferadb.com")
    .access_token(&tokens.access_token)
    .build()
    .await?;
```

**Local Server Helper**:

```rust
// SDK can spin up a local server to receive the callback
let oauth = OAuthPkce::new(config);
let tokens = oauth
    .with_local_server(8080)  // Listen on localhost:8080
    .open_browser(true)        // Automatically open browser
    .timeout(Duration::from_secs(300))  // 5 minute timeout
    .execute()
    .await?;

// Tokens received automatically
println!("Logged in successfully!");
```

### Token Management

```rust
// Inspect token without verification
let info = client.tokens().inspect(&token_string)?;

println!("Subject: {}", info.subject);
println!("Issuer: {}", info.issuer);
println!("Expires: {:?}", info.expires_at);
println!("Scopes: {:?}", info.scopes);
println!("Claims: {:?}", info.claims);

// Verify token signature (fetches JWKS if needed)
let verified = client.tokens().verify(&token_string).await?;

if verified.valid {
    println!("Token is valid");
    println!("Subject: {}", verified.claims.subject);
} else {
    println!("Token invalid: {}", verified.error.unwrap());
}

// Manual token refresh
let new_tokens = client.tokens().refresh(&refresh_token).await?;

println!("New access token: {}", new_tokens.access_token);
```

**Auto-Refresh Configuration**:

```rust
// Client with automatic token refresh
let client = Client::builder()
    .url("https://api.inferadb.com")
    .access_token(&tokens.access_token)
    .refresh_token(&tokens.refresh_token)
    .refresh(RefreshConfig::default()
        .enabled(true)  // Automatically refresh before expiry
        .threshold(Duration::from_secs(60))  // Refresh 60s before expiry
        .on_refresh(|new_tokens| {
            // Save new tokens to secure storage
            save_tokens(new_tokens);
        }))
    .build()
    .await?;
```

### Registration

```rust
// User registration
let result = client
    .auth()
    .register(RegisterRequest {
        email: "user@example.com".into(),
        name: "User Name".into(),
        password: "secure_password".into(),
    })
    .await?;

match result {
    RegisterResult::Success { user_id, verification_required } => {
        println!("Registered user: {}", user_id);
        if verification_required {
            println!("Check email for verification link");
        }
    }
    RegisterResult::EmailExists => {
        println!("Email already registered");
    }
}

// Verify email
client
    .auth()
    .verify_email(verification_token)
    .await?;
```

### Logout

```rust
// Logout (revoke current token)
client.auth().logout().await?;

// Logout from all devices
client.auth().logout_all().await?;
```

### Authentication Types

```rust
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<&'static str>,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<Duration>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub subject: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub issued_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub claims: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct TokenVerification {
    pub valid: bool,
    pub claims: Option<TokenInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub enum RegisterResult {
    Success {
        user_id: String,
        verification_required: bool,
    },
    EmailExists,
    WeakPassword(Vec<String>),
    InvalidEmail,
}
```

---

## Error Handling

### Error Type Design

```rust
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,

    // Protocol-native details
    request_id: Option<String>,
    retry_after: Option<Duration>,
    grpc_status: Option<GrpcStatus>,
    http_status: Option<u16>,

    // For schema/authorization errors
    details: Option<ErrorDetails>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    // Client errors (4xx)
    InvalidInput,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    PreconditionFailed,

    // Server errors (5xx)
    ServerError,
    ServiceUnavailable,

    // Network errors
    ConnectionFailed,
    Timeout,
    TlsError,

    // Protocol errors
    ProtocolError,
    InvalidResponse,

    // Authorization-specific errors
    SchemaViolation,
    CyclicRelationship,
    MaxDepthExceeded,

    // SDK errors
    ConfigurationError,
    SerializationError,
    BuildError,
    ShuttingDown,

    // Resilience errors
    CircuitOpen,
}
```

### Protocol Status → ErrorKind Mapping

The SDK normalizes HTTP and gRPC status codes into `ErrorKind` variants:

| HTTP | gRPC                         | ErrorKind            | Description                                                   |
| ---- | ---------------------------- | -------------------- | ------------------------------------------------------------- |
| 400  | `INVALID_ARGUMENT`           | `InvalidInput`       | Malformed request, invalid parameters                         |
| 401  | `UNAUTHENTICATED`            | `Unauthorized`       | Missing/expired/invalid credentials                           |
| 403  | `PERMISSION_DENIED`          | `Forbidden`          | Authenticated but not authorized for **management** operation |
| 404  | `NOT_FOUND`                  | `NotFound`           | Resource doesn't exist                                        |
| 409  | `ALREADY_EXISTS` / `ABORTED` | `Conflict`           | Concurrent modification, optimistic lock failure              |
| 412  | `FAILED_PRECONDITION`        | `PreconditionFailed` | Precondition (e.g., consistency token) not met                |
| 422  | `INVALID_ARGUMENT`           | `SchemaViolation`    | Relationship violates schema constraints                      |
| 429  | `RESOURCE_EXHAUSTED`         | `RateLimited`        | Rate limit exceeded, check `retry_after()`                    |
| 500  | `INTERNAL`                   | `ServerError`        | Server-side error                                             |
| 503  | `UNAVAILABLE`                | `ServiceUnavailable` | Server temporarily unavailable                                |
| —    | `DEADLINE_EXCEEDED`          | `Timeout`            | Request timed out                                             |
| —    | —                            | `ConnectionFailed`   | TCP/TLS connection failure                                    |
| —    | —                            | `TlsError`           | TLS handshake/certificate error                               |

### check() vs require() Error Semantics

**Critical distinction**: Authorization _decisions_ (allowed/denied) are **not errors**. Only _failures_ (network, auth, validation) are errors.

| Scenario                       | `check().await?` returns               | `check().require().await?` returns     |
| ------------------------------ | -------------------------------------- | -------------------------------------- |
| User is allowed                | `Ok(true)`                             | `Ok(())`                               |
| User is denied (no permission) | `Ok(false)`                            | `Err(AccessDenied)`                    |
| Invalid credentials (401)      | `Err(Error { kind: Unauthorized })`    | `Err(Error { kind: Unauthorized })`    |
| Relationship violates schema   | `Err(Error { kind: SchemaViolation })` | `Err(Error { kind: SchemaViolation })` |
| Network timeout                | `Err(Error { kind: Timeout })`         | `Err(Error { kind: Timeout })`         |
| Rate limited (429)             | `Err(Error { kind: RateLimited })`     | `Err(Error { kind: RateLimited })`     |

**Note**: `AccessDenied` (the struct returned by `require()`) is **distinct** from `ErrorKind::Forbidden`:

- `AccessDenied` struct → Authorization decision was "deny" (normal business logic)
- `ErrorKind::Forbidden` → Authenticated but lacking permission for a **management API** operation (e.g., user can't modify vault settings)

```rust
// Authorization check - denial is Ok(false), not an error
let allowed = vault.check("user:alice", "view", "doc:1").await?;
// allowed: bool - false means "not authorized" (expected business outcome)

// With require() - denial becomes Err(AccessDenied)
vault.check("user:alice", "view", "doc:1")
    .require()
    .await?;  // Err(AccessDenied { subject, permission, resource, ... })

// Management API - 403 becomes ErrorKind::Forbidden
vault.schemas().update(new_schema).await?;
// Err(Error { kind: Forbidden }) if user lacks schema management permission
```

### Retry Recommendations by ErrorKind

| ErrorKind            | Auto-Retry     | User Action            | Notes                                         |
| -------------------- | -------------- | ---------------------- | --------------------------------------------- |
| `Timeout`            | ✅ Yes         | Wait and retry         | Respects `RetryConfig`                        |
| `ConnectionFailed`   | ✅ Yes         | Check network          | Exponential backoff                           |
| `ServiceUnavailable` | ✅ Yes         | Wait and retry         | Server is temporarily down                    |
| `RateLimited`        | ✅ Yes         | Honor `retry_after()`  | SDK respects Retry-After header               |
| `Unauthorized`       | ❌ No          | Re-authenticate        | Token expired or revoked                      |
| `Forbidden`          | ❌ No          | Check permissions      | User lacks management permission              |
| `InvalidInput`       | ❌ No          | Fix request            | Client bug - won't succeed on retry           |
| `NotFound`           | ❌ No          | Verify resource exists | Resource may have been deleted                |
| `Conflict`           | ⚠️ Conditional | Re-fetch and retry     | Optimistic lock failure - re-read state first |
| `SchemaViolation`    | ❌ No          | Fix relationship       | Violates schema constraints                   |
| `ServerError`        | ⚠️ Conditional | Report to support      | May be transient; limited retries             |

**SDK auto-retry behavior**:

```rust
// These are retried automatically (up to RetryConfig.max_retries):
// - Timeout, ConnectionFailed, ServiceUnavailable, RateLimited

// These are NOT retried (fail immediately):
// - InvalidInput, Unauthorized, Forbidden, NotFound, SchemaViolation

// Conflict is special - SDK doesn't auto-retry, but you should:
match vault.relationships().write(rel).await {
    Err(e) if e.kind() == ErrorKind::Conflict => {
        // Re-fetch current state, resolve conflict, retry
        let current = vault.relationships().get(rel.id()).await?;
        let resolved = resolve_conflict(rel, current);
        vault.relationships().write(resolved).await?;
    }
    other => other?,
}
```

### Protocol-Native Error Details

Expose underlying protocol information for debugging and observability:

```rust
impl Error {
    // Core accessors
    pub fn kind(&self) -> ErrorKind { self.kind }
    pub fn message(&self) -> &str { &self.message }
    pub fn request_id(&self) -> Option<&str> { self.request_id.as_deref() }

    // Retry guidance
    pub fn retry_after(&self) -> Option<Duration> { self.retry_after }
    pub fn is_retriable(&self) -> bool {
        matches!(self.kind, ErrorKind::Timeout | ErrorKind::ConnectionFailed |
                 ErrorKind::ServiceUnavailable | ErrorKind::RateLimited)
    }

    // Protocol-specific details
    pub fn http_status(&self) -> Option<u16> { self.http_status }
    pub fn grpc_status(&self) -> Option<&GrpcStatus> { self.grpc_status.as_ref() }

    // Authorization-specific details
    pub fn schema_error(&self) -> Option<&SchemaError> {
        self.details.as_ref().and_then(|d| d.schema_error.as_ref())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Constructors for common error cases
    // ─────────────────────────────────────────────────────────────────────────

    /// Create an invalid input error with a message.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidInput,
            message: message.into(),
            source: None,
            request_id: None,
            retry_after: None,
            grpc_status: None,
            http_status: Some(400),
            details: None,
        }
    }

    /// Create a not found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            message: message.into(),
            source: None,
            request_id: None,
            retry_after: None,
            grpc_status: None,
            http_status: Some(404),
            details: None,
        }
    }

    /// Create an internal/server error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ServerError,
            message: message.into(),
            source: None,
            request_id: None,
            retry_after: None,
            grpc_status: None,
            http_status: Some(500),
            details: None,
        }
    }
}

// Display implementation for user-friendly error messages
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)?;
        if let Some(req_id) = &self.request_id {
            write!(f, " (request_id: {})", req_id)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => write!(f, "invalid input"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Forbidden => write!(f, "forbidden"),
            Self::NotFound => write!(f, "not found"),
            Self::Conflict => write!(f, "conflict"),
            Self::RateLimited => write!(f, "rate limited"),
            Self::Timeout => write!(f, "timeout"),
            Self::ConnectionFailed => write!(f, "connection failed"),
            Self::ServiceUnavailable => write!(f, "service unavailable"),
            Self::ShuttingDown => write!(f, "client shutting down"),
            // ... other variants
            _ => write!(f, "{:?}", self),
        }
    }
}
```

**Display Output Examples**:

```rust
// Error Display output examples:
// "rate limited: Too many requests (request_id: req_abc123)"
// "timeout: Request timed out after 30s (request_id: req_def456)"
// "unauthorized: Invalid API key"

// Use in error chains with thiserror:
#[derive(thiserror::Error, Debug)]
enum MyAppError {
    #[error("authorization check failed")]
    AuthCheck(#[from] inferadb::Error),
}

// Error output: "authorization check failed: rate limited: Too many requests"
```

**Protocol-Specific Error Details**:

```rust
/// gRPC status information (when using gRPC transport)
#[derive(Debug, Clone)]
pub struct GrpcStatus {
    /// gRPC status code (tonic::Code as i32)
    pub code: i32,
    /// Error message from server
    pub message: String,
    /// Serialized google.rpc.Status details
    pub details: Vec<u8>,
}

/// Extended error details for authorization failures
#[derive(Debug, Clone)]
pub enum ErrorDetails {
    /// Schema validation failure
    Schema(SchemaError),
    /// Authorization check details (why denied)
    Authorization(AuthorizationError),
    /// Precondition failure details
    Precondition(PreconditionError),
}

#[derive(Debug, Clone)]
pub struct SchemaError {
    pub entity_type: Option<String>,
    pub relation: Option<String>,
    pub violation: SchemaViolation,
}

#[derive(Debug, Clone)]
pub enum SchemaViolation {
    UnknownEntityType,
    UnknownRelation,
    InvalidSubjectType { expected: Vec<String>, got: String },
    CyclicRelationship { path: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct AuthorizationError {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub reason: DenialReason,
}

#[derive(Debug, Clone)]
pub enum DenialReason {
    /// No relationship path exists between subject and resource
    NoPath,
    /// ABAC condition evaluated to false
    ConditionFailed(String),
    /// Explicit deny rule matched
    Explicit,
}

/// Error details when a precondition fails
#[derive(Debug, Clone)]
pub struct PreconditionError {
    /// The precondition that failed
    pub precondition: PreconditionSummary,
    /// Human-readable description of why it failed
    pub message: String,
    /// Current state that caused the failure (if available)
    pub current_state: Option<String>,
}

/// Summary of a failed precondition for error reporting
#[derive(Debug, Clone)]
pub enum PreconditionSummary {
    /// Expected relationship to not exist, but it did
    UnexpectedExists {
        resource: String,
        relation: String,
        subject: String,
    },
    /// Expected relationship to exist, but it didn't
    ExpectedExists {
        resource: String,
        relation: String,
        subject: String,
    },
    /// Consistency token didn't match
    TokenMismatch {
        expected: String,
        actual: Option<String>,
    },
}
```

**Using Protocol Details**:

```rust
match vault.relationships().write(relationship).await {
    Ok(_) => {},
    Err(e) => {
        // Log with full context
        tracing::error!(
            error_kind = ?e.kind(),
            request_id = ?e.request_id(),
            http_status = ?e.http_status(),
            grpc_code = ?e.grpc_status().map(|s| s.code),
            "Write failed: {}",
            e.message()
        );

        // Handle schema-specific errors
        if let Some(schema_err) = e.schema_error() {
            match &schema_err.violation {
                SchemaViolation::UnknownRelation => {
                    eprintln!("Unknown relation '{}' on type '{}'",
                        schema_err.relation.as_deref().unwrap_or("?"),
                        schema_err.entity_type.as_deref().unwrap_or("?"));
                }
                SchemaViolation::InvalidSubjectType { expected, got } => {
                    eprintln!("Invalid subject type: expected {:?}, got {}", expected, got);
                }
                _ => {}
            }
        }
    }
}
```

### Error Handling Patterns

```rust
use inferadb::{Error, ErrorKind};

match vault.check("user:alice", "view", "doc:1").await {
    Ok(allowed) => println!("Allowed: {}", allowed),
    Err(e) => {
        match e.kind() {
            ErrorKind::Unauthorized => {
                // Re-authenticate
            }
            ErrorKind::RateLimited => {
                // Back off
                if let Some(retry_after) = e.retry_after() {
                    tokio::time::sleep(retry_after).await;
                }
            }
            ErrorKind::Timeout | ErrorKind::ServiceUnavailable => {
                // Retry with backoff
            }
            _ => {
                // Log and fail
                eprintln!("Error: {} (request_id: {:?})", e, e.request_id());
            }
        }
    }
}
```

---

## Retry & Resilience

### Automatic Retry

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .retry(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10))
        .backoff_multiplier(2.0)
        .jitter(0.1))
    .build()
    .await?;
```

### Retriable Errors

| Error Kind           | Retriable | Notes                   |
| -------------------- | --------- | ----------------------- |
| `Timeout`            | Yes       | Network timeout         |
| `ConnectionFailed`   | Yes       | Connection dropped      |
| `ServiceUnavailable` | Yes       | Server overloaded       |
| `RateLimited`        | Yes       | With `retry_after`      |
| `ServerError`        | Maybe     | 5xx without body        |
| `Unauthorized`       | No        | Credentials invalid     |
| `Forbidden`          | No        | Permission denied       |
| `NotFound`           | No        | Resource doesn't exist  |
| `InvalidInput`       | No        | Bad request             |
| `ShuttingDown`       | No        | Client is shutting down |

### Retry Budget

```rust
// Prevent retry storms under load
let client = Client::builder()
    .retry(RetryConfig::default()
        .retry_budget(RetryBudget::new()
            .ttl(Duration::from_secs(10))
            .min_retries_per_second(10)
            .retry_ratio(0.1)))  // Max 10% retries
    .build()
    .await?;
```

### Client-Side Rate Limiting

Proactively limit outgoing requests to avoid hitting server rate limits. This is useful when you know your application's traffic patterns and want to smooth out request spikes.

```rust
use inferadb::RateLimiter;

// Configure client-side rate limiting
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .rate_limiter(RateLimiter::new()
        .requests_per_second(100)       // Max 100 req/s
        .burst_size(20)                 // Allow bursts of 20
        .per_operation(OperationType::Check, 500)  // Higher limit for checks
        .per_operation(OperationType::Write, 50))  // Lower for writes
    .build()
    .await?;
```

**Rate Limiter Behavior**:

| Scenario                  | Behavior                                            |
| ------------------------- | --------------------------------------------------- |
| Under limit               | Request proceeds immediately                        |
| At limit, burst available | Request proceeds, decrements burst                  |
| At limit, no burst        | Future yields until token available                 |
| Backpressure timeout      | Returns `Error { kind: RateLimited }` (client-side) |

```rust
// Configure backpressure behavior
let client = Client::builder()
    .rate_limiter(RateLimiter::new()
        .requests_per_second(100)
        .backpressure_timeout(Duration::from_secs(5))  // Wait max 5s for token
        .on_backpressure(BackpressureStrategy::Yield)) // Default: yield
    .build()
    .await?;

// Alternative: fail fast instead of waiting
let client = Client::builder()
    .rate_limiter(RateLimiter::new()
        .requests_per_second(100)
        .on_backpressure(BackpressureStrategy::Error)) // Return error immediately
    .build()
    .await?;
```

**Client vs Server Rate Limiting**:

| Aspect      | Client-Side                      | Server-Side                      |
| ----------- | -------------------------------- | -------------------------------- |
| Enforcement | Proactive, before request sent   | Reactive, after request received |
| Error type  | Immediate, no network round-trip | 429 response from server         |
| Retry-After | Computed locally                 | Provided by server               |
| Use case    | Smoothing traffic, known limits  | Hard enforcement, multi-tenant   |

**Rate Limiter Types**:

```rust
/// Client-side rate limiter using token bucket algorithm
#[derive(Debug, Clone)]
pub struct RateLimiter {
    requests_per_second: u32,
    burst_size: u32,
    per_operation_limits: HashMap<OperationType, u32>,
    backpressure_timeout: Duration,
    backpressure_strategy: BackpressureStrategy,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 10,
            per_operation_limits: HashMap::new(),
            backpressure_timeout: Duration::from_secs(30),
            backpressure_strategy: BackpressureStrategy::Yield,
        }
    }

    pub fn requests_per_second(mut self, rps: u32) -> Self {
        self.requests_per_second = rps;
        self
    }

    pub fn burst_size(mut self, size: u32) -> Self {
        self.burst_size = size;
        self
    }

    pub fn per_operation(mut self, op: OperationType, limit: u32) -> Self {
        self.per_operation_limits.insert(op, limit);
        self
    }

    pub fn backpressure_timeout(mut self, timeout: Duration) -> Self {
        self.backpressure_timeout = timeout;
        self
    }

    pub fn on_backpressure(mut self, strategy: BackpressureStrategy) -> Self {
        self.backpressure_strategy = strategy;
        self
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Operation types for per-operation rate limits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    /// Authorization checks (check, check_batch, expand)
    Check,
    /// Read operations (list resources, subjects, relationships)
    Read,
    /// Write operations (write, delete relationships)
    Write,
    /// Watch streams
    Watch,
    /// Management operations (schemas, tokens, etc.)
    Management,
}

/// Strategy when rate limit backpressure is reached
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureStrategy {
    /// Yield until a token is available (default)
    #[default]
    Yield,
    /// Return an error immediately
    Error,
    /// Drop the request silently (for fire-and-forget scenarios)
    Drop,
}
```

### Idempotency-Aware Retry

Retry behavior is unified in `RetryConfig`, which includes both mechanics (backoff, jitter) and policy (per-operation-category settings):

```rust
// Default RetryConfig already includes safe idempotency defaults:
// - reads: retry on all transient errors
// - idempotent_writes: retry on all transient errors
// - non_idempotent_writes: retry only on connection errors (before send)

let client = Client::builder()
    .url("https://api.inferadb.com")
    .retry(RetryConfig::default())  // Uses safe defaults
    .build()
    .await?;

// Customize per-category behavior:
let client = Client::builder()
    .url("https://api.inferadb.com")
    .retry(RetryConfig::default()
        .reads(OperationRetry::enabled().max_retries(5))
        .idempotent_writes(OperationRetry::enabled().max_retries(3))
        .non_idempotent_writes(OperationRetry::disabled()))
    .build()
    .await?;
```

**How the SDK Determines Idempotency**:

```rust
// Automatically idempotent: reads
vault.check("user:alice", "view", "doc:1").await?;  // Safe to retry

// Idempotent by request_id: writes with explicit ID
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(Uuid::new_v4())  // Server deduplicates by request_id
    .await?;  // Safe to retry with same request_id

// Non-idempotent: writes without request_id
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    // No request_id - could create duplicates if retried incorrectly
    .await?;  // Only retry on connection errors before send
```

**Configuring Idempotency Behavior**:

```rust
let client = Client::builder()
    .retry(RetryConfig {
        // Mechanics
        max_retries: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(10),
        backoff_multiplier: 2.0,
        jitter: 0.1,
        budget: None,  // Optional retry budget to prevent storms
        // Policy by operation category
        reads: OperationRetry::enabled().max_retries(5),
        idempotent_writes: OperationRetry::enabled().max_retries(3),
        non_idempotent_writes: OperationRetry::disabled(),
    })
    .auto_request_id(true)  // All writes get request IDs automatically
    .build()
    .await?;
```

**Retry Decision Matrix**:

| Operation                  | Has Request ID        | Error Type            | Retry?      |
| -------------------------- | --------------------- | --------------------- | ----------- |
| `check()`                  | N/A                   | Any transient         | Yes         |
| `relationships().write()`  | Yes                   | Any transient         | Yes         |
| `relationships().write()`  | No                    | Connection (pre-send) | Yes         |
| `relationships().write()`  | No                    | Connection (mid-send) | No (unsafe) |
| `relationships().write()`  | No                    | Server error          | No (unsafe) |
| `relationships().delete()` | Inherently idempotent | Any transient         | Yes         |

### Per-Operation Timeouts

Override client-level timeouts for individual operations:

```rust
// Critical path with shorter timeout
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .timeout(Duration::from_millis(100))  // Override client default
    .await?;

// Batch operation with longer timeout
let results = vault
    .check_batch(large_batch)
    .timeout(Duration::from_secs(60))
    .collect()
    .await?;

// Write with custom timeout
vault.relationships()
    .write(relationship)
    .timeout(Duration::from_secs(5))
    .await?;
```

**Timeout Hierarchy**:

| Level      | Scope            | Default     | Override             |
| ---------- | ---------------- | ----------- | -------------------- |
| Client     | All operations   | 30 seconds  | `.request_timeout()` |
| Operation  | Single request   | From client | `.timeout()`         |
| Connection | TCP connect only | 10 seconds  | `.connect_timeout()` |

**Timeout Semantics (Important)**:

| Timeout Type             | Applies To                   | Includes Retries? | Notes                          |
| ------------------------ | ---------------------------- | ----------------- | ------------------------------ |
| `.timeout()`             | Total operation time         | **Yes**           | Wall-clock from call to result |
| `.per_attempt_timeout()` | Each individual attempt      | No                | Per network round-trip         |
| `.connect_timeout()`     | TCP connection establishment | No                | Per connection attempt         |

```rust
// Total timeout: Operation must complete (including all retries) within 5s
vault.check("user:alice", "view", "doc:1")
    .timeout(Duration::from_secs(5))
    .await?;

// Per-attempt timeout: Each attempt gets 1s, but total can be longer with retries
vault.check("user:alice", "view", "doc:1")
    .per_attempt_timeout(Duration::from_secs(1))  // Individual attempt limit
    .timeout(Duration::from_secs(10))             // Overall deadline
    .await?;

// Example timeline with retries:
// Attempt 1: 0s → 1s (timeout, retry)
// Attempt 2: 1.1s → 2.1s (timeout, retry)
// Attempt 3: 2.2s → 2.5s (success!)
// Total: 2.5s - within 10s deadline
```

**Timeout Error Behavior**:

```rust
match vault.check("user:alice", "view", "doc:1").timeout(Duration::from_millis(100)).await {
    Err(e) if e.kind() == ErrorKind::Timeout => {
        // e.is_deadline() - true if total deadline exceeded
        // e.is_per_attempt() - true if single attempt timed out (may still retry)
        if e.is_deadline() {
            tracing::error!("Operation deadline exceeded, no more retries");
        }
    }
    result => result?,
}
```

### Cancellation & Abort

Operations can be cancelled by dropping the future or using explicit abort handles:

**Cancellation by Dropping**:

```rust
// Dropping the future cancels the operation
let check_future = vault.check("user:alice", "view", "doc:1");

tokio::select! {
    result = check_future => {
        // Operation completed
        handle_result(result?);
    }
    _ = shutdown_signal() => {
        // Future dropped here - operation cancelled
        return Ok(());
    }
}
```

**Explicit Abort Handles**:

```rust
use inferadb::AbortHandle;

// Spawn with abort handle for long-running operations
let (handle, abort) = vault
    .relationships().list()
    .stream()
    .with_abort_handle();

// Start processing in background
tokio::spawn(async move {
    while let Some(rel) = handle.next().await {
        process(rel?);
    }
});

// Later: abort if needed
if should_cancel {
    abort.abort();  // Stream terminates with Err(Cancelled)
}
```

**Batch Cancellation**:

```rust
// Batch operations can be partially cancelled
let mut stream = vault.check_batch(checks).stream();

let mut results = Vec::new();
while let Some(result) = stream.next().await {
    match result {
        Ok((check, allowed)) => results.push((check, allowed)),
        Err(e) if e.is_cancelled() => {
            // Batch was cancelled - results contains partial data
            break;
        }
        Err(e) => return Err(e),
    }
}
```

**Cancellation Safety**:

| Operation                                   | Cancellation Safe | Notes                         |
| ------------------------------------------- | ----------------- | ----------------------------- |
| `check()`                                   | Yes               | No side effects               |
| `check_batch()`                             | Yes               | Partial results available     |
| `relationships().write()`                   | No                | May or may not have completed |
| `relationships().write_batch()`             | No                | Atomic - all or nothing       |
| `relationships().delete()`                  | Yes               | Idempotent - safe to retry    |
| `resources/subjects/relationships().list()` | Yes               | Partial results available     |
| `watch()`                                   | Yes               | Clean disconnect              |

### Circuit Breaker

Circuit breakers prevent cascade failures by temporarily stopping requests to a failing service. Unlike retry, which handles transient failures, circuit breakers protect against sustained outages.

```rust
use inferadb::CircuitBreakerConfig;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .circuit_breaker(CircuitBreakerConfig::default()
        .failure_threshold(5)           // Open after 5 consecutive failures
        .success_threshold(2)           // Close after 2 successes in half-open
        .timeout(Duration::from_secs(30))  // Try half-open after 30s
        .failure_rate_threshold(0.5)    // Or open at 50% failure rate
        .minimum_requests(10))          // Need 10 requests before rate applies
    .build()
    .await?;
```

**Circuit Breaker States**:

```text
┌─────────────────────────────────────────────────────────────────────┐
│                     Circuit Breaker States                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌──────────┐    failure_threshold    ┌──────────┐                │
│   │  CLOSED  │ ───────exceeded───────► │   OPEN   │                │
│   │          │                         │          │                │
│   │ (normal  │                         │ (reject  │                │
│   │  traffic)│ ◄──success_threshold──  │  fast)   │                │
│   └──────────┘          met            └────┬─────┘                │
│        ▲                                    │                       │
│        │                              timeout                       │
│        │                              elapsed                       │
│        │                                    │                       │
│        │     success_threshold        ┌────▼─────┐                 │
│        └────────── met ───────────────│HALF-OPEN │                 │
│                                       │          │                 │
│              failure ─────────────────│ (test    │                 │
│              (back to OPEN)           │  traffic)│                 │
│                                       └──────────┘                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**State Behaviors**:

| State     | Behavior                                     | Transitions To             |
| --------- | -------------------------------------------- | -------------------------- |
| Closed    | All requests pass through normally           | Open (on threshold breach) |
| Open      | Requests fail immediately with `CircuitOpen` | Half-Open (after timeout)  |
| Half-Open | Limited requests to test recovery            | Closed or Open             |

**Circuit Breaker Types**:

```rust
/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to open the circuit
    failure_threshold: u32,
    /// Number of successes in half-open state to close the circuit
    success_threshold: u32,
    /// Duration to wait before transitioning from open to half-open
    timeout: Duration,
    /// Failure rate threshold (0.0-1.0) to open the circuit
    failure_rate_threshold: Option<f64>,
    /// Minimum requests before failure rate is evaluated
    minimum_requests: u32,
    /// Which errors count as failures
    failure_predicate: FailurePredicate,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            failure_rate_threshold: None,
            minimum_requests: 10,
            failure_predicate: FailurePredicate::default(),
        }
    }
}

impl CircuitBreakerConfig {
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn failure_rate_threshold(mut self, rate: f64) -> Self {
        self.failure_rate_threshold = Some(rate);
        self
    }

    pub fn minimum_requests(mut self, count: u32) -> Self {
        self.minimum_requests = count;
        self
    }

    /// Customize which errors count as circuit breaker failures
    pub fn failure_predicate(mut self, predicate: FailurePredicate) -> Self {
        self.failure_predicate = predicate;
        self
    }
}

/// Determines which errors count toward circuit breaker failure threshold
#[derive(Debug, Clone)]
pub struct FailurePredicate {
    /// Count these error kinds as failures (default: Timeout, ConnectionFailed, ServiceUnavailable)
    include: Vec<ErrorKind>,
    /// Exclude these error kinds from failure count
    exclude: Vec<ErrorKind>,
}

impl Default for FailurePredicate {
    /// Default: Timeout, ConnectionFailed, ServiceUnavailable, ServerError are failures
    fn default() -> Self {
        Self {
            include: vec![
                ErrorKind::Timeout,
                ErrorKind::ConnectionFailed,
                ErrorKind::ServiceUnavailable,
                ErrorKind::ServerError,
            ],
            exclude: vec![],
        }
    }
}

impl FailurePredicate {
    /// Only count specific error kinds as failures
    pub fn only(kinds: impl IntoIterator<Item = ErrorKind>) -> Self {
        Self {
            include: kinds.into_iter().collect(),
            exclude: vec![],
        }
    }

    /// Add an error kind to exclude from failure count
    pub fn exclude(mut self, kind: ErrorKind) -> Self {
        self.exclude.push(kind);
        self
    }
}
```

**Circuit Breaker Error**:

```rust
// When circuit is open, requests fail immediately
match vault.check("user:alice", "view", "doc:1").await {
    Err(e) if e.kind() == ErrorKind::CircuitOpen => {
        // Circuit is open - don't retry, use fallback
        tracing::warn!("Circuit open, using cached decision");
        use_cached_decision("user:alice", "view", "doc:1")
    }
    Err(e) => Err(e.into()),
    Ok(allowed) => Ok(allowed),
}
```

**Monitoring Circuit State**:

```rust
// Access circuit breaker state for observability
let state = client.circuit_breaker_state();
println!("Circuit state: {:?}", state.current_state());
println!("Failure count: {}", state.failure_count());
println!("Success count: {}", state.success_count());

// Register a callback for state changes
client.on_circuit_state_change(|old, new| {
    tracing::warn!(
        old_state = ?old,
        new_state = ?new,
        "Circuit breaker state changed"
    );
    // Alert, update metrics, etc.
});
```

**Per-Operation Circuit Breakers**:

```rust
// Different circuit breakers for different operation types
let client = Client::builder()
    .circuit_breaker_for(OperationType::Check, CircuitBreakerConfig::default()
        .failure_threshold(10)
        .timeout(Duration::from_secs(15)))
    .circuit_breaker_for(OperationType::Write, CircuitBreakerConfig::default()
        .failure_threshold(3)
        .timeout(Duration::from_secs(60)))
    .build()
    .await?;

// Check per-operation circuit state
let check_circuit = client.circuit_stats_for(OperationType::Check);
let write_circuit = client.circuit_stats_for(OperationType::Write);
```

**Circuit State Inspection**:

```rust
/// Detailed circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitStats {
    /// Current state of the circuit
    pub state: CircuitState,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Number of consecutive successes (in half-open state)
    pub success_count: u32,
    /// Time when circuit will transition from Open to HalfOpen
    pub opens_at: Option<Instant>,
    /// Time remaining until circuit transitions (if Open)
    pub time_until_half_open: Option<Duration>,
    /// Total number of times circuit has opened
    pub total_opens: u64,
    /// Last failure reason (if any)
    pub last_failure: Option<String>,
    /// Timestamp of last state change
    pub last_state_change: Instant,
}

impl CircuitStats {
    /// Returns true if requests will be accepted
    pub fn is_accepting_requests(&self) -> bool {
        matches!(self.state, CircuitState::Closed | CircuitState::HalfOpen)
    }
}

// Usage
let stats = client.circuit_stats();
println!("Circuit state: {:?}", stats.state);
println!("Failure count: {}", stats.failure_count);
println!("Accepting requests: {}", stats.is_accepting_requests());

if stats.state == CircuitState::Open {
    if let Some(remaining) = stats.time_until_half_open {
        println!("Circuit reopens in: {:?}", remaining);
    }
}
```

**Circuit Breaker Events**:

```rust
/// Events emitted by the circuit breaker
#[derive(Debug, Clone)]
pub enum CircuitEvent {
    /// Circuit transitioned to open state
    Opened { failure_count: u32, last_error: String },
    /// Circuit transitioned to half-open state
    HalfOpened,
    /// Circuit transitioned to closed state
    Closed { success_count: u32 },
}

// Subscribe to circuit breaker state changes
let mut circuit_events = client.circuit_events();
tokio::spawn(async move {
    while let Some(event) = circuit_events.recv().await {
        match event {
            CircuitEvent::Opened { failure_count, last_error } => {
                tracing::warn!(
                    "Circuit opened after {} failures: {}",
                    failure_count, last_error
                );
                // Alert on-call, switch to fallback mode
            }
            CircuitEvent::HalfOpened => {
                tracing::info!("Circuit half-opened, testing connection");
            }
            CircuitEvent::Closed { success_count } => {
                tracing::info!(
                    "Circuit closed after {} successful probes",
                    success_count
                );
            }
        }
    }
});
```

**Manual Circuit Control**:

```rust
// Force circuit state (use with caution)
client.circuit_force_open();   // Immediately open the circuit
client.circuit_force_close();  // Immediately close the circuit
client.circuit_reset();        // Reset to initial closed state

// Useful for maintenance windows
async fn maintenance_mode(client: &Client) {
    client.circuit_force_open();
    // Perform maintenance...
    client.circuit_reset();
}
```

**Integration with tower**:

For users who prefer the tower ecosystem, the SDK is compatible with `tower::limit::CircuitBreaker`:

```rust
use tower::ServiceBuilder;
use inferadb::tower::InferaDbService;

let service = ServiceBuilder::new()
    .layer(tower::limit::ConcurrencyLimitLayer::new(100))
    .layer(tower::limit::RateLimitLayer::new(1000, Duration::from_secs(1)))
    // Use tower's circuit breaker instead of built-in
    .service(InferaDbService::new(client));
```

---

## Graceful Degradation

### Fail-Open vs Fail-Closed

```rust
// Fail-closed (default, more secure)
let allowed = vault
    .check("user:alice", "view", "document:readme")
    .on_error(OnError::FailClosed)  // Default
    .await
    .unwrap_or(false);

// Fail-open (for non-critical paths only - use with extreme caution)
let allowed = vault
    .check("user:alice", "view", "document:readme")
    .on_error(OnError::FailOpen)  // Logs WARN when triggered
    .await
    .unwrap_or(true);
```

### Comprehensive Degradation Configuration

Configure global degradation behavior for production resilience:

```rust
use inferadb::{
    DegradationConfig, FailureMode,
    CheckFallbackStrategy, WriteFallbackStrategy,
};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .degradation(DegradationConfig::new()
        // Default failure mode for authorization checks
        .on_check_failure(FailureMode::FailClosed)

        // Use cached decisions when service is unavailable for checks
        .on_check_unavailable(CheckFallbackStrategy::UseCache {
            max_age: Duration::from_secs(300),  // Accept cache entries up to 5 min old
        })

        // For write failures, queue for later retry
        .on_write_failure(WriteFallbackStrategy::Queue {
            max_queue_size: 1000,
            flush_interval: Duration::from_secs(5),
        })

        // Alert when degradation activates
        .on_degradation_start(|reason| {
            tracing::warn!("Entering degraded mode: {}", reason);
            metrics::counter!("inferadb.degradation.activations").increment(1);
        })

        // Alert when service recovers
        .on_degradation_end(|| {
            tracing::info!("Exiting degraded mode, service recovered");
        }))
    .build()
    .await?;
```

**Degradation Configuration Types**:

```rust
/// Global degradation behavior configuration.
///
/// Uses operation-specific fallback strategies to ensure type safety:
/// - Check operations use `CheckFallbackStrategy` (returns bool)
/// - Write operations use `WriteFallbackStrategy` (may queue)
/// - Read operations use `ReadFallbackStrategy` (returns cached data)
#[derive(Debug, Clone)]
pub struct DegradationConfig {
    /// Default behavior when authorization checks fail
    on_check_failure: FailureMode,
    /// Strategy when service is unavailable for checks
    on_check_unavailable: CheckFallbackStrategy,
    /// Strategy when write operations fail
    on_write_failure: WriteFallbackStrategy,
    /// Strategy when read operations fail
    on_read_failure: ReadFallbackStrategy,
    /// Callback when degradation mode activates
    on_degradation_start: Option<Arc<dyn Fn(String) + Send + Sync>>,
    /// Callback when service recovers
    on_degradation_end: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl DegradationConfig {
    /// Create a new degradation configuration with secure defaults.
    pub fn new() -> Self {
        Self {
            on_check_failure: FailureMode::FailClosed,
            on_check_unavailable: CheckFallbackStrategy::Error,
            on_write_failure: WriteFallbackStrategy::Error,
            on_read_failure: ReadFallbackStrategy::Error,
            on_degradation_start: None,
            on_degradation_end: None,
        }
    }

    /// Set behavior when authorization checks fail.
    pub fn on_check_failure(mut self, mode: FailureMode) -> Self {
        self.on_check_failure = mode;
        self
    }

    /// Set strategy when service is unavailable for checks.
    pub fn on_check_unavailable(mut self, strategy: CheckFallbackStrategy) -> Self {
        self.on_check_unavailable = strategy;
        self
    }

    /// Set strategy when write operations fail.
    pub fn on_write_failure(mut self, strategy: WriteFallbackStrategy) -> Self {
        self.on_write_failure = strategy;
        self
    }

    /// Set strategy when read operations fail.
    pub fn on_read_failure(mut self, strategy: ReadFallbackStrategy) -> Self {
        self.on_read_failure = strategy;
        self
    }

    /// Set callback when degradation mode activates.
    pub fn on_degradation_start<F>(mut self, callback: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_degradation_start = Some(Arc::new(callback));
        self
    }

    /// Set callback when service recovers from degradation.
    pub fn on_degradation_end<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_degradation_end = Some(Arc::new(callback));
        self
    }
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// How to handle authorization check failures.
///
/// Naming uses industry-standard "fail-open" / "fail-closed" terminology
/// to clearly distinguish error-handling behavior from authorization outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailureMode {
    /// Deny access on failure (fail-closed, secure default).
    /// Use for security-critical paths where false positives are acceptable.
    #[default]
    FailClosed,

    /// Allow access on failure (fail-open, use with extreme caution).
    /// Only use for non-critical paths where availability trumps security.
    /// Always logs at WARN level when triggered.
    FailOpen,

    /// Return error to caller (let application decide).
    /// Use when the calling code needs to implement custom fallback logic.
    Propagate,
}

/// Type alias for per-request error handling configuration.
/// Same as `FailureMode` but semantically describes request-level behavior.
pub type OnError = FailureMode;

// -----------------------------------------------------------------------------
// Operation-Specific Fallback Strategies
// -----------------------------------------------------------------------------
// Fallback strategies are split by operation type to ensure type safety.
// A check fallback returns bool, a write fallback may queue, a read fallback
// returns cached data. Mixing these would be a type error.

/// Fallback strategy for authorization checks when service is unavailable
#[derive(Debug, Clone)]
pub enum CheckFallbackStrategy {
    /// Return error immediately
    Error,

    /// Use cached decision if available and not too old
    UseCache {
        /// Maximum age of cached decision to accept
        max_age: Duration,
    },

    /// Return a default decision (use with caution)
    Default(bool),

    /// Call a custom fallback function
    Custom(Arc<dyn Fn(&CheckRequest) -> Result<bool, Error> + Send + Sync>),
}

/// Fallback strategy for write operations when service is unavailable
#[derive(Debug, Clone)]
pub enum WriteFallbackStrategy {
    /// Return error immediately (default, safest)
    Error,

    /// Queue operation for later retry
    Queue {
        /// Maximum number of operations to queue
        max_queue_size: usize,
        /// How often to attempt flushing the queue
        flush_interval: Duration,
    },

    /// Call a custom fallback function
    Custom(Arc<dyn Fn(&WriteRequest) -> Result<WriteAction, Error> + Send + Sync>),
}

/// Action to take for queued writes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    /// Queue the write for later retry
    Queue,
    /// Drop the write silently (use with caution)
    Drop,
    /// Return error to caller
    Error,
}

/// Fallback strategy for read operations when service is unavailable
#[derive(Debug, Clone)]
pub enum ReadFallbackStrategy {
    /// Return error immediately
    Error,

    /// Use cached value if available and not too old
    UseCache {
        /// Maximum age of cached value to accept
        max_age: Duration,
    },

    /// Return empty result set
    Empty,

    /// Call a custom fallback function
    Custom(Arc<dyn Fn(&ReadRequest) -> Result<ReadAction, Error> + Send + Sync>),
}

/// Action to take for degraded reads
#[derive(Debug, Clone)]
pub enum ReadAction {
    /// Use cached data
    UseCached,
    /// Return empty result
    Empty,
    /// Return error
    Error,
}
```

**Per-Operation Degradation**:

```rust
// Override global degradation for specific operations
let allowed = vault
    .check("user:alice", "view", "public-doc:readme")
    .on_error(OnError::FailOpen)  // This specific check can fail-open (logs WARN)
    .fallback(CheckFallbackStrategy::Default(true))
    .await?;

// Critical operation - never use fallback
let allowed = vault
    .check("user:alice", "delete", "doc:sensitive")
    .on_error(OnError::Propagate)  // Always return errors
    .fallback(CheckFallbackStrategy::Error)  // No fallback allowed
    .await?;
```

**Degradation Metrics**:

| Metric                               | Type    | Description                         |
| ------------------------------------ | ------- | ----------------------------------- |
| `inferadb.degradation.active`        | Gauge   | 1 if in degraded mode, 0 otherwise  |
| `inferadb.degradation.activations`   | Counter | Number of times degradation started |
| `inferadb.degradation.cache_hits`    | Counter | Successful cache fallbacks          |
| `inferadb.degradation.cache_misses`  | Counter | Cache fallback failures (too old)   |
| `inferadb.degradation.queue_size`    | Gauge   | Current size of retry queue         |
| `inferadb.degradation.queue_flushes` | Counter | Number of queue flush attempts      |

### Circuit Breaker Integration

Circuit breakers integrate with graceful degradation to provide automatic fallback when the service is unavailable. See [Circuit Breaker](#circuit-breaker) in the Retry & Resilience section for full configuration details.

**Quick Reference**:

```rust
// When circuit is open, use degradation fallback
match vault.check("user:alice", "view", "doc:1").await {
    Err(e) if e.kind() == ErrorKind::CircuitOpen => {
        // Circuit is open - use cached decision or fail-open/fail-closed
        tracing::warn!("Circuit open, using fallback");
        use_cached_decision_or_default()
    }
    result => result,
}
```

The circuit breaker works in conjunction with:

- **`on_check_unavailable(CheckFallbackStrategy::UseCache { ... })`** - Use cached decisions when circuit is open
- **`on_check_failure(FailureMode::FailClosed)`** - Fail-closed when no cache available
- **Circuit events** - Alert when entering/exiting degraded mode

---

## Observability

### Health Checks

Monitor service health and component status.

```rust
// Basic health check
let health = client.health().await?;

println!("Status: {:?}", health.status);
println!("Healthy: {}", health.is_healthy());

// Detailed health check
let health = client
    .health()
    .verbose(true)
    .await?;

println!("Overall: {:?}", health.status);
for (component, status) in &health.components {
    println!("  {}: {:?}", component, status.status);
    if let Some(message) = &status.message {
        println!("    {}", message);
    }
}

// Check specific components
let health = client
    .health()
    .components(&["api", "storage", "cache"])
    .await?;
```

**Health Types**: See [Health Response Structure](#health-response-structure) for `HealthResponse`, `HealthStatus`, and `ComponentHealth` type definitions.

### Ping & Latency

Measure network latency and connectivity.

```rust
// Simple ping
let latency = client.ping().await?;
println!("Latency: {:?}", latency);

// Ping with statistics
let stats = client
    .ping()
    .count(10)
    .await?;

println!("Ping statistics:");
println!("  Min: {:?}", stats.min);
println!("  Max: {:?}", stats.max);
println!("  Avg: {:?}", stats.avg);
println!("  Stddev: {:?}", stats.stddev);
println!("  Packet loss: {:.1}%", stats.packet_loss_percent());

// Ping specific target
let stats = client
    .ping()
    .target(PingTarget::Engine)  // or Control, or specific endpoint
    .count(5)
    .await?;
```

**Ping Types**:

```rust
#[derive(Debug, Clone)]
pub struct PingStats {
    pub count: u32,
    pub successful: u32,
    pub failed: u32,
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub stddev: Duration,
    pub samples: Vec<Duration>,
}

impl PingStats {
    pub fn packet_loss_percent(&self) -> f64 {
        if self.count == 0 { 0.0 }
        else { (self.failed as f64 / self.count as f64) * 100.0 }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PingTarget {
    /// Default - pings the configured endpoint
    Default,
    /// Ping the engine API
    Engine,
    /// Ping the control API
    Control,
}
```

### Rate Limit Visibility

Monitor rate limit status proactively before hitting limits:

```rust
// Get current rate limit status
let limits = client.rate_limits().await?;

println!("Rate Limits:");
println!("  Requests: {}/{} (resets in {:?})",
    limits.requests.remaining,
    limits.requests.limit,
    limits.requests.reset_in);
println!("  Writes: {}/{} (resets in {:?})",
    limits.writes.remaining,
    limits.writes.limit,
    limits.writes.reset_in);

// Check if approaching limits
if limits.requests.remaining < 100 {
    tracing::warn!("Approaching request rate limit");
}

// Rate limit headers are also available on responses
let result = vault.check("user:alice", "view", "doc:1").await?;
if let Some(remaining) = result.rate_limit_remaining() {
    metrics::gauge!("inferadb.rate_limit.remaining").set(remaining as f64);
}
```

**Rate Limit Types**:

```rust
#[derive(Debug, Clone)]
pub struct RateLimits {
    /// Overall request rate limit
    pub requests: RateLimitInfo,
    /// Write operation rate limit
    pub writes: RateLimitInfo,
    /// Batch operation rate limit
    pub batch: RateLimitInfo,
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the window
    pub limit: u32,
    /// Requests remaining in current window
    pub remaining: u32,
    /// Time until the window resets
    pub reset_in: Duration,
    /// Timestamp when the window resets
    pub reset_at: DateTime<Utc>,
}

impl RateLimitInfo {
    /// Percentage of rate limit consumed
    pub fn usage_percent(&self) -> f64 {
        ((self.limit - self.remaining) as f64 / self.limit as f64) * 100.0
    }

    /// Whether rate limit is exhausted
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }
}
```

### Connection Pool Metrics

Monitor connection pool health and utilization:

```rust
// Get pool statistics
let pool = client.pool_stats();

println!("Connection Pool:");
println!("  Active connections: {}", pool.active);
println!("  Idle connections: {}", pool.idle);
println!("  Pending requests: {}", pool.pending);
println!("  Total connections: {}/{}", pool.total, pool.max_size);

// Monitor pool utilization
if pool.utilization_percent() > 80.0 {
    tracing::warn!(
        utilization = pool.utilization_percent(),
        "Connection pool utilization high"
    );
}

// Pool statistics for metrics export
let pool = client.pool_stats();
metrics::gauge!("inferadb.pool.active").set(pool.active as f64);
metrics::gauge!("inferadb.pool.idle").set(pool.idle as f64);
metrics::gauge!("inferadb.pool.pending").set(pool.pending as f64);
metrics::gauge!("inferadb.pool.utilization").set(pool.utilization_percent());
```

**Pool Stats Types**:

```rust
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Connections currently in use
    pub active: u32,
    /// Connections available for reuse
    pub idle: u32,
    /// Requests waiting for a connection
    pub pending: u32,
    /// Total connections (active + idle)
    pub total: u32,
    /// Maximum pool size
    pub max_size: u32,
    /// Connections created since startup
    pub connections_created: u64,
    /// Connections closed since startup
    pub connections_closed: u64,
    /// Connection wait time histogram (p50, p95, p99)
    pub wait_time: LatencyStats,
}

impl PoolStats {
    /// Pool utilization as percentage
    pub fn utilization_percent(&self) -> f64 {
        if self.max_size == 0 { 0.0 }
        else { (self.active as f64 / self.max_size as f64) * 100.0 }
    }

    /// Whether pool is under pressure
    pub fn is_saturated(&self) -> bool {
        self.pending > 0 && self.idle == 0
    }
}

/// Latency percentile statistics
#[derive(Debug, Clone)]
pub struct LatencyStats {
    /// Median latency (50th percentile)
    pub p50: Duration,
    /// 95th percentile latency
    pub p95: Duration,
    /// 99th percentile latency
    pub p99: Duration,
    /// Maximum observed latency
    pub max: Duration,
}
```

### Diagnostics

Comprehensive connectivity and configuration diagnostics.

```rust
// Run full diagnostics
let diagnostics = client
    .diagnostics()
    .run()
    .await?;

println!("Diagnostics Report");
println!("==================");

for check in &diagnostics.checks {
    let icon = if check.passed { "✓" } else { "✗" };
    println!("{} {}: {}", icon, check.name, check.message);

    if let Some(suggestion) = &check.suggestion {
        println!("  → {}", suggestion);
    }
}

println!("\nOverall: {}", if diagnostics.all_passed() { "PASS" } else { "FAIL" });
```

**Selective Diagnostics**:

```rust
// Run specific checks
let diagnostics = client
    .diagnostics()
    .check_dns()
    .check_tls()
    .check_auth()
    .check_permissions()
    .run()
    .await?;

// Skip certain checks
let diagnostics = client
    .diagnostics()
    .skip(&["permissions"])  // Skip permission check
    .run()
    .await?;
```

**Diagnostics Types**:

```rust
#[derive(Debug, Clone)]
pub struct DiagnosticsReport {
    pub checks: Vec<DiagnosticCheck>,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
}

impl DiagnosticsReport {
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    pub fn failed_checks(&self) -> Vec<&DiagnosticCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub suggestion: Option<String>,
    pub duration: Duration,
    pub details: Option<serde_json::Value>,
}

/// Available diagnostic checks
pub mod checks {
    pub const DNS: &str = "dns";
    pub const TLS: &str = "tls";
    pub const CONNECTIVITY: &str = "connectivity";
    pub const AUTH: &str = "auth";
    pub const PERMISSIONS: &str = "permissions";
    pub const VAULT_ACCESS: &str = "vault_access";
    pub const LATENCY: &str = "latency";
}
```

### Tracing Integration

```rust
// With tracing feature enabled
let client = Client::builder()
    .url("https://api.inferadb.com")
    .with_tracing()
    .build()
    .await?;

// Spans are automatically created for each operation:
// inferadb.check{subject="user:alice", permission="view", resource="doc:1"}
//   └── inferadb.transport.request{method="POST", path="/access/v1/evaluation"}
```

### Metrics

```rust
// With metrics feature enabled
let client = Client::builder()
    .url("https://api.inferadb.com")
    .with_metrics()
    .build()
    .await?;

// Emitted metrics:
// - inferadb_requests_total{operation, status}
// - inferadb_request_duration_seconds{operation}
// - inferadb_connection_pool_size
// - inferadb_cache_hits_total
// - inferadb_cache_misses_total
```

### OpenTelemetry

Full OpenTelemetry integration for distributed tracing and metrics export.

**Basic Configuration**:

```rust
// Simple OTLP integration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .with_opentelemetry(OtelConfig::new("my-service")
        .endpoint("http://otel-collector:4317"))
    .build()
    .await?;
```

**Advanced Configuration with Provider Injection**:

```rust
use opentelemetry::global;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::metrics::MeterProvider;

// Use your existing OpenTelemetry providers
let tracer_provider: TracerProvider = /* your tracer provider */;
let meter_provider: MeterProvider = /* your meter provider */;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .observability(ObservabilityConfig::new()
        .tracer_provider(tracer_provider)
        .meter_provider(meter_provider)
        .propagator(TraceContextPropagator::new())
        .service_name("my-service")
        .service_version(env!("CARGO_PKG_VERSION")))
    .build()
    .await?;
```

**Observability Configuration Types**:

```rust
/// Comprehensive observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// OpenTelemetry tracer provider for distributed tracing
    tracer_provider: Option<Arc<dyn TracerProvider + Send + Sync>>,
    /// OpenTelemetry meter provider for metrics
    meter_provider: Option<Arc<dyn MeterProvider + Send + Sync>>,
    /// Context propagator (default: W3C TraceContext)
    propagator: Option<Box<dyn TextMapPropagator + Send + Sync>>,
    /// Service name for telemetry identification
    service_name: Option<String>,
    /// Service version
    service_version: Option<String>,
    /// Custom resource attributes
    resource_attributes: HashMap<String, String>,
    /// Sampling configuration
    sampling: SamplingConfig,
    /// Custom span attributes to include on all spans
    default_span_attributes: HashMap<String, AttributeValue>,
}

impl ObservabilityConfig {
    pub fn new() -> Self {
        Self {
            tracer_provider: None,
            meter_provider: None,
            propagator: None,
            service_name: None,
            service_version: None,
            resource_attributes: HashMap::new(),
            sampling: SamplingConfig::default(),
            default_span_attributes: HashMap::new(),
        }
    }

    /// Inject an existing tracer provider
    pub fn tracer_provider(
        mut self,
        provider: impl TracerProvider + Send + Sync + 'static,
    ) -> Self {
        self.tracer_provider = Some(Arc::new(provider));
        self
    }

    /// Inject an existing meter provider
    pub fn meter_provider(
        mut self,
        provider: impl MeterProvider + Send + Sync + 'static,
    ) -> Self {
        self.meter_provider = Some(Arc::new(provider));
        self
    }

    /// Set the context propagator (default: W3C TraceContext)
    pub fn propagator(
        mut self,
        propagator: impl TextMapPropagator + Send + Sync + 'static,
    ) -> Self {
        self.propagator = Some(Box::new(propagator));
        self
    }

    /// Set service name for telemetry
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Set service version
    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    /// Add custom resource attributes
    pub fn resource_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.resource_attributes.insert(key.into(), value.into());
        self
    }

    /// Configure sampling behavior
    pub fn sampling(mut self, config: SamplingConfig) -> Self {
        self.sampling = config;
        self
    }

    /// Add a default span attribute to all spans
    pub fn default_span_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.default_span_attributes.insert(key.into(), value.into());
        self
    }
}

/// Sampling configuration for traces
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// Base sampling ratio (0.0-1.0)
    pub ratio: f64,
    /// Always sample errors
    pub always_sample_errors: bool,
    /// Always sample slow operations (above threshold)
    pub slow_operation_threshold: Option<Duration>,
    /// Parent-based sampling (inherit from parent span)
    pub parent_based: bool,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            ratio: 1.0,  // Sample everything by default
            always_sample_errors: true,
            slow_operation_threshold: Some(Duration::from_millis(100)),
            parent_based: true,
        }
    }
}

impl SamplingConfig {
    /// Sample all traces
    pub fn always() -> Self {
        Self { ratio: 1.0, ..Default::default() }
    }

    /// Never sample (disable tracing)
    pub fn never() -> Self {
        Self { ratio: 0.0, always_sample_errors: false, slow_operation_threshold: None, parent_based: false }
    }

    /// Sample a percentage of traces
    pub fn ratio(ratio: f64) -> Self {
        Self { ratio: ratio.clamp(0.0, 1.0), ..Default::default() }
    }
}

/// Attribute value types for span attributes and resource attributes.
/// Compatible with OpenTelemetry attribute value types.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit floating point
    Float(f64),
    /// Array of strings
    StringArray(Vec<String>),
    /// Array of booleans
    BoolArray(Vec<bool>),
    /// Array of integers
    IntArray(Vec<i64>),
    /// Array of floats
    FloatArray(Vec<f64>),
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for AttributeValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for AttributeValue {
    fn from(i: i32) -> Self {
        Self::Int(i as i64)
    }
}

impl From<f64> for AttributeValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}
```

**Span Attributes Added by SDK**:

| Attribute                  | Description                         | Example                    |
| -------------------------- | ----------------------------------- | -------------------------- |
| `inferadb.operation`       | Operation type                      | `check`, `write`, `expand` |
| `inferadb.vault_id`        | Vault being accessed                | `vlt_01JFQGK...`           |
| `inferadb.organization_id` | Organization                        | `org_8675309...`           |
| `inferadb.subject`         | Subject of check (if not sensitive) | `user:alice`               |
| `inferadb.permission`      | Permission being checked            | `view`                     |
| `inferadb.resource_type`   | Resource type                       | `document`                 |
| `inferadb.allowed`         | Check result (on check operations)  | `true`                     |
| `inferadb.cached`          | Whether result was from cache       | `false`                    |
| `inferadb.consistency`     | Consistency level used              | `strong`                   |

**Disable Sensitive Attributes**:

```rust
// Don't include subject/resource in spans (for PII compliance)
let client = Client::builder()
    .observability(ObservabilityConfig::new()
        .redact_sensitive_attributes(true))  // Exclude subject, resource from spans
    .build()
    .await?;
```

**Metrics Emitted**:

| Metric                                  | Type      | Labels                     |
| --------------------------------------- | --------- | -------------------------- |
| `inferadb.client.requests`              | Counter   | `operation`, `status`      |
| `inferadb.client.request.duration`      | Histogram | `operation`                |
| `inferadb.client.connections.active`    | Gauge     | —                          |
| `inferadb.client.connections.idle`      | Gauge     | —                          |
| `inferadb.client.cache.hits`            | Counter   | `cache_type`               |
| `inferadb.client.cache.misses`          | Counter   | `cache_type`               |
| `inferadb.client.circuit_breaker.state` | Gauge     | `state` (closed/open/half) |
| `inferadb.client.retries`               | Counter   | `operation`, `attempt`     |

### W3C Trace Context Propagation

Full W3C Trace Context standard support for distributed tracing across service boundaries:

```rust
use inferadb::tracing::{TraceContext, Propagator};

// Extract trace context from incoming request
let trace_context = TraceContext::extract_from_headers(&request.headers())?;

// Create vault-scoped trace context
let vault = client.organization("org_8675309...").vault("vlt_01JFQGK...").with_tracing(trace_context);

// All subsequent operations inherit the trace context
let allowed = vault.check("user:alice", "view", "doc:1").await?;
// Traces will show: service-a -> inferadb-sdk -> inferadb-api

// Manual trace context creation
let trace_context = TraceContext::new()
    .with_trace_id(TraceId::random())
    .with_span_id(SpanId::random())
    .with_sampled(true);
```

**Tracing Types**:

````rust
/// W3C Trace Context representation for distributed tracing.
#[derive(Debug, Clone)]
pub struct TraceContext {
    trace_id: TraceId,
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    sampled: bool,
    trace_state: Option<String>,
}

impl TraceContext {
    /// Create a new trace context with random IDs.
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::random(),
            span_id: SpanId::random(),
            parent_span_id: None,
            sampled: true,
            trace_state: None,
        }
    }

    /// Extract trace context from HTTP headers (W3C Trace Context format).
    pub fn extract_from_headers(headers: &HeaderMap) -> Result<Self, Error> {
        let traceparent = headers
            .get("traceparent")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::invalid_argument("missing traceparent header"))?;

        let mut context = Self::from_traceparent(traceparent)?;

        // Optionally parse tracestate
        if let Some(tracestate) = headers.get("tracestate").and_then(|v| v.to_str().ok()) {
            context.trace_state = Some(tracestate.to_owned());
        }

        Ok(context)
    }

    /// Inject trace context into HTTP headers.
    pub fn inject_into_headers(&self, headers: &mut HeaderMap) {
        if let Ok(value) = http::HeaderValue::from_str(&self.to_traceparent()) {
            headers.insert("traceparent", value);
        }
        if let Some(state) = &self.trace_state {
            if let Ok(value) = http::HeaderValue::from_str(state) {
                headers.insert("tracestate", value);
            }
        }
    }

    /// Set the trace ID.
    pub fn with_trace_id(mut self, id: TraceId) -> Self {
        self.trace_id = id;
        self
    }

    /// Set the span ID.
    pub fn with_span_id(mut self, id: SpanId) -> Self {
        self.span_id = id;
        self
    }

    /// Set the parent span ID.
    pub fn with_parent_span_id(mut self, id: SpanId) -> Self {
        self.parent_span_id = Some(id);
        self
    }

    /// Set whether this trace is sampled.
    pub fn with_sampled(mut self, sampled: bool) -> Self {
        self.sampled = sampled;
        self
    }

    /// Get the trace ID.
    pub fn trace_id(&self) -> &TraceId { &self.trace_id }

    /// Get the span ID.
    pub fn span_id(&self) -> &SpanId { &self.span_id }

    /// Check if this trace is sampled.
    pub fn is_sampled(&self) -> bool { self.sampled }

    /// Get the trace state (if set).
    pub fn trace_state(&self) -> Option<&str> {
        self.trace_state.as_deref()
    }

    /// Parse from W3C traceparent header format.
    /// Format: "00-{trace_id}-{span_id}-{flags}"
    pub fn from_traceparent(header: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return Err(Error::invalid_argument("invalid traceparent format"));
        }
        let trace_id = TraceId::from_hex(parts[1])?;
        let span_id = SpanId::from_hex(parts[2])?;
        let flags = u8::from_str_radix(parts[3], 16)
            .map_err(|_| Error::invalid_argument("invalid flags"))?;
        Ok(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            sampled: flags & 0x01 != 0,
            trace_state: None,
        })
    }

    /// Format as W3C traceparent header.
    pub fn to_traceparent(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id.to_hex(), self.span_id.to_hex(), flags)
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 128-bit trace identifier (W3C Trace Context format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId([u8; 16]);

impl TraceId {
    /// Generate a random trace ID.
    pub fn random() -> Self {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes).expect("random bytes");
        Self(bytes)
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        if hex.len() != 32 {
            return Err(Error::invalid_argument("trace_id must be 32 hex characters"));
        }
        let mut bytes = [0u8; 16];
        hex::decode_to_slice(hex, &mut bytes)
            .map_err(|_| Error::invalid_argument("invalid hex"))?;
        Ok(Self(bytes))
    }

    /// Get as hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Get raw bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// 64-bit span identifier (W3C Trace Context format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId([u8; 8]);

impl SpanId {
    /// Generate a random span ID.
    pub fn random() -> Self {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).expect("random bytes");
        Self(bytes)
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        if hex.len() != 16 {
            return Err(Error::invalid_argument("span_id must be 16 hex characters"));
        }
        let mut bytes = [0u8; 8];
        hex::decode_to_slice(hex, &mut bytes)
            .map_err(|_| Error::invalid_argument("invalid hex"))?;
        Ok(Self(bytes))
    }

    /// Get as hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Get raw bytes.
    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Trait for propagating trace context across service boundaries.
/// Compatible with OpenTelemetry's TextMapPropagator.
pub trait Propagator: Send + Sync {
    /// Extract trace context from carrier (e.g., HTTP headers).
    fn extract(&self, carrier: &dyn Carrier) -> Option<TraceContext>;

    /// Inject trace context into carrier (e.g., HTTP headers).
    fn inject(&self, context: &TraceContext, carrier: &mut dyn CarrierMut);
}

/// Read-only carrier for trace context extraction.
pub trait Carrier {
    fn get(&self, key: &str) -> Option<&str>;
}

/// Mutable carrier for trace context injection.
pub trait CarrierMut {
    fn set(&mut self, key: &str, value: String);
}

/// W3C Trace Context propagator (default).
/// Handles `traceparent` and `tracestate` headers.
pub struct TraceContextPropagator;

impl TraceContextPropagator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TraceContextPropagator {
    fn default() -> Self {
        Self::new()
    }
}

impl Propagator for TraceContextPropagator {
    fn extract(&self, carrier: &dyn Carrier) -> Option<TraceContext> {
        let traceparent = carrier.get("traceparent")?;
        // Parse W3C traceparent format: version-trace_id-span_id-flags
        // e.g., "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        TraceContext::from_traceparent(traceparent).ok()
    }

    fn inject(&self, context: &TraceContext, carrier: &mut dyn CarrierMut) {
        carrier.set("traceparent", context.to_traceparent());
        if let Some(state) = context.trace_state() {
            carrier.set("tracestate", state.to_string());
        }
    }
}

// Implement Carrier for common types
impl Carrier for http::HeaderMap {
    fn get(&self, key: &str) -> Option<&str> {
        self.get(key)?.to_str().ok()
    }
}

impl CarrierMut for http::HeaderMap {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = http::header::HeaderName::try_from(key) {
            if let Ok(val) = http::header::HeaderValue::try_from(value) {
                self.insert(name, val);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Framework Integration Types
// ─────────────────────────────────────────────────────────────────────────────

/// Extension trait for trace context propagation across async boundaries.
///
/// This trait is automatically implemented for types that carry trace context,
/// enabling seamless propagation through spawned tasks and streams.
pub trait TraceContextExt {
    /// Returns the current trace context, if any.
    fn trace_context(&self) -> Option<TraceContext>;

    /// Attaches a trace context to be propagated.
    fn with_trace_context(self, context: TraceContext) -> Self;
}

// Axum integration (feature = "axum")
pub mod integrations {
    pub mod axum {
        use super::super::*;

        /// Tower layer that extracts W3C trace context from incoming requests
        /// and makes it available to InferaDB client operations.
        ///
        /// # Example
        ///
        /// ```rust
        /// use inferadb::integrations::axum::InferaDbTraceLayer;
        ///
        /// let app = Router::new()
        ///     .route("/api/check", post(check_permission))
        ///     .layer(InferaDbTraceLayer::new(client.clone()));
        /// ```
        #[derive(Clone)]
        pub struct InferaDbTraceLayer {
            client: crate::Client,
        }

        impl InferaDbTraceLayer {
            /// Creates a new layer that propagates trace context to the given client.
            pub fn new(client: crate::Client) -> Self {
                Self { client }
            }
        }

        impl<S> tower::Layer<S> for InferaDbTraceLayer {
            type Service = InferaDbTraceService<S>;

            fn layer(&self, inner: S) -> Self::Service {
                InferaDbTraceService {
                    inner,
                    client: self.client.clone(),
                }
            }
        }

        /// Service that wraps handlers with trace context extraction.
        #[derive(Clone)]
        pub struct InferaDbTraceService<S> {
            inner: S,
            client: crate::Client,
        }
    }

    pub mod actix {
        use super::super::*;

        /// Actix Web middleware that extracts W3C trace context from incoming requests
        /// and propagates it to InferaDB client operations.
        ///
        /// # Example
        ///
        /// ```rust
        /// use inferadb::integrations::actix::TracingMiddleware;
        ///
        /// App::new()
        ///     .wrap(TracingMiddleware::new(client.clone()))
        ///     .service(web::resource("/check").to(check_permission))
        /// ```
        #[derive(Clone)]
        pub struct TracingMiddleware {
            client: crate::Client,
        }

        impl TracingMiddleware {
            /// Creates new middleware that propagates trace context to the given client.
            pub fn new(client: crate::Client) -> Self {
                Self { client }
            }
        }

        // Actix middleware implementation would go here
        // impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for TracingMiddleware { ... }
    }
}
````

#### Async Boundary Propagation

Trace context automatically propagates across async boundaries:

```rust
use inferadb::tracing::TraceContextExt;

// Context propagates through spawned tasks
let vault = vault.clone();
tokio::spawn(async move {
    // Trace context is automatically captured and restored
    vault.check("user:alice", "view", "doc:1").await
});

// Context flows through streams
let mut stream = vault.resources().accessible_by("user:alice").with_permission("view").stream();
while let Some(resource) = stream.next().await {
    // Each item inherits parent trace context
    process(resource?);
}
```

#### Framework Integration

Automatic trace context extraction for popular frameworks:

```rust
// Axum middleware
use inferadb::integrations::axum::InferaDbTraceLayer;

let app = Router::new()
    .route("/api/documents", get(list_documents))
    .layer(InferaDbTraceLayer::new(client.clone()));

// Actix middleware
use inferadb::integrations::actix::TracingMiddleware;

App::new()
    .wrap(TracingMiddleware::new(client.clone()))
    .service(web::resource("/documents").to(list_documents))
```

#### Inject Context to Outgoing Requests

```rust
use inferadb::tracing::Propagator;

// Get current trace context from SDK
let trace_context = client.current_trace_context();

// Inject into outgoing HTTP request
let mut headers = HeaderMap::new();
trace_context.inject_into_headers(&mut headers);

// Standard W3C headers are set:
// - traceparent: 00-{trace_id}-{span_id}-{flags}
// - tracestate: inferadb=...
```

---

## Testing Support

### MockClient: The Hero Testing Pattern

**Start here.** `MockClient` mirrors the production API, so your tests look like production code with only the client swapped:

```rust
use inferadb::testing::MockClient;

// Your production code - accepts any AuthorizationClient
async fn get_document(
    authz: &impl AuthorizationClient,
    user: &str,
    doc_id: &str,
) -> Result<Document, AppError> {
    authz.check(user, "view", doc_id)
        .require()
        .await?;
    fetch_document(doc_id).await
}

// Your test - swap MockClient for VaultClient
#[tokio::test]
async fn test_get_document_authorized() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .build();

    let result = get_document(&mock, "user:alice", "doc:1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_document_denied() {
    let mock = MockClient::builder()
        .check("user:bob", "view", "doc:1", false)
        .build();

    let result = get_document(&mock, "user:bob", "doc:1").await;
    assert!(matches!(result, Err(AppError::AccessDenied(_))));
}
```

**MockClient features:**

```rust
let mock = MockClient::builder()
    // Explicit check results
    .check("user:alice", "view", "doc:1", true)
    .check("user:alice", "edit", "doc:1", false)

    // Wildcard patterns
    .check_pattern("user:admin", "*", "*", true)  // Admin can do anything
    .check_pattern("*", "view", "doc:public", true)  // Anyone can view public

    // Simulate errors
    .check_error("user:alice", "view", "doc:broken", Error::new(ErrorKind::Timeout, "timeout"))

    // Mock relationship writes
    .write_ok(Relationship::new("doc:1", "viewer", "user:alice"))
    .write_error(Relationship::new("doc:bad", "viewer", "user:alice"),
                 Error::new(ErrorKind::SchemaViolation, "invalid"))

    // Verify calls were made
    .verify_on_drop(true)
    .build();

// After test, mock verifies all expected calls were made
```

### Testing Hierarchy

| Client Type      | Use Case                            | Speed      | Fidelity            |
| ---------------- | ----------------------------------- | ---------- | ------------------- |
| `MockClient`     | Unit tests, fast isolation          | ⚡ Fastest | Stub responses      |
| `InMemoryClient` | Integration tests, real graph logic | 🚀 Fast    | Full engine, no I/O |
| `TestVault`      | E2E tests against real InferaDB     | 🐢 Slower  | Production behavior |

### InMemoryClient for Integration Tests

When you need real permission evaluation logic (inheritance, unions, ABAC):

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_permission_inheritance() {
    // Real schema, real evaluation engine
    let vault = InMemoryClient::with_schema(r#"
        entity User {}
        entity Folder {
            relations { owner: User }
            permissions { view: owner, delete: owner }
        }
        entity Document {
            relations { parent: Folder }
            permissions {
                view: parent.view,
                delete: parent.delete
            }
        }
    "#);

    // Seed relationships
    vault.relationships().write_batch([
        Relationship::new("folder:docs", "owner", "user:alice"),
        Relationship::new("doc:readme", "parent", "folder:docs"),
    ]).await.unwrap();

    // Test inheritance works correctly
    assert!(vault.check("user:alice", "view", "doc:readme").await.unwrap());
    assert!(vault.check("user:alice", "delete", "doc:readme").await.unwrap());
    assert!(!vault.check("user:bob", "view", "doc:readme").await.unwrap());
}
```

### TestVault for E2E Tests

For tests against a real InferaDB instance:

```rust
use inferadb::testing::{TestVault, TestConfig, test_client};

#[tokio::test]
#[ignore]  // Requires running InferaDB
async fn integration_test() {
    let config = TestConfig::new("http://localhost:8080", "test-token")
        .with_organization_id("org_test...");
    let client = test_client(config).await.unwrap();
    let org = client.organization("org_test...");
    let vault = TestVault::create(&org).await.unwrap();

    // Tests run in isolated vault
    vault.relationships().write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());

    // Vault cleaned up on drop
}
```

### Decision Trace Snapshot Testing

Use `assert_decision_trace!` to catch regressions in permission evaluation logic:

```rust
use inferadb::testing::{InMemoryClient, assert_decision_trace};

#[tokio::test]
async fn test_view_permission_trace() {
    let vault = InMemoryClient::with_schema(include_str!("schema.ipl"));
    seed_test_data(&vault).await;

    // Snapshot the decision trace - fails if logic changes
    assert_decision_trace!(
        vault,
        subject: "user:alice",
        permission: "view",
        resource: "doc:readme",
        snapshot: "view_permission_alice_readme"  // Saved to tests/snapshots/
    );
}
```

**Snapshot file format** (`tests/snapshots/view_permission_alice_readme.json`):

```json
{
  "allowed": true,
  "decision_path": [
    { "step": "check", "entity": "doc:readme", "permission": "view" },
    { "step": "expand", "relation": "parent", "target": "folder:docs" },
    { "step": "check", "entity": "folder:docs", "permission": "view" },
    { "step": "expand", "relation": "owner", "target": "user:alice" },
    { "step": "match", "subject": "user:alice" }
  ],
  "rules_evaluated": ["doc:view → parent.view", "folder:view → owner"]
}
```

**Updating snapshots:**

```bash
# Review and update all snapshots
INFERADB_UPDATE_SNAPSHOTS=1 cargo test

# Update specific snapshot
INFERADB_UPDATE_SNAPSHOTS=view_permission_alice_readme cargo test
```

### Simulation + Snapshot for What-If Testing

Combine `simulate()` with snapshots to test schema changes:

```rust
use inferadb::testing::{InMemoryClient, SimulationSnapshot};

#[tokio::test]
async fn test_schema_migration_preserves_access() {
    let vault = InMemoryClient::with_schema(include_str!("schema_v1.ipl"));
    seed_production_data(&vault).await;

    // Capture current behavior as baseline
    let baseline = SimulationSnapshot::capture(&vault, &[
        ("user:alice", "view", "doc:1"),
        ("user:alice", "edit", "doc:1"),
        ("user:bob", "view", "doc:1"),
        ("team:engineering#member", "view", "repo:backend"),
    ]).await;

    // Simulate with new schema
    let new_schema = include_str!("schema_v2.ipl");
    let results = vault.simulate()
        .with_schema(new_schema)
        .checks(&baseline.checks())
        .execute()
        .await?;

    // Assert no permission changes (or expected changes only)
    baseline.assert_unchanged(&results);
    // Or: baseline.assert_diff(&results, expected_changes);
}
```

### AuthorizationClient Trait

All client types implement a common trait for dependency injection:

```rust
use futures::stream::BoxStream;

#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    async fn check_batch(&self, checks: Vec<(&str, &str, &str)>) -> Result<Vec<bool>, Error>;

    async fn write(&self, relationship: Relationship) -> Result<(), Error>;
    async fn write_batch(&self, relationships: Vec<Relationship>) -> Result<(), Error>;
    async fn delete(&self, relationship: Relationship) -> Result<(), Error>;

    fn simulate(&self) -> SimulateBuilder;
    async fn explain(&self, subject: &str, permission: &str, resource: &str) -> Result<PermissionExplanation, Error>;
    // ... additional methods
}

// Implemented by all client types:
impl AuthorizationClient for VaultClient { /* production */ }
impl AuthorizationClient for MockClient { /* unit tests */ }
impl AuthorizationClient for InMemoryClient { /* integration tests */ }
```

### Testing Type Definitions

````rust
/// Mock client for unit testing.
/// Allows stubbing check results and verifying calls.
pub struct MockClient {
    expectations: Arc<Mutex<MockExpectations>>,
    verify_on_drop: bool,
}

impl MockClient {
    /// Create a builder for configuring the mock.
    pub fn builder() -> MockClientBuilder {
        MockClientBuilder::new()
    }
}

/// Builder for configuring MockClient expectations.
pub struct MockClientBuilder {
    check_expectations: Vec<CheckExpectation>,
    check_patterns: Vec<PatternExpectation>,
    write_expectations: Vec<WriteExpectation>,
    verify_on_drop: bool,
}

impl MockClientBuilder {
    fn new() -> Self;

    /// Stub a specific check result.
    pub fn check(
        mut self,
        subject: &str,
        permission: &str,
        resource: &str,
        result: bool,
    ) -> Self;

    /// Stub checks matching a pattern (* as wildcard).
    pub fn check_pattern(
        mut self,
        subject: &str,
        permission: &str,
        resource: &str,
        result: bool,
    ) -> Self;

    /// Stub a check to return an error.
    pub fn check_error(
        mut self,
        subject: &str,
        permission: &str,
        resource: &str,
        error: Error,
    ) -> Self;

    /// Stub a successful write operation.
    pub fn write_ok(mut self, relationship: Relationship<'static>) -> Self;

    /// Stub a write operation to return an error.
    pub fn write_error(
        mut self,
        relationship: Relationship<'static>,
        error: Error,
    ) -> Self;

    /// Enable verification that all expectations were consumed on drop.
    pub fn verify_on_drop(mut self, verify: bool) -> Self;

    /// Build the configured MockClient.
    pub fn build(self) -> MockClient;
}

/// Internal state tracking mock expectations and call history.
/// Used by MockClient to match calls against stubbed responses.
struct MockExpectations {
    check_expectations: Vec<CheckExpectation>,
    check_patterns: Vec<PatternExpectation>,
    write_expectations: Vec<WriteExpectation>,
    call_history: Vec<MockCall>,
}

/// A single check expectation (exact match).
struct CheckExpectation {
    subject: String,
    permission: String,
    resource: String,
    result: Result<bool, Error>,
    times: ExpectedCalls,
    actual_calls: usize,
}

/// A pattern-based check expectation (supports wildcards).
struct PatternExpectation {
    subject_pattern: String,
    permission_pattern: String,
    resource_pattern: String,
    result: bool,
}

/// A write operation expectation.
struct WriteExpectation {
    relationship: Relationship<'static>,
    result: Result<(), Error>,
}

/// Expected number of calls for an expectation.
enum ExpectedCalls {
    Exact(usize),
    AtLeast(usize),
    AtMost(usize),
    Any,
}

/// A recorded call to the mock client.
#[derive(Debug, Clone)]
pub struct MockCall {
    pub operation: MockOperation,
    pub timestamp: Instant,
}

/// The type of operation performed on the mock.
#[derive(Debug, Clone)]
pub enum MockOperation {
    Check { subject: String, permission: String, resource: String },
    Write { relationship: Relationship<'static> },
    Delete { relationship: Relationship<'static> },
}

/// In-memory client with full authorization engine.
/// Provides real permission evaluation without network I/O.
pub struct InMemoryClient {
    engine: AuthorizationEngine,
    relationships: InMemoryStore,
}

impl InMemoryClient {
    /// Create an InMemoryClient with the given IPL schema.
    pub fn with_schema(schema: &str) -> Self;

    /// Create an InMemoryClient from a schema and initial relationships.
    pub fn with_schema_and_data(
        schema: &str,
        relationships: Vec<Relationship<'static>>,
    ) -> Self;

    /// Access the relationship store for seeding test data.
    pub fn relationships(&self) -> &RelationshipsClient;
}

/// Test vault connected to a real InferaDB instance.
/// Creates an isolated vault that is automatically cleaned up on drop.
pub struct TestVault {
    client: VaultClient,
    vault_id: VaultId,
    cleanup_on_drop: bool,
}

impl TestVault {
    /// Create a new test vault in the given organization.
    pub async fn create(org: &OrganizationClient) -> Result<Self, Error>;

    /// Create with a specific schema.
    pub async fn create_with_schema(
        org: &OrganizationClient,
        schema: &str,
    ) -> Result<Self, Error>;

    /// Prevent cleanup on drop (for debugging failed tests).
    pub fn preserve(mut self) -> Self;
}

impl Deref for TestVault {
    type Target = VaultClient;
    fn deref(&self) -> &Self::Target { &self.client }
}

impl Drop for TestVault {
    fn drop(&mut self) {
        // Spawns cleanup task if cleanup_on_drop is true
    }
}

/// Configuration for integration test clients.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// URL of the test server
    pub url: String,
    /// Authentication token for test requests
    pub token: String,
    /// Optional organization ID to scope tests to
    pub organization_id: Option<String>,
    /// Optional vault ID to scope tests to
    pub vault_id: Option<String>,
}

impl TestConfig {
    pub fn new(url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            token: token.into(),
            organization_id: None,
            vault_id: None,
        }
    }

    pub fn organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    pub fn vault(mut self, vault_id: impl Into<String>) -> Self {
        self.vault_id = Some(vault_id.into());
        self
    }
}

/// Create a client configured for integration testing.
///
/// # Example
/// ```rust
/// let client = test_client(TestConfig::new(
///     "http://localhost:8080",
///     "test-token",
/// )).await?;
/// ```
pub async fn test_client(config: TestConfig) -> Result<Client, Error>;

/// Captures authorization decisions for snapshot comparison.
pub struct SimulationSnapshot {
    checks: Vec<(String, String, String)>,
    results: Vec<SimulationResult>,
}

impl SimulationSnapshot {
    /// Capture current authorization state for a set of checks.
    pub async fn capture(
        client: &impl AuthorizationClient,
        checks: &[(&str, &str, &str)],
    ) -> Self;

    /// Get the checks used in this snapshot.
    pub fn checks(&self) -> Vec<(&str, &str, &str)>;

    /// Assert that results match exactly.
    pub fn assert_unchanged(&self, other: &[SimulationResult]);

    /// Assert that only specified changes occurred.
    pub fn assert_diff(&self, other: &[SimulationResult], expected: &[SnapshotDiff]);
}

/// Individual result in a simulation snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub allowed: bool,
}

/// Expected change in a snapshot diff.
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub was_allowed: bool,
    pub now_allowed: bool,
}

/// Assert that a decision trace matches a saved snapshot.
///
/// # Example
/// ```rust
/// assert_decision_trace!(
///     vault,
///     subject: "user:alice",
///     permission: "view",
///     resource: "doc:readme",
///     snapshot: "view_permission_alice_readme"
/// );
/// ```
#[macro_export]
macro_rules! assert_decision_trace {
    ($client:expr, subject: $subj:expr, permission: $perm:expr, resource: $res:expr, snapshot: $name:expr) => {
        // Expands to:
        // 1. Execute explain() to get decision trace
        // 2. Load snapshot from tests/snapshots/{$name}.json
        // 3. Compare and assert equality
        // 4. If INFERADB_UPDATE_SNAPSHOTS is set, update snapshot file
    };
}
````

For comprehensive testing patterns including the AuthzTest DSL and scenario testing, see [Testing Guide](docs/guides/testing.md).

---

## Integration Patterns

For comprehensive framework integration patterns, see [Integration Patterns Guide](docs/guides/integration-patterns.md), which covers:

- **Framework Extractors** - Axum/Actix permission extractors and attribute macros
- **Authorization Patterns** - Hero pattern (`require()`), convenience helpers (`then()`, `filter_authorized()`)
- **Permission Aggregation** - `check_all()`, `check_any()`, `PermissionMatrix`
- **Structured Audit Context** - Request-scoped audit logging

---

## Caching

For caching strategies and invalidation patterns, see [Caching Guide](docs/guides/caching.md), which covers:

- Time-based, event-driven, and hierarchical invalidation
- Cache warming and metrics
- Configuration options

Basic cache configuration via client builder:

```rust
use inferadb::{Client, CacheConfig};
use std::time::Duration;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_secs(300))   // 5 minutes
        .schema_ttl(Duration::from_secs(3600)))       // 1 hour
    .build()
    .await?;
```

---

## Type System

### Type-Safe Relationships with Derive Macros

```rust
use inferadb::derive::{Resource, Subject};

#[derive(Resource)]
#[resource(type = "document")]
struct Document {
    #[resource(id)]
    id: String,
}

#[derive(Subject)]
#[subject(type = "user")]
struct User {
    #[subject(id)]
    id: String,
}

// Type-safe API
let doc = Document { id: "readme".into() };
let user = User { id: "alice".into() };

vault.check(&user, "view", &doc).await?;
vault.relationships().write(Relationship::typed(&doc, "viewer", &user)).await?;
```

### Generic Type Constraints

```rust
impl VaultClient {
    pub async fn check<S, R>(&self, subject: S, permission: &str, resource: R) -> Result<bool, Error>
    where
        S: Into<SubjectRef>,
        R: Into<ResourceRef>,
    {
        let subject = subject.into();
        let resource = resource.into();
        // ...
    }
}

// Accepts strings
vault.check("user:alice", "view", "doc:1").await?;

// Accepts typed values
vault.check(&user, "view", &doc).await?;

// Accepts references
vault.check(user.as_subject(), "view", doc.as_resource()).await?;
```

---

## Protocol Support

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
└─► Otherwise: Use gRPC (default)
```

### Transport Selection

```rust
/// Available transport implementations
pub enum Transport {
    /// gRPC over HTTP/2 (default) - best performance, streaming support
    Grpc,
    /// REST over HTTP/1.1 - universal compatibility, firewall-friendly
    Http,
    /// In-memory mock - for testing without network
    Mock,
}

// Use gRPC (default, best performance)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .transport(Transport::Grpc)
    .build()
    .await?;

// Use HTTP/REST (firewall-friendly, browser-compatible)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .transport(Transport::Http)
    .build()
    .await?;

// Use mock transport (for testing)
let client = Client::builder()
    .transport(Transport::Mock)
    .build()
    .await?;
```

### Transport Fallback Behavior

When both `grpc` and `rest` features are enabled (default), the SDK automatically handles transport fallback:

```rust
/// Transport fallback configuration
pub enum TransportStrategy {
    /// Use gRPC only, fail if unavailable
    GrpcOnly,
    /// Use REST only
    RestOnly,
    /// Prefer gRPC, automatically fall back to REST on failure (default)
    PreferGrpc { fallback_on: FallbackTrigger },
    /// Prefer REST, automatically fall back to gRPC on failure
    PreferRest { fallback_on: FallbackTrigger },
}

/// Conditions that trigger transport fallback
#[derive(Debug, Clone)]
pub struct FallbackTrigger {
    /// Connection failures (TCP, TLS handshake)
    pub connection_error: bool,
    /// HTTP/2 protocol errors (gRPC requires HTTP/2)
    pub protocol_error: bool,
    /// Specific HTTP status codes (e.g., 502 Bad Gateway from proxy)
    pub status_codes: Vec<u16>,
    /// Timeout on initial connection
    pub connect_timeout: bool,
}

impl Default for FallbackTrigger {
    fn default() -> Self {
        Self {
            connection_error: true,
            protocol_error: true,
            status_codes: vec![502, 503],
            connect_timeout: true,
        }
    }
}
```

**Configuration Examples**:

```rust
// Default: prefer gRPC, fall back to REST on connection issues
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .transport_strategy(TransportStrategy::PreferGrpc {
        fallback_on: FallbackTrigger::default(),
    })
    .build()
    .await?;

// Disable automatic fallback (fail fast)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .transport_strategy(TransportStrategy::GrpcOnly)
    .build()
    .await?;

// Prefer REST for firewall-restricted environments
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .transport_strategy(TransportStrategy::PreferRest {
        fallback_on: FallbackTrigger {
            connection_error: true,
            protocol_error: false,  // Don't fallback on protocol errors
            status_codes: vec![],
            connect_timeout: true,
        },
    })
    .build()
    .await?;
```

**Fallback Behavior**:

| Scenario                | Default Behavior                           |
| ----------------------- | ------------------------------------------ |
| gRPC connection refused | Automatically retry with REST              |
| HTTP/2 not supported    | Automatically retry with REST (HTTP/1.1)   |
| 502/503 from proxy      | Automatically retry with REST              |
| gRPC timeout            | Retry with REST if `connect_timeout: true` |
| gRPC auth failure (401) | No fallback (not a transport issue)        |
| gRPC rate limit (429)   | No fallback (not a transport issue)        |
| REST connection refused | Fail (no further fallback)                 |

**Observability**:

```rust
// Check which transport is active
let stats = client.transport_stats();
println!("Active transport: {:?}", stats.active_transport);
println!("Fallback count: {}", stats.fallback_count);
println!("Last fallback reason: {:?}", stats.last_fallback_reason);

// Subscribe to transport events
let mut events = client.transport_events();
tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            TransportEvent::FallbackTriggered { from, to, reason } => {
                tracing::warn!(
                    "Transport fallback: {:?} -> {:?}, reason: {:?}",
                    from, to, reason
                );
            }
            TransportEvent::Restored { transport } => {
                tracing::info!("Transport restored: {:?}", transport);
            }
        }
    }
});
```

**Transport Stats Types**:

```rust
/// Transport layer statistics
#[derive(Debug, Clone)]
pub struct TransportStats {
    /// Currently active transport
    pub active_transport: Transport,
    /// Number of times fallback was triggered
    pub fallback_count: u64,
    /// Reason for the last fallback (if any)
    pub last_fallback_reason: Option<FallbackReason>,
    /// Timestamp of last fallback
    pub last_fallback_at: Option<Instant>,
    /// gRPC-specific stats (if gRPC enabled)
    pub grpc: Option<GrpcStats>,
    /// REST-specific stats (if REST enabled)
    pub rest: Option<RestStats>,
}

#[derive(Debug, Clone)]
pub enum FallbackReason {
    ConnectionRefused,
    ProtocolError(String),
    StatusCode(u16),
    ConnectTimeout,
}

#[derive(Debug, Clone)]
pub struct GrpcStats {
    pub requests_sent: u64,
    pub requests_failed: u64,
    pub streams_opened: u64,
    pub streams_active: u32,
}

#[derive(Debug, Clone)]
pub struct RestStats {
    pub requests_sent: u64,
    pub requests_failed: u64,
    pub sse_connections: u64,
    pub sse_active: u32,
}

/// Transport events for monitoring
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// Transport switched due to fallback
    FallbackTriggered {
        from: Transport,
        to: Transport,
        reason: FallbackReason,
    },
    /// Original transport restored
    Restored {
        transport: Transport,
    },
}
```

### Streaming Behavior

| Operation                                       | gRPC                       | REST                    |
| ----------------------------------------------- | -------------------------- | ----------------------- |
| `check()`                                       | Unary                      | POST                    |
| `check_batch()`                                 | Bidirectional stream       | SSE stream              |
| `resources().accessible_by().with_permission()` | Server stream              | SSE stream              |
| `subjects().with_permission().on_resource()`    | Server stream              | SSE stream              |
| `relationships().list()`                        | Server stream              | SSE stream              |
| `watch()`                                       | Server stream (continuous) | SSE stream (continuous) |
| `relationships().write_batch()`                 | Client stream              | POST                    |

### Transport Escape Hatches

For advanced scenarios requiring custom transport configuration, the SDK exposes escape hatches to the underlying HTTP clients:

**Escape Hatch Methods**:

```rust
impl<Url, Auth> ClientBuilder<Url, Auth> {
    /// Use a custom reqwest Client for REST transport.
    /// When set, the SDK will use this client instead of creating its own.
    /// Note: SDK retry/timeout configuration will not apply to custom clients.
    #[cfg(feature = "rest")]
    pub fn with_http_client(self, client: reqwest::Client) -> Self {
        Self { http_client: Some(client), ..self }
    }

    /// Use a custom tonic Channel for gRPC transport.
    /// When set, the SDK will use this channel instead of creating its own.
    /// Note: SDK retry/timeout configuration will not apply to custom channels.
    #[cfg(feature = "grpc")]
    pub fn with_grpc_channel(self, channel: tonic::transport::Channel) -> Self {
        Self { grpc_channel: Some(channel), ..self }
    }

    /// Add custom middleware to the request pipeline.
    pub fn middleware(self, middleware: impl Middleware) -> Self {
        Self { middlewares: self.middlewares.push(Box::new(middleware)), ..self }
    }
}
```

**Custom reqwest Client (REST)**:

```rust
use reqwest::Client as ReqwestClient;

// Build your own reqwest client with custom configuration
let http_client = ReqwestClient::builder()
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .proxy(reqwest::Proxy::https("http://proxy.corp.internal:3128")?)
    .danger_accept_invalid_certs(true)  // For testing only!
    .build()?;

// Use it with the SDK
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .with_http_client(http_client)  // Escape hatch
    .build()
    .await?;
```

**Custom tonic Channel (gRPC)**:

```rust
use tonic::transport::{Channel, Endpoint, ClientTlsConfig};

// Build your own tonic channel with custom configuration
let tls = ClientTlsConfig::new()
    .ca_certificate(Certificate::from_pem(include_bytes!("ca.pem")))
    .domain_name("api.inferadb.com");

let channel = Endpoint::from_static("https://api.inferadb.com")
    .tls_config(tls)?
    .timeout(Duration::from_secs(60))
    .connect_timeout(Duration::from_secs(10))
    .tcp_keepalive(Some(Duration::from_secs(30)))
    .http2_keep_alive_interval(Duration::from_secs(30))
    .http2_keep_alive_timeout(Duration::from_secs(10))
    .connect()
    .await?;

// Use it with the SDK
let client = Client::builder()
    .credentials(creds)
    .with_grpc_channel(channel)  // Escape hatch
    .build()
    .await?;
```

**When to Use Escape Hatches**:

| Scenario                         | Solution                              |
| -------------------------------- | ------------------------------------- |
| Corporate proxy                  | Custom reqwest with `.proxy()`        |
| mTLS/client certificates         | Custom tonic with `ClientTlsConfig`   |
| Custom DNS resolution            | Custom reqwest with `.resolve()`      |
| H2C (plaintext HTTP/2)           | Custom tonic endpoint                 |
| Connection through load balancer | Custom channel with specific settings |
| Testing with invalid certs       | Custom client (never in production!)  |

**mTLS (Mutual TLS) Example**:

For environments requiring client certificate authentication:

```rust
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

// Load certificates
let ca_cert = Certificate::from_pem(std::fs::read("ca.pem")?);
let client_cert = std::fs::read("client.pem")?;
let client_key = std::fs::read("client-key.pem")?;
let client_identity = Identity::from_pem(&client_cert, &client_key);

// Configure mTLS
let tls = ClientTlsConfig::new()
    .ca_certificate(ca_cert)
    .identity(client_identity)
    .domain_name("api.inferadb.com");

// Build gRPC channel with mTLS
let channel = Channel::from_static("https://api.inferadb.com:443")
    .tls_config(tls)?
    .connect()
    .await?;

// Use with SDK
let client = Client::builder()
    .credentials(ClientCredentialsConfig {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("jwt-key.pem")?,
        certificate_id: Some("jwt-key-2024".into()),
    })
    .with_grpc_channel(channel)  // mTLS channel
    .build()
    .await?;
```

**REST with mTLS**:

```rust
use reqwest::{Certificate, Identity};

// Load certificates
let ca_cert = Certificate::from_pem(&std::fs::read("ca.pem")?)?;
let client_cert = std::fs::read("client.pem")?;
let client_key = std::fs::read("client-key.pem")?;
let identity = Identity::from_pem(&[client_cert, client_key].concat())?;

// Build HTTP client with mTLS
let http_client = reqwest::Client::builder()
    .add_root_certificate(ca_cert)
    .identity(identity)
    .build()?;

// Use with SDK
let client = Client::builder()
    .credentials(creds)
    .with_http_client(http_client)  // mTLS client
    .build()
    .await?;
```

**mTLS vs SDK Authentication**:

| Layer            | Purpose                                    |
| ---------------- | ------------------------------------------ |
| mTLS (transport) | Authenticates the _connection_ to InferaDB |
| JWT (SDK auth)   | Authenticates the _application_ identity   |

Both can be used together: mTLS ensures only authorized hosts can connect, while JWT identifies which application is making requests.

**Caveats**:

- When using escape hatches, the SDK cannot apply automatic retries at transport level
- You are responsible for TLS configuration
- Some SDK features (like protocol auto-detection) may not work

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

# Platform support
wasm = ["rest", "getrandom/js", "chrono/wasmbind"]  # Browser/WASM (REST only)

# Optional features
tracing = ["tracing", "tracing-futures"]
metrics = ["metrics", "metrics-exporter-prometheus"]
test-utils = ["inferadb-test"]
opentelemetry = ["opentelemetry", "tracing-opentelemetry"]
blocking = ["tokio/rt"]
derive = ["inferadb-macros"]
serde = ["dep:serde", "chrono/serde", "uuid/serde"]

# Development/testing only - NEVER use in production
insecure = []  # Enables TlsConfig::insecure() and Client::insecure()
```

### Feature Interaction Matrix

| Combination           | Valid | Notes                                      |
| --------------------- | ----- | ------------------------------------------ |
| `grpc` + `rest`       | ✅    | Default - prefers gRPC, falls back to REST |
| `grpc` only           | ✅    | gRPC exclusive                             |
| `rest` only           | ✅    | REST exclusive - smaller binary            |
| Neither               | ❌    | Compile error                              |
| `rustls`              | ✅    | Recommended - pure Rust                    |
| `native-tls`          | ✅    | System TLS                                 |
| `wasm` + `grpc`       | ❌    | gRPC not supported in browsers             |
| `wasm` + `native-tls` | ❌    | No system TLS in WASM                      |
| `blocking` + `grpc`   | ✅    | Blocking with gRPC                         |

### Compile-Time Feature Validation

The SDK uses compile-time checks to catch invalid feature combinations before building:

```rust
// In src/lib.rs - compile-time feature validation
#[cfg(all(not(feature = "grpc"), not(feature = "rest")))]
compile_error!(
    "At least one transport must be enabled. Add `grpc` or `rest` feature:\n\
     inferadb = { version = \"0.1\", features = [\"rest\"] }"
);

#[cfg(all(feature = "wasm", feature = "grpc"))]
compile_error!(
    "gRPC is not supported in WASM environments. Use REST:\n\
     inferadb = { version = \"0.1\", features = [\"rest\", \"wasm\"] }"
);

#[cfg(all(feature = "wasm", feature = "native-tls"))]
compile_error!(
    "native-tls is not available in WASM. Browser provides TLS:\n\
     inferadb = { version = \"0.1\", features = [\"rest\", \"wasm\"] }"
);

#[cfg(all(feature = "wasm", feature = "blocking"))]
compile_error!(
    "Blocking APIs are not available in WASM (single-threaded):\n\
     Remove the `blocking` feature for WASM builds."
);

#[cfg(all(feature = "rustls", feature = "native-tls"))]
compile_error!(
    "Cannot enable both `rustls` and `native-tls`. Choose one:\n\
     - `rustls`: Pure Rust, recommended for most use cases\n\
     - `native-tls`: System TLS, may be required for corporate environments"
);
```

**Result**: Users get clear error messages at compile time if they configure incompatible features:

```text
error: At least one transport must be enabled. Add `grpc` or `rest` feature:
       inferadb = { version = "0.1", features = ["rest"] }
  --> src/lib.rs:15:1
   |
15 | compile_error!(
   | ^^^^^^^^^^^^^^^
```

### Feature Documentation in IDE

The crate uses `#[cfg_attr]` to provide feature-aware documentation:

````rust
/// Check permission (returns boolean result).
///
/// # Transport Support
/// - **gRPC**: Full support with streaming
/// - **REST**: Full support via POST `/check`
///
/// # Example
/// ```rust
#[cfg_attr(feature = "grpc", doc = "// Using gRPC (default, fastest)")]
#[cfg_attr(feature = "rest", doc = "// Using REST (when gRPC unavailable)")]
/// let allowed = vault.check("user:alice", "view", "doc:1").await?;
/// ```
pub async fn check(...) -> Result<bool, Error> { ... }
````

---

## WASM / Browser Usage

The SDK supports WebAssembly (WASM) for browser and edge runtime environments with some constraints.

### WASM Feature Configuration

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "wasm"] }
```

### WASM Constraints

| Feature            | Browser/WASM | Native      | Notes                             |
| ------------------ | ------------ | ----------- | --------------------------------- |
| gRPC transport     | ❌           | ✅          | gRPC requires HTTP/2 trailers     |
| REST transport     | ✅           | ✅          | Full support via fetch API        |
| TLS                | ✅ (browser) | ✅ (rustls) | Browser handles TLS automatically |
| Connection pooling | Limited      | Full        | Browser manages connections       |
| File system access | ❌           | ✅          | No credential files in browser    |
| Environment vars   | ❌           | ✅          | Use explicit configuration        |
| Blocking APIs      | ❌           | ✅          | WASM is single-threaded           |

### Browser Client Configuration

```rust
// Browser/WASM client configuration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(Credentials::api_key("key_..."))  // Explicit credentials
    .transport(Transport::Rest)  // REST only in browser
    .build()
    .await?;
```

### Credential Handling in Browser

In WASM environments, credentials typically come from JavaScript interop or OAuth flows:

```rust
// Credentials from JavaScript (e.g., session storage, JS config)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: get_token_from_js(),
    })
    .build()
    .await?;

// OAuth flow with browser redirect
let oauth = OAuthFlow::browser()
    .client_id("client_...")
    .redirect_uri("https://myapp.com/callback")
    .build();

// Returns URL to redirect user to
let auth_url = oauth.authorization_url();
```

### Edge Runtime Compatibility

For Cloudflare Workers, Deno Deploy, and similar edge runtimes:

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "wasm"] }
```

```rust
// Edge runtime configuration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(Credentials::api_key(env.secret("INFERADB_API_KEY")?))
    .build()
    .await?;
```

### WASM Binary Size

| Configuration              | Approximate Size |
| -------------------------- | ---------------- |
| `rest` + `wasm`            | ~400 KB          |
| `rest` + `wasm` + `serde`  | ~500 KB          |
| Full default (native only) | N/A              |

Use `wasm-opt` and `wasm-pack` for production builds to minimize size.

---

## Stability Policy

This section defines API stability guarantees for SDK consumers.

### Stability Tiers

| Tier         | Description            | Guarantees                                                |
| ------------ | ---------------------- | --------------------------------------------------------- |
| **Stable**   | Core public API        | Semver-protected; breaking changes only in major versions |
| **Unstable** | Experimental features  | May change in minor versions; behind feature flags        |
| **Internal** | Implementation details | No stability guarantees; `#[doc(hidden)]`                 |

### Stable API Surface

The following are considered **stable** and follow semantic versioning:

| Category            | Examples                                                      |
| ------------------- | ------------------------------------------------------------- |
| **Public types**    | `Client`, `VaultClient`, `Error`, `ErrorKind`, `Relationship` |
| **Public traits**   | `AuthorizationClient`, `Resource`, `Subject`                  |
| **Builder methods** | `.check()`, `.require()`, `.with_context()`, `.timeout()`     |
| **Error variants**  | All `ErrorKind` variants and their semantics                  |
| **Feature flags**   | `grpc`, `rest`, `rustls`, `native-tls`, `tracing`, `metrics`  |

### Unstable API Surface

The following are behind feature flags and may change in minor versions:

| Feature Flag         | API Surface                                 | Notes                            |
| -------------------- | ------------------------------------------- | -------------------------------- |
| `derive`             | `#[derive(Resource)]`, `#[derive(Subject)]` | Schema codegen macros            |
| `experimental-cache` | `CacheConfig::adaptive()`                   | Experimental cache strategies    |
| `internal-testing`   | `InMemoryClient` internals                  | Test utilities subject to change |

### What Constitutes a Breaking Change

**Breaking (requires major version bump):**

- Removing or renaming public types, traits, or methods
- Changing method signatures (parameters, return types)
- Adding required parameters to existing methods
- Changing the behavior of `ErrorKind` variants
- Removing or renaming feature flags

**Non-breaking (allowed in minor versions):**

- Adding new public types, traits, or methods
- Adding new `ErrorKind` variants
- Adding optional parameters with defaults
- Adding new feature flags
- Performance improvements
- Bug fixes that don't change documented behavior

### Deprecation Policy

1. **Deprecation notice**: At least one minor version before removal
2. **Compile-time warning**: `#[deprecated]` attribute with migration guidance
3. **Documentation**: Migration path documented in MIGRATION.md
4. **Removal**: Only in next major version

````rust
/// **Deprecated since 0.3.0**: Use `on_check_failure()` instead.
///
/// # Migration
/// ```rust
/// // Before (0.2.x):
/// .failure_mode(FailureMode::Deny)
///
/// // After (0.3.0+):
/// .on_check_failure(FailureMode::FailClosed)
/// ```
#[deprecated(since = "0.3.0", note = "Use on_check_failure() instead")]
pub fn failure_mode(self, mode: FailureMode) -> Self { ... }
````

### MSRV (Minimum Supported Rust Version)

| SDK Version | MSRV | Notes                     |
| ----------- | ---- | ------------------------- |
| 0.1.x       | 1.70 | Initial release           |
| 0.2.x       | 1.75 | Async trait stabilization |

MSRV bumps are considered **non-breaking** but will be documented in CHANGELOG.md.

---

## Safety Guarantees

### Panic Safety

| Operation             | Panics? | Notes                         |
| --------------------- | ------- | ----------------------------- |
| All public methods    | No      | Return `Result<T, Error>`     |
| Internal parsing      | No      | Invalid input → Error variant |
| Arithmetic            | No      | Uses checked/saturating ops   |
| Index access          | No      | Uses `.get()` not `[]`        |
| `unwrap()`/`expect()` | No      | Never used on fallible ops    |

### Unsafe Code Audit

| Location      | Reason           | Justification          |
| ------------- | ---------------- | ---------------------- |
| `tonic` (dep) | FFI for gRPC     | Well-audited           |
| `hyper` (dep) | HTTP performance | Well-audited           |
| `ring` (dep)  | Cryptography     | Audited crypto library |
| SDK itself    | **None**         | Pure safe Rust         |

### Concurrency Safety

| Type          | `Send` | `Sync` | Notes                        |
| ------------- | ------ | ------ | ---------------------------- |
| `Client`      | ✅     | ✅     | Safe to share across threads |
| `Error`       | ✅     | ✅     | Can be sent between threads  |
| `Decision`    | ✅     | ✅     | Immutable after creation     |
| `WatchStream` | ✅     | ❌     | Single consumer only         |
| `MockClient`  | ✅     | ✅     | Safe for parallel tests      |

### Must-Use Annotations

All builder methods that return `Self` are marked `#[must_use]` to prevent accidental drops:

```rust
impl<'a> CheckRequest<'a> {
    /// Enable decision trace for debugging
    #[must_use]
    pub fn trace(mut self, enabled: bool) -> Self {
        self.trace = enabled;
        self
    }

    /// Add ABAC context to the check
    #[must_use]
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Override timeout for this operation
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
```

**Compiler Warning on Unused Builder**:

```rust
// ⚠️ Warning: unused `CheckRequest` that must be used
vault.check("user:alice", "view", "doc:1")
    .trace(true);  // Warning! Builder not awaited

// ✅ Correct: builder is awaited
vault.check("user:alice", "view", "doc:1")
    .trace(true)
    .await?;
```

**Types with `#[must_use]`**:

| Type                       | Reason                                  |
| -------------------------- | --------------------------------------- |
| `CheckRequest`             | Must be awaited to execute              |
| `WriteRelationshipBuilder` | Must be awaited to execute              |
| `DeleteWhereBuilder`       | Must call `.execute()` or `.dry_run()`  |
| `RelationshipsListBuilder` | Must call `.stream()` or `.collect()`   |
| `ClientBuilder`            | Must call `.build()`                    |
| `Result<T, Error>`         | Standard Rust - results must be handled |

**Types WITHOUT `#[must_use]`**:

Configuration structs like `RetryConfig`, `CacheConfig`, and `FallbackTrigger` intentionally do **not** have `#[must_use]`. This allows users to create configs in advance without warnings:

```rust
// ✅ No warning - configs can be created and stored for later use
fn get_retry_config() -> RetryConfig {
    RetryConfig::default()
        .max_retries(5)
        .initial_backoff(Duration::from_millis(100))
}

// Later...
let client = Client::builder()
    .url("https://api.inferadb.com")
    .retry(get_retry_config())
    .build()
    .await?;
```

The compile-time safety comes from the builder methods themselves, not the config structs.

---

## Release Strategy

### Version Policy

- **Major** (1.0 → 2.0): Breaking API changes
- **Minor** (1.0 → 1.1): New features, backward compatible
- **Patch** (1.0.0 → 1.0.1): Bug fixes only

### Pre-1.0 Policy

During 0.x development:

- Minor version bumps may include breaking changes
- Document all breaking changes in CHANGELOG.md
- Provide migration guides in MIGRATION.md

### Supported Rust Versions

- **MSRV**: Rust 1.70+
- Tested on: stable, beta, nightly
- No nightly-only features in default build

### Binary Size Targets

| Configuration         | Target Size |
| --------------------- | ----------- |
| Default (gRPC + REST) | < 8MB       |
| REST only             | < 4MB       |
| Minimal               | < 2MB       |

---

## Security Considerations

### Key Management

- Never log private keys or tokens
- Support key rotation without client restart
- Clear sensitive data from memory when done

### Sensitive Memory Handling (zeroize)

Private keys and tokens are cleared from memory when dropped using the `zeroize` crate:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};
use secrecy::{Secret, ExposeSecret};

/// Error type for private key operations
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// Key file not found at the specified path
    #[error("key file not found: {0}")]
    FileNotFound(std::path::PathBuf),

    /// Failed to read key file
    #[error("failed to read key file: {0}")]
    ReadError(#[source] std::io::Error),

    /// Invalid PEM format
    #[error("invalid PEM format: {0}")]
    InvalidPem(String),

    /// Unsupported key algorithm (only Ed25519 and RSA supported)
    #[error("unsupported key algorithm: {0}")]
    UnsupportedAlgorithm(String),

    /// Key material is malformed
    #[error("malformed key: {0}")]
    Malformed(String),
}

/// Private key wrapper that zeroizes on drop
#[derive(ZeroizeOnDrop)]
pub struct Ed25519PrivateKey {
    // Inner bytes are zeroized when struct is dropped
    inner: [u8; 32],
}

impl Ed25519PrivateKey {
    /// Parse a private key from PEM-encoded bytes
    pub fn from_pem(pem: &[u8]) -> Result<Self, KeyError> {
        // Parse PEM, copy into zeroizing container
        let bytes = parse_ed25519_private_key(pem)?;
        Ok(Self { inner: bytes })
    }

    /// Load a private key from a PEM file
    pub fn from_pem_file(path: impl AsRef<std::path::Path>) -> Result<Self, KeyError> {
        let path = path.as_ref();
        let pem = std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                KeyError::FileNotFound(path.to_path_buf())
            } else {
                KeyError::ReadError(e)
            }
        })?;
        Self::from_pem(&pem)
    }

    // Explicitly expose for signing only
    fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

// When key is dropped, memory is overwritten with zeros
impl Drop for Ed25519PrivateKey {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

/// Token wrapper using secrecy for logging safety
pub struct AccessToken(Secret<String>);

impl AccessToken {
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

// Debug impl redacts the value
impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AccessToken").field(&"[REDACTED]").finish()
    }
}
```

**Security Guarantees**:

| Data Type                 | Zeroize on Drop    | Redacted in Logs            | Memory Protection          |
| ------------------------- | ------------------ | --------------------------- | -------------------------- |
| `Ed25519PrivateKey`       | Yes                | Yes                         | Stack-only, no heap copies |
| `AccessToken`             | Yes                | Yes (via `Secret`)          | Heap, single location      |
| `ClientCredentialsConfig` | Yes (contains key) | Partial (client_id visible) | Key protected              |
| `RefreshToken`            | Yes                | Yes                         | Heap, single location      |

### Key ID (kid) Derivation

How the SDK determines the `kid` claim for JWT signing:

```rust
/// Key ID derivation strategy
pub enum KeyIdStrategy {
    /// Use explicit certificate_id from ClientCredentialsConfig
    Explicit(String),

    /// Derive from public key (default)
    /// kid = base64url(sha256(public_key_bytes)[0..8])
    DeriveFromPublicKey,

    /// Fetch from JWKS endpoint (/.well-known/jwks.json)
    FetchFromJwks,
}

impl ClientCredentialsConfig {
    fn derive_kid(&self) -> String {
        match &self.certificate_id {
            Some(kid) => kid.clone(),
            None => {
                // Derive kid from public key
                let public_key = self.private_key.public_key();
                let hash = sha256(public_key.as_bytes());
                base64url_encode(&hash[0..8])
            }
        }
    }
}
```

**Usage**:

```rust
// Explicit kid (recommended for production)
let creds = ClientCredentialsConfig {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
    certificate_id: Some("key-2024-01".into()),  // Explicit kid
};

// Auto-derived kid (convenient for development)
let creds = ClientCredentialsConfig {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
    certificate_id: None,  // SDK derives from public key
};
```

### Transport Security

- TLS 1.2+ required for production
- Certificate pinning available for high-security
- No `insecure()` in production

### Insecure Mode for Development

The SDK provides an explicit escape hatch for development/testing only:

```rust
// ⚠️ DEVELOPMENT ONLY - This disables TLS verification
let client = Client::builder()
    .url("https://localhost:8443")  // Self-signed cert
    .credentials(creds)
    .insecure()  // Disables TLS verification
    .build()
    .await?;
```

**Safety Mechanisms**:

The `insecure()` method is only available when the `insecure` feature flag is enabled. This ensures developers make an explicit compile-time decision to allow insecure connections.

```rust
// Cargo.toml - only enable for development
[dependencies]
inferadb = { version = "0.1", features = ["insecure"] }

// In code
#[cfg(feature = "insecure")]
let client = Client::builder()
    .url("https://localhost:8443")
    .credentials(creds)
    .insecure()  // Only available with "insecure" feature
    .build()
    .await?;
```

**Implementation**:

```rust
impl ClientBuilder<HasUrl, HasAuth, HasVault> {
    /// Disable TLS verification (DEVELOPMENT ONLY)
    ///
    /// # Security Warning
    ///
    /// This method disables TLS certificate verification, making the
    /// connection vulnerable to man-in-the-middle attacks. Only use
    /// this for local development or testing.
    ///
    /// # Availability
    ///
    /// This method is only available when the `insecure` feature flag is enabled.
    /// This is a compile-time decision to prevent accidental use in production.
    #[cfg(feature = "insecure")]
    pub fn insecure(mut self) -> Self {
        tracing::warn!(
            target: "inferadb::security",
            "TLS verification disabled - this is insecure!"
        );

        self.tls_config = Some(TlsConfig::insecure());
        self
    }
}

// Without the feature, method doesn't exist
// Cargo.toml:
// [features]
// insecure = []  # Must be explicitly enabled
```

### Credential Scope Best Practices

Follow the principle of least privilege when configuring client credentials:

```rust
// ✅ Good: Narrow scopes for specific service
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "order-service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: Some("order-svc-2024".into()),
    })
    .build()
    .await?;

// The client_id "order-service" should be registered in InferaDB
// with only the permissions it needs:
// - read: check permissions on orders
// - write: create relationships for new orders
// NOT: admin, schema management, etc.
```

**Scope Recommendations by Use Case**:

| Use Case                  | Recommended Client Permissions     |
| ------------------------- | ---------------------------------- |
| API Gateway               | `check` only                       |
| Microservice (read-heavy) | `check`, `list`                    |
| Microservice (writes)     | `check`, `list`, `write`, `delete` |
| Admin dashboard           | All except `schema:*`              |
| CI/CD pipeline            | `schema:push`, `schema:validate`   |
| Data migration            | `write`, `delete`, `list`          |

**Organization-Scoped Credentials**:

```rust
// Restrict client to specific organizations
// (configured in InferaDB, not the SDK)

// Client registered with org restrictions:
// - org_id: "org_acme_corp"
// - vaults: ["vlt_production", "vlt_staging"]

// SDK honors these restrictions automatically
let vault = client
    .organization("org_acme_corp")
    .vault("vlt_production");  // ✅ Allowed

let vault = client
    .organization("org_other")  // ❌ API returns ErrorKind::Forbidden
    .vault("vlt_their_data");
```

**Bearer Token Scope Inheritance**:

```rust
// Bearer tokens inherit scopes from the user session
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: user_session.access_token.clone(),
    })
    .build()
    .await?;

// Operations are limited to what the user can do
// If user only has read access, writes will fail
```

### Input Validation

- Validate all inputs before sending
- Sanitize entity IDs (no injection attacks)
- Limit request sizes

### Audit Requirements

- Log all authorization decisions (configurable)
- Include request IDs for traceability
- Support compliance logging formats
