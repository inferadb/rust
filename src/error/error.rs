//! Main error type for the InferaDB SDK.

use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;

use super::ErrorKind;

/// The primary error type for InferaDB SDK operations.
///
/// `Error` provides rich context for debugging and error handling:
/// - [`kind()`](Error::kind): Categorization for `match` statements
/// - [`request_id()`](Error::request_id): Correlation ID for support
/// - [`retry_after()`](Error::retry_after): Delay hint for rate limits
/// - [`is_retriable()`](Error::is_retriable): Quick retry decision
///
/// ## Error Hierarchy
///
/// ```text
/// Error
/// ├── kind: ErrorKind          (category for matching)
/// ├── message: String          (human-readable description)
/// ├── request_id: Option       (server-assigned correlation ID)
/// ├── retry_after: Option      (rate limit delay hint)
/// └── source: Option           (underlying cause)
/// ```
///
/// ## Example
///
/// ```rust
/// use inferadb::{Error, ErrorKind};
///
/// fn handle_error(err: Error) {
///     match err.kind() {
///         ErrorKind::RateLimited => {
///             if let Some(delay) = err.retry_after() {
///                 println!("Rate limited, retry after {:?}", delay);
///             }
///         }
///         ErrorKind::Unauthorized => {
///             println!("Invalid credentials");
///         }
///         kind if kind.is_retriable() => {
///             println!("Transient error, will retry");
///         }
///         _ => {
///             println!("Permanent error: {}", err);
///         }
///     }
///
///     // Always log request_id for support
///     if let Some(id) = err.request_id() {
///         eprintln!("Request ID: {}", id);
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Error {
    /// The error category.
    kind: ErrorKind,

    /// Human-readable error message.
    message: Cow<'static, str>,

    /// Server-assigned request ID for correlation.
    request_id: Option<String>,

    /// Recommended delay before retrying (for rate limits).
    retry_after: Option<Duration>,

    /// The underlying error, if any.
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
    /// Creates a new error with the given kind and message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::{Error, ErrorKind};
    ///
    /// let err = Error::new(ErrorKind::InvalidArgument, "subject cannot be empty");
    /// assert_eq!(err.kind(), ErrorKind::InvalidArgument);
    /// ```
    pub fn new(kind: ErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind,
            message: message.into(),
            request_id: None,
            retry_after: None,
            source: None,
        }
    }

    /// Creates an error from a kind with a default message.
    pub fn from_kind(kind: ErrorKind) -> Self {
        let message = match kind {
            ErrorKind::Unauthorized => "authentication failed",
            ErrorKind::Forbidden => "permission denied",
            ErrorKind::NotFound => "resource not found",
            ErrorKind::InvalidArgument => "invalid argument",
            ErrorKind::SchemaViolation => "schema violation",
            ErrorKind::RateLimited => "rate limit exceeded",
            ErrorKind::Unavailable => "service unavailable",
            ErrorKind::Timeout => "request timed out",
            ErrorKind::Internal => "internal server error",
            ErrorKind::Cancelled => "request cancelled",
            ErrorKind::CircuitOpen => "circuit breaker open",
            ErrorKind::Connection => "connection failed",
            ErrorKind::Protocol => "protocol error",
            ErrorKind::Configuration => "configuration error",
            ErrorKind::Unknown => "unknown error",
        };
        Self::new(kind, message)
    }

    /// Returns the error kind for categorization.
    ///
    /// Use this for `match` expressions to handle different error types:
    ///
    /// ```rust
    /// use inferadb::{Error, ErrorKind};
    ///
    /// fn should_retry(err: &Error) -> bool {
    ///     err.kind().is_retriable()
    /// }
    /// ```
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the server-assigned request ID, if available.
    ///
    /// Always include this in error logs for support correlation:
    ///
    /// ```rust
    /// use inferadb::Error;
    ///
    /// fn log_error(err: &Error) {
    ///     if let Some(request_id) = err.request_id() {
    ///         eprintln!("Error (request_id: {}): {}", request_id, err);
    ///     } else {
    ///         eprintln!("Error: {}", err);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Returns the recommended retry delay for rate limit errors.
    ///
    /// This is populated from the `Retry-After` header or equivalent.
    /// Always prefer this value over a fixed delay for rate limit handling.
    ///
    /// ```rust
    /// use inferadb::{Error, ErrorKind};
    /// use std::time::Duration;
    ///
    /// async fn with_rate_limit_handling<T, F, Fut>(f: F) -> Result<T, Error>
    /// where
    ///     F: Fn() -> Fut,
    ///     Fut: std::future::Future<Output = Result<T, Error>>,
    /// {
    ///     loop {
    ///         match f().await {
    ///             Ok(v) => return Ok(v),
    ///             Err(e) if e.kind() == ErrorKind::RateLimited => {
    ///                 let delay = e.retry_after().unwrap_or(Duration::from_secs(1));
    ///                 tokio::time::sleep(delay).await;
    ///             }
    ///             Err(e) => return Err(e),
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    /// Returns `true` if this error is generally safe to retry.
    ///
    /// This is a convenience method equivalent to `self.kind().is_retriable()`.
    ///
    /// Retriable errors include:
    /// - `Unavailable` - service temporarily down
    /// - `Timeout` - request timed out
    /// - `RateLimited` - rate limit exceeded (use `retry_after()`)
    /// - `CircuitOpen` - circuit breaker tripped
    /// - `Connection` - network connectivity issues
    #[inline]
    pub fn is_retriable(&self) -> bool {
        self.kind.is_retriable()
    }

    /// Sets the request ID for this error.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sets the retry-after duration for this error.
    #[must_use]
    pub fn with_retry_after(mut self, duration: Duration) -> Self {
        self.retry_after = Some(duration);
        self
    }

    /// Sets the source error for this error.
    #[must_use]
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    // Convenience constructors for common error types

    /// Creates an unauthorized error.
    pub fn unauthorized(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Unauthorized, message)
    }

    /// Creates a forbidden error.
    pub fn forbidden(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Forbidden, message)
    }

    /// Creates a not found error.
    pub fn not_found(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    /// Creates an invalid argument error.
    pub fn invalid_argument(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }

    /// Creates a schema violation error.
    pub fn schema_violation(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::SchemaViolation, message)
    }

    /// Creates a rate limited error.
    pub fn rate_limited(retry_after: Option<Duration>) -> Self {
        let mut err = Self::from_kind(ErrorKind::RateLimited);
        if let Some(duration) = retry_after {
            err.retry_after = Some(duration);
        }
        err
    }

    /// Creates an unavailable error.
    pub fn unavailable(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    /// Creates a timeout error.
    pub fn timeout(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Timeout, message)
    }

    /// Creates an internal error.
    pub fn internal(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    /// Creates a cancelled error.
    pub fn cancelled() -> Self {
        Self::from_kind(ErrorKind::Cancelled)
    }

    /// Creates a circuit open error.
    pub fn circuit_open() -> Self {
        Self::from_kind(ErrorKind::CircuitOpen)
    }

    /// Creates a connection error.
    pub fn connection(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Connection, message)
    }

    /// Creates a protocol error.
    pub fn protocol(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Protocol, message)
    }

    /// Creates a configuration error.
    pub fn configuration(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Configuration, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)?;

        if let Some(ref request_id) = self.request_id {
            write!(f, " (request_id: {})", request_id)?;
        }

        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn StdError + 'static))
    }
}

// Implement From for common error types

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self::from_kind(kind)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        let kind = match err.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::Forbidden,
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected => ErrorKind::Connection,
            std::io::ErrorKind::TimedOut => ErrorKind::Timeout,
            _ => ErrorKind::Internal,
        };
        Error::new(kind, err.to_string()).with_source(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::configuration(format!("invalid URL: {}", err)).with_source(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::protocol(format!("JSON error: {}", err)).with_source(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_new() {
        let err = Error::new(ErrorKind::InvalidArgument, "test message");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);
        assert!(err.to_string().contains("test message"));
        assert!(err.request_id().is_none());
        assert!(err.retry_after().is_none());
    }

    #[test]
    fn test_error_from_kind() {
        let err = Error::from_kind(ErrorKind::Unauthorized);
        assert_eq!(err.kind(), ErrorKind::Unauthorized);
        assert!(err.to_string().contains("authentication failed"));
    }

    #[test]
    fn test_error_with_request_id() {
        let err = Error::new(ErrorKind::Internal, "server error")
            .with_request_id("req_abc123");
        assert_eq!(err.request_id(), Some("req_abc123"));
        assert!(err.to_string().contains("req_abc123"));
    }

    #[test]
    fn test_error_with_retry_after() {
        let err = Error::rate_limited(Some(Duration::from_secs(30)));
        assert_eq!(err.kind(), ErrorKind::RateLimited);
        assert_eq!(err.retry_after(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_error_is_retriable() {
        assert!(Error::from_kind(ErrorKind::Timeout).is_retriable());
        assert!(Error::from_kind(ErrorKind::Unavailable).is_retriable());
        assert!(Error::from_kind(ErrorKind::RateLimited).is_retriable());
        assert!(!Error::from_kind(ErrorKind::Unauthorized).is_retriable());
        assert!(!Error::from_kind(ErrorKind::NotFound).is_retriable());
    }

    #[test]
    fn test_error_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "underlying error");
        let err = Error::new(ErrorKind::Connection, "connection failed")
            .with_source(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn test_convenience_constructors() {
        assert_eq!(Error::unauthorized("test").kind(), ErrorKind::Unauthorized);
        assert_eq!(Error::forbidden("test").kind(), ErrorKind::Forbidden);
        assert_eq!(Error::not_found("test").kind(), ErrorKind::NotFound);
        assert_eq!(Error::invalid_argument("test").kind(), ErrorKind::InvalidArgument);
        assert_eq!(Error::schema_violation("test").kind(), ErrorKind::SchemaViolation);
        assert_eq!(Error::unavailable("test").kind(), ErrorKind::Unavailable);
        assert_eq!(Error::timeout("test").kind(), ErrorKind::Timeout);
        assert_eq!(Error::internal("test").kind(), ErrorKind::Internal);
        assert_eq!(Error::cancelled().kind(), ErrorKind::Cancelled);
        assert_eq!(Error::circuit_open().kind(), ErrorKind::CircuitOpen);
        assert_eq!(Error::connection("test").kind(), ErrorKind::Connection);
        assert_eq!(Error::protocol("test").kind(), ErrorKind::Protocol);
        assert_eq!(Error::configuration("test").kind(), ErrorKind::Configuration);
    }

    #[test]
    fn test_from_error_kind() {
        let err: Error = ErrorKind::Timeout.into();
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let err: Error = io_err.into();
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }

    #[test]
    fn test_display_format() {
        let err = Error::new(ErrorKind::NotFound, "vault not found")
            .with_request_id("req_xyz789");
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("vault not found"));
        assert!(display.contains("req_xyz789"));
    }
}
