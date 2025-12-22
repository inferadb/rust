//! Error types for the InferaDB SDK.
//!
//! The SDK provides two main error types:
//! - [`Error`]: General SDK errors (network, auth, validation, etc.)
//! - [`AccessDenied`]: Authorization denial (subject lacks permission)
//!
//! ## Key Invariant
//!
//! `check()` returns `Ok(false)` for denied access, not `Err`. Only `require()`
//! converts denial to an error (`AccessDenied`).
//!
//! ```rust,ignore
//! // check() - denial is Ok(false)
//! let allowed = vault.check("user:alice", "view", "doc:1").await?;
//!
//! // require() - denial is Err(AccessDenied)
//! vault.check("user:alice", "view", "doc:1").require().await?;
//! ```

mod access_denied;
mod core;
mod kind;

pub use access_denied::AccessDenied;
pub use core::Error;
pub use kind::ErrorKind;

/// A specialized `Result` type for InferaDB operations.
pub type Result<T> = std::result::Result<T, Error>;
