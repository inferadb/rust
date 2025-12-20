# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0] - YYYY-MM-DD

### Added

- Initial release
- `Client` with builder pattern for configuration
- Authentication via client credentials (Ed25519 JWT assertions)
- Authentication via bearer token
- Authentication via environment variables (`Client::from_env()`)
- Authorization checks (`check()`, `check_batch()`)
- Relationship management (`write()`, `delete()`, `write_batch()`)
- Lookup operations (`list_resources()`, `list_subjects()`)
- Watch streams for real-time updates
- Caching with configurable TTL
- Retry with exponential backoff
- Circuit breaker for graceful degradation
- gRPC and REST transport support
- OpenTelemetry integration
- Mock client for testing
- In-memory client for unit tests
- Blocking/sync API

[Unreleased]: https://github.com/inferadb/rust-sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/inferadb/rust-sdk/releases/tag/v0.1.0
