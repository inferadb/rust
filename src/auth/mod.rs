//! Authentication and credentials for the InferaDB SDK.
//!
//! This module provides types for authenticating with InferaDB:
//!
//! - [`Ed25519PrivateKey`]: Ed25519 private key for JWT signing
//! - [`Credentials`]: Authentication credentials (client credentials or bearer)
//! - [`CredentialsProvider`]: Trait for custom credential providers
//! - [`ClientCredentialsConfig`]: OAuth 2.0 client credentials configuration
//! - [`BearerCredentialsConfig`]: Direct bearer token configuration
//!
//! ## Recommended: Client Credentials
//!
//! For production use, client credentials with Ed25519 keys are recommended:
//!
//! ```rust,ignore
//! use inferadb::{Client, ClientCredentialsConfig, Ed25519PrivateKey};
//!
//! let client = Client::builder()
//!     .url("https://api.inferadb.com")
//!     .credentials(ClientCredentialsConfig {
//!         client_id: "your-client-id".into(),
//!         private_key: Ed25519PrivateKey::from_pem_file("private-key.pem")?,
//!         certificate_id: None,
//!     })
//!     .build()
//!     .await?;
//! ```
//!
//! ## Development: Bearer Token
//!
//! For development, you can use a pre-generated bearer token:
//!
//! ```rust,ignore
//! use inferadb::{Client, BearerCredentialsConfig};
//!
//! let client = Client::builder()
//!     .url("https://api.inferadb.com")
//!     .credentials(BearerCredentialsConfig::new("your-api-token"))
//!     .build()
//!     .await?;
//! ```

// Allow dead code for auth types not yet integrated
#![allow(dead_code)]

mod credentials;
mod ed25519;
mod provider;

pub use credentials::{BearerCredentialsConfig, ClientCredentialsConfig, Credentials};
pub use ed25519::Ed25519PrivateKey;
pub use provider::{CredentialsFuture, CredentialsProvider};
