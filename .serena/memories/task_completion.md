# InferaDB Rust SDK - Task Completion Checklist

## Before Marking a Task Complete

Run the full CI check:

```bash
make ci
```

This runs:
1. Format check (`make fmt-check`)
2. Clippy linting (`make clippy`)
3. Unit tests (`make test`)
4. Documentation check (`make doc-check`)

## Individual Checks

### 1. Format Code

```bash
make fmt
```

### 2. Run Linting

```bash
make clippy
```

Clippy treats all warnings as errors (`-D warnings`).

### 3. Run Tests

```bash
# Unit tests
make test

# If you modified integration test-related code
make test-integration  # requires local InferaDB
```

### 4. Check Documentation

```bash
make doc-check
```

All public items must be documented. This catches missing or broken documentation.

## PR Checklist

- [ ] CI passes (`make ci`)
- [ ] Tests pass (`make test`)
- [ ] No clippy warnings (`make clippy`)
- [ ] Code formatted (`make fmt`)
- [ ] Documentation updated for public API changes
- [ ] CHANGELOG.md updated (under `[Unreleased]`)
- [ ] Version bumped if needed (for breaking changes)

## Notes

- Formatting requires nightly: `cargo +nightly fmt`
- Generated protobuf code (`src/transport/proto/`) is committed to the repo
- If proto files changed, run `make proto` to regenerate
