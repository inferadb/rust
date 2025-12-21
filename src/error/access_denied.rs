//! AccessDenied error type for authorization denial.

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

/// Error returned when authorization is explicitly denied.
///
/// This type is **distinct from SDK errors** (`Error`). It represents a successful
/// authorization check that resulted in denial, not a failure to check.
///
/// ## When is AccessDenied Returned?
///
/// - `check()` returns `Ok(false)` for denial (not an error)
/// - `require()` returns `Err(AccessDenied)` for denial
///
/// ```rust,ignore
/// use inferadb::VaultClient;
///
/// async fn example(vault: &VaultClient) -> Result<(), Box<dyn std::error::Error>> {
///     // check() - denial is Ok(false), not an error
///     let allowed = vault.check("user:alice", "view", "doc:secret").await?;
///     if !allowed {
///         println!("Access denied (but no error)");
///     }
///
///     // require() - denial IS an error (AccessDenied)
///     vault.check("user:alice", "view", "doc:secret")
///         .require()
///         .await?; // Returns Err(AccessDenied) if denied
///
///     Ok(())
/// }
/// ```
///
/// ## Key Invariant
///
/// `AccessDenied` is NOT the same as `ErrorKind::Forbidden`:
///
/// | Type              | Meaning                              | Example                     |
/// |-------------------|--------------------------------------|-----------------------------|
/// | `AccessDenied`    | Subject lacks permission to resource | Alice can't view doc:secret |
/// | `ErrorKind::Forbidden` | API caller lacks control plane permission | Can't manage vault |
///
/// ## Rich Context
///
/// `AccessDenied` includes the authorization context for debugging:
///
/// ```rust
/// use inferadb::AccessDenied;
///
/// fn handle_denied(denied: &AccessDenied) {
///     println!("Subject: {}", denied.subject());
///     println!("Permission: {}", denied.permission());
///     println!("Resource: {}", denied.resource());
///
///     if let Some(reason) = denied.reason() {
///         println!("Reason: {}", reason);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AccessDenied {
    /// The subject that was denied.
    subject: Cow<'static, str>,

    /// The permission that was checked.
    permission: Cow<'static, str>,

    /// The resource that was checked.
    resource: Cow<'static, str>,

    /// Optional reason for the denial.
    reason: Option<Cow<'static, str>>,

    /// Optional request ID for correlation.
    request_id: Option<String>,
}

impl AccessDenied {
    /// Creates a new AccessDenied error.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject (e.g., "user:alice") that was denied
    /// * `permission` - The permission (e.g., "view") that was checked
    /// * `resource` - The resource (e.g., "document:readme") that was checked
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::AccessDenied;
    ///
    /// let denied = AccessDenied::new("user:alice", "delete", "document:readme");
    /// assert_eq!(denied.subject(), "user:alice");
    /// assert_eq!(denied.permission(), "delete");
    /// assert_eq!(denied.resource(), "document:readme");
    /// ```
    pub fn new(
        subject: impl Into<Cow<'static, str>>,
        permission: impl Into<Cow<'static, str>>,
        resource: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            reason: None,
            request_id: None,
        }
    }

    /// Returns the subject that was denied access.
    ///
    /// This is typically in the format "type:id", e.g., "user:alice" or "team:engineering".
    #[inline]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the permission that was checked.
    ///
    /// For example: "view", "edit", "delete", "admin".
    #[inline]
    pub fn permission(&self) -> &str {
        &self.permission
    }

    /// Returns the resource that was checked.
    ///
    /// This is typically in the format "type:id", e.g., "document:readme" or "folder:reports".
    #[inline]
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Returns the denial reason, if available.
    ///
    /// The reason provides additional context about why access was denied.
    /// This may include information about missing relationships or failed conditions.
    #[inline]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Returns the request ID, if available.
    ///
    /// Include this in logs for debugging and support correlation.
    #[inline]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Sets the denial reason.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<Cow<'static, str>>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets the request ID.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Returns a formatted string suitable for logging.
    ///
    /// This includes all available context in a structured format.
    pub fn to_log_string(&self) -> String {
        let mut parts = vec![
            format!("subject={}", self.subject),
            format!("permission={}", self.permission),
            format!("resource={}", self.resource),
        ];

        if let Some(ref reason) = self.reason {
            parts.push(format!("reason={}", reason));
        }

        if let Some(ref request_id) = self.request_id {
            parts.push(format!("request_id={}", request_id));
        }

        format!("access_denied: {}", parts.join(" "))
    }
}

impl fmt::Display for AccessDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "access denied: {} cannot {} {}",
            self.subject, self.permission, self.resource
        )?;

        if let Some(ref reason) = self.reason {
            write!(f, " ({})", reason)?;
        }

        Ok(())
    }
}

impl StdError for AccessDenied {}

/// Allows converting AccessDenied to the main Error type.
///
/// Note: This creates an `Error` with kind `Forbidden`, but `AccessDenied`
/// and `ErrorKind::Forbidden` have different semantic meanings:
/// - `AccessDenied`: Subject lacks permission (data plane)
/// - `ErrorKind::Forbidden`: Caller lacks API permission (control plane)
impl From<AccessDenied> for super::Error {
    fn from(denied: AccessDenied) -> Self {
        super::Error::new(
            super::ErrorKind::Forbidden,
            format!(
                "{} cannot {} {}",
                denied.subject, denied.permission, denied.resource
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_denied_new() {
        let denied = AccessDenied::new("user:alice", "delete", "document:secret");
        assert_eq!(denied.subject(), "user:alice");
        assert_eq!(denied.permission(), "delete");
        assert_eq!(denied.resource(), "document:secret");
        assert!(denied.reason().is_none());
        assert!(denied.request_id().is_none());
    }

    #[test]
    fn test_access_denied_with_reason() {
        let denied = AccessDenied::new("user:bob", "view", "folder:private")
            .with_reason("no viewer relationship");
        assert_eq!(denied.reason(), Some("no viewer relationship"));
    }

    #[test]
    fn test_access_denied_with_request_id() {
        let denied = AccessDenied::new("user:charlie", "edit", "doc:1")
            .with_request_id("req_abc123");
        assert_eq!(denied.request_id(), Some("req_abc123"));
    }

    #[test]
    fn test_access_denied_display() {
        let denied = AccessDenied::new("user:alice", "delete", "document:readme");
        let display = denied.to_string();
        assert!(display.contains("user:alice"));
        assert!(display.contains("delete"));
        assert!(display.contains("document:readme"));
    }

    #[test]
    fn test_access_denied_display_with_reason() {
        let denied = AccessDenied::new("user:alice", "delete", "document:readme")
            .with_reason("permission not granted");
        let display = denied.to_string();
        assert!(display.contains("permission not granted"));
    }

    #[test]
    fn test_access_denied_to_log_string() {
        let denied = AccessDenied::new("user:alice", "view", "doc:1")
            .with_reason("no access")
            .with_request_id("req_xyz");
        let log = denied.to_log_string();
        assert!(log.contains("subject=user:alice"));
        assert!(log.contains("permission=view"));
        assert!(log.contains("resource=doc:1"));
        assert!(log.contains("reason=no access"));
        assert!(log.contains("request_id=req_xyz"));
    }

    #[test]
    fn test_access_denied_into_error() {
        use crate::{Error, ErrorKind};
        let denied = AccessDenied::new("user:alice", "delete", "doc:1");
        let err: Error = denied.into();
        assert_eq!(err.kind(), ErrorKind::Forbidden);
    }

    #[test]
    fn test_access_denied_with_owned_strings() {
        let subject = String::from("user:dynamic");
        let permission = String::from("custom_perm");
        let resource = String::from("resource:123");

        let denied = AccessDenied::new(subject, permission, resource);
        assert_eq!(denied.subject(), "user:dynamic");
        assert_eq!(denied.permission(), "custom_perm");
        assert_eq!(denied.resource(), "resource:123");
    }

    #[test]
    fn test_access_denied_is_error() {
        // Verify AccessDenied implements std::error::Error
        fn takes_error<E: std::error::Error>(_: &E) {}
        let denied = AccessDenied::new("user:test", "view", "doc:1");
        takes_error(&denied);
    }
}
