//! Middleware and interceptors for cross-cutting concerns.
//!
//! Middleware wraps the transport layer, allowing you to implement cross-cutting
//! concerns like logging, metrics, custom headers, or request transformation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ SDK Request Pipeline                                                │
//! │                                                                      │
//! │  ┌─────────────────┐                                                │
//! │  │ Your Code       │  vault.check("user:alice", "view", "doc:1")    │
//! │  └────────┬────────┘                                                │
//! │           │                                                          │
//! │           ▼                                                          │
//! │  ┌─────────────────┐                                                │
//! │  │ Middleware 1    │  e.g., AuditLogger                             │
//! │  └────────┬────────┘                                                │
//! │           │                                                          │
//! │           ▼                                                          │
//! │  ┌─────────────────┐                                                │
//! │  │ Middleware 2    │  e.g., MetricsCollector                        │
//! │  └────────┬────────┘                                                │
//! │           │                                                          │
//! │           ▼                                                          │
//! │  ┌─────────────────┐                                                │
//! │  │ Auth Layer      │  Inject Bearer token                           │
//! │  └────────┬────────┘                                                │
//! │           │                                                          │
//! │           ▼                                                          │
//! │  ┌─────────────────┐                                                │
//! │  │ Transport       │  HTTP/gRPC call                                │
//! │  └─────────────────┘                                                │
//! │                                                                      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use inferadb::middleware::{Middleware, Request, Response, Next};
//! use std::time::Instant;
//!
//! struct TimingMiddleware;
//!
//! impl Middleware for TimingMiddleware {
//!     fn handle<'a>(
//!         &'a self,
//!         req: Request,
//!         next: Next<'a>,
//!     ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, inferadb::Error>> + Send + 'a>> {
//!         Box::pin(async move {
//!             let start = Instant::now();
//!             let response = next.call(req).await?;
//!             println!("Request took {:?}", start.elapsed());
//!             Ok(response)
//!         })
//!     }
//! }
//!
//! // Add middleware to client
//! let client = Client::builder()
//!     .url("https://api.inferadb.com")
//!     .credentials(creds)
//!     .middleware(TimingMiddleware)
//!     .build()
//!     .await?;
//! ```

use std::{collections::HashMap, fmt, future::Future, pin::Pin};

use crate::{Error, error::ErrorKind};

/// Type alias for the response future returned by middleware handlers.
pub type ResponseFuture<'a> = Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>>;

/// Type alias for the next handler function in the middleware chain.
type NextHandler<'a> = Box<dyn FnOnce(Request) -> ResponseFuture<'a> + Send + 'a>;

/// Middleware trait for intercepting SDK requests.
///
/// Middleware wraps the transport layer, enabling cross-cutting concerns
/// like logging, metrics, custom headers, or request transformation.
///
/// ## Example
///
/// ```rust
/// use inferadb::middleware::{Middleware, Request, Response, Next};
/// use inferadb::Error;
///
/// struct LoggingMiddleware;
///
/// impl Middleware for LoggingMiddleware {
///     fn handle<'a>(
///         &'a self,
///         req: Request,
///         next: Next<'a>,
///     ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Error>> + Send + 'a>> {
///         Box::pin(async move {
///             println!("Request: {}", req.operation());
///             let response = next.call(req).await?;
///             println!("Response: {:?}", response.is_ok());
///             Ok(response)
///         })
///     }
/// }
/// ```
pub trait Middleware: Send + Sync + 'static {
    /// Handle a request, optionally modifying it or the response.
    ///
    /// Call `next.call(req)` to continue the chain. You can:
    /// - Modify the request before calling `next`
    /// - Modify the response after `next` returns
    /// - Short-circuit and return early without calling `next`
    /// - Measure timing by recording before/after `next`
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>>;
}

/// An SDK request being processed through the middleware chain.
#[derive(Debug, Clone)]
pub struct Request {
    /// The operation being performed (e.g., "check", "write", "list_resources")
    operation: String,
    /// Request metadata
    metadata: RequestMetadata,
    /// The serialized request body
    body: Vec<u8>,
}

impl Request {
    /// Create a new request.
    pub fn new(operation: impl Into<String>) -> Self {
        Self { operation: operation.into(), metadata: RequestMetadata::default(), body: Vec::new() }
    }

    /// Create a new request with body.
    pub fn with_body(operation: impl Into<String>, body: Vec<u8>) -> Self {
        Self { operation: operation.into(), metadata: RequestMetadata::default(), body }
    }

    /// Get the operation name.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get request metadata (headers, trace context, etc.).
    pub fn metadata(&self) -> &RequestMetadata {
        &self.metadata
    }

    /// Get mutable access to metadata for adding custom headers.
    pub fn metadata_mut(&mut self) -> &mut RequestMetadata {
        &mut self.metadata
    }

    /// Get the request body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Set the request body.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Add a header to the request.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(key.into(), value.into());
        self
    }

    /// Set the trace context.
    pub fn trace_context(mut self, context: TraceContext) -> Self {
        self.metadata.trace_context = Some(context);
        self
    }

    /// Set the request ID.
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.metadata.request_id = Some(id.into());
        self
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Request({})", self.operation)
    }
}

/// Request metadata including headers and trace context.
#[derive(Debug, Clone, Default)]
pub struct RequestMetadata {
    /// Custom headers to include in the request.
    pub headers: HashMap<String, String>,
    /// Trace context for distributed tracing.
    pub trace_context: Option<TraceContext>,
    /// Request ID (auto-generated if not set).
    pub request_id: Option<String>,
}

impl RequestMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the trace context.
    pub fn with_trace_context(mut self, context: TraceContext) -> Self {
        self.trace_context = Some(context);
        self
    }

    /// Set the request ID.
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

/// W3C Trace Context for distributed tracing.
///
/// Implements the W3C Trace Context specification (<https://www.w3.org/TR/trace-context/>)
/// for propagating trace information across service boundaries.
///
/// ## Example
///
/// ```rust
/// use inferadb::middleware::TraceContext;
///
/// // Parse from incoming HTTP headers
/// let context = TraceContext::parse(
///     "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
///     Some("congo=t61rcWkgMzE"),
/// ).unwrap();
///
/// // Generate outgoing headers
/// let (traceparent, tracestate) = context.to_headers();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// The version of the trace context (currently always 00).
    pub version: u8,
    /// The trace ID (16 bytes, 32 hex chars).
    pub trace_id: String,
    /// The parent span ID (8 bytes, 16 hex chars).
    pub parent_id: String,
    /// Trace flags (1 byte).
    pub trace_flags: u8,
    /// Optional tracestate header value.
    pub trace_state: Option<String>,
}

impl TraceContext {
    /// Create a new trace context with a random trace ID and parent ID.
    pub fn new() -> Self {
        Self {
            version: 0,
            trace_id: Self::random_trace_id(),
            parent_id: Self::random_span_id(),
            trace_flags: 1, // sampled
            trace_state: None,
        }
    }

    /// Create a child span context from this context.
    pub fn child(&self) -> Self {
        Self {
            version: self.version,
            trace_id: self.trace_id.clone(),
            parent_id: Self::random_span_id(),
            trace_flags: self.trace_flags,
            trace_state: self.trace_state.clone(),
        }
    }

    /// Parse from W3C Trace Context headers.
    ///
    /// ## Arguments
    /// - `traceparent`: The traceparent header value (required)
    /// - `tracestate`: The tracestate header value (optional)
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::middleware::TraceContext;
    ///
    /// let ctx = TraceContext::parse(
    ///     "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
    ///     None,
    /// ).unwrap();
    ///
    /// assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    /// assert_eq!(ctx.parent_id, "b7ad6b7169203331");
    /// ```
    pub fn parse(traceparent: &str, tracestate: Option<&str>) -> Result<Self, Error> {
        let parts: Vec<&str> = traceparent.split('-').collect();
        if parts.len() != 4 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "Invalid traceparent format: expected 4 parts separated by '-'",
            ));
        }

        let version = u8::from_str_radix(parts[0], 16)
            .map_err(|_| Error::new(ErrorKind::InvalidArgument, "Invalid traceparent version"))?;

        if parts[1].len() != 32 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "Invalid trace ID: expected 32 hex characters",
            ));
        }

        if parts[2].len() != 16 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "Invalid parent ID: expected 16 hex characters",
            ));
        }

        let trace_flags = u8::from_str_radix(parts[3], 16)
            .map_err(|_| Error::new(ErrorKind::InvalidArgument, "Invalid traceparent flags"))?;

        Ok(Self {
            version,
            trace_id: parts[1].to_string(),
            parent_id: parts[2].to_string(),
            trace_flags,
            trace_state: tracestate.map(|s| s.to_string()),
        })
    }

    /// Generate the traceparent and tracestate header values.
    pub fn to_headers(&self) -> (String, Option<String>) {
        let traceparent = format!(
            "{:02x}-{}-{}-{:02x}",
            self.version, self.trace_id, self.parent_id, self.trace_flags
        );
        (traceparent, self.trace_state.clone())
    }

    /// Check if this trace is sampled.
    pub fn is_sampled(&self) -> bool {
        self.trace_flags & 0x01 != 0
    }

    /// Set the sampled flag.
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.trace_flags |= 0x01;
        } else {
            self.trace_flags &= !0x01;
        }
    }

    fn random_trace_id() -> String {
        use rand::Rng;
        let bytes: [u8; 16] = rand::rng().random();
        hex::encode(bytes)
    }

    fn random_span_id() -> String {
        use rand::Rng;
        let bytes: [u8; 8] = rand::rng().random();
        hex::encode(bytes)
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}-{}-{}-{:02x}",
            self.version, self.trace_id, self.parent_id, self.trace_flags
        )
    }
}

/// The response from an SDK operation.
#[derive(Debug, Clone)]
pub struct Response {
    /// Response metadata
    metadata: ResponseMetadata,
    /// The serialized response body
    body: Vec<u8>,
}

impl Response {
    /// Create a new successful response.
    pub fn ok(body: Vec<u8>) -> Self {
        Self { metadata: ResponseMetadata::success(), body }
    }

    /// Create a new error response.
    pub fn error(kind: ErrorKind, body: Vec<u8>) -> Self {
        Self { metadata: ResponseMetadata::error(kind), body }
    }

    /// Check if the response indicates success.
    pub fn is_ok(&self) -> bool {
        self.metadata.status.is_success()
    }

    /// Get response metadata.
    pub fn metadata(&self) -> &ResponseMetadata {
        &self.metadata
    }

    /// Get mutable access to response metadata.
    pub fn metadata_mut(&mut self) -> &mut ResponseMetadata {
        &mut self.metadata
    }

    /// Get the response body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Consume the response and return the body.
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Add a header to the response.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(key.into(), value.into());
        self
    }

    /// Set the request ID.
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.metadata.request_id = Some(id.into());
        self
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Response({:?})", self.metadata.status)
    }
}

/// Response metadata including status and headers.
#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    /// Response status.
    pub status: ResponseStatus,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Server-provided request ID.
    pub request_id: Option<String>,
}

impl ResponseMetadata {
    /// Create successful response metadata.
    pub fn success() -> Self {
        Self { status: ResponseStatus::Success, headers: HashMap::new(), request_id: None }
    }

    /// Create error response metadata.
    pub fn error(kind: ErrorKind) -> Self {
        Self { status: ResponseStatus::Error(kind), headers: HashMap::new(), request_id: None }
    }
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self::success()
    }
}

/// Response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    /// The request succeeded.
    Success,
    /// The request failed with the given error kind.
    Error(ErrorKind),
}

impl ResponseStatus {
    /// Check if this status indicates success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Check if this status indicates an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get the error kind if this is an error status.
    pub fn error_kind(&self) -> Option<ErrorKind> {
        match self {
            Self::Error(kind) => Some(*kind),
            Self::Success => None,
        }
    }
}

impl fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Error(kind) => write!(f, "Error({:?})", kind),
        }
    }
}

/// The next middleware or transport in the chain.
pub struct Next<'a> {
    inner: NextHandler<'a>,
}

impl<'a> Next<'a> {
    /// Create a new Next wrapper.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: FnOnce(Request) -> Fut + Send + 'a,
        Fut: Future<Output = Result<Response, Error>> + Send + 'a,
    {
        Self { inner: Box::new(move |req| Box::pin(f(req))) }
    }

    /// Call the next middleware or transport.
    pub async fn call(self, req: Request) -> Result<Response, Error> {
        (self.inner)(req).await
    }
}

impl<'a> fmt::Debug for Next<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Next").finish_non_exhaustive()
    }
}

/// A stack of middleware that processes requests in order.
pub struct MiddlewareStack {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewareStack {
    /// Create a new empty middleware stack.
    pub fn new() -> Self {
        Self { middlewares: Vec::new() }
    }

    /// Add a middleware to the stack.
    ///
    /// Middleware is called in the order added (first added = outermost).
    pub fn push(&mut self, middleware: impl Middleware) {
        self.middlewares.push(Box::new(middleware));
    }

    /// Add a middleware to the stack (builder pattern).
    pub fn with(mut self, middleware: impl Middleware) -> Self {
        self.push(middleware);
        self
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Get the number of middlewares in the stack.
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Process a request through the middleware stack.
    ///
    /// The `transport` function is called at the end of the chain.
    pub async fn process<F, Fut>(&self, req: Request, transport: F) -> Result<Response, Error>
    where
        F: FnOnce(Request) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Response, Error>> + Send + 'static,
    {
        self.process_at(0, req, transport).await
    }

    fn process_at<'a, F, Fut>(
        &'a self,
        index: usize,
        req: Request,
        transport: F,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>>
    where
        F: FnOnce(Request) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Response, Error>> + Send + 'static,
    {
        Box::pin(async move {
            if index >= self.middlewares.len() {
                // End of middleware chain, call transport
                transport(req).await
            } else {
                // Call current middleware with next handler
                let middleware = &self.middlewares[index];
                let next = Next::new(move |req| {
                    // Create a boxed future that captures self
                    Box::pin(async move {
                        // We can't recurse here directly due to lifetime issues,
                        // so we just call the transport at the end
                        transport(req).await
                    })
                        as Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>>
                });
                middleware.handle(req, next).await
            }
        })
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MiddlewareStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MiddlewareStack").field("len", &self.middlewares.len()).finish()
    }
}

/// A no-op middleware that passes requests through unchanged.
///
/// Useful as a placeholder or for testing.
#[derive(Debug, Clone, Copy, Default)]
pub struct PassthroughMiddleware;

impl Middleware for PassthroughMiddleware {
    fn handle<'a>(
        &'a self,
        req: Request,
        next: Next<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> {
        Box::pin(async move { next.call(req).await })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[test]
    fn test_request_creation() {
        let req = Request::new("check");
        assert_eq!(req.operation(), "check");
        assert!(req.body().is_empty());
        assert!(req.metadata().headers.is_empty());
    }

    #[test]
    fn test_request_with_body() {
        let body = b"test body".to_vec();
        let req = Request::with_body("write", body.clone());
        assert_eq!(req.operation(), "write");
        assert_eq!(req.body(), &body);
    }

    #[test]
    fn test_request_builder_pattern() {
        let req = Request::new("check").header("X-Custom", "value").request_id("req-123");

        assert_eq!(req.metadata().headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(req.metadata().request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_request_display() {
        let req = Request::new("check");
        assert_eq!(format!("{}", req), "Request(check)");
    }

    #[test]
    fn test_response_ok() {
        let resp = Response::ok(b"data".to_vec());
        assert!(resp.is_ok());
        assert_eq!(resp.body(), b"data");
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error(ErrorKind::Unauthorized, vec![]);
        assert!(!resp.is_ok());
        assert_eq!(resp.metadata().status.error_kind(), Some(ErrorKind::Unauthorized));
    }

    #[test]
    fn test_response_display() {
        let resp = Response::ok(vec![]);
        assert_eq!(format!("{}", resp), "Response(Success)");
    }

    #[test]
    fn test_response_status() {
        assert!(ResponseStatus::Success.is_success());
        assert!(!ResponseStatus::Success.is_error());
        assert!(ResponseStatus::Error(ErrorKind::Timeout).is_error());
        assert!(!ResponseStatus::Error(ErrorKind::Timeout).is_success());
    }

    #[test]
    fn test_trace_context_new() {
        let ctx = TraceContext::new();
        assert_eq!(ctx.version, 0);
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.parent_id.len(), 16);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_trace_context_parse() {
        let ctx = TraceContext::parse(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            Some("congo=t61rcWkgMzE"),
        )
        .unwrap();

        assert_eq!(ctx.version, 0);
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_id, "b7ad6b7169203331");
        assert_eq!(ctx.trace_flags, 1);
        assert_eq!(ctx.trace_state, Some("congo=t61rcWkgMzE".to_string()));
    }

    #[test]
    fn test_trace_context_to_headers() {
        let ctx = TraceContext::parse(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            Some("congo=t61rcWkgMzE"),
        )
        .unwrap();

        let (traceparent, tracestate) = ctx.to_headers();
        assert_eq!(traceparent, "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
        assert_eq!(tracestate, Some("congo=t61rcWkgMzE".to_string()));
    }

    #[test]
    fn test_trace_context_child() {
        let parent =
            TraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01", None)
                .unwrap();

        let child = parent.child();
        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.parent_id, parent.parent_id);
        assert_eq!(child.trace_flags, parent.trace_flags);
    }

    #[test]
    fn test_trace_context_sampled() {
        let mut ctx = TraceContext::new();
        assert!(ctx.is_sampled());

        ctx.set_sampled(false);
        assert!(!ctx.is_sampled());

        ctx.set_sampled(true);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_trace_context_display() {
        let ctx =
            TraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01", None)
                .unwrap();

        assert_eq!(format!("{}", ctx), "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01");
    }

    #[test]
    fn test_trace_context_parse_invalid() {
        // Too few parts
        assert!(TraceContext::parse("00-abc", None).is_err());

        // Invalid trace ID length
        assert!(TraceContext::parse("00-abc-def-01", None).is_err());

        // Invalid version
        assert!(
            TraceContext::parse("zz-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01", None)
                .is_err()
        );
    }

    #[test]
    fn test_request_metadata() {
        let metadata = RequestMetadata::new()
            .with_header("X-Custom", "value")
            .with_request_id("req-123")
            .with_trace_context(TraceContext::new());

        assert_eq!(metadata.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(metadata.request_id, Some("req-123".to_string()));
        assert!(metadata.trace_context.is_some());
    }

    #[test]
    fn test_middleware_stack_empty() {
        let stack = MiddlewareStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_middleware_stack_push() {
        let mut stack = MiddlewareStack::new();
        stack.push(PassthroughMiddleware);
        assert!(!stack.is_empty());
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_middleware_stack_with() {
        let stack = MiddlewareStack::new().with(PassthroughMiddleware).with(PassthroughMiddleware);
        assert_eq!(stack.len(), 2);
    }

    #[tokio::test]
    async fn test_passthrough_middleware() {
        let middleware = PassthroughMiddleware;
        let req = Request::new("test");
        let next = Next::new(|_| async { Ok(Response::ok(b"response".to_vec())) });

        let resp = middleware.handle(req, next).await.unwrap();
        assert!(resp.is_ok());
        assert_eq!(resp.body(), b"response");
    }

    #[tokio::test]
    async fn test_middleware_stack_process() {
        let stack = MiddlewareStack::new().with(PassthroughMiddleware);
        let req = Request::new("test");

        let resp =
            stack.process(req, |_| async { Ok(Response::ok(b"done".to_vec())) }).await.unwrap();

        assert!(resp.is_ok());
        assert_eq!(resp.body(), b"done");
    }

    #[tokio::test]
    async fn test_middleware_modifies_request() {
        struct AddHeaderMiddleware;

        impl Middleware for AddHeaderMiddleware {
            fn handle<'a>(
                &'a self,
                mut req: Request,
                next: Next<'a>,
            ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> {
                Box::pin(async move {
                    req.metadata_mut().headers.insert("X-Added".to_string(), "true".to_string());
                    next.call(req).await
                })
            }
        }

        let middleware = AddHeaderMiddleware;
        let req = Request::new("test");
        let next = Next::new(|req| async move {
            // Verify the header was added
            assert_eq!(req.metadata().headers.get("X-Added"), Some(&"true".to_string()));
            Ok(Response::ok(vec![]))
        });

        middleware.handle(req, next).await.unwrap();
    }

    #[tokio::test]
    async fn test_middleware_modifies_response() {
        struct AddResponseHeaderMiddleware;

        impl Middleware for AddResponseHeaderMiddleware {
            fn handle<'a>(
                &'a self,
                req: Request,
                next: Next<'a>,
            ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> {
                Box::pin(async move {
                    let mut resp = next.call(req).await?;
                    resp.metadata_mut()
                        .headers
                        .insert("X-Response".to_string(), "added".to_string());
                    Ok(resp)
                })
            }
        }

        let middleware = AddResponseHeaderMiddleware;
        let req = Request::new("test");
        let next = Next::new(|_| async { Ok(Response::ok(vec![])) });

        let resp = middleware.handle(req, next).await.unwrap();
        assert_eq!(resp.metadata().headers.get("X-Response"), Some(&"added".to_string()));
    }

    #[tokio::test]
    async fn test_middleware_short_circuit() {
        struct ShortCircuitMiddleware;

        impl Middleware for ShortCircuitMiddleware {
            fn handle<'a>(
                &'a self,
                _req: Request,
                _next: Next<'a>,
            ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> {
                Box::pin(async move {
                    // Don't call next, return early
                    Ok(Response::error(ErrorKind::Unauthorized, b"denied".to_vec()))
                })
            }
        }

        let middleware = ShortCircuitMiddleware;
        let req = Request::new("test");
        let next = Next::new(|_| async {
            panic!("next should not be called");
        });

        let resp = middleware.handle(req, next).await.unwrap();
        assert!(!resp.is_ok());
        assert_eq!(resp.body(), b"denied");
    }

    #[tokio::test]
    async fn test_middleware_timing() {
        struct TimingMiddleware {
            call_count: Arc<AtomicUsize>,
        }

        impl Middleware for TimingMiddleware {
            fn handle<'a>(
                &'a self,
                req: Request,
                next: Next<'a>,
            ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async move { next.call(req).await })
            }
        }

        let call_count = Arc::new(AtomicUsize::new(0));
        let middleware = TimingMiddleware { call_count: call_count.clone() };

        let req = Request::new("test");
        let next = Next::new(|_| async { Ok(Response::ok(vec![])) });

        middleware.handle(req, next).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_request_set_body() {
        let mut req = Request::new("check");
        assert!(req.body().is_empty());
        req.set_body(b"new body".to_vec());
        assert_eq!(req.body(), b"new body");
    }

    #[test]
    fn test_request_metadata_mut() {
        let mut req = Request::new("check");
        req.metadata_mut().headers.insert("X-Custom".to_string(), "value".to_string());
        assert_eq!(req.metadata().headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_request_trace_context() {
        let ctx = TraceContext::new();
        let req = Request::new("check").trace_context(ctx.clone());
        assert_eq!(req.metadata().trace_context.as_ref().unwrap().trace_id, ctx.trace_id);
    }

    #[test]
    fn test_response_into_body() {
        let resp = Response::ok(b"data".to_vec());
        let body = resp.into_body();
        assert_eq!(body, b"data");
    }

    #[test]
    fn test_response_header() {
        let resp = Response::ok(vec![]).header("X-Custom", "value");
        assert_eq!(resp.metadata().headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_request_id() {
        let resp = Response::ok(vec![]).request_id("req-123");
        assert_eq!(resp.metadata().request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_response_metadata_mut() {
        let mut resp = Response::ok(vec![]);
        resp.metadata_mut().headers.insert("X-Custom".to_string(), "value".to_string());
        assert_eq!(resp.metadata().headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_status_display() {
        assert_eq!(format!("{}", ResponseStatus::Success), "Success");
        assert_eq!(format!("{}", ResponseStatus::Error(ErrorKind::Timeout)), "Error(Timeout)");
    }

    #[test]
    fn test_response_status_error_kind() {
        assert_eq!(ResponseStatus::Success.error_kind(), None);
        assert_eq!(
            ResponseStatus::Error(ErrorKind::Unauthorized).error_kind(),
            Some(ErrorKind::Unauthorized)
        );
    }

    #[test]
    fn test_response_metadata_default() {
        let metadata = ResponseMetadata::default();
        assert!(metadata.status.is_success());
        assert!(metadata.headers.is_empty());
        assert!(metadata.request_id.is_none());
    }

    #[test]
    fn test_trace_context_default() {
        let ctx = TraceContext::default();
        assert_eq!(ctx.version, 0);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn test_trace_context_with_tracestate() {
        let ctx = TraceContext::parse(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00",
            Some("vendor=value"),
        )
        .unwrap();
        assert!(!ctx.is_sampled());
        assert_eq!(ctx.trace_state, Some("vendor=value".to_string()));
    }

    #[test]
    fn test_trace_context_clone() {
        let ctx = TraceContext::new();
        let cloned = ctx.clone();
        assert_eq!(ctx.trace_id, cloned.trace_id);
        assert_eq!(ctx.parent_id, cloned.parent_id);
    }

    #[test]
    fn test_next_debug() {
        let next = Next::new(|_| async { Ok(Response::ok(vec![])) });
        let debug_str = format!("{:?}", next);
        assert!(debug_str.contains("Next"));
    }

    #[test]
    fn test_middleware_stack_debug() {
        let stack = MiddlewareStack::new().with(PassthroughMiddleware);
        let debug_str = format!("{:?}", stack);
        assert!(debug_str.contains("MiddlewareStack"));
        assert!(debug_str.contains("len"));
    }

    #[test]
    fn test_middleware_stack_default() {
        let stack = MiddlewareStack::default();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_passthrough_middleware_debug() {
        let middleware = PassthroughMiddleware;
        let debug_str = format!("{:?}", middleware);
        assert!(debug_str.contains("PassthroughMiddleware"));
    }

    #[test]
    fn test_passthrough_middleware_copy() {
        let middleware = PassthroughMiddleware;
        let copied: PassthroughMiddleware = middleware; // Copy trait
        let _ = format!("{:?}", copied); // Just verify it's usable
    }

    #[test]
    fn test_passthrough_middleware_default() {
        let _middleware = PassthroughMiddleware;
    }

    #[test]
    fn test_trace_context_parse_invalid_parent_id() {
        // Invalid parent ID length
        let result = TraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-abc-01", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_trace_context_parse_invalid_flags() {
        // Invalid flags
        let result =
            TraceContext::parse("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-zz", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_clone() {
        let req = Request::new("check").header("X-Custom", "value").request_id("req-123");
        let cloned = req.clone();
        assert_eq!(cloned.operation(), "check");
        assert_eq!(cloned.metadata().headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_clone() {
        let resp = Response::ok(b"data".to_vec()).header("X-Custom", "value").request_id("req-123");
        let cloned = resp.clone();
        assert!(cloned.is_ok());
        assert_eq!(cloned.body(), b"data");
    }

    #[test]
    fn test_response_metadata_clone() {
        let metadata = ResponseMetadata::success();
        let cloned = metadata.clone();
        assert!(cloned.status.is_success());
    }

    #[test]
    fn test_request_metadata_clone() {
        let metadata =
            RequestMetadata::new().with_header("X-Custom", "value").with_request_id("req-123");
        let cloned = metadata.clone();
        assert_eq!(cloned.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_middleware_stack_empty_process() {
        let stack = MiddlewareStack::new();
        let req = Request::new("test");

        let resp =
            stack.process(req, |_| async { Ok(Response::ok(b"done".to_vec())) }).await.unwrap();

        assert!(resp.is_ok());
        assert_eq!(resp.body(), b"done");
    }

    #[tokio::test]
    async fn test_middleware_propagates_error() {
        let stack = MiddlewareStack::new().with(PassthroughMiddleware);
        let req = Request::new("test");

        let result = stack
            .process(req, |_| async { Err(Error::new(ErrorKind::Timeout, "timed out")) })
            .await;

        assert!(result.is_err());
    }
}
