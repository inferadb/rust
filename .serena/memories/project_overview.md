# InferaDB Rust SDK - Project Overview

## Purpose

Official Rust SDK for InferaDB - a distributed, Google Zanzibar-inspired authorization engine. Provides ergonomic, type-safe access to InferaDB's authorization and management APIs.

Published to crates.io as `inferadb`.

## Key Features

- **Rust-Native & Async**: Built on Tokio, integrates with Tracing
- **Compile-Time Safety**: Catch permission model mistakes during build
- **Standards-Based**: Built on AuthZEN with multi-tenant isolation
- **Multiple Transports**: gRPC and REST support
- **Testing Utilities**: MockClient, InMemoryClient, TestVault

## Tech Stack

- **Language**: Rust (MSRV 1.88.0)
- **Async Runtime**: Tokio
- **gRPC**: Tonic + Prost
- **HTTP/REST**: Reqwest
- **Cryptography**: ed25519-dalek (Ed25519 signing)
- **TLS**: Rustls (default) or native-tls
- **Error Handling**: thiserror
- **Serialization**: serde + serde_json
- **Testing**: tokio-test, wiremock, proptest

## Workspace Structure

```
inferadb/              # Main SDK crate
├── src/               # Source code
├── tests/             # Integration tests
├── docs/              # Documentation guides
└── examples/          # Example code

inferadb-derive/       # Derive macros crate (optional)
examples/              # Standalone examples package
proto/                 # Protobuf definitions (git submodule)
```

## Core Modules

- `src/client/` - Client builder and core implementation
- `src/vault/` - Authorization API (check, relationships, lookups, watch)
- `src/control/` - Management API (orgs, vaults, schemas, members)
- `src/auth/` - Authentication (Ed25519 keys, JWT, credentials)
- `src/transport/` - gRPC and REST transport layers
- `src/testing/` - Test utilities (MockClient, InMemoryClient)
- `src/types/` - Core types (Entity, Relationship, Decision, etc.)
- `src/error/` - Error types and handling

## Documentation

- API Reference: https://docs.rs/inferadb
- Guides in `docs/guides/`:
  - Installation
  - Authentication
  - Authorization API
  - Integration Patterns
  - Error Handling
  - Testing
  - Schema Design
  - Production Checklist

## Dependencies & Tooling

- **Mise**: Tool version manager (optional but recommended)
- **Makefile**: Convenient commands for development
- **rust-toolchain.toml**: Stable Rust with rustfmt, clippy, rust-analyzer
- **cargo-llvm-cov**: Code coverage tool
