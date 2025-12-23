# Contributing to InferaDB Rust SDK

Thank you for your interest in contributing to the InferaDB Rust SDK! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- [Mise](https://mise.jdx.dev/) - Tool version manager (recommended)
- Rust 1.75+ (stable) - Managed by mise or rustup
- Rust nightly (for formatting)
- Docker and Docker Compose (for integration tests)
- [inferadb/deploy](https://github.com/inferadb/deploy) - Development environment (for integration tests)

### Getting Started

```bash
# Clone the SDK repository
git clone https://github.com/inferadb/rust
cd rust-sdk

# One-time setup (installs Rust toolchain and dev tools)
make setup

# Build the project
make build

# Run tests
make test

# See all available commands
make help
```

### Makefile Targets

| Target          | Description                             |
| --------------- | --------------------------------------- |
| `make setup`    | One-time dev environment setup via Mise |
| `make build`    | Build all workspace crates              |
| `make test`     | Run unit tests                          |
| `make test-all` | Run unit + integration tests            |
| `make check`    | Run format check + clippy               |
| `make coverage` | Run tests with coverage report          |
| `make doc`      | Build documentation                     |
| `make proto`    | Regenerate protobuf code and format     |
| `make ci`       | Full CI pipeline (format, lint, test)   |

### Running with Local InferaDB

Integration tests require a running InferaDB instance. Use the official development environment from [inferadb/deploy](https://github.com/inferadb/deploy):

```bash
# Clone the deploy repository (if you haven't already)
git clone https://github.com/inferadb/deploy
cd deploy

# Start the development environment
./scripts/dev-up.sh

# The environment includes:
# - InferaDB Engine (authorization API)
# - InferaDB Control (management API)
# - FoundationDB (storage)
# - Supporting services
```

Once the development environment is running, return to the SDK directory and run integration tests:

```bash
cd /path/to/inferadb-rust-sdk

# Run integration tests against local InferaDB
make test-integration

# Or run specific integration tests
cargo test --test integration --features "rest,insecure" -- --ignored
```

#### Environment Variables

The integration tests use these environment variables (with defaults for local development):

| Variable               | Default                 | Description                                 |
| ---------------------- | ----------------------- | ------------------------------------------- |
| `INFERADB_URL`         | `http://localhost:8080` | InferaDB Engine URL                         |
| `INFERADB_CONTROL_URL` | `http://localhost:8081` | InferaDB Control URL                        |
| `INFERADB_CLIENT_ID`   | -                       | Test client ID (created by dev environment) |
| `INFERADB_PRIVATE_KEY` | -                       | Path to test private key                    |

The dev environment from `inferadb/deploy` automatically configures test credentials.

## Code Style

We use Rust's standard formatting and linting tools. Use the Makefile for convenience:

```bash
# Format code (requires nightly)
make fmt

# Lint with clippy
make clippy

# Run all checks (format + clippy)
make check

# Build documentation
make doc
```

### Style Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` defaults (no custom configuration)
- All public items must have documentation
- No `unwrap()` or `expect()` in library code (except in tests)
- Prefer `?` operator over explicit `match` for error handling

## gRPC Code Generation

The SDK uses [tonic](https://github.com/hyperium/tonic) for gRPC support. Protobuf definitions are in `proto/inferadb.proto` and generated Rust code lives in `src/transport/proto/`.

### When to Regenerate

Regenerate protobuf code when:

- The `proto/inferadb.proto` file is updated
- Upgrading tonic or prost versions
- Generated code gets out of sync

### How to Regenerate

```bash
make proto
```

This command:

1. Touches the proto file to trigger regeneration
2. Runs `cargo build --features grpc` to invoke tonic-build
3. Formats the generated code with `make fmt`

### Notes

- Generated code is committed to the repository for reproducible builds
- The `build.rs` script handles code generation via tonic-build
- Only the gRPC client is generated (no server code)

## Testing Guidelines

### Unit Tests

Use mocks for unit tests to avoid network dependencies:

```rust
use inferadb::testing::MockClient;

#[tokio::test]
async fn test_check_returns_true_for_allowed() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .build();

    assert!(mock.check("user:alice", "view", "doc:1").await.unwrap());
}

#[tokio::test]
async fn test_check_returns_false_for_denied() {
    let mock = MockClient::builder()
        .check("user:bob", "delete", "doc:1", false)
        .build();

    assert!(!mock.check("user:bob", "delete", "doc:1").await.unwrap());
}
```

### Integration Tests

Integration tests require a running InferaDB instance. See [Running with Local InferaDB](#running-with-local-inferadb) for setup instructions using the [inferadb/deploy](https://github.com/inferadb/deploy) development environment.

Use `#[ignore]` for tests that require a running InferaDB instance:

```rust
#[tokio::test]
#[ignore]  // Run with: cargo test --ignored
async fn integration_test_check() {
    let client = test_client().await;
    let vault = TestVault::create(&client).await.unwrap();

    vault.write(Relationship::new("doc:1", "viewer", "user:alice")).await.unwrap();
    assert!(vault.check("user:alice", "view", "doc:1").await.unwrap());
}
```

Run integration tests with:

```bash
# Ensure inferadb/deploy dev environment is running first
make test-integration
```

### Test Organization

```text
tests/
├── unit/           # Unit tests with mocks
├── integration/    # Tests requiring InferaDB instance
└── fixtures/       # Shared test data and helpers
```

## Pull Request Process

### Before Submitting

Run the full CI pipeline locally:

```bash
make ci
```

This runs format checks, clippy, tests, and documentation checks.

Or run individual steps:

```bash
make fmt        # Format code
make check      # Format check + clippy
make test       # Run tests
make doc-check  # Check documentation
```

### PR Checklist

- [ ] CI passes (`make ci`)
- [ ] Tests pass (`make test`)
- [ ] No clippy warnings (`make clippy`)
- [ ] Code formatted (`make fmt`)
- [ ] Documentation updated for public API changes
- [ ] CHANGELOG.md updated (under `[Unreleased]`)
- [ ] Version bumped if needed (for breaking changes)

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat: add batch check support
fix: resolve token refresh race condition
docs: update authentication examples
refactor: simplify error handling
test: add integration tests for watch streams
chore: update dependencies
```

### Breaking Changes

For breaking changes:

1. Add `BREAKING CHANGE:` footer to commit message
2. Document upgrade instructions in CHANGELOG.md
3. Bump major version (or minor for 0.x)

## Reporting Issues

File issues at: <https://github.com/inferadb/rust/issues>

### Bug Reports

Include:

- SDK version (`cargo pkgid inferadb`)
- Rust version (`rustc --version`)
- Operating system and version
- Minimal reproduction code
- Full error message with request ID if available
- Expected vs actual behavior

### Feature Requests

Include:

- Use case description
- Proposed API (if applicable)
- Alternative solutions considered

## Getting Help

- **Questions:** Open a [Discussion](https://github.com/inferadb/rust/discussions)
- **Bugs:** Open an [Issue](https://github.com/inferadb/rust/issues)
- **Security:** Email <security@inferadb.com> (do not open public issues)

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
