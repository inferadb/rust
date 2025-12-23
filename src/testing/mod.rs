//! Testing utilities for InferaDB SDK.
//!
//! This module provides tools for testing applications that use the SDK:
//!
//! - [`MockClient`]: A mock client with expectation verification
//! - [`InMemoryClient`]: An in-memory client with real graph semantics
//! - [`AuthorizationClient`]: Object-safe trait for dependency injection
//!
//! ## Quick Start
//!
//! ```rust
//! use inferadb::testing::{MockClient, AuthorizationClient};
//!
//! // Create a mock client
//! let mock = MockClient::new()
//!     .expect_check("user:alice", "view", "doc:1", true);
//!
//! // Use the mock in tests
//! async fn test_with_mock(client: &dyn AuthorizationClient) {
//!     let allowed = client.check("user:alice", "view", "doc:1").await.unwrap();
//!     assert!(allowed);
//! }
//! ```
//!
//! ## MockClient vs InMemoryClient
//!
//! | Feature | MockClient | InMemoryClient |
//! |---------|------------|----------------|
//! | Expectation verification | ✓ | ✗ |
//! | Graph traversal | ✗ | ✓ |
//! | Schema validation | ✗ | ✓ |
//! | Relationship storage | ✗ | ✓ |
//! | Best for | Unit tests | Integration tests |

mod authorization_client;
mod in_memory;
mod mock_client;
mod test_vault;

pub use authorization_client::AuthorizationClient;
pub use in_memory::InMemoryClient;
pub use mock_client::MockClient;
pub use test_vault::{TestRelationshipsClient, TestResourcesClient, TestSubjectsClient, TestVault};
