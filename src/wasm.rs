//! WebAssembly (WASM) support for the InferaDB SDK.
//!
//! This module provides WASM-specific utilities and compatibility helpers
//! for running the SDK in browser and other WASM environments.
//!
//! ## Feature Flag
//!
//! Enable WASM support by adding the `wasm` feature:
//!
//! ```toml
//! [dependencies]
//! inferadb = { version = "0.1", features = ["wasm"] }
//! ```
//!
//! ## Browser Usage
//!
//! When targeting `wasm32-unknown-unknown`, the SDK uses:
//! - `getrandom` with JS bindings for cryptographic randomness
//! - Browser's Fetch API via reqwest for HTTP requests
//! - `wasm-bindgen-futures` for async runtime integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use inferadb::prelude::*;
//! use wasm_bindgen_futures::spawn_local;
//!
//! // In a WASM context (e.g., from a button click handler)
//! spawn_local(async move {
//!     let client = Client::builder()
//!         .url("https://api.inferadb.com")
//!         .bearer_token("your-token")
//!         .build()
//!         .await
//!         .expect("Failed to create client");
//!
//!     let vault = client.organization("org_123").vault("vlt_456");
//!     let allowed = vault.check("user:alice", "view", "doc:readme").await;
//!     web_sys::console::log_1(&format!("Allowed: {:?}", allowed).into());
//! });
//! ```
//!
//! ## Limitations
//!
//! When running in WASM:
//! - The blocking API is not available (no threads in WASM)
//! - gRPC transport is not supported (use REST instead)
//! - Some timing operations may have reduced precision
//! - File system operations (like loading PEM keys) are not available

/// WASM platform detection.
///
/// Returns `true` if the SDK is running on a WASM target.
#[inline]
pub const fn is_wasm() -> bool {
    cfg!(target_arch = "wasm32")
}

/// Browser platform detection.
///
/// Returns `true` if the SDK appears to be running in a browser context.
/// This checks for the `wasm32-unknown-unknown` target which is typically
/// used for browser WASM.
#[inline]
pub const fn is_browser() -> bool {
    cfg!(all(target_arch = "wasm32", target_os = "unknown"))
}

/// Node.js/Deno platform detection.
///
/// Returns `true` if the SDK appears to be running in a Node.js or Deno context.
#[inline]
pub const fn is_node() -> bool {
    cfg!(all(target_arch = "wasm32", not(target_os = "unknown")))
}

/// Check if the current platform supports threads.
///
/// WASM typically doesn't have thread support (unless using wasm-threads),
/// so this returns `false` for WASM targets.
#[inline]
pub const fn has_threads() -> bool {
    !cfg!(target_arch = "wasm32")
}

/// Check if the current platform supports file system operations.
///
/// Browsers don't have direct file system access, so this returns `false`
/// for browser WASM targets.
#[inline]
pub const fn has_filesystem() -> bool {
    !is_browser()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_wasm() {
        // On native platforms, this should be false
        #[cfg(not(target_arch = "wasm32"))]
        assert!(!is_wasm());

        #[cfg(target_arch = "wasm32")]
        assert!(is_wasm());
    }

    #[test]
    fn test_is_browser() {
        // On native platforms, this should be false
        #[cfg(not(target_arch = "wasm32"))]
        assert!(!is_browser());
    }

    #[test]
    fn test_has_threads() {
        // On native platforms, this should be true
        #[cfg(not(target_arch = "wasm32"))]
        assert!(has_threads());

        #[cfg(target_arch = "wasm32")]
        assert!(!has_threads());
    }

    #[test]
    fn test_has_filesystem() {
        // On native platforms, this should be true
        #[cfg(not(target_arch = "wasm32"))]
        assert!(has_filesystem());
    }
}
