//! Error kind enumeration for categorizing SDK errors.

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
/// | `Connection`      | Yes       | Retry with backoff         |
/// | `CircuitOpen`     | Yes       | Wait for circuit reset     |
/// | `Unauthorized`    | No        | Fix credentials            |
/// | `Forbidden`       | No        | Fix permissions            |
/// | `NotFound`        | No        | Resource doesn't exist     |
/// | `Conflict`        | No*       | Resolve conflict first     |
/// | `SchemaViolation` | No        | Fix schema/query           |
/// | `InvalidArgument` | No        | Fix input                  |
///
/// *Conflict errors may be retriable after resolving the underlying conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Authentication failed (invalid or expired credentials).
    ///
    /// HTTP: 401 Unauthorized
    /// gRPC: UNAUTHENTICATED
    ///
    /// **Not retriable.** Fix credentials and retry.
    #[error("unauthorized")]
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
    #[error("forbidden")]
    Forbidden,

    /// Requested resource was not found.
    ///
    /// HTTP: 404 Not Found
    /// gRPC: NOT_FOUND
    ///
    /// **Not retriable.** The resource doesn't exist.
    #[error("not found")]
    NotFound,

    /// Invalid request argument or payload.
    ///
    /// HTTP: 400 Bad Request
    /// gRPC: INVALID_ARGUMENT
    ///
    /// **Not retriable.** Fix the input and retry.
    #[error("invalid argument")]
    InvalidArgument,

    /// Request violates the schema (invalid relation, type, or permission).
    ///
    /// HTTP: 400 Bad Request (with schema error details)
    /// gRPC: FAILED_PRECONDITION
    ///
    /// **Not retriable.** Fix the schema or query.
    #[error("schema violation")]
    SchemaViolation,

    /// Rate limit exceeded.
    ///
    /// HTTP: 429 Too Many Requests
    /// gRPC: RESOURCE_EXHAUSTED
    ///
    /// **Retriable.** Use `Error::retry_after()` for the recommended delay.
    #[error("rate limited")]
    RateLimited,

    /// Service temporarily unavailable.
    ///
    /// HTTP: 503 Service Unavailable
    /// gRPC: UNAVAILABLE
    ///
    /// **Retriable.** Retry with exponential backoff.
    #[error("service unavailable")]
    Unavailable,

    /// Request timed out.
    ///
    /// HTTP: 504 Gateway Timeout or client-side timeout
    /// gRPC: DEADLINE_EXCEEDED
    ///
    /// **Retriable.** Retry with exponential backoff.
    #[error("timeout")]
    Timeout,

    /// Internal server error.
    ///
    /// HTTP: 500 Internal Server Error
    /// gRPC: INTERNAL
    ///
    /// **Not retriable** by default. May indicate a bug on the server.
    #[error("internal error")]
    Internal,

    /// Request was cancelled (typically by the client).
    ///
    /// gRPC: CANCELLED
    ///
    /// **Not retriable.** The operation was intentionally cancelled.
    #[error("cancelled")]
    Cancelled,

    /// Circuit breaker is open, requests are being rejected.
    ///
    /// This is a client-side error indicating that the circuit breaker
    /// has tripped due to too many failures.
    ///
    /// **Retriable** after the circuit breaker timeout.
    #[error("circuit breaker open")]
    CircuitOpen,

    /// Connection error (DNS, TLS handshake, network unreachable).
    ///
    /// **Retriable.** May indicate transient network issues.
    #[error("connection error")]
    Connection,

    /// Protocol error (malformed response, unexpected status).
    ///
    /// **Not retriable.** May indicate version mismatch or corruption.
    #[error("protocol error")]
    Protocol,

    /// Configuration error (invalid URL, missing credentials).
    ///
    /// **Not retriable.** Fix the configuration.
    #[error("configuration error")]
    Configuration,

    /// Unknown or unexpected error.
    ///
    /// Used as a catch-all for unrecognized error codes.
    #[error("unknown error")]
    Unknown,

    /// Conflict with existing resource state.
    ///
    /// HTTP: 409 Conflict
    /// gRPC: ALREADY_EXISTS or ABORTED
    ///
    /// **Conditionally retriable.** Resolve the conflict (e.g., re-fetch
    /// and merge) before retrying.
    #[error("conflict")]
    Conflict,

    /// Transport layer error.
    ///
    /// Generic transport error for HTTP/gRPC issues that don't fit
    /// more specific categories.
    #[error("transport error")]
    Transport,

    /// Invalid response from server.
    ///
    /// Response could not be parsed or was malformed.
    ///
    /// **Not retriable** without server-side fix.
    #[error("invalid response")]
    InvalidResponse,
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
            ErrorKind::Conflict => 409,
            ErrorKind::RateLimited => 429,
            ErrorKind::Timeout => 504,
            ErrorKind::Unavailable => 503,
            ErrorKind::Internal => 500,
            ErrorKind::Cancelled => 499, // Client Closed Request
            ErrorKind::CircuitOpen => 503,
            ErrorKind::Connection => 502,
            ErrorKind::Protocol | ErrorKind::Transport => 502,
            ErrorKind::Configuration => 500,
            ErrorKind::InvalidResponse => 502,
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
            409 => ErrorKind::Conflict,
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
            Code::AlreadyExists => ErrorKind::Conflict,
            Code::PermissionDenied => ErrorKind::Forbidden,
            Code::ResourceExhausted => ErrorKind::RateLimited,
            Code::FailedPrecondition => ErrorKind::SchemaViolation,
            Code::Aborted => ErrorKind::Conflict,
            Code::OutOfRange => ErrorKind::InvalidArgument,
            Code::Unimplemented => ErrorKind::Protocol,
            Code::Internal => ErrorKind::Internal,
            Code::Unavailable => ErrorKind::Unavailable,
            Code::DataLoss => ErrorKind::Internal,
            Code::Unauthenticated => ErrorKind::Unauthorized,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retriable() {
        // Retriable errors
        assert!(ErrorKind::Unavailable.is_retriable());
        assert!(ErrorKind::Timeout.is_retriable());
        assert!(ErrorKind::RateLimited.is_retriable());
        assert!(ErrorKind::CircuitOpen.is_retriable());
        assert!(ErrorKind::Connection.is_retriable());

        // Non-retriable errors
        assert!(!ErrorKind::Unauthorized.is_retriable());
        assert!(!ErrorKind::Forbidden.is_retriable());
        assert!(!ErrorKind::NotFound.is_retriable());
        assert!(!ErrorKind::InvalidArgument.is_retriable());
        assert!(!ErrorKind::SchemaViolation.is_retriable());
        assert!(!ErrorKind::Internal.is_retriable());
        assert!(!ErrorKind::Cancelled.is_retriable());
        assert!(!ErrorKind::Protocol.is_retriable());
        assert!(!ErrorKind::Configuration.is_retriable());
        assert!(!ErrorKind::Unknown.is_retriable());
    }

    #[test]
    fn test_http_status_code() {
        // All error kinds should have an HTTP status code
        assert_eq!(ErrorKind::Unauthorized.http_status_code(), 401);
        assert_eq!(ErrorKind::Forbidden.http_status_code(), 403);
        assert_eq!(ErrorKind::NotFound.http_status_code(), 404);
        assert_eq!(ErrorKind::InvalidArgument.http_status_code(), 400);
        assert_eq!(ErrorKind::SchemaViolation.http_status_code(), 400);
        assert_eq!(ErrorKind::RateLimited.http_status_code(), 429);
        assert_eq!(ErrorKind::Unavailable.http_status_code(), 503);
        assert_eq!(ErrorKind::Timeout.http_status_code(), 504);
        assert_eq!(ErrorKind::Internal.http_status_code(), 500);
        assert_eq!(ErrorKind::Cancelled.http_status_code(), 499);
        assert_eq!(ErrorKind::CircuitOpen.http_status_code(), 503);
        assert_eq!(ErrorKind::Connection.http_status_code(), 502);
        assert_eq!(ErrorKind::Protocol.http_status_code(), 502);
        assert_eq!(ErrorKind::Configuration.http_status_code(), 500);
        assert_eq!(ErrorKind::Unknown.http_status_code(), 500);
    }

    #[test]
    fn test_from_http_status() {
        // Direct mappings
        assert_eq!(ErrorKind::from_http_status(400), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_http_status(401), ErrorKind::Unauthorized);
        assert_eq!(ErrorKind::from_http_status(403), ErrorKind::Forbidden);
        assert_eq!(ErrorKind::from_http_status(404), ErrorKind::NotFound);
        assert_eq!(ErrorKind::from_http_status(429), ErrorKind::RateLimited);
        assert_eq!(ErrorKind::from_http_status(499), ErrorKind::Cancelled);
        assert_eq!(ErrorKind::from_http_status(500), ErrorKind::Internal);
        assert_eq!(ErrorKind::from_http_status(502), ErrorKind::Protocol);
        assert_eq!(ErrorKind::from_http_status(503), ErrorKind::Unavailable);
        assert_eq!(ErrorKind::from_http_status(504), ErrorKind::Timeout);

        // 4xx range falls back to InvalidArgument
        assert_eq!(ErrorKind::from_http_status(405), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_http_status(422), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_http_status(451), ErrorKind::InvalidArgument);

        // 5xx range falls back to Internal
        assert_eq!(ErrorKind::from_http_status(501), ErrorKind::Internal);
        assert_eq!(ErrorKind::from_http_status(505), ErrorKind::Internal);

        // Other status codes return Unknown
        assert_eq!(ErrorKind::from_http_status(200), ErrorKind::Unknown);
        assert_eq!(ErrorKind::from_http_status(301), ErrorKind::Unknown);
    }

    #[test]
    fn test_display() {
        // All error kinds should have a display string
        assert_eq!(format!("{}", ErrorKind::Unauthorized), "unauthorized");
        assert_eq!(format!("{}", ErrorKind::Forbidden), "forbidden");
        assert_eq!(format!("{}", ErrorKind::NotFound), "not found");
        assert_eq!(format!("{}", ErrorKind::InvalidArgument), "invalid argument");
        assert_eq!(format!("{}", ErrorKind::SchemaViolation), "schema violation");
        assert_eq!(format!("{}", ErrorKind::RateLimited), "rate limited");
        assert_eq!(format!("{}", ErrorKind::Unavailable), "service unavailable");
        assert_eq!(format!("{}", ErrorKind::Timeout), "timeout");
        assert_eq!(format!("{}", ErrorKind::Internal), "internal error");
        assert_eq!(format!("{}", ErrorKind::Cancelled), "cancelled");
        assert_eq!(format!("{}", ErrorKind::CircuitOpen), "circuit breaker open");
        assert_eq!(format!("{}", ErrorKind::Connection), "connection error");
        assert_eq!(format!("{}", ErrorKind::Protocol), "protocol error");
        assert_eq!(format!("{}", ErrorKind::Configuration), "configuration error");
        assert_eq!(format!("{}", ErrorKind::Unknown), "unknown error");
    }

    #[test]
    fn test_error_kind_clone_and_eq() {
        let kind = ErrorKind::Timeout;
        let cloned = kind;
        assert_eq!(kind, cloned);
    }

    #[test]
    fn test_error_kind_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ErrorKind::Timeout);
        set.insert(ErrorKind::Unavailable);
        set.insert(ErrorKind::Timeout); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_is_retriable_remaining() {
        // Test remaining non-retriable error kinds
        assert!(!ErrorKind::Conflict.is_retriable());
        assert!(!ErrorKind::Transport.is_retriable());
        assert!(!ErrorKind::InvalidResponse.is_retriable());
    }

    #[test]
    fn test_http_status_code_remaining() {
        // Test remaining variants have correct status codes
        assert_eq!(ErrorKind::Conflict.http_status_code(), 409);
        assert_eq!(ErrorKind::Transport.http_status_code(), 502);
        assert_eq!(ErrorKind::InvalidResponse.http_status_code(), 502);
    }

    #[test]
    fn test_display_remaining() {
        // Test remaining variants have display strings
        assert_eq!(format!("{}", ErrorKind::Conflict), "conflict");
        assert_eq!(format!("{}", ErrorKind::Transport), "transport error");
        assert_eq!(format!("{}", ErrorKind::InvalidResponse), "invalid response");
    }

    #[test]
    fn test_error_kind_debug() {
        let kind = ErrorKind::Timeout;
        let debug = format!("{:?}", kind);
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn test_from_http_status_conflict() {
        // 409 Conflict should map to ErrorKind::Conflict
        assert_eq!(ErrorKind::from_http_status(409), ErrorKind::Conflict);
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn test_from_grpc_code() {
        use tonic::Code;

        // Test all gRPC code mappings
        assert_eq!(ErrorKind::from_grpc_code(Code::Ok), ErrorKind::Unknown);
        assert_eq!(ErrorKind::from_grpc_code(Code::Cancelled), ErrorKind::Cancelled);
        assert_eq!(ErrorKind::from_grpc_code(Code::Unknown), ErrorKind::Unknown);
        assert_eq!(ErrorKind::from_grpc_code(Code::InvalidArgument), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_grpc_code(Code::DeadlineExceeded), ErrorKind::Timeout);
        assert_eq!(ErrorKind::from_grpc_code(Code::NotFound), ErrorKind::NotFound);
        assert_eq!(ErrorKind::from_grpc_code(Code::AlreadyExists), ErrorKind::Conflict);
        assert_eq!(ErrorKind::from_grpc_code(Code::PermissionDenied), ErrorKind::Forbidden);
        assert_eq!(ErrorKind::from_grpc_code(Code::ResourceExhausted), ErrorKind::RateLimited);
        assert_eq!(ErrorKind::from_grpc_code(Code::FailedPrecondition), ErrorKind::SchemaViolation);
        assert_eq!(ErrorKind::from_grpc_code(Code::Aborted), ErrorKind::Conflict);
        assert_eq!(ErrorKind::from_grpc_code(Code::OutOfRange), ErrorKind::InvalidArgument);
        assert_eq!(ErrorKind::from_grpc_code(Code::Unimplemented), ErrorKind::Protocol);
        assert_eq!(ErrorKind::from_grpc_code(Code::Internal), ErrorKind::Internal);
        assert_eq!(ErrorKind::from_grpc_code(Code::Unavailable), ErrorKind::Unavailable);
        assert_eq!(ErrorKind::from_grpc_code(Code::DataLoss), ErrorKind::Internal);
        assert_eq!(ErrorKind::from_grpc_code(Code::Unauthenticated), ErrorKind::Unauthorized);
    }
}
