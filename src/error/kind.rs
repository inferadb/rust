//! Error kind enumeration for categorizing SDK errors.

use std::fmt;

/// Categorization of SDK errors.
///
/// This enum provides a stable interface for matching on error types, enabling
/// different handling strategies for different failure modes.
///
/// ## Retriable vs Non-Retriable
///
/// | ErrorKind         | Retriable | Action                     |
/// |-------------------|-----------|----------------------------|
/// | `Unavailable`     | Yes       | Retry with backoff         |
/// | `Timeout`         | Yes       | Retry with backoff         |
/// | `RateLimited`     | Yes       | Use `retry_after()` delay  |
/// | `Unauthorized`    | No        | Fix credentials            |
/// | `Forbidden`       | No        | Fix permissions            |
/// | `NotFound`        | No        | Resource doesn't exist     |
/// | `SchemaViolation` | No        | Fix schema/query           |
/// | `InvalidArgument` | No        | Fix input                  |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Authentication failed (invalid or expired credentials).
    ///
    /// HTTP: 401 Unauthorized
    /// gRPC: UNAUTHENTICATED
    ///
    /// **Not retriable.** Fix credentials and retry.
    Unauthorized,

    /// Authorization failed (valid credentials but insufficient permissions).
    ///
    /// This is for **control plane** operations (e.g., managing vaults),
    /// not authorization check results. See [`AccessDenied`] for authorization
    /// check denials.
    ///
    /// HTTP: 403 Forbidden
    /// gRPC: PERMISSION_DENIED
    ///
    /// **Not retriable.** Fix permissions and retry.
    ///
    /// [`AccessDenied`]: crate::AccessDenied
    Forbidden,

    /// Requested resource was not found.
    ///
    /// HTTP: 404 Not Found
    /// gRPC: NOT_FOUND
    ///
    /// **Not retriable.** The resource doesn't exist.
    NotFound,

    /// Invalid request argument or payload.
    ///
    /// HTTP: 400 Bad Request
    /// gRPC: INVALID_ARGUMENT
    ///
    /// **Not retriable.** Fix the input and retry.
    InvalidArgument,

    /// Request violates the schema (invalid relation, type, or permission).
    ///
    /// HTTP: 400 Bad Request (with schema error details)
    /// gRPC: FAILED_PRECONDITION
    ///
    /// **Not retriable.** Fix the schema or query.
    SchemaViolation,

    /// Rate limit exceeded.
    ///
    /// HTTP: 429 Too Many Requests
    /// gRPC: RESOURCE_EXHAUSTED
    ///
    /// **Retriable.** Use `Error::retry_after()` for the recommended delay.
    RateLimited,

    /// Service temporarily unavailable.
    ///
    /// HTTP: 503 Service Unavailable
    /// gRPC: UNAVAILABLE
    ///
    /// **Retriable.** Retry with exponential backoff.
    Unavailable,

    /// Request timed out.
    ///
    /// HTTP: 504 Gateway Timeout or client-side timeout
    /// gRPC: DEADLINE_EXCEEDED
    ///
    /// **Retriable.** Retry with exponential backoff.
    Timeout,

    /// Internal server error.
    ///
    /// HTTP: 500 Internal Server Error
    /// gRPC: INTERNAL
    ///
    /// **Not retriable** by default. May indicate a bug on the server.
    Internal,

    /// Request was cancelled (typically by the client).
    ///
    /// gRPC: CANCELLED
    ///
    /// **Not retriable.** The operation was intentionally cancelled.
    Cancelled,

    /// Circuit breaker is open, requests are being rejected.
    ///
    /// This is a client-side error indicating that the circuit breaker
    /// has tripped due to too many failures.
    ///
    /// **Retriable** after the circuit breaker timeout.
    CircuitOpen,

    /// Connection error (DNS, TLS handshake, network unreachable).
    ///
    /// **Retriable.** May indicate network issues.
    Connection,

    /// Protocol error (malformed response, unexpected status).
    ///
    /// **Not retriable.** May indicate version mismatch or corruption.
    Protocol,

    /// Configuration error (invalid URL, missing credentials).
    ///
    /// **Not retriable.** Fix the configuration.
    Configuration,

    /// Unknown or unexpected error.
    ///
    /// Used as a catch-all for unrecognized error codes.
    Unknown,
}

impl ErrorKind {
    /// Returns `true` if this error kind is generally safe to retry.
    ///
    /// Retriable errors include:
    /// - `Unavailable` - service temporarily down
    /// - `Timeout` - request timed out
    /// - `RateLimited` - rate limit exceeded (use `retry_after()`)
    /// - `CircuitOpen` - circuit breaker tripped
    /// - `Connection` - network connectivity issues
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::ErrorKind;
    ///
    /// let kind = ErrorKind::Timeout;
    /// assert!(kind.is_retriable());
    ///
    /// let kind = ErrorKind::Unauthorized;
    /// assert!(!kind.is_retriable());
    /// ```
    #[inline]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            ErrorKind::Unavailable
                | ErrorKind::Timeout
                | ErrorKind::RateLimited
                | ErrorKind::CircuitOpen
                | ErrorKind::Connection
        )
    }

    /// Returns the default HTTP status code for this error kind.
    ///
    /// This is useful for mapping SDK errors to HTTP responses.
    #[inline]
    pub fn http_status_code(&self) -> u16 {
        match self {
            ErrorKind::Unauthorized => 401,
            ErrorKind::Forbidden => 403,
            ErrorKind::NotFound => 404,
            ErrorKind::InvalidArgument | ErrorKind::SchemaViolation => 400,
            ErrorKind::RateLimited => 429,
            ErrorKind::Timeout => 504,
            ErrorKind::Unavailable => 503,
            ErrorKind::Internal => 500,
            ErrorKind::Cancelled => 499, // Client Closed Request
            ErrorKind::CircuitOpen => 503,
            ErrorKind::Connection => 502,
            ErrorKind::Protocol => 502,
            ErrorKind::Configuration => 500,
            ErrorKind::Unknown => 500,
        }
    }

    /// Creates an `ErrorKind` from an HTTP status code.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            400 => ErrorKind::InvalidArgument,
            401 => ErrorKind::Unauthorized,
            403 => ErrorKind::Forbidden,
            404 => ErrorKind::NotFound,
            429 => ErrorKind::RateLimited,
            499 => ErrorKind::Cancelled,
            500 => ErrorKind::Internal,
            502 => ErrorKind::Protocol,
            503 => ErrorKind::Unavailable,
            504 => ErrorKind::Timeout,
            _ if (400..500).contains(&status) => ErrorKind::InvalidArgument,
            _ if status >= 500 => ErrorKind::Internal,
            _ => ErrorKind::Unknown,
        }
    }

    /// Creates an `ErrorKind` from a gRPC status code.
    #[cfg(feature = "grpc")]
    pub fn from_grpc_code(code: tonic::Code) -> Self {
        use tonic::Code;
        match code {
            Code::Ok => ErrorKind::Unknown, // Shouldn't happen
            Code::Cancelled => ErrorKind::Cancelled,
            Code::Unknown => ErrorKind::Unknown,
            Code::InvalidArgument => ErrorKind::InvalidArgument,
            Code::DeadlineExceeded => ErrorKind::Timeout,
            Code::NotFound => ErrorKind::NotFound,
            Code::AlreadyExists => ErrorKind::InvalidArgument,
            Code::PermissionDenied => ErrorKind::Forbidden,
            Code::ResourceExhausted => ErrorKind::RateLimited,
            Code::FailedPrecondition => ErrorKind::SchemaViolation,
            Code::Aborted => ErrorKind::Internal,
            Code::OutOfRange => ErrorKind::InvalidArgument,
            Code::Unimplemented => ErrorKind::Protocol,
            Code::Internal => ErrorKind::Internal,
            Code::Unavailable => ErrorKind::Unavailable,
            Code::DataLoss => ErrorKind::Internal,
            Code::Unauthenticated => ErrorKind::Unauthorized,
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Unauthorized => write!(f, "unauthorized"),
            ErrorKind::Forbidden => write!(f, "forbidden"),
            ErrorKind::NotFound => write!(f, "not found"),
            ErrorKind::InvalidArgument => write!(f, "invalid argument"),
            ErrorKind::SchemaViolation => write!(f, "schema violation"),
            ErrorKind::RateLimited => write!(f, "rate limited"),
            ErrorKind::Unavailable => write!(f, "service unavailable"),
            ErrorKind::Timeout => write!(f, "timeout"),
            ErrorKind::Internal => write!(f, "internal error"),
            ErrorKind::Cancelled => write!(f, "cancelled"),
            ErrorKind::CircuitOpen => write!(f, "circuit breaker open"),
            ErrorKind::Connection => write!(f, "connection error"),
            ErrorKind::Protocol => write!(f, "protocol error"),
            ErrorKind::Configuration => write!(f, "configuration error"),
            ErrorKind::Unknown => write!(f, "unknown error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retriable() {
        assert!(ErrorKind::Unavailable.is_retriable());
        assert!(ErrorKind::Timeout.is_retriable());
        assert!(ErrorKind::RateLimited.is_retriable());
        assert!(ErrorKind::CircuitOpen.is_retriable());
        assert!(ErrorKind::Connection.is_retriable());

        assert!(!ErrorKind::Unauthorized.is_retriable());
        assert!(!ErrorKind::Forbidden.is_retriable());
        assert!(!ErrorKind::NotFound.is_retriable());
        assert!(!ErrorKind::InvalidArgument.is_retriable());
        assert!(!ErrorKind::SchemaViolation.is_retriable());
        assert!(!ErrorKind::Internal.is_retriable());
        assert!(!ErrorKind::Cancelled.is_retriable());
    }

    #[test]
    fn test_http_status_code() {
        assert_eq!(ErrorKind::Unauthorized.http_status_code(), 401);
        assert_eq!(ErrorKind::Forbidden.http_status_code(), 403);
        assert_eq!(ErrorKind::NotFound.http_status_code(), 404);
        assert_eq!(ErrorKind::InvalidArgument.http_status_code(), 400);
        assert_eq!(ErrorKind::RateLimited.http_status_code(), 429);
        assert_eq!(ErrorKind::Unavailable.http_status_code(), 503);
        assert_eq!(ErrorKind::Timeout.http_status_code(), 504);
        assert_eq!(ErrorKind::Internal.http_status_code(), 500);
    }

    #[test]
    fn test_from_http_status() {
        assert_eq!(ErrorKind::from_http_status(400), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_http_status(401), ErrorKind::Unauthorized);
        assert_eq!(ErrorKind::from_http_status(403), ErrorKind::Forbidden);
        assert_eq!(ErrorKind::from_http_status(404), ErrorKind::NotFound);
        assert_eq!(ErrorKind::from_http_status(429), ErrorKind::RateLimited);
        assert_eq!(ErrorKind::from_http_status(500), ErrorKind::Internal);
        assert_eq!(ErrorKind::from_http_status(503), ErrorKind::Unavailable);
        assert_eq!(ErrorKind::from_http_status(504), ErrorKind::Timeout);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ErrorKind::Unauthorized), "unauthorized");
        assert_eq!(format!("{}", ErrorKind::RateLimited), "rate limited");
        assert_eq!(
            format!("{}", ErrorKind::SchemaViolation),
            "schema violation"
        );
    }
}
