# Contributing to InferaDB Rust SDK

Thank you for your interest in contributing to the InferaDB Rust SDK! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.70+ (stable)
- Rust nightly (for formatting)
- Docker (for integration tests)

### Getting Started

```bash
# Clone the SDK repository
git clone https://github.com/inferadb/rust-sdk
cd rust-sdk

# Install dependencies
cargo build --workspace

# Run tests
cargo nextest run --workspace

# Or with standard cargo test
cargo test --workspace
```

### Running with Local InferaDB

```bash
# Start local InferaDB
docker-compose up -d

# Run integration tests
INFERADB_URL=http://localhost:8080 cargo test --features integration
```

## Code Style

We use Rust's standard formatting and linting tools:

```bash
# Format code (requires nightly)
cargo +nightly fmt --all

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Check documentation
cargo doc --workspace --no-deps
```

### Style Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` defaults (no custom configuration)
- All public items must have documentation
- No `unwrap()` or `expect()` in library code (except in tests)
- Prefer `?` operator over explicit `match` for error handling

## Testing Guidelines

### Unit Tests

Use mocks for unit tests to avoid network dependencies:

```rust
use inferadb_test::MockClient;

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
cargo test --ignored --features integration
```

### Test Organization

```
tests/
├── unit/           # Unit tests with mocks
├── integration/    # Tests requiring InferaDB instance
└── fixtures/       # Shared test data and helpers
```

## Pull Request Process

### Before Submitting

1. **Run the full test suite:**
   ```bash
   cargo nextest run --workspace
   ```

2. **Check for clippy warnings:**
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. **Format your code:**
   ```bash
   cargo +nightly fmt --all
   ```

4. **Build documentation:**
   ```bash
   cargo doc --workspace --no-deps
   ```

### PR Checklist

- [ ] Tests pass (`cargo nextest run --workspace`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo +nightly fmt --all`)
- [ ] Documentation updated for public API changes
- [ ] CHANGELOG.md updated (under `[Unreleased]`)
- [ ] Version bumped if needed (for breaking changes)

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
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
2. Update MIGRATION.md with upgrade instructions
3. Bump major version (or minor for 0.x)

## Reporting Issues

File issues at: https://github.com/inferadb/rust-sdk/issues

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

- **Questions:** Open a [Discussion](https://github.com/inferadb/rust-sdk/discussions)
- **Bugs:** Open an [Issue](https://github.com/inferadb/rust-sdk/issues)
- **Security:** Email security@inferadb.com (do not open public issues)

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
