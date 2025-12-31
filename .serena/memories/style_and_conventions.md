# InferaDB Rust SDK - Style & Conventions

## Formatting

- Uses `rustfmt` with nightly toolchain
- Run: `make fmt` or `cargo +nightly fmt --all`
- Configuration in `.rustfmt.toml`:
  - `style_edition = "2024"`
  - `group_imports = "StdExternalCrate"` (std, then external, then crate)
  - `imports_granularity = "Crate"` (merge imports by crate)
  - `wrap_comments = true`
  - `normalize_comments = true`
  - `use_small_heuristics = "MAX"`

## Rust Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- MSRV: Rust 1.88.0+ (stable)
- All public items must have documentation
- No `unwrap()` or `expect()` in library code (only in tests)
- Prefer `?` operator over explicit `match` for error handling
- Use `thiserror` for error types

## Linting

- Clippy is mandatory: `make clippy`
- All clippy warnings treated as errors: `-D warnings`
- No lint suppressions without justification

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add batch check support
fix: resolve token refresh race condition
docs: update authentication examples
refactor: simplify error handling
test: add integration tests for watch streams
chore: update dependencies
```

For breaking changes, add `BREAKING CHANGE:` footer.

## Project Structure

```
src/
├── lib.rs              # Library entry point
├── prelude.rs          # Common re-exports
├── client/             # Client implementation
│   ├── builder.rs      # Client builder pattern
│   ├── inner.rs        # Core client logic
│   └── health.rs       # Health checking
├── vault/              # Authorization API operations
│   ├── client.rs       # Vault client
│   ├── watch.rs        # Real-time updates
│   ├── explain.rs      # Permission explanations
│   └── simulate.rs     # Policy simulation
├── control/            # Management API
│   ├── organizations.rs
│   ├── vaults.rs
│   ├── schemas.rs
│   └── ...
├── auth/               # Authentication
│   ├── ed25519.rs      # Ed25519 key handling
│   ├── credentials.rs  # Credential types
│   └── provider.rs     # Auth providers
├── transport/          # Transport layer
│   ├── grpc.rs         # gRPC transport
│   ├── rest.rs         # REST transport
│   ├── mock.rs         # Mock transport for testing
│   └── proto/          # Generated protobuf code
├── types/              # Core types
├── error/              # Error types
├── config/             # Configuration
├── testing/            # Test utilities (MockClient, etc.)
└── middleware/         # Framework integrations
```

## Feature Flags

Default features: `grpc`, `rest`, `rustls`

Optional:
- `tracing` - Tracing integration
- `blocking` - Blocking API
- `derive` - Derive macros
- `native-tls` - Alternative to rustls
- `wasm` - WASM support

## Testing Patterns

- Unit tests: Use `MockClient` from `inferadb::testing`
- Integration tests: Require local InferaDB instance
- Use `#[ignore]` for integration tests
- Test files in `tests/` directory
