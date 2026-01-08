# InferaDB Rust SDK - Task Completion Checklist

## Before Marking a Task Complete

Run the full CI check sequence:

```bash
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --lib
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

## Individual Checks

### 1. Format Code

```bash
cargo +nightly fmt --all
```

### 2. Run Linting

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Clippy treats all warnings as errors (`-D warnings`).

### 3. Run Tests

```bash
# Unit tests
cargo test --lib

# If you modified integration test-related code
cargo test --test integration  # requires local InferaDB
```

### 4. Check Documentation

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

All public items must be documented. This catches missing or broken documentation.

## PR Checklist

- [ ] Format check passes (`cargo +nightly fmt --all -- --check`)
- [ ] No clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
- [ ] Tests pass (`cargo test --lib`)
- [ ] Documentation check passes (`RUSTDOCFLAGS="-D warnings" cargo doc`)
- [ ] Code formatted (`cargo +nightly fmt --all`)
- [ ] Documentation updated for public API changes
- [ ] CHANGELOG.md updated (under `[Unreleased]`)
- [ ] Version bumped if needed (for breaking changes)

## Notes

- Formatting requires nightly: `cargo +nightly fmt`
- Generated protobuf code (`src/transport/proto/`) is committed to the repo
- If proto files changed, regenerate with: `cargo build --features grpc`
