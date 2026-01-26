//! User-Agent header generation for SDK telemetry.
//!
//! Provides a consistent User-Agent string across all transports (REST and gRPC)
//! to help with usage analytics, debugging, and deprecation planning.

use std::sync::OnceLock;

/// SDK name used in the User-Agent string.
#[allow(dead_code)] // Used only when transports are enabled
const SDK_NAME: &str = "inferadb-rust";

/// SDK version from Cargo.toml.
#[allow(dead_code)] // Used only when transports are enabled
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cached User-Agent string (computed once on first access).
#[allow(dead_code)] // Used only when transports are enabled
static USER_AGENT: OnceLock<String> = OnceLock::new();

/// Returns the User-Agent string for SDK requests.
///
/// Format: `inferadb-rust/0.1.0 (rust/1.75.0; darwin/aarch64)`
///
/// Components:
/// - SDK name and version
/// - Rust version (compile-time)
/// - OS and architecture
///
/// The string is computed once and cached for subsequent calls.
#[allow(dead_code)] // Used only when transports are enabled
pub fn user_agent() -> &'static str {
    USER_AGENT.get_or_init(|| {
        format!(
            "{}/{} ({}; {}/{})",
            SDK_NAME,
            SDK_VERSION,
            rust_version(),
            os_name(),
            std::env::consts::ARCH,
        )
    })
}

/// Returns the Rust version string.
#[allow(dead_code)] // Used only when transports are enabled
fn rust_version() -> &'static str {
    // This is set at compile time by rustc
    concat!("rust/", env!("CARGO_PKG_RUST_VERSION"))
}

/// Returns a normalized OS name.
#[allow(dead_code)] // Used only when transports are enabled
fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent_format() {
        let ua = user_agent();

        // Should start with SDK name and version
        assert!(ua.starts_with("inferadb-rust/"));

        // Should contain rust version
        assert!(ua.contains("rust/"));

        // Should contain OS and arch
        assert!(ua.contains(std::env::consts::ARCH));

        // Should be properly formatted with parentheses
        assert!(ua.contains('('));
        assert!(ua.contains(')'));
    }

    #[test]
    fn test_user_agent_cached() {
        // Multiple calls should return the same reference
        let ua1 = user_agent();
        let ua2 = user_agent();
        assert!(std::ptr::eq(ua1, ua2));
    }

    #[test]
    fn test_os_normalization() {
        // Just verify it returns something reasonable
        let os = os_name();
        assert!(!os.is_empty());
    }
}
