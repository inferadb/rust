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
| [docs/guides/multi-tenant.md](docs/guides/multi-tenant.md)                     | SDK Users    | Multi-organization SaaS patterns               |
| [docs/guides/temporal-permissions.md](docs/guides/temporal-permissions.md)     | SDK Users    | Time-based permission constraints              |
| [docs/internal/competitive-analysis.md](docs/internal/competitive-analysis.md) | Internal     | Competitive positioning                        |

---

## Table of Contents

### Part 1: Vision & Architecture

- [Design Philosophy](#design-philosophy)
- [Architecture Overview](#architecture-overview)
- [Crate Structure](#crate-structure)

### Part 2: Client Design

- [Client Builder](#client-builder)
- [Typestate Builder Pattern](#typestate-builder-pattern)
- [Authentication](#authentication)
- [Connection Management](#connection-management)
  - [Client Cloning Semantics](#client-cloning-semantics)
- [Health Check & Lifecycle](#health-check--lifecycle)
- [Vault Scoping](#vault-scoping)
- [Middleware & Interceptors](#middleware-and-interceptors)

### Part 3: Type System & Safety

- [Type-Safe Relationships](#type-safe-relationships)
- [Zero-Copy APIs](#zero-copy-apis)
- [Async Trait Objects & DI](#async-trait-objects--di)

### Part 4: Engine API Design

- [Authorization Checks](#authorization-checks)
  - [Batch Size Constraints](#batch-size-constraints)
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
- [Retry & Resilience](#retry--resilience)
- [Graceful Degradation](#graceful-degradation)
- [Observability](#observability)
  - [Health Checks](#health-checks)
  - [Ping & Latency](#ping--latency)
  - [Diagnostics](#diagnostics)
- [W3C Trace Context Propagation](#w3c-trace-context-propagation)
- [Testing Support](#testing-support)

### Part 7: Implementation Details

- [Protocol Support](#protocol-support)
- [Feature Flags](#feature-flags)
- [Safety Guarantees](#safety-guarantees)
- [Release Strategy](#release-strategy)

---

## Design Philosophy

### Core Principles

1. **Zero-friction authentication**: SDK self-manages tokens, refresh cycles, and credential rotation. Developers provide credentials once and forget about auth.

2. **Unified service URL**: Single endpoint routes to both Engine and Control APIs transparently. No separate clients or configuration.

3. **Type-safe by default**: Leverage Rust's type system to prevent invalid states. Relationship tuples, permissions, and resources are typed at compile time.

4. **Streaming-first**: All list operations support streaming for memory efficiency. Batch operations stream results as they complete.

5. **Protocol flexibility**: Support both gRPC (high performance) and REST (universal compatibility) with feature flags.

6. **Observability built-in**: First-class tracing, metrics, and structured logging without configuration.

7. **Testing as a feature**: Mock clients, simulation mode, and test utilities are first-class SDK features.

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

---

## Architecture Overview

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                              InferaDB Rust SDK                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                             Client                                  │   │
│  │  .access() → AccessClient    .control() → ControlClient             │   │
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
│  │      AccessClient           │  │         ControlClient               │  │
│  │  (via .access())            │  │  (via .control())                   │  │
│  │  ┌───────────────────────┐  │  │  ┌───────────────────────────────┐  │  │
│  │  │ check()               │  │  │  │ organizations() / organization()│  │
│  │  │ check_batch()         │  │  │  │ vaults() / vault()            │  │  │
│  │  │ expand()              │  │  │  │ clients() / client()          │  │  │
│  │  │ list_resources()      │  │  │  │ account()                     │  │  │
│  │  │ list_subjects()       │  │  │  │ invitations() / invitation()  │  │  │
│  │  │ list_relationships()  │  │  │  │ jwks()                        │  │  │
│  │  │ write() / delete()    │  │  │  └───────────────────────────────┘  │  │
│  │  │ watch()               │  │  │                                     │  │
│  │  │ simulate()            │  │  │                                     │  │
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

## Client Builder

### Design Goals

1. **Ergonomic defaults** - Minimize required configuration
2. **Type-safe construction** - Invalid configurations don't compile
3. **Lazy connection** - Don't block on network during build

### Builder Pattern

```rust
use inferadb::prelude::*;

// Minimal setup with client credentials
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials("client_id", "path/to/private_key.pem")
    .build()
    .await?;

// Vault must be specified explicitly on each operation
let allowed = client
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
    .credentials(Credentials {
        client_id: "my_service".into(),
        private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
        certificate_id: None,
    })

    // Connection pool
    .pool_size(20)
    .idle_timeout(Duration::from_secs(60))

    // Retry behavior
    .retries(Retries::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10)))

    // Protocol preference
    .prefer_grpc()  // Falls back to REST if unavailable

    // Build (validates and creates client)
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

// Const configuration - validated at compile time
const CONFIG: ClientConfig = ClientConfig::const_builder()
    .url("https://api.inferadb.com")
    .pool_size(20)
    .connect_timeout_secs(10)
    .build_const();  // Compile-time validation

let client = Client::from_config(CONFIG)
    .credentials(creds)
    .build()
    .await?;
```

---

## Typestate Builder Pattern

Use phantom types to catch builder errors at compile time, not runtime.

### Typestate Design Goals

1. **Compile-time enforcement** - Missing required fields don't compile
2. **Clear error messages** - Type errors indicate what's missing
3. **IDE support** - Autocomplete shows only valid next steps

### Type States

```rust
// Marker types for builder state
mod state {
    pub struct NoUrl;
    pub struct HasUrl;
    pub struct NoAuth;
    pub struct HasAuth;
}

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
    pub fn credentials(self, creds: Credentials)
        -> ClientBuilder<Url, HasAuth>
    {
        ClientBuilder {
            auth: Some(AuthConfig::ClientCredentials(creds)),
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
    pub fn retries(mut self, config: Retries) -> Self { /* ... */ }
}
```

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
    .token_refresh_threshold(0.8)  // Refresh at 80% of lifetime
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
let vault = client.access().vault("vlt_01JFQGK...");
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
    .token_refresh_config(TokenRefreshConfig {
        // Retry transient failures (network, 5xx)
        max_retries: 3,
        retry_backoff: Duration::from_millis(100),

        // Don't retry auth failures (401, 403)
        retry_on_auth_failure: false,

        // How long before expiry to start refresh
        refresh_threshold: Duration::from_secs(60),

        // Fallback: allow requests with soon-expiring token during refresh
        grace_period: Duration::from_secs(10),
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

```rust
// Ed25519 private key from file
let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
    certificate_id: None,  // Auto-detect from JWKS
};

// Ed25519 private key from bytes
let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem(include_bytes!("key.pem"))?,
    certificate_id: Some("kid-123".into()),  // Specific key ID
};

// Bearer token (for user-initiated requests)
let client = Client::builder()
    .bearer_token("eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9...")
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

For dynamic credential management (key rotation, secrets managers, HSMs), use the `CredentialsProvider` trait:

```rust
/// Trait for dynamic credential resolution.
/// Implement this for custom key management integrations.
#[async_trait]
pub trait CredentialsProvider: Send + Sync + 'static {
    /// Returns the current credentials for signing.
    /// Called before each token refresh, allowing for key rotation.
    async fn get_credentials(&self) -> Result<ClientCredentials, CredentialsError>;

    /// Optional: Called when credentials fail authentication.
    /// Allows providers to invalidate cached credentials.
    async fn on_auth_failure(&self, _error: &AuthError) {
        // Default: no-op
    }
}
```

**Built-in Implementations**:

```rust
// Static credentials (default) - credentials never change
impl CredentialsProvider for ClientCredentials {
    async fn get_credentials(&self) -> Result<ClientCredentials, CredentialsError> {
        Ok(self.clone())
    }
}

// Environment variable provider
pub struct EnvCredentialsProvider {
    client_id_var: String,
    private_key_var: String,
}

impl EnvCredentialsProvider {
    pub fn new(client_id_var: &str, private_key_var: &str) -> Self {
        Self {
            client_id_var: client_id_var.into(),
            private_key_var: private_key_var.into(),
        }
    }
}

#[async_trait]
impl CredentialsProvider for EnvCredentialsProvider {
    async fn get_credentials(&self) -> Result<ClientCredentials, CredentialsError> {
        let client_id = std::env::var(&self.client_id_var)
            .map_err(|_| CredentialsError::NotFound(self.client_id_var.clone()))?;
        let private_key_pem = std::env::var(&self.private_key_var)
            .map_err(|_| CredentialsError::NotFound(self.private_key_var.clone()))?;

        Ok(ClientCredentials {
            client_id,
            private_key: Ed25519PrivateKey::from_pem(private_key_pem.as_bytes())?,
            certificate_id: None,
        })
    }
}
```

**AWS Secrets Manager Integration**:

```rust
pub struct AwsSecretsProvider {
    client: aws_sdk_secretsmanager::Client,
    secret_id: String,
    cache: RwLock<Option<(ClientCredentials, Instant)>>,
    cache_ttl: Duration,
}

#[async_trait]
impl CredentialsProvider for AwsSecretsProvider {
    async fn get_credentials(&self) -> Result<ClientCredentials, CredentialsError> {
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
            .ok_or(CredentialsError::InvalidFormat("missing secret_string"))?;

        let parsed: SecretsPayload = serde_json::from_str(secret_string)
            .map_err(|e| CredentialsError::InvalidFormat(e))?;

        let creds = ClientCredentials {
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
// Static credentials (existing API, unchanged)
let client = Client::builder()
    .credentials(creds)
    .build()
    .await?;

// Dynamic credentials via provider
let client = Client::builder()
    .credentials_provider(AwsSecretsProvider::new(
        secrets_client,
        "inferadb/production/credentials",
    ))
    .build()
    .await?;

// Environment-based credentials
let client = Client::builder()
    .credentials_provider(EnvCredentialsProvider::new(
        "INFERADB_CLIENT_ID",
        "INFERADB_PRIVATE_KEY",
    ))
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
    let vault = client.access().vault("vlt_01JFQGK...");
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
    pub latency: Duration,
    pub components: HashMap<String, ComponentHealth>,
    pub server_version: Option<String>,
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

All authorization operations require explicit vault specification to prevent accidental cross-vault operations.

### Quick Start

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

// Access API: authorization checks and relationship management
let allowed = client
    .access()
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "doc:1")
    .await?;

// Control API: management operations
let orgs = client.control().organizations().list().await?;
```

### API Conventions

The SDK provides two distinct API surfaces accessible from the `Client`:

| Method      | Returns         | Use Case                                       |
| ----------- | --------------- | ---------------------------------------------- |
| `access()`  | `AccessClient`  | Authorization checks, relationships, streaming |
| `control()` | `ControlClient` | Organizations, vaults, schemas, audit logs     |

Both clients use noun-based scoping for resource hierarchy:

| Method                        | Returns       | Use Case                         |
| ----------------------------- | ------------- | -------------------------------- |
| `access().vault(id)`          | `VaultAccess` | Vault-scoped authorization ops   |
| `control().organization(id)`  | `OrgClient`   | Organization-scoped management   |
| `control().vault(id)`         | `VaultClient` | Vault-scoped management          |

### Single Operation

```rust
// Vault specified inline for each operation
let allowed = client
    .access()
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "doc:1")
    .await?;
```

### Multiple Operations (Same Vault)

```rust
// Scoped client for multiple operations on same vault
let production = client.access().vault("vlt_01JFQGK...");
production.check("user:alice", "view", "doc:1").await?;
production.write(Relationship::new("doc:1", "viewer", "user:bob")).await?;

// Different vault for staging
let staging = client.access().vault("vlt_02STAGING...");
staging.check("user:alice", "view", "doc:1").await?;
```

### Organization and Vault Hierarchy

Vaults are owned by organizations. You can chain scoping to express this hierarchy:

```rust
// Access API with explicit organization + vault chain
let allowed = client
    .access()
    .organization("org_8675309")
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "doc:1")
    .await?;

// Direct vault access (when organization is implicit/known)
let allowed = client
    .access()
    .vault("vlt_01JFQGK...")
    .check("user:alice", "view", "doc:1")
    .await?;
```

### VaultAccess Design

`VaultAccess` is owned and cheaply cloneable (uses `Arc` internally, like `Client`):

```rust
/// A client scoped to a specific vault for authorization operations.
/// Cheaply cloneable - can be stored, passed to tasks, shared across threads.
#[derive(Clone)]
pub struct VaultAccess {
    inner: AccessClient,  // AccessClient uses Arc internally - clone is O(1)
    vault_id: String,
}

impl AccessClient {
    /// Create a vault-scoped access client
    pub fn vault(&self, vault_id: impl Into<String>) -> VaultAccess {
        VaultAccess {
            inner: self.clone(),  // Cheap Arc clone
            vault_id: vault_id.into(),
        }
    }
}

impl VaultAccess {
    pub async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    pub async fn write(&self, relationship: Relationship<'_>) -> Result<(), Error>;
    pub async fn delete(&self, relationship: Relationship<'_>) -> Result<(), Error>;
    pub fn list_resources(&self, subject: &str, permission: &str) -> ResourceStream;
    pub fn list_subjects(&self, permission: &str, resource: &str) -> SubjectStream;
    pub fn watch(&self) -> WatchStream;
    // ... all authorization operations, scoped to vault
}
```

**Design Rationale:**

We considered offering both borrowed (`VaultAccess<'a>`) and owned (`VaultAccess`) variants, but rejected this for simplicity:

- `Client` already uses `Arc` internally - cloning is just a refcount increment (O(1))
- The borrowed variant would save only one atomic increment per call
- Two variants would force users to understand Rust lifetimes and choose between methods
- Standard practice in Rust SDKs (`reqwest::Client`, AWS SDK) is to make clients cheaply clonable

**Usage Examples**:

```rust
// Inline use
client.access().vault("vlt_01JFQGK...").check("user:alice", "view", "doc:1").await?;

// Store for reuse
let vault = client.access().vault("vlt_01JFQGK...");
vault.check("user:alice", "view", "doc:1").await?;
vault.check("user:alice", "edit", "doc:1").await?;

// Store in struct (no lifetime parameter needed)
struct MyService {
    authz: VaultAccess,
}

// Pass to spawned task (VaultAccess is 'static)
let vault = client.access().vault("vlt_01JFQGK...");
tokio::spawn(async move {
    vault.check("user:alice", "view", "doc:1").await
});
```

### AccessClient and ControlClient

The SDK separates authorization operations from management operations:

```rust
/// Client for authorization operations (checks, relationships, streaming)
#[derive(Clone)]
pub struct AccessClient {
    inner: Client,
}

impl Client {
    /// Get the access client for authorization operations
    pub fn access(&self) -> AccessClient {
        AccessClient { inner: self.clone() }
    }
}

impl AccessClient {
    /// Scope to a vault
    pub fn vault(&self, vault_id: impl Into<String>) -> VaultAccess { ... }

    /// Scope to an organization first, then vault
    pub fn organization(&self, org_id: impl Into<String>) -> OrgAccess { ... }
}

/// Client for control plane operations (management, schemas, audit)
#[derive(Clone)]
pub struct ControlClient {
    inner: Client,
}

impl Client {
    /// Get the control client for management operations
    pub fn control(&self) -> ControlClient {
        ControlClient { inner: self.clone() }
    }
}

impl ControlClient {
    pub fn organizations(&self) -> OrganizationsClient { ... }
    pub fn organization(&self, id: &str) -> OrgClient { ... }
    pub fn vaults(&self) -> VaultsClient { ... }
    pub fn vault(&self, id: &str) -> VaultClient { ... }
    // ... management operations
}
```

### Multi-Organization SaaS Pattern

For multi-organization applications where each organization has their own vault:

```rust
// Extract organization from request and scope to their vault
async fn handle_request(
    client: &Client,
    org_vault_id: &str,  // Vault ID for this organization
    request: Request,
) -> Result<Response, Error> {
    // All authorization in this handler uses organization's vault
    let vault = client.access().vault(org_vault_id);

    let user = extract_user(&request);
    let resource = extract_resource(&request);

    if !vault.check(&user, "access", &resource).await? {
        return Err(Error::Forbidden);
    }

    // Process request...
}
```

### Vault Validation

```rust
// Validate vault exists before operations
let vault = client.vault("vlt_01JFQGK...");
vault.validate().await?;  // Fails if vault doesn't exist or no access

// Or check access without failing
if client.has_vault_access("vlt_01JFQGK...").await? {
    // Safe to use vault
}
```

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

### Adding Custom Middleware

```rust
use inferadb::middleware::{Middleware, Request, Response};

struct AuditLogger {
    logger: Logger,
}

impl Middleware for AuditLogger {
    async fn handle(&self, req: Request, next: Next) -> Result<Response, Error> {
        let start = Instant::now();
        let operation = req.operation().to_string();

        let response = next.call(req).await;

        self.logger.log(AuditEntry {
            operation,
            duration: start.elapsed(),
            success: response.is_ok(),
        });

        response
    }
}

let client = Client::builder()
    .url("https://api.inferadb.com")
    .middleware(AuditLogger::new())
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
vault.write(Relationship::new("doc:1", "viwer", "user:alice")).await?;  // "viwer" typo
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
vault.write(Relationship::new("document:readme", "viewer", "user:alice")).await?;

// Type-safe builder
vault.write(
    doc.viewer().is(&user)
).await?;

// Batch with mixed types
vault.write_batch([
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

// VaultAccess methods accept both strong types and strings
vault.check(subject, "view", resource).await?;  // Strong types
vault.check("user:alice", "view", "doc:readme").await?;  // Strings (still works)

// Relationship accepts EntityRef for resource/subject
let rel = Relationship::new(
    EntityRef::new("document", "readme"),
    "viewer",
    SubjectRef::parse("user:alice")?,
);
```

---

## Zero-Copy APIs

Support borrowed data for high-volume paths where allocation matters.

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

```rust
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
}
```

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

---

## Async Trait Objects & DI

Enable dependency injection and testing with trait objects.

### Dual Trait Design

The SDK provides two trait variants optimized for different use cases:

1. **Generic trait** (`Authorize`) - Maximum performance, no allocation overhead
2. **Object-safe trait** (`AuthorizationClient`) - Enables `dyn` usage and dependency injection

**Why Two Traits?**

| Consideration        | Generic Trait                | Object-Safe Trait         |
| -------------------- | ---------------------------- | ------------------------- |
| Performance          | Monomorphized, zero overhead | vtable dispatch           |
| Allocation           | Borrows possible             | May require owned data    |
| `dyn` compatible     | No (uses generics)           | Yes                       |
| Dependency injection | Generics only                | Works with `Arc<dyn ...>` |
| Testing mocks        | Direct                       | Via trait object          |

### The Authorize Trait (Generic, High-Performance)

```rust
/// High-performance authorization trait.
/// Uses generics and associated types for zero-cost abstractions.
/// NOT object-safe - use AuthorizationClient for dyn.
pub trait Authorize {
    type CheckFuture<'a>: Future<Output = Result<bool, Error>> + Send + 'a
    where
        Self: 'a;

    type WriteFuture<'a>: Future<Output = Result<(), Error>> + Send + 'a
    where
        Self: 'a;

    /// Check authorization with borrowed parameters (zero allocation)
    fn check<'a>(
        &'a self,
        subject: &'a str,
        permission: &'a str,
        resource: &'a str,
    ) -> Self::CheckFuture<'a>;

    /// Check with context
    fn check_with_context<'a>(
        &'a self,
        subject: &'a str,
        permission: &'a str,
        resource: &'a str,
        context: &'a Context,
    ) -> Self::CheckFuture<'a>;

    /// Write relationship
    fn write<'a>(&'a self, relationship: &'a Relationship<'_>) -> Self::WriteFuture<'a>;
}

// Client implements the generic trait with concrete future types
impl Authorize for Client {
    type CheckFuture<'a> = impl Future<Output = Result<bool, Error>> + Send + 'a;
    type WriteFuture<'a> = impl Future<Output = Result<(), Error>> + Send + 'a;

    fn check<'a>(
        &'a self,
        subject: &'a str,
        permission: &'a str,
        resource: &'a str,
    ) -> Self::CheckFuture<'a> {
        async move {
            // Direct implementation, no boxing
            self.inner_check(subject, permission, resource).await
        }
    }
    // ...
}
```

### The AuthorizationClient Trait (Object-Safe)

```rust
use async_trait::async_trait;

/// Object-safe authorization trait for dependency injection.
/// Implemented by Client, MockClient, InMemoryClient.
/// Use this when you need `dyn AuthorizationClient`.
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error>;

    async fn check_with_context(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
        context: &Context,
    ) -> Result<bool, Error>;

    async fn check_batch(
        &self,
        checks: &[(&str, &str, &str)],
    ) -> Result<Vec<bool>, Error>;

    async fn write(&self, relationship: &Relationship<'_>) -> Result<(), Error>;

    async fn delete(&self, relationship: &Relationship<'_>) -> Result<(), Error>;

    async fn health_check(&self) -> Result<bool, Error>;
}

// Auto-implement object-safe trait for anything implementing generic trait
impl<T: Authorize + Send + Sync> AuthorizationClient for T {
    async fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error> {
        Authorize::check(self, subject, permission, resource).await
    }
    // ... delegate other methods
}
```

### Choosing Between Traits

```rust
// For maximum performance: Use generic bounds
async fn process_request<A: Authorize>(
    authz: &A,
    request: Request,
) -> Result<Response, Error> {
    // Monomorphized - no vtable, inlinable
    authz.check(&request.user, "access", &request.resource).await?;
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
// Production
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .build()
    .await?;

let app = App::new(Arc::new(client) as SharedAuthz);

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
    // Create mock with expected calls
    let mock = MockClient::builder()
        .expect_check("user:alice", "view", "doc:1")
        .returning(true)
        .expect_check("user:bob", "view", "doc:1")
        .returning(false)
        .build();

    // Test through trait object
    let authz: &dyn AuthorizationClient = &mock;

    assert!(authz.check("user:alice", "view", "doc:1").await.unwrap());
    assert!(!authz.check("user:bob", "view", "doc:1").await.unwrap());

    // Verify all expected calls were made
    mock.verify();
}
```

---

## Authorization Checks

All authorization operations require vault scoping via `client.access().vault(...)`.

### API Design

```rust
// Get vault-scoped client for authorization operations
let vault = client.access().vault("vlt_01JFQGK...");

// Simple check - returns bool
let allowed = vault
    .check("user:alice", "view", "document:readme")
    .await?;

// Check with ABAC context
let allowed = vault
    .check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.50")
        .insert("mfa_verified", true))
    .await?;

// Detailed decision with trace
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

```rust
/// Builder for authorization check requests.
/// Implements IntoFuture for ergonomic await syntax.
pub struct CheckRequest<'a> {
    vault: &'a VaultAccess,
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
    context: Option<Context>,
    trace: bool,
    consistency: Option<ConsistencyToken>,
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
```

**Usage patterns enabled by IntoFuture**:

```rust
// Get vault-scoped client for authorization operations
let vault = client.access().vault("vlt_01JFQGK...");

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

| Method        | Transforms To          | Returns                 | Notes                    |
| ------------- | ---------------------- | ----------------------- | ------------------------ |
| `.detailed()` | `DetailedCheckRequest` | `Decision`              | Full decision with trace |
| `.require()`  | `RequireCheckRequest`  | `Result<(), Forbidden>` | Early-return pattern     |

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

### Result Ergonomics

The SDK provides three core patterns for handling authorization results. Choose the simplest that fits your use case.

| Pattern               | Use When                             | Returns                    |
| --------------------- | ------------------------------------ | -------------------------- |
| `require()`           | Early-return on denial (most common) | `Result<(), Forbidden>`    |
| `then(closure)`       | Conditionally execute on success     | `Result<Option<T>, Error>` |
| `filter_authorized()` | Filter collections by permission     | `Result<Vec<T>, Error>`    |

#### Require Pattern (Early Return on Denial)

```rust
// Most common pattern - fail fast on denial:
vault.check("user:alice", "view", "doc:1")
    .require()  // Converts bool to Result<(), Forbidden>
    .await?;    // Early returns on denial

// Continue with authorized operation...
let doc = fetch_document("doc:1").await?;

// For custom errors, use standard Rust composition:
let allowed = vault.check("user:alice", "view", "doc:1").await?;
if !allowed {
    return Err(AppError::AccessDenied {
        user: user_id.clone(),
        resource: doc_id.clone(),
    });
}
```

#### Then Pattern (Conditional Execution)

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

#### Filter Pattern (Batch Authorization)

```rust
// Filter a collection to only authorized items:
let accessible_docs = vault
    .filter_authorized("user:alice", "view", &documents, |doc| format!("document:{}", doc.id))
    .await?;
```

#### Require Types

```rust
/// Error returned when authorization is denied
#[derive(Debug, Clone)]
pub struct Forbidden {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub request_id: Option<String>,
}

impl std::error::Error for Forbidden {}

impl Forbidden {
    /// Convert to HTTP status code
    pub fn status_code(&self) -> u16 { 403 }

    /// Convert to a user-safe message
    pub fn user_message(&self) -> &'static str {
        "You don't have permission to perform this action"
    }
}

// Integrates with common error types
impl From<Forbidden> for axum::http::StatusCode {
    fn from(_: Forbidden) -> Self { Self::FORBIDDEN }
}

impl From<Forbidden> for actix_web::error::Error {
    fn from(e: Forbidden) -> Self {
        actix_web::error::ErrorForbidden(e.user_message())
    }
}
```

### Batch Checks

```rust
// Batch check - single round-trip for multiple checks
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

// Streaming batch results
let mut stream = vault
    .check_batch(checks)
    .stream();

while let Some(result) = stream.next().await {
    let (check, allowed) = result?;
    handle_result(check, allowed);
}
```

### Batch Size Constraints

Understanding batch limits prevents surprises in production:

| Operation            | Max Batch Size | Recommended   | Notes                                    |
| -------------------- | -------------- | ------------- | ---------------------------------------- |
| `check_batch()`      | 1,000          | 100-500       | Larger batches increase latency variance |
| `write_batch()`      | 10,000         | 1,000-5,000   | Transactional - all or nothing           |
| `delete_batch()`     | 10,000         | 1,000-5,000   | Transactional - all or nothing           |
| `list_*().collect()` | Unlimited      | Use streaming | Memory-bound by client                   |

```rust
// Exceeding limits returns an error
let checks: Vec<_> = (0..2000).map(|i| ("user:alice", "view", format!("doc:{}", i))).collect();
let result = vault.check_batch(&checks).collect().await;
// Err(Error { kind: InvalidInput, message: "Batch size 2000 exceeds limit of 1000" })

// Chunk large batches automatically
use futures::stream::{self, StreamExt, TryStreamExt};

async fn chunked_batch_check(
    vault: &VaultAccess,
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

| Operation                 | Semantics     | Failure Behavior                                 |
| ------------------------- | ------------- | ------------------------------------------------ |
| `check_batch()`           | **Streaming** | Individual check failures don't affect others    |
| `write_batch()`           | **Atomic**    | All or nothing - partial failure rolls back      |
| `delete_batch()`          | **Atomic**    | All or nothing - partial failure rolls back      |
| `write_batch_streaming()` | **Streaming** | Each write independent, partial success possible |

**Atomic Batches (write_batch, delete_batch)**:

```rust
// Atomic: Either all writes succeed, or none do
let result = vault
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
let mut stream = vault
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

| Use Case                               | Recommended                 |
| -------------------------------------- | --------------------------- |
| Adding user to multiple resources      | `write_batch()` (atomic)    |
| Bulk import (tolerate partial failure) | `write_batch_streaming()`   |
| Authorization checks                   | `check_batch()` (streaming) |
| Remove user from all resources         | `delete_batch()` (atomic)   |
| Cleanup/migration                      | `delete_batch_streaming()`  |

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
```

### Querying Decision Trees

```rust
let decision = client
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
let decision = client
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
let explanation = client
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
```

### Explanation Options

```rust
// Get all possible access paths (not just the first match)
let explanation = client
    .explain_permission("user:alice", "view", "document:readme")
    .all_paths(true)
    .await?;

// Limit traversal depth for performance
let explanation = client
    .explain_permission("user:alice", "view", "document:readme")
    .max_depth(5)
    .await?;

// Include schema definition in response
let explanation = client
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
let explanation = client
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
let result = client
    .simulate()
    .check("user:alice", "view", "document:secret")
    .with_relationship(Relationship::new("user:alice", "viewer", "document:secret"))
    .await?;

assert!(result.allowed);
```

### Complex Scenarios

```rust
// Simulate multiple relationships
let result = client
    .simulate()
    .check("user:alice", "edit", "document:report")
    .with_relationships([
        Relationship::new("user:alice", "member", "group:editors"),
        Relationship::new("group:editors", "editor", "document:report"),
    ])
    .await?;

// Simulate relationship removal
let result = client
    .simulate()
    .check("user:bob", "view", "document:readme")
    .without_relationship(Relationship::new("user:bob", "viewer", "document:readme"))
    .await?;

// Combined add and remove
let result = client
    .simulate()
    .check("user:alice", "view", "document:readme")
    .with_relationship(Relationship::new("user:alice", "viewer", "document:readme"))
    .without_relationship(Relationship::new("group:all", "viewer", "document:readme"))
    .await?;
```

### Batch Simulation

```rust
// Simulate multiple checks with the same hypothetical state
let results = client
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

### Simulation Response

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// The authorization result
    pub allowed: bool,

    /// The check that was performed
    pub check: AuthorizationCheck,

    /// Relationships that were added for simulation
    pub added_relationships: Vec<Relationship<'static>>,

    /// Relationships that were removed for simulation
    pub removed_relationships: Vec<Relationship<'static>>,

    /// Full decision trace (if requested)
    pub trace: Option<DecisionNode>,

    /// Whether the result differs from current state
    pub differs_from_current: bool,
}

// Check if simulation changes the outcome
let result = client
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

### Write Operations

```rust
// Single write
client
    .write(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Batch write
client
    .write_batch([
        Relationship::new("folder:docs", "viewer", "group:engineering#member"),
        Relationship::new("document:readme", "parent", "folder:docs"),
    ])
    .await?;

// Conditional write (only if doesn't exist)
client
    .write(Relationship::new("document:readme", "viewer", "user:bob"))
    .unless_exists()
    .await?;
```

### Delete Operations

```rust
// Delete specific relationship
client
    .delete(Relationship::new("document:readme", "viewer", "user:alice"))
    .await?;

// Delete all relationships matching filter
client
    .delete_all()
    .resource("document:readme")
    .relation("viewer")
    .execute()
    .await?;
```

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

**Type Alias for Owned Data**: When you need `'static` lifetime (e.g., storing in collections, sending across threads):

```rust
/// Convenience alias for owned relationships
pub type OwnedRelationship = Relationship<'static>;

// Use when storing or sending
let rel: OwnedRelationship = Relationship::owned("doc:1", "viewer", "user:alice");
```

### Relationship History

Query the change history of relationships for auditing and debugging:

```rust
// Get history for a specific relationship
let history = client
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
let history = client
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
let result = client
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
let result = client
    .relationships()
    .validate(Relationship::new("user:alice", "viewer", "document:readme"))
    .against_schema(schema_id)
    .await?;
```

**Batch Validation**:

```rust
// Validate multiple relationships efficiently
let results = client
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
let preview = client
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

client
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)
    .await?;

// Safe to retry with same request ID - server deduplicates
client
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
vault.write(Relationship::new("doc:1", "viewer", "user:alice")).await?;
```

### Request ID in Responses

```rust
// All responses include request ID for debugging
let result = vault
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
vault
    .write_batch([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:1", "editor", "user:bob"),
    ])
    .request_id(Uuid::new_v4())
    .await?;

// Or individual IDs per operation
vault
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
vault.write(rel).request_id(id).await?;  // Executes, returns Ok

// Second call within window - returns cached result
vault.write(rel).request_id(id).await?;  // Returns cached Ok

// After window expires - executes again
// (may fail if relationship already exists)
```

### Conditional Writes

```rust
// Write only if not exists (idempotent by nature)
vault
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .unless_exists()
    .await?;

// Write with precondition
vault
    .write(Relationship::new("doc:1", "owner", "user:alice"))
    .precondition(Precondition::not_exists("doc:1", "owner", "*"))
    .await?;

// Atomic compare-and-swap
vault
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
let write_result = vault
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
async fn add_viewer(vault: &VaultAccess, doc: &str, user: &str) -> Result<ConsistencyToken, Error> {
    let result = vault
        .write(Relationship::new(doc, "viewer", user))
        .await?;
    Ok(result.consistency_token())
}

// Service B: Uses token from Service A to ensure consistency
async fn check_access(
    vault: &VaultAccess,
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
let resources = client
    .list_resources("user:alice", "view")
    .resource_type("document")
    .collect()
    .await?;

// Paginated results
let page = client
    .list_resources("user:alice", "view")
    .resource_type("document")
    .page_size(100)
    .page(1)
    .await?;
```

### List Subjects

```rust
// Find all users who can edit this document
let subjects = client
    .list_subjects("edit", "document:readme")
    .subject_type("user")
    .collect()
    .await?;
```

### List Relationships

```rust
// List all relationships for a resource
let relationships = client
    .list_relationships()
    .resource("document:readme")
    .collect()
    .await?;

// Filter by relation
let viewers = client
    .list_relationships()
    .resource("document:readme")
    .relation("viewer")
    .collect()
    .await?;

// Filter by subject
let alice_rels = client
    .list_relationships()
    .subject("user:alice")
    .collect()
    .await?;
```

---

## Streaming & Watch

True streaming without hidden buffering - `.collect()` is opt-in, not the default path.

### API Principle: Single vs Multi-Value Returns

The SDK uses different patterns based on return cardinality:

| Return Type         | Pattern                              | Example                                                       |
| ------------------- | ------------------------------------ | ------------------------------------------------------------- |
| **Single value**    | `IntoFuture` (direct `.await`)       | `vault.check(...).await?` → `bool`                           |
| **Multiple values** | Explicit `.stream()` or `.collect()` | `vault.list_resources(...).collect().await?` → `Vec<String>` |

**Why this distinction?**

- Single-value operations are always bounded and predictable
- Multi-value operations could return thousands of items - you must explicitly choose streaming vs collecting

```rust
// Single value: use IntoFuture, await directly
let allowed = vault.check("user:alice", "view", "doc:1").await?;

// Multiple values: must choose .stream() or .collect()
let docs = vault.list_resources("user:alice", "view").collect().await?;
// OR
let mut stream = vault.list_resources("user:alice", "view").stream();
```

### Explicit Stream API

```rust
use futures::{Stream, StreamExt, TryStreamExt};

// Returns impl Stream - NO buffering, NO collect
let stream = vault
    .list_resources("user:alice", "view")
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
    .list_resources("user:alice", "view")
    .collect()  // Loads ALL into memory
    .await?;

// ✅ True streaming: Process without buffering
let mut stream = vault
    .list_resources("user:alice", "view")
    .stream();

while let Some(resource) = stream.try_next().await? {
    // Process one at a time, constant memory
}

// ✅ Bounded collection: When you need a Vec but want limits
let resources = vault
    .list_resources("user:alice", "view")
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
    .list_resources("user:alice", "view")
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
    .list_resources("user:alice", "view")
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
let mut stream = vault.list_resources("user:alice", "view").stream();

while let Some(result) = stream.next().await {
    match result {
        Ok(resource) => process(resource),
        Err(e) if e.is_transient() => {
            // Network hiccup - stream may continue
            tracing::warn!("Transient error: {}", e);
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
    .grpc_config(GrpcConfig {
        max_message_size: 16 * 1024 * 1024,  // 16MB
        stream_window_size: 1024 * 1024,      // 1MB flow control window
        keep_alive_interval: Duration::from_secs(30),
    })
    .rest_config(RestConfig {
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
let mut stream = client
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
let mut stream = client
    .watch()
    .filter(WatchFilter::resource_type("document"))
    .filter(WatchFilter::operations([Operation::Create]))
    .from_revision(12345)  // Resume from checkpoint
    .run()
    .await?;
```

### Resumable Streams

```rust
// Automatic reconnection with resume
let stream = client
    .watch()
    .resumable()  // Auto-handles disconnects
    .run()
    .await?;
```

### Backpressure Handling

```rust
// Stream respects backpressure - server won't overwhelm client
let mut stream = client
    .list_resources("user:alice", "view")
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

---

## Caching

### Local Decision Cache

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .cache(CacheConfig::default()
        .max_entries(10_000)
        .ttl(Duration::from_secs(60))
        .negative_ttl(Duration::from_secs(10)))  // Cache denials shorter
    .build()
    .await?;
```

### Cache Invalidation via Watch

```rust
// Combined caching + watch for real-time consistency
let client = Client::builder()
    .url("https://api.inferadb.com")
    .cache(CacheConfig::default())
    .cache_invalidation(CacheInvalidation::Watch)  // Use watch stream
    .build()
    .await?;
```

---

## Vault Statistics

Understanding vault usage patterns with comprehensive statistics.

### Basic Statistics

```rust
// Get vault statistics
let stats = client.vault_stats(vault_id).await?;

println!("Total relationships: {}", stats.total_relationships);
println!("Entity types: {:?}", stats.entity_type_counts);
println!("Relation distribution: {:?}", stats.relation_counts);
println!("Last modified: {:?}", stats.last_modified);
```

### Statistics by Type

```rust
// Detailed breakdown by entity type
let stats = client
    .vault_stats(vault_id)
    .group_by(GroupBy::EntityType)
    .await?;

for (entity_type, count) in &stats.by_entity_type {
    println!("{}: {} relationships", entity_type, count);
}

// Breakdown by relation
let stats = client
    .vault_stats(vault_id)
    .group_by(GroupBy::Relation)
    .await?;

for (relation, count) in &stats.by_relation {
    println!("{}: {} relationships", relation, count);
}
```

### Historical Trends

```rust
// Get trends over time
let trends = client
    .vault_stats(vault_id)
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
```

---

## Bulk Operations

High-performance export and import operations for vault data.

### Export Relationships

```rust
// Stream export for memory efficiency
let mut stream = client
    .export(vault_id)
    .stream();

while let Some(batch) = stream.try_next().await? {
    for relationship in batch.relationships {
        process(relationship);
    }
}

// Collect all (use with caution for large vaults)
let export = client
    .export(vault_id)
    .collect()
    .await?;

println!("Exported {} relationships", export.relationships.len());
```

### Filtered Export

```rust
// Export specific entity types
let export = client
    .export(vault_id)
    .resource_types(&["document", "folder"])
    .subject_types(&["user", "group"])
    .collect()
    .await?;

// Export with time filter
let export = client
    .export(vault_id)
    .changed_since(timestamp)
    .collect()
    .await?;
```

### Export with Schema

```rust
// Include schema in export
let export = client
    .export(vault_id)
    .include_schema(true)
    .collect()
    .await?;

println!("Schema version: {}", export.schema.as_ref().unwrap().version);

// Include metadata (timestamps, actors)
let export = client
    .export(vault_id)
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
client
    .export(vault_id)
    .to_file("backup.json")
    .format(ExportFormat::JsonLines)
    .await?;

// With compression
client
    .export(vault_id)
    .to_file("backup.json.gz")
    .format(ExportFormat::JsonLines)
    .compress(Compression::Gzip)
    .await?;
```

### Import Relationships

```rust
// Import from file
let result = client
    .import(vault_id)
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

let result = client
    .import(vault_id)
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
let result = client
    .import(vault_id)
    .from_file("backup.json")
    .mode(ImportMode::Merge)
    .await?;

// Upsert mode - update existing
let result = client
    .import(vault_id)
    .from_file("backup.json")
    .mode(ImportMode::Upsert)
    .await?;

// Replace mode - full replacement (use with caution!)
let result = client
    .import(vault_id)
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

let result = client
    .import(vault_id)
    .from_file("backup.json")
    .on_conflict(ConflictResolution::Skip)
    .await?;

// With detailed conflict reporting
let result = client
    .import(vault_id)
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
let result = client
    .import(vault_id)
    .from_file("backup.json")
    .atomic(true)
    .await?;

// If any relationship fails, entire import is rolled back
```

### Async Import (Background Job)

```rust
// Start import as background job
let job = client
    .import(vault_id)
    .from_file("large-backup.json")
    .start_async()
    .await?;

println!("Import job started: {}", job.id);

// Check job status
loop {
    let status = client.import_status(&job.id).await?;

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
client.cancel_import(&job.id).await?;
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

The Control API handles organization and resource management.

### Common Operations

```rust
// Access control plane
let control = client.control();

// Most common operations
let orgs = control.organizations().list().await?;
let vault = control.vaults().create(CreateVault { name: "prod", .. }).await?;
let schema = control.vault(&vault.id).schemas().get_active().await?;
```

### API Hierarchy Convention

The Control API follows a consistent singular/plural resource pattern:

```text
client.control()
    .{resources}()           // Plural: collection operations (list, create)
    .{resource}(&id)         // Singular: instance operations (get, update, delete)
        .{sub_resources}()   // Plural: sub-collection operations
```

**Standard Pattern**: Plural for collections, singular when scoped to a specific instance:

```rust
// ✅ Correct: Plural for collections, singular for instances
control.organizations().list().await?;           // List all orgs
control.organization(&org_id).members().list().await?;  // Members of specific org
control.vaults().create(CreateVault { .. }).await?;     // Create in collection
control.vault(&vault_id).schemas().list().await?;       // Schemas of specific vault

// ❌ Avoid: Plural when working with specific instance
control.organizations(&org_id).get().await?;  // Should be .organization(&id)
```

### Complete Hierarchy

```rust
let control = client.control();

// Account (current user) - no ID needed
control.account().get().await?;
control.account().emails().list().await?;
control.account().sessions().list().await?;

// Organizations
control.organizations().list().await?;
control.organizations().create(CreateOrg { .. }).await?;
control.organization(&org_id).get().await?;
control.organization(&org_id).update(UpdateOrg { .. }).await?;
control.organization(&org_id).members().list().await?;
control.organization(&org_id).invitations().create(invite).await?;
control.organization(&org_id).teams().list().await?;

// Vaults (scoped to organization)
control.vaults().list().await?;  // All vaults user can access
control.vaults().create(CreateVault { org_id, .. }).await?;
control.vault(&vault_id).get().await?;
control.vault(&vault_id).schemas().list().await?;
control.vault(&vault_id).schemas().push(content).await?;
control.vault(&vault_id).tokens().list().await?;
control.vault(&vault_id).roles().list().await?;

// API Clients
control.clients().list().await?;
control.clients().create(CreateClient { .. }).await?;
control.client(&client_id).get().await?;
control.client(&client_id).certificates().list().await?;
control.client(&client_id).certificates().create(cert).await?;

// Audit Logs (scoped to organization)
control.organization(&org_id).audit_logs().list().await?;
control.organization(&org_id).audit_logs().actor(&user_id).list().await?;
```

---

## Account Management

Manage the authenticated user's account, emails, and sessions.

### Get Account Details

```rust
// Get current user information
let account = client.control().account().get().await?;

println!("User ID: {}", account.id);
println!("Name: {}", account.name);
println!("Email: {}", account.primary_email);
println!("Created: {}", account.created_at);
```

### Update Account

```rust
// Update account details
let updated = client
    .control()
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
let emails = client.control().account().emails().list().await?;

for email in &emails {
    println!("{} (primary: {}, verified: {})",
        email.address,
        email.is_primary,
        email.verified
    );
}

// Add a new email
let email = client
    .control()
    .account()
    .emails()
    .add("new@example.com")
    .await?;

println!("Verification sent to: {}", email.address);

// Verify email with token
client
    .control()
    .account()
    .emails()
    .verify(&email.id, verification_token)
    .await?;

// Set as primary
client
    .control()
    .account()
    .emails()
    .set_primary(&email.id)
    .await?;

// Remove email
client
    .control()
    .account()
    .emails()
    .remove(&email.id)
    .await?;
```

### Session Management

```rust
// List active sessions
let sessions = client.control().account().sessions().list().await?;

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
    .control()
    .account()
    .sessions()
    .revoke(&session_id)
    .await?;

// Revoke all other sessions (security measure)
let revoked = client
    .control()
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
    .control()
    .account()
    .password()
    .request_reset("user@example.com")
    .await?;

// Complete password reset with token
client
    .control()
    .account()
    .password()
    .reset(reset_token, "new_password")
    .await?;

// Change password (when logged in)
client
    .control()
    .account()
    .password()
    .change("current_password", "new_password")
    .await?;
```

### Delete Account

```rust
// Delete account (requires confirmation)
client
    .control()
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
// List organizations
let orgs = client.control().organizations().list().await?;

for org in &orgs {
    println!("{}: {} ({})", org.id, org.name, org.role);
}

// Create organization
let org = client
    .control()
    .organizations()
    .create(CreateOrganization {
        name: "Acme Corp".into(),
        slug: Some("acme".into()),
        ..Default::default()
    })
    .await?;

// Get organization details
let org = client
    .control()
    .organization(&org_id)
    .get()
    .await?;

// Update organization
let org = client
    .control()
    .organization(&org_id)
    .update(UpdateOrganization {
        name: Some("Acme Corporation".into()),
        ..Default::default()
    })
    .await?;

// Delete organization (requires owner role)
client
    .control()
    .organization(&org_id)
    .delete()
    .confirm("DELETE ACME")  // Safety confirmation
    .await?;
```

### Organization Lifecycle

```rust
// Suspend organization (admin only)
client
    .control()
    .organization(&org_id)
    .suspend()
    .reason("Billing issue")
    .await?;

// Resume suspended organization
client
    .control()
    .organization(&org_id)
    .resume()
    .await?;

// Leave organization (self-removal)
client
    .control()
    .organization(&org_id)
    .leave()
    .await?;
```

### Organization Members

```rust
// List members
let members = client
    .control()
    .organization(&org_id)
    .members()
    .list()
    .await?;

for member in &members {
    println!("{}: {} ({})", member.user_id, member.name, member.role);
}

// Update member role
client
    .control()
    .organization(&org_id)
    .member(&user_id)
    .update_role(OrganizationRole::Admin)
    .await?;

// Remove member
client
    .control()
    .organization(&org_id)
    .member(&user_id)
    .remove()
    .await?;
```

### Organization Invitations

```rust
// List pending invitations
let invitations = client
    .control()
    .organization(&org_id)
    .invitations()
    .list()
    .await?;

// Create invitation
let invitation = client
    .control()
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
    .control()
    .organization(&org_id)
    .invitation(&invitation.id)
    .resend()
    .await?;

// Delete (cancel) invitation
client
    .control()
    .organization(&org_id)
    .invitation(&invitation.id)
    .delete()
    .await?;

// Accept invitation (from invitee's perspective)
client
    .control()
    .invitation(invitation_token)
    .accept()
    .await?;

// Decline invitation
client
    .control()
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
    .control()
    .organization(&org_id)
    .roles()
    .list()
    .await?;

// Grant role
client
    .control()
    .organization(&org_id)
    .roles()
    .grant(&user_id, OrganizationRole::Admin)
    .await?;

// Update role
client
    .control()
    .organization(&org_id)
    .role(&user_id)
    .update(OrganizationRole::Member)
    .await?;

// Revoke role (remove from org)
client
    .control()
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
// List teams in organization
let teams = client
    .control()
    .organization(&org_id)
    .teams()
    .list()
    .await?;

for team in &teams {
    println!("{}: {} ({} members)",
        team.id,
        team.name,
        team.member_count
    );
}

// Create team
let team = client
    .control()
    .organization(&org_id)
    .teams()
    .create(CreateTeam {
        name: "Engineering".into(),
        description: Some("Engineering team".into()),
        ..Default::default()
    })
    .await?;

// Get team details
let team = client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .get()
    .await?;

// Update team
let team = client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .update(UpdateTeam {
        name: Some("Platform Engineering".into()),
        ..Default::default()
    })
    .await?;

// Delete team
client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .delete()
    .await?;
```

### Team Members

```rust
// List team members
let members = client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .members()
    .list()
    .await?;

// Add member to team
client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .members()
    .add(&user_id, TeamRole::Member)
    .await?;

// Update member's team role
client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .member(&user_id)
    .update_role(TeamRole::Lead)
    .await?;

// Remove member from team
client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .member(&user_id)
    .remove()
    .await?;
```

### Team Vault Grants

```rust
// List vault grants for team
let grants = client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .grants()
    .list()
    .await?;

for grant in &grants {
    println!("Vault {}: {} access", grant.vault_id, grant.role);
}

// Grant vault access to team
let grant = client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .grants()
    .create(CreateVaultGrant {
        vault_id: vault_id.into(),
        role: VaultRole::ReadWrite,
    })
    .await?;

// Update grant
client
    .control()
    .organization(&org_id)
    .team(&team_id)
    .grant(&grant.id)
    .update(VaultRole::Admin)
    .await?;

// Revoke grant
client
    .control()
    .organization(&org_id)
    .team(&team_id)
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

Manage authorization vaults with role-based access control.

### Vault CRUD

```rust
// List vaults (all vaults user has access to)
let vaults = client
    .control()
    .vaults()
    .list()
    .await?;

for vault in &vaults {
    println!("{}: {} ({})", vault.id, vault.name, vault.role);
}

// Create vault
let vault = client
    .control()
    .vaults()
    .create(CreateVault {
        name: "production".into(),
        organization_id: org_id.into(),
        description: Some("Production authorization data".into()),
        ..Default::default()
    })
    .await?;

// Get vault details
let vault = client
    .control()
    .vault(&vault_id)
    .get()
    .await?;

// Update vault
let vault = client
    .control()
    .vault(&vault_id)
    .update(UpdateVault {
        name: Some("prod-main".into()),
        ..Default::default()
    })
    .await?;

// Delete vault
client
    .control()
    .vault(&vault_id)
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

// List user role assignments
let assignments = client
    .control()
    .vault(&vault_id)
    .roles()
    .list()
    .await?;

// Grant role to user
client
    .control()
    .vault(&vault_id)
    .roles()
    .grant(&user_id, VaultRole::ReadWrite)
    .await?;

// Update role
client
    .control()
    .vault(&vault_id)
    .role(&assignment_id)
    .update(VaultRole::Admin)
    .await?;

// Revoke role
client
    .control()
    .vault(&vault_id)
    .role(&assignment_id)
    .revoke()
    .await?;
```

### Team Vault Roles

```rust
// List team role assignments
let team_assignments = client
    .control()
    .vault(&vault_id)
    .team_roles()
    .list()
    .await?;

// Grant role to team
client
    .control()
    .vault(&vault_id)
    .team_roles()
    .grant(&team_id, VaultRole::ReadWrite)
    .await?;

// Update team role
client
    .control()
    .vault(&vault_id)
    .team_role(&assignment_id)
    .update(VaultRole::ReadOnly)
    .await?;

// Revoke team role
client
    .control()
    .vault(&vault_id)
    .team_role(&assignment_id)
    .revoke()
    .await?;
```

### Vault Tokens

```rust
// List vault API tokens
let tokens = client
    .control()
    .vault(&vault_id)
    .tokens()
    .list()
    .await?;

for token in &tokens {
    println!("{}: {} (expires: {:?})",
        token.id,
        token.name,
        token.expires_at
    );
}

// Generate new token
let token = client
    .control()
    .vault(&vault_id)
    .tokens()
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
client
    .control()
    .vault(&vault_id)
    .token(&token.id)
    .revoke()
    .await?;

// Revoke all tokens (emergency)
client
    .control()
    .vault(&vault_id)
    .tokens()
    .revoke_all()
    .confirm(true)
    .await?;
```

### Vault Types

```rust
#[derive(Debug, Clone)]
pub struct Vault {
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
    pub scopes: Option<Vec<&'static str>>,
}
```

---

## Client Management

Manage API clients for machine-to-machine authentication.

### Client CRUD

```rust
// List clients
let clients = client.control().clients().list().await?;

for c in &clients {
    println!("{}: {} (active: {})", c.id, c.name, c.active);
}

// Create client
let api_client = client
    .control()
    .clients()
    .create(CreateClient {
        name: "backend-service".into(),
        description: Some("Main backend service".into()),
        ..Default::default()
    })
    .await?;

// Get client details
let api_client = client
    .control()
    .client(&client_id)
    .get()
    .await?;

// Update client
let api_client = client
    .control()
    .client(&client_id)
    .update(UpdateClient {
        name: Some("backend-api".into()),
        ..Default::default()
    })
    .await?;

// Delete client
client
    .control()
    .client(&client_id)
    .delete()
    .await?;
```

### Client Lifecycle

```rust
// Deactivate client (emergency disable)
client
    .control()
    .client(&client_id)
    .deactivate()
    .reason("Security incident")
    .await?;

// Reactivate client
client
    .control()
    .client(&client_id)
    .activate()
    .await?;
```

### Certificate Management

```rust
// List certificates for client
let certs = client
    .control()
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
let cert = client
    .control()
    .client(&client_id)
    .certificates()
    .create(CreateCertificate {
        name: "primary-key-2024".into(),
        public_key: public_key_pem.into(),
    })
    .await?;

// Get certificate details
let cert = client
    .control()
    .client(&client_id)
    .certificate(&cert_id)
    .get()
    .await?;

// Revoke certificate
client
    .control()
    .client(&client_id)
    .certificate(&cert_id)
    .revoke()
    .reason("Key rotation")
    .await?;

// Delete certificate (after revocation)
client
    .control()
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

#[derive(Debug, Clone)]
pub struct Certificate {
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
pub struct CreateCertificate {
    pub name: String,
    pub public_key: String,
}
```

---

## Schema Management

### Basic Usage

```rust
// Get active schema for a vault
let schema = client.control().vault(&vault_id).schemas().get_active().await?;

// Push new schema (validates automatically)
let version = client.control().vault(&vault_id).schemas()
    .push(schema_content)
    .message("Added team support")
    .await?;

// Activate the new version
client.control().vault(&vault_id).schemas().activate(&version.id).await?;
```

### Schema Introspection

```rust
// Get active schema
let schema = client.control().vault(&vault_id).schemas().get_active().await?;

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
// Get schema with version info
let schema = client.control().vault(&vault_id).schemas().get_active().await?;
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
// List all schema versions
let versions = client
    .control()
    .vault(&vault_id).schemas()
    .list()
    .await?;

for version in &versions {
    println!("{}: {} (active: {})",
        version.id,
        version.message.as_deref().unwrap_or("no message"),
        version.active
    );
}

// Include inactive versions
let all_versions = client
    .control()
    .vault(&vault_id).schemas()
    .list()
    .include_inactive(true)
    .await?;

// Get specific version
let schema = client
    .control()
    .vault(&vault_id).schemas()
    .get(&schema_id)
    .await?;

// Get currently active version
let active = client
    .control()
    .vault(&vault_id).schemas()
    .get_active()
    .await?;

// Compare two versions
let diff = client
    .control()
    .vault(&vault_id).schemas()
    .diff(&from_schema_id, &to_schema_id)
    .await?;

println!("Added entities: {:?}", diff.added_entities);
println!("Removed entities: {:?}", diff.removed_entities);
println!("Modified entities: {:?}", diff.modified_entities);
println!("Breaking changes: {}", diff.has_breaking_changes);
```

### Schema Lifecycle

```rust
// Push new schema version (without activating)
let version = client
    .control()
    .vault(&vault_id).schemas()
    .push(schema_content)
    .message("Added team support")
    .await?;

println!("Pushed version: {}", version.id);

// Activate a version
client
    .control()
    .vault(&vault_id).schemas()
    .activate(&version.id)
    .await?;

// Rollback to specific version
client
    .control()
    .vault(&vault_id).schemas()
    .rollback_to(&previous_version_id)
    .await?;

// Rollback to previous version
client
    .control()
    .vault(&vault_id).schemas()
    .rollback_to_previous()
    .await?;

// Copy schema to another vault
client
    .control()
    .vault(&vault_id).schemas()
    .copy_to(&target_vault_id)
    .schema(&schema_id)  // or .active() for active schema
    .activate(true)       // activate in target vault
    .await?;
```

### Canary Deployments

```rust
// Activate with canary deployment
client
    .control()
    .vault(&vault_id).schemas()
    .activate(&version.id)
    .canary(CanaryConfig {
        percentage: 10,  // Route 10% of traffic to new schema
        duration: Some(Duration::from_secs(30 * 60)),  // 30 min observation
    })
    .await?;

// Check canary status
let status = client
    .control()
    .vault(&vault_id).schemas()
    .canary_status()
    .await?;

println!("Canary percentage: {}%", status.percentage);
println!("Canary errors: {}", status.canary_metrics.error_count);
println!("Baseline errors: {}", status.baseline_metrics.error_count);

if status.has_anomalies {
    println!("Anomalies detected: {:?}", status.anomalies);
}

// Gradually increase canary percentage
client
    .control()
    .vault(&vault_id).schemas()
    .canary_adjust(25)  // Increase to 25%
    .await?;

// Promote canary to 100%
client
    .control()
    .vault(&vault_id).schemas()
    .canary_promote()
    .await?;

// Rollback canary (revert to baseline)
client
    .control()
    .vault(&vault_id).schemas()
    .canary_rollback()
    .await?;
```

### Pre-flight Checks

```rust
// Run pre-flight checks before activating
let preflight = client
    .control()
    .vault(&vault_id).schemas()
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
// List recent audit events
let events = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .list()
    .await?;

for event in &events {
    println!("[{}] {} performed {} on {}",
        event.timestamp,
        event.actor,
        event.action,
        event.resource
    );
}

// Filter by actor
let user_events = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .actor(&user_id)
    .list()
    .await?;

// Filter by action
let create_events = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .action("vault.create")
    .list()
    .await?;

// Filter by resource
let vault_events = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .resource_type("vault")
    .resource_id(&vault_id)
    .list()
    .await?;

// Time range filter
let recent_events = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .from(start_time)
    .to(end_time)
    .list()
    .await?;

// Combine filters
let filtered = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .actor(&user_id)
    .action("relationship.write")
    .from(start_time)
    .list()
    .await?;
```

### Stream Audit Logs

```rust
// Stream for large result sets
let mut stream = client
    .control()
    .organization(&org_id)
    .audit_logs()
    .from(start_time)
    .stream();

while let Some(event) = stream.try_next().await? {
    process_audit_event(event);
}

// Export to file
client
    .control()
    .organization(&org_id)
    .audit_logs()
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
    .auto_refresh(true)  // Automatically refresh before expiry
    .refresh_threshold(Duration::from_secs(60))  // Refresh 60s before expiry
    .on_token_refresh(|new_tokens| {
        // Save new tokens to secure storage
        save_tokens(new_tokens);
    })
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
}

/// gRPC status information (when using gRPC transport)
#[derive(Debug, Clone)]
pub struct GrpcStatus {
    pub code: i32,          // tonic::Code as i32
    pub message: String,
    pub details: Vec<u8>,   // Serialized google.rpc.Status details
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
    NoPath,                    // No relationship path exists
    ConditionFailed(String),   // ABAC condition failed
    Explicit,                  // Explicit deny rule
}
```

**Using Protocol Details**:

```rust
match vault.write(relationship).await {
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
    .retries(Retries::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10))
        .backoff_multiplier(2.0)
        .jitter(0.1))
    .build()
    .await?;
```

### Retriable Errors

| Error Kind           | Retriable | Notes                  |
| -------------------- | --------- | ---------------------- |
| `Timeout`            | Yes       | Network timeout        |
| `ConnectionFailed`   | Yes       | Connection dropped     |
| `ServiceUnavailable` | Yes       | Server overloaded      |
| `RateLimited`        | Yes       | With `retry_after`     |
| `ServerError`        | Maybe     | 5xx without body       |
| `Unauthorized`       | No        | Credentials invalid    |
| `Forbidden`          | No        | Permission denied      |
| `NotFound`           | No        | Resource doesn't exist |
| `InvalidInput`       | No        | Bad request            |

### Retry Budget

```rust
// Prevent retry storms under load
let client = Client::builder()
    .retries(Retries::default()
        .retry_budget(RetryBudget::new()
            .ttl(Duration::from_secs(10))
            .min_retries_per_second(10)
            .retry_ratio(0.1)))  // Max 10% retries
    .build()
    .await?;
```

### Idempotency-Aware Retry Policy

Mutations require different retry behavior than reads:

```rust
/// Retry policy that considers operation idempotency
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// For read operations (check, list, expand) - always safe to retry
    pub reads: RetryConfig,

    /// For idempotent writes (with request_id) - safe to retry
    pub idempotent_writes: RetryConfig,

    /// For non-idempotent writes - retry only on connection errors before send
    pub non_idempotent_writes: RetryConfig,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            reads: RetryConfig::default()
                .max_retries(3)
                .initial_backoff(Duration::from_millis(50)),

            idempotent_writes: RetryConfig::default()
                .max_retries(3)
                .initial_backoff(Duration::from_millis(100)),

            // Only retry if we know the request wasn't sent
            non_idempotent_writes: RetryConfig::default()
                .max_retries(1)
                .retry_on(RetryOn::ConnectionError)  // Not server errors
        }
    }
}
```

**How the SDK Determines Idempotency**:

```rust
// Automatically idempotent: reads
vault.check("user:alice", "view", "doc:1").await?;  // Safe to retry

// Idempotent by request_id: writes with explicit ID
vault
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(Uuid::new_v4())  // Server deduplicates by request_id
    .await?;  // Safe to retry with same request_id

// Non-idempotent: writes without request_id
vault
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    // No request_id - could create duplicates if retried incorrectly
    .await?;  // Only retry on connection errors before send
```

**Configuring Idempotency Behavior**:

```rust
let client = Client::builder()
    .retry_policy(RetryPolicy {
        reads: RetryConfig::default().max_retries(5),
        idempotent_writes: RetryConfig::default().max_retries(3),
        non_idempotent_writes: RetryConfig::disabled(),  // Never retry
    })
    .auto_request_id(true)  // All writes get request IDs automatically
    .build()
    .await?;
```

**Retry Decision Matrix**:

| Operation  | Has Request ID        | Error Type            | Retry?      |
| ---------- | --------------------- | --------------------- | ----------- |
| `check()`  | N/A                   | Any transient         | Yes         |
| `write()`  | Yes                   | Any transient         | Yes         |
| `write()`  | No                    | Connection (pre-send) | Yes         |
| `write()`  | No                    | Connection (mid-send) | No (unsafe) |
| `write()`  | No                    | Server error          | No (unsafe) |
| `delete()` | Inherently idempotent | Any transient         | Yes         |

---

## Graceful Degradation

### Fail-Open vs Fail-Closed

```rust
// Fail-closed (default, more secure)
let allowed = client
    .check("user:alice", "view", "document:readme")
    .on_error(OnError::Deny)  // Default
    .await
    .unwrap_or(false);

// Fail-open (for non-critical paths)
let allowed = client
    .check("user:alice", "view", "document:readme")
    .on_error(OnError::Allow)
    .await
    .unwrap_or(true);
```

### Circuit Breaker

```rust
use inferadb::resilience::CircuitBreaker;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .circuit_breaker(CircuitBreaker::default()
        .failure_threshold(5)
        .success_threshold(2)
        .timeout(Duration::from_secs(30)))
    .build()
    .await?;

// Check circuit state
if client.circuit_state() == CircuitState::Open {
    // Use fallback
    return Ok(cached_decision);
}
```

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

**Health Types**:

```rust
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
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
    Degraded,
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

```rust
// Full OTLP integration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .with_opentelemetry(OtelConfig {
        service_name: "my-service",
        endpoint: "http://otel-collector:4317",
    })
    .build()
    .await?;
```

### W3C Trace Context Propagation

Full W3C Trace Context standard support for distributed tracing across service boundaries:

```rust
use inferadb::tracing::{TraceContext, Propagator};

// Extract trace context from incoming request
let trace_context = TraceContext::extract_from_headers(&request.headers())?;

// Create vault-scoped trace context
let vault = client.access().vault("vlt_01JFQGK...").with_tracing(trace_context);

// All subsequent operations inherit the trace context
let allowed = vault.check("user:alice", "view", "doc:1").await?;
// Traces will show: service-a -> inferadb-sdk -> inferadb-api

// Manual trace context creation
let trace_context = TraceContext::new()
    .with_trace_id(TraceId::random())
    .with_span_id(SpanId::random())
    .with_sampled(true);
```

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
let mut stream = vault.list_resources("user:alice", "viewer").stream();
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

### Testing Trait Abstraction

All client types implement a common trait for testability:

```rust
/// Trait for authorization operations, implemented by all client types
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn check(&self, subject: &str, permission: &str, resource: &str) -> Result<bool, Error>;
    async fn check_batch(&self, checks: Vec<(&str, &str, &str)>) -> Result<Vec<bool>, Error>;
    async fn write(&self, relationship: Relationship) -> Result<(), Error>;
    async fn delete(&self, relationship: Relationship) -> Result<(), Error>;
    // ... other methods
}

// Implemented by:
impl AuthorizationClient for Client { /* real client */ }
impl AuthorizationClient for MockClient { /* mock client */ }
impl AuthorizationClient for InMemoryClient { /* in-memory client */ }
```

### Mock Client for Unit Tests

```rust
use inferadb::testing::MockClient;

#[tokio::test]
async fn test_document_access() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:alice", "edit", "doc:1", false)
        .check("user:bob", "view", "doc:1", false)
        .build();

    // Test your code
    assert!(mock.check("user:alice", "view", "doc:1").await.unwrap());
    assert!(!mock.check("user:alice", "edit", "doc:1").await.unwrap());
}
```

### In-Memory Client for Integration Tests

```rust
use inferadb::testing::InMemoryClient;

#[tokio::test]
async fn test_permission_inheritance() {
    let vault = InMemoryClient::with_schema(include_str!("schema.ipl"));

    // Seed data
    vault.write_batch([
        Relationship::new("folder:docs", "owner", "user:alice"),
        Relationship::new("doc:readme", "parent", "folder:docs"),
    ]).await.unwrap();

    // Test inheritance
    assert!(vault.check("user:alice", "view", "doc:readme").await.unwrap());
    assert!(vault.check("user:alice", "delete", "doc:readme").await.unwrap());
}
```

### Test Vault for Isolated Integration Tests

```rust
use inferadb::testing::TestVault;

#[tokio::test]
#[ignore]  // Requires running InferaDB
async fn integration_test() {
    let client = test_client().await;
    let vault = TestVault::create(&client).await.unwrap();

    // Tests run in isolated vault
    vault.write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());

    // Vault cleaned up on drop
}
```

For comprehensive testing patterns including the AuthzTest DSL, scenario testing, and snapshot testing, see [Testing Guide](docs/guides/testing.md).

---

## Integration Patterns

For comprehensive framework integration patterns, see [Integration Patterns Guide](docs/guides/integration-patterns.md), which covers:

- **Framework Extractors** - Axum/Actix permission extractors and attribute macros
- **Result Ergonomics** - `require()`, `then()`, `filter_authorized()` patterns
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
    .cache(CacheConfig::new()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_mins(5))
        .schema_ttl(Duration::from_hours(1)))
    .build()?;
```

---

## Multi-Organization Support

For multi-organization SaaS applications, see [Multi-Organization Guide](docs/guides/multi-tenant.md), which covers:

- Organization-scoped clients
- Framework middleware for organization extraction
- Cross-organization operations
- Organization isolation testing

---

## Time-Based Permissions

For temporal permission constraints, see [Temporal Permissions Guide](docs/guides/temporal-permissions.md), which covers:

- Expiring permissions
- Scheduled permissions
- Time-windowed access
- Time-aware queries

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
vault.write(Relationship::typed(&doc, "viewer", &user)).await?;
```

### Generic Type Constraints

```rust
impl VaultAccess {
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
└─► Otherwise: Use defaults (gRPC + REST fallback)
```

### Streaming Behavior

| Operation          | gRPC                       | REST                    |
| ------------------ | -------------------------- | ----------------------- |
| `check()`          | Unary                      | POST                    |
| `check_batch()`    | Bidirectional stream       | SSE stream              |
| `list_resources()` | Server stream              | SSE stream              |
| `list_subjects()`  | Server stream              | SSE stream              |
| `watch()`          | Server stream (continuous) | SSE stream (continuous) |
| `write_batch()`    | Client stream              | POST                    |

### Transport Escape Hatches

For advanced scenarios requiring custom transport configuration, the SDK exposes escape hatches to the underlying HTTP clients:

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

# Optional features
tracing = ["tracing", "tracing-futures"]
metrics = ["metrics", "metrics-exporter-prometheus"]
test-utils = ["inferadb-test"]
opentelemetry = ["opentelemetry", "tracing-opentelemetry"]
blocking = ["tokio/rt"]
derive = ["inferadb-macros"]
serde = ["dep:serde", "chrono/serde", "uuid/serde"]
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

/// Private key wrapper that zeroizes on drop
#[derive(ZeroizeOnDrop)]
pub struct Ed25519PrivateKey {
    // Inner bytes are zeroized when struct is dropped
    inner: [u8; 32],
}

impl Ed25519PrivateKey {
    pub fn from_pem(pem: &[u8]) -> Result<Self, Error> {
        // Parse PEM, copy into zeroizing container
        let bytes = parse_ed25519_private_key(pem)?;
        Ok(Self { inner: bytes })
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

| Data Type           | Zeroize on Drop    | Redacted in Logs            | Memory Protection          |
| ------------------- | ------------------ | --------------------------- | -------------------------- |
| `Ed25519PrivateKey` | Yes                | Yes                         | Stack-only, no heap copies |
| `AccessToken`       | Yes                | Yes (via `Secret`)          | Heap, single location      |
| `ClientCredentials` | Yes (contains key) | Partial (client_id visible) | Key protected              |
| `RefreshToken`      | Yes                | Yes                         | Heap, single location      |

### Key ID (kid) Derivation

How the SDK determines the `kid` claim for JWT signing:

```rust
/// Key ID derivation strategy
pub enum KeyIdStrategy {
    /// Use explicit certificate_id from ClientCredentials
    Explicit(String),

    /// Derive from public key (default)
    /// kid = base64url(sha256(public_key_bytes)[0..8])
    DeriveFromPublicKey,

    /// Fetch from JWKS endpoint (/.well-known/jwks.json)
    FetchFromJwks,
}

impl ClientCredentials {
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
let creds = ClientCredentials {
    client_id: "my_service".into(),
    private_key: Ed25519PrivateKey::from_pem_file("key.pem")?,
    certificate_id: Some("key-2024-01".into()),  // Explicit kid
};

// Auto-derived kid (convenient for development)
let creds = ClientCredentials {
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
    /// # Panics (in production builds)
    ///
    /// When compiled with `cfg(not(debug_assertions))` (release mode),
    /// this method will panic unless INFERADB_ALLOW_INSECURE=1 is set.
    pub fn insecure(mut self) -> Self {
        #[cfg(not(debug_assertions))]
        {
            if std::env::var("INFERADB_ALLOW_INSECURE").as_deref() != Ok("1") {
                panic!(
                    "insecure() called in release build without INFERADB_ALLOW_INSECURE=1. \
                     This is a security risk and is disabled by default."
                );
            }
        }

        tracing::warn!(
            target: "inferadb::security",
            "TLS verification disabled - this is insecure!"
        );

        self.tls_config = TlsConfig::Insecure;
        self
    }
}
```

**Compile-time Detection**:

```rust
// Feature flag to completely remove insecure() from release builds
#[cfg(feature = "insecure")]
pub fn insecure(mut self) -> Self { /* ... */ }

// Without the feature, method doesn't exist
// Cargo.toml:
// [features]
// insecure = []  # Must be explicitly enabled
```

### Input Validation

- Validate all inputs before sending
- Sanitize entity IDs (no injection attacks)
- Limit request sizes

### Audit Requirements

- Log all authorization decisions (configurable)
- Include request IDs for traceability
- Support compliance logging formats
