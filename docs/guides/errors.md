# Error Handling

The InferaDB SDK provides typed errors that enable precise handling of failure scenarios.

## Error Types

```rust
use inferadb::{Error, ErrorKind};

match vault.check("user:alice", "view", "doc:1").await {
    Ok(allowed) => { /* handle result */ }
    Err(e) => {
        match e.kind() {
            ErrorKind::Unauthorized => { /* credentials invalid */ }
            ErrorKind::Forbidden => { /* insufficient permissions */ }
            ErrorKind::NotFound => { /* resource/vault not found */ }
            ErrorKind::RateLimited => { /* back off and retry */ }
            ErrorKind::SchemaViolation => { /* invalid relation/permission */ }
            ErrorKind::Unavailable => { /* service temporarily down */ }
            ErrorKind::Timeout => { /* request timed out */ }
            _ => { /* other error */ }
        }
    }
}
```

## check() vs require()

The SDK provides two patterns for authorization checks:

| Method      | Returns                    | Use Case                              |
| ----------- | -------------------------- | ------------------------------------- |
| `check()`   | `Result<bool, Error>`      | When you need the boolean value       |
| `require()` | `Result<(), AccessDenied>` | Guard clauses, early-return on denial |

```rust
// check() - returns bool, denial is Ok(false)
let allowed = vault.check("user:alice", "view", "doc:1").await?;
if !allowed {
    return Err(AppError::Forbidden);
}

// require() - denial is Err(AccessDenied), integrates with ?
vault.check("user:alice", "view", "doc:1")
    .require()
    .await?;  // Returns early on denial
```

**Key invariant**: `check()` returns `Ok(false)` for denied access. Only `require()` converts denial to an error.

## AccessDenied Error

The `AccessDenied` error integrates with web frameworks:

```rust
use inferadb::AccessDenied;

// Axum
impl IntoResponse for AccessDenied {
    fn into_response(self) -> Response {
        StatusCode::FORBIDDEN.into_response()
    }
}

// Actix-web
impl ResponseError for AccessDenied {
    fn status_code(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}
```

## Retriable Errors

Check if an error is safe to retry:

```rust
match vault.check(subject, permission, resource).await {
    Ok(allowed) => Ok(allowed),
    Err(e) if e.is_retriable() => {
        // Safe to retry: Unavailable, Timeout, RateLimited
        let delay = e.retry_after().unwrap_or(Duration::from_millis(100));
        tokio::time::sleep(delay).await;
        // Retry...
    }
    Err(e) => Err(e),  // Not retriable
}
```

**Retriable errors**:

| ErrorKind         | Retry? | Notes                         |
| ----------------- | ------ | ----------------------------- |
| `Unavailable`     | Yes    | Service temporarily down      |
| `Timeout`         | Yes    | Request timed out             |
| `RateLimited`     | Yes    | Use `retry_after()` for delay |
| `Unauthorized`    | No     | Fix credentials               |
| `Forbidden`       | No     | Fix permissions               |
| `NotFound`        | No     | Resource doesn't exist        |
| `SchemaViolation` | No     | Fix schema/query              |
| `InvalidArgument` | No     | Fix input                     |

## Request IDs

All errors include request IDs for debugging:

```rust
match vault.check(subject, permission, resource).await {
    Err(e) => {
        if let Some(request_id) = e.request_id() {
            tracing::error!(
                request_id = %request_id,
                error = %e,
                "Authorization check failed"
            );
        }
    }
    Ok(_) => {}
}
```

## Error Context

Errors include context for debugging:

```rust
let e: Error = /* ... */;

// Error kind for matching
e.kind();  // ErrorKind::RateLimited

// Request ID for support
e.request_id();  // Some("req_abc123...")

// Retry guidance for rate limits
e.retry_after();  // Some(Duration::from_secs(5))

// Full error chain
eprintln!("{:#}", e);
```

## Retry Configuration

Configure retry behavior per operation category:

```rust
use inferadb::{RetryConfig, OperationRetry, RetryBudget};

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .retry(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10))
        // Retry budget prevents retry storms
        .retry_budget(RetryBudget::default()
            .retry_ratio(0.1)  // Max 10% retries
            .min_retries_per_second(10))
        // Per-category settings
        .reads(OperationRetry::enabled())
        .idempotent_writes(OperationRetry::enabled())
        .non_idempotent_writes(OperationRetry::connection_only()))
    .build()
    .await?;
```

### Operation Categories

| Category               | Default Behavior      | Notes                                 |
| ---------------------- | --------------------- | ------------------------------------- |
| `reads`                | Retry all errors      | Checks, lookups - always safe         |
| `idempotent_writes`    | Retry all errors      | Writes with request ID                |
| `non_idempotent_writes`| Connection errors only| Safe: request didn't reach server     |

### Retry Budget

Prevents cascading failures under load:

```rust
RetryBudget::default()
    .ttl(Duration::from_secs(10))     // Tracking window
    .min_retries_per_second(10)       // Always allow 10/sec
    .retry_ratio(0.1)                 // Max 10% of requests
```

## Request ID for Idempotent Writes

Ensure safe retries for mutations:

```rust
use uuid::Uuid;

// Generate ID once, reuse for retries
let request_id = Uuid::new_v4();

vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)
    .await?;

// Safe to retry with same ID - server deduplicates
vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .request_id(request_id)  // Same ID
    .await?;
```

### Auto-Generated Request IDs

```rust
let client = Client::builder()
    .auto_request_id(true)  // Generate UUID for each mutation
    .build()
    .await?;
```

## Best Practices

1. **Use `require()` for guards** - Cleaner code, integrates with `?`
2. **Log request IDs** - Essential for debugging production issues
3. **Handle rate limits** - Use `retry_after()` for backoff
4. **Fail closed** - Default to denying access on errors
5. **Categorize errors** - Distinguish user errors from system errors
6. **Use retry budgets** - Prevent retry storms in production
7. **Use request IDs** - Enable safe retries for writes
