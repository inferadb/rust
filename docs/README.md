# InferaDB Rust SDK Documentation

Supplementary documentation for the InferaDB Rust SDK. For the main README and quickstart, see the [project root](../README.md).

## Getting Started

| Guide                                                  | Description                                            |
| ------------------------------------------------------ | ------------------------------------------------------ |
| [Integration Patterns](guides/integration-patterns.md) | Framework integration (Axum, Actix-web, GraphQL, gRPC) |
| [Error Handling](guides/errors.md)                     | Error types, `require()` pattern, retries              |
| [Testing](guides/testing.md)                           | MockClient, InMemoryClient, TestVault                  |
| [Migration Guide](guides/migration.md)                 | Migrate from RBAC, SpiceDB, OpenFGA, Oso               |

## Schema & Design

| Guide                                                          | Description                                       |
| -------------------------------------------------------------- | ------------------------------------------------- |
| [Schema Design Patterns](guides/schema-design.md)              | Role hierarchies, resource hierarchies, groups    |
| [Authorization Scenarios](guides/authorization-scenarios.md)   | Multi-tenant SaaS, document sharing, API keys     |
| [Schema Versioning](guides/schema-versioning.md)               | Safe schema updates, migrations, rollback         |

## Core Concepts

| Guide                                        | Description                                  |
| -------------------------------------------- | -------------------------------------------- |
| [Consistency & Watch](guides/consistency.md) | Consistency tokens, real-time change streams |
| [Caching](guides/caching.md)                 | Cache configuration, TTL, invalidation       |

## Management

| Guide                                   | Description                                        |
| --------------------------------------- | -------------------------------------------------- |
| [Control API](guides/control-api.md)    | Organizations, vaults, schemas, members, audit     |
| [Advanced Features](guides/advanced.md) | Simulation, explain, export/import, type-safe APIs |

## Production

| Guide                                                  | Description                              |
| ------------------------------------------------------ | ---------------------------------------- |
| [Performance Tuning](guides/performance-tuning.md)     | Connection pools, batching, latency      |
| [Production Checklist](guides/production-checklist.md) | Pre-deployment verification              |
| [Debugging Authorization](guides/debugging.md)         | Explain API, traces, troubleshooting     |

## Reference

| Document                              | Description                 |
| ------------------------------------- | --------------------------- |
| [Troubleshooting](troubleshooting.md) | Common issues and solutions |

## Internal

| Document                                                 | Description                               |
| -------------------------------------------------------- | ----------------------------------------- |
| [Competitive Analysis](internal/competitive-analysis.md) | SDK comparison with SpiceDB, OpenFGA, Oso |

## Quick Links

- **[README.md](../README.md)** - Quickstart and installation
- **[SDK Development.md](../SDK%20Development.md)** - Complete API design document
- **[CHANGELOG.md](../CHANGELOG.md)** - Version history
- **[MIGRATION.md](../MIGRATION.md)** - Upgrade guides
