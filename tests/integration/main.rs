//! Integration tests for the InferaDB Rust SDK.
//!
//! These tests run against the dev environment deployed via:
//!   deploy/scripts/dev-up.sh
//!
//! # Running Tests
//!
//! ```bash
//! # Start the dev environment first
//! cd deploy && ./scripts/dev-up.sh
//!
//! # Run integration tests (single-threaded to avoid conflicts)
//! cargo test --test integration -- --test-threads=1
//!
//! # Run with verbose output
//! cargo test --test integration -- --test-threads=1 --nocapture
//!
//! # Run a specific test
//! cargo test --test integration test_fixture_creation -- --nocapture
//! ```
//!
//! # Environment Variables
//!
//! - `INFERADB_API_URL`: Override the API URL (default: auto-discovered from Tailscale)
//!
//! # Prerequisites
//!
//! 1. Tailscale must be installed and connected to your tailnet
//! 2. The dev environment must be running (`deploy/scripts/dev-up.sh`)

mod client_tests;
mod common;
mod control_tests;
mod transport_tests;
mod vault_advanced_tests;
mod vault_tests;
