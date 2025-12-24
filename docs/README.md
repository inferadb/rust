# InferaDB Rust SDK Documentation

Supplementary documentation for the InferaDB Rust SDK. For the main README and quickstart, see the [project root](../README.md).

## Getting Started

| Guide                                                  | Description                                            |
| ------------------------------------------------------ | ------------------------------------------------------ |
| [Installation](guides/installation.md)                 | Feature flags, optimized builds, TLS options, MSRV     |
| [Authentication](guides/authentication.md)             | Client credentials, bearer tokens, key management      |
| [Authorization API](guides/authorization-api.md)       | Permission checks, relationships, lookups, watch       |
| [Integration Patterns](guides/integration-patterns.md) | Framework integration (Axum, Actix-web, GraphQL, gRPC) |
| [Error Handling](guides/errors.md)                     | Error types, retries, graceful degradation             |
| [Testing](guides/testing.md)                           | MockClient, InMemoryClient, TestVault                  |

## Schema & Design

| Guide                                                        | Description                                    |
| ------------------------------------------------------------ | ---------------------------------------------- |
| [Schema Design Patterns](guides/schema-design.md)            | Role hierarchies, resource hierarchies, groups |
| [Authorization Scenarios](guides/authorization-scenarios.md) | Multi-tenant SaaS, document sharing, API keys  |
| [Schema Versioning](guides/schema-versioning.md)             | Safe schema updates, migrations, rollback      |

## Core Concepts

| Guide                                        | Description                                  |
| -------------------------------------------- | -------------------------------------------- |
| [Consistency & Watch](guides/consistency.md) | Consistency tokens, real-time change streams |
| [Caching](guides/caching.md)                 | Cache configuration, TTL, invalidation       |

## Management

| Guide                                   | Description                                        |
| --------------------------------------- | -------------------------------------------------- |
| [Management API](guides/management-api.md) | Organizations, vaults, schemas, members, audit     |
| [Advanced Features](guides/advanced.md) | Simulation, explain, export/import, type-safe APIs |

## Production

| Guide                                                  | Description                          |
| ------------------------------------------------------ | ------------------------------------ |
| [Observability](guides/observability.md)               | Tracing, metrics, OpenTelemetry      |
| [Performance Tuning](guides/performance-tuning.md)     | Connection pools, batching, latency  |
| [Production Checklist](guides/production-checklist.md) | Pre-deployment verification          |
| [Debugging Authorization](guides/debugging.md)         | Explain API, traces, troubleshooting |

## Reference

| Document                              | Description                 |
| ------------------------------------- | --------------------------- |
| [Troubleshooting](troubleshooting.md) | Common issues and solutions |

## Internal

| Document                                                 | Description                                    |
| -------------------------------------------------------- | ---------------------------------------------- |
| [SDK Development](internal/SDK_DEVELOPMENT.md)           | Complete API design document (source of truth) |
| [Agentic Development](internal/AGENTIC_DEVELOPMENT.md)   | Token-efficient AI agent reference             |
| [Competitive Analysis](internal/competitive-analysis.md) | SDK comparison with SpiceDB, OpenFGA, Oso      |

## Quick Links

- **[README.md](../README.md)** - Quickstart and installation
- **[CHANGELOG.md](../CHANGELOG.md)** - Version history
