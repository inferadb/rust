# InferaDB Rust SDK - Development Commands

## Quick Reference

| Command | Description |
|---------|-------------|
| `make help` | Show all available make targets |
| `make build` | Build the project with all features |
| `make test` | Run unit tests |
| `make check` | Run format check + clippy linting |
| `make fmt` | Format code (requires nightly) |
| `make ci` | Full CI pipeline (format, lint, test, doc-check) |

## Build Commands

```bash
# Build all workspace crates with all features
make build
# Or directly:
cargo build --workspace --all-features
```

## Test Commands

```bash
# Unit tests only
make test
# Or: cargo test --lib

# Integration tests (requires local InferaDB via inferadb/deploy)
make test-integration
# Or: cargo test --test integration

# All tests
make test-all
# Or: cargo test --lib --test integration
```

## Code Quality

```bash
# Format code (requires nightly toolchain)
make fmt
# Or: cargo +nightly fmt --all

# Check formatting without modifying
make fmt-check
# Or: cargo +nightly fmt --all -- --check

# Run clippy linter
make clippy
# Or: cargo clippy --workspace --all-targets -- -D warnings

# Both format check and clippy
make check
```

## Documentation

```bash
# Build docs
make doc
# Or: cargo doc --workspace --no-deps

# Build and open in browser
make doc-open
# Or: cargo doc --workspace --no-deps --open

# Check docs for warnings
make doc-check
# Or: RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Coverage

```bash
# Run tests with coverage
make coverage
# Or: cargo llvm-cov --lib --ignore-filename-regex 'proto|inferadb\.authorization\.v1'

# Generate HTML coverage report
make coverage-html
```

## Code Generation

```bash
# Regenerate protobuf code from proto/inferadb.proto
make proto
```

## Setup & CI

```bash
# One-time development setup (installs toolchain and tools)
make setup

# Full CI pipeline
make ci
```

## Running Examples

```bash
cargo run -p inferadb-examples --bin basic_check
cargo run -p inferadb-examples --bin batch_operations
cargo run -p inferadb-examples --bin axum_middleware
```

## System Utilities (Darwin/macOS)

Standard Unix utilities available: `git`, `ls`, `cd`, `grep`, `find`
Note: Some GNU flags may not be available on macOS (BSD versions).
