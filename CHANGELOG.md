# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/inferadb/rust/compare/v0.1.0...HEAD)

## [0.1.0](https://github.com/inferadb/rust/releases/tag/v0.1.0)

### Added

- Initial release of the InferaDB Rust SDK
- `Client` builder with gRPC and REST transport support
- Authentication (client credentials, bearer token, environment variables)
- Authorization checks (`check`, `check_batch`)
- Relationship management (`write`, `delete`)
- Lookup operations (`list_resources`, `list_subjects`)
- Watch streams for real-time updates
- Retry, caching, and circuit breaker support
- Mock and in-memory clients for testing
