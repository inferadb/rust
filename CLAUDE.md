# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Official Rust SDK for InferaDB authorization service. Provides ergonomic, type-safe access to InferaDB's authorization and management APIs with support for both gRPC and REST transports.

## Build Commands

```bash
make setup           # One-time setup (installs Rust toolchain + nightly)
make build           # Build all workspace crates
make test            # Unit tests only
make test-all        # Unit + integration tests (requires dev environment)
make check           # Format check + clippy
make ci              # Full CI pipeline
make proto           # Regenerate protobuf code and format
make coverage        # Run tests with coverage report
```

Single test: `cargo test <test_name>`

Integration tests require a running InferaDB instance from [inferadb/deploy](https://github.com/inferadb/deploy).

## Architecture

### Client Hierarchy

```
Client → OrganizationClient → VaultClient
```

- **Client**: Top-level, manages connections and authentication
- **OrganizationClient**: Organization-scoped operations
- **VaultClient**: Authorization operations (check, relationships, watch)

### Module Structure

- `src/client/` - Client builder (typestate pattern) and hierarchy
- `src/vault/` - VaultClient with check, explain, simulate, watch operations
- `src/transport/` - gRPC (tonic) and REST (reqwest) implementations
- `src/control/` - Control plane API (organizations, vaults, members, teams, schemas, audit)
- `src/auth/` - Authentication (ClientCredentials, Bearer, Ed25519 keys)
- `src/testing/` - MockClient (expectations), InMemoryClient (graph semantics), AuthorizationClient trait
- `src/error/` - Error types where denial is `Ok(false)`, not `Err`

### Key Conventions

- **Argument order**: `check(subject, permission, resource)` - "Can subject do X to resource?"
- **Relationship order**: `Relationship::new(resource, relation, subject)` - "resource has relation subject"
- **Denial is not an error**: `check()` returns `Ok(false)` for denied access; use `require()` for `Err(AccessDenied)`

### Feature Flags

| Feature    | Default | Description                      |
| ---------- | ------- | -------------------------------- |
| `grpc`     | Yes     | gRPC transport via tonic         |
| `rest`     | Yes     | REST transport via reqwest       |
| `rustls`   | Yes     | Pure-Rust TLS                    |
| `insecure` | No      | Allow HTTP for local dev         |
| `derive`   | No      | Proc macros for Resource/Subject |

### Code Generation

Protobuf code is committed to `src/transport/proto/`. Regenerate with `make proto` when:

- `proto/inferadb.proto` is updated
- Upgrading tonic/prost versions

### Style

- Format with `cargo +nightly fmt --all`
- Lint with `cargo clippy --workspace --all-targets -- -D warnings`
- No `unwrap()`/`expect()` in library code (tests only)
- All public items require documentation
