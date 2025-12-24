# Installation & Configuration

## Basic Installation

Add inferadb to your `Cargo.toml`:

```toml
[dependencies]
inferadb = "0.1"
```

This includes both gRPC and REST transports with pure-Rust TLS.

## Feature Flags

| Feature      | Default | Description                            |
| ------------ | ------- | -------------------------------------- |
| `grpc`       | Yes     | gRPC transport (faster, streaming)     |
| `rest`       | Yes     | REST transport (broader compatibility) |
| `rustls`     | Yes     | Pure-Rust TLS                          |
| `native-tls` | No      | System TLS (OpenSSL/Schannel)          |
| `tracing`    | No      | OpenTelemetry integration              |
| `blocking`   | No      | Sync/blocking API wrapper              |
| `derive`     | No      | Proc macros for type-safe schemas      |
| `wasm`       | No      | Browser/WASM support (REST only)       |

## Optimized Builds

### REST Only (Smaller Binary)

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "rustls"] }
```

### gRPC Only (Lower Latency)

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["grpc", "rustls"] }
```

### With Tracing

```toml
[dependencies]
inferadb = { version = "0.1", features = ["tracing"] }
```

## TLS Options

### Pure-Rust TLS (Default)

Uses `rustls` with Mozilla's root certificates. No system dependencies.

### Native TLS

Uses the system TLS library (OpenSSL on Linux, Secure Transport on macOS, Schannel on Windows):

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["grpc", "rest", "native-tls"] }
```

## Minimum Supported Rust Version

The MSRV is **1.88.0**. We target approximately two releases behind stable.

- MSRV increases are documented in the [CHANGELOG](../../CHANGELOG.md)
- The `rust-version` field in `Cargo.toml` enforces this at build time
- Earlier compiler versions are not guaranteed to work

## WASM / Browser Support

For browser environments, use REST-only with the `wasm` feature:

```toml
[dependencies]
inferadb = { version = "0.1", default-features = false, features = ["rest", "wasm"] }
```

Note: gRPC is not supported in WASM environments.
