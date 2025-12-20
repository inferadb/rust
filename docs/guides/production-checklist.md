# Production Checklist

Use this checklist before deploying applications using the InferaDB Rust SDK to production.

## Authentication & Security

- [ ] **Use client credentials authentication** (not bearer tokens) for service-to-service communication
- [ ] **Store private keys securely** (environment variables, secrets manager, or HSM)
- [ ] **Never commit private keys** to version control
- [ ] **Rotate keys periodically** and have a rotation procedure documented
- [ ] **Use TLS** - never use `.insecure()` in production
- [ ] **Pin TLS certificates** for high-security environments
- [ ] **Audit service permissions** - principle of least privilege

```rust
// Production client configuration
let client = Client::builder()
    .url("https://api.inferadb.com")
    .client_credentials(ClientCredentials {
        client_id: std::env::var("INFERADB_CLIENT_ID")?,
        private_key: Ed25519PrivateKey::from_pem(
            &std::env::var("INFERADB_PRIVATE_KEY")?
        )?,
        certificate_id: None,
    })
    .default_vault(&std::env::var("INFERADB_VAULT_ID")?)
    .build()
    .await?;
```

## Connection Management

- [ ] **Reuse clients** - create once, share across requests
- [ ] **Configure connection pool** appropriately for your load
- [ ] **Set appropriate timeouts** for your SLA requirements
- [ ] **Configure retry policy** with exponential backoff
- [ ] **Test failover behavior** before going live

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .pool_size(50)  // Size for expected concurrency
    .connect_timeout(Duration::from_secs(5))
    .request_timeout(Duration::from_secs(30))
    .retry_config(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(100))
        .max_backoff(Duration::from_secs(10)))
    .build()
    .await?;
```

## Error Handling

- [ ] **Handle all error kinds** appropriately
- [ ] **Log request IDs** for debugging and support
- [ ] **Implement circuit breakers** for dependent services
- [ ] **Define fallback behavior** for authorization failures
- [ ] **Alert on elevated error rates**

```rust
match client.check(subject, permission, resource).await {
    Ok(allowed) => Ok(allowed),
    Err(e) => {
        // Log with request ID for debugging
        tracing::error!(
            request_id = ?e.request_id(),
            error_kind = ?e.kind(),
            "Authorization check failed"
        );

        match e.kind() {
            // Fail closed for auth errors
            ErrorKind::Unauthorized | ErrorKind::Forbidden => {
                Err(AppError::AuthorizationError)
            }
            // Retriable errors - circuit breaker decision
            _ if e.is_retriable() => {
                if circuit_breaker.is_open() {
                    // Fail closed or use cached decision
                    Err(AppError::ServiceUnavailable)
                } else {
                    Err(AppError::TemporaryFailure)
                }
            }
            _ => Err(AppError::InternalError),
        }
    }
}
```

## Performance

- [ ] **Use batch operations** for multiple checks
- [ ] **Enable caching** for read-heavy workloads
- [ ] **Use gRPC transport** for latency-sensitive paths
- [ ] **Profile authorization latency** in your application
- [ ] **Set up latency alerts** (p50, p95, p99)

```rust
// Batch checks instead of sequential
let results = client
    .check_batch([
        (subject, "read", resource),
        (subject, "write", resource),
        (subject, "delete", resource),
    ])
    .collect()
    .await?;

// Enable caching for repeated checks
let client = Client::builder()
    .cache(CacheConfig::default()
        .max_entries(10_000)
        .ttl(Duration::from_secs(60)))
    .build()
    .await?;
```

## Observability

- [ ] **Enable tracing** for distributed traces
- [ ] **Export metrics** to your monitoring system
- [ ] **Set up dashboards** for authorization metrics
- [ ] **Configure alerts** for error rates and latency
- [ ] **Include request IDs** in application logs

```rust
// Enable OpenTelemetry integration
let client = Client::builder()
    .with_tracing()
    .with_metrics()
    .build()
    .await?;
```

### Key Metrics to Monitor

| Metric | Alert Threshold | Description |
|--------|-----------------|-------------|
| `inferadb.check.latency_p99` | > 100ms | Authorization check latency |
| `inferadb.check.error_rate` | > 1% | Check error rate |
| `inferadb.connection.pool_exhausted` | > 0 | Connection pool saturation |
| `inferadb.token.refresh_failures` | > 0 | Authentication issues |
| `inferadb.cache.hit_rate` | < 50% | Cache effectiveness |

## Testing

- [ ] **Unit tests** use `MockClient` (no network)
- [ ] **Integration tests** run against test vault
- [ ] **Load tests** completed with production-like data
- [ ] **Chaos testing** for failure scenarios
- [ ] **Test key rotation** procedure

```rust
// Unit tests with mocks
#[tokio::test]
async fn test_authorization_logic() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:alice", "delete", "doc:1", false)
        .build();

    assert!(authorize_action(&mock, "alice", "view", "doc:1").await?);
    assert!(!authorize_action(&mock, "alice", "delete", "doc:1").await?);
}

// Integration tests with isolated vault
#[tokio::test]
#[ignore]  // Run with --ignored
async fn integration_test() {
    let client = test_client().await;
    let vault = TestVault::create(&client).await?;

    // Test with real service, isolated data
    vault.write(Relationship::new("doc:1", "owner", "user:alice")).await?;
    assert!(vault.check("user:alice", "delete", "doc:1").await?);
}
```

## Deployment

- [ ] **Gradual rollout** with canary deployments
- [ ] **Feature flags** for new authorization logic
- [ ] **Rollback plan** documented and tested
- [ ] **Health checks** include authorization service connectivity
- [ ] **Graceful shutdown** drains in-flight requests

```rust
// Health check endpoint
async fn health_check(client: &Client) -> Result<(), Error> {
    // Lightweight check that verifies connectivity
    client.health().await
}

// Graceful shutdown
async fn shutdown(client: Client) {
    // Client drops cleanly, completing in-flight requests
    drop(client);
}
```

## Documentation

- [ ] **Runbook** for authorization-related incidents
- [ ] **On-call documentation** for common issues
- [ ] **Architecture diagram** showing authorization flow
- [ ] **Data flow documentation** for compliance

## Compliance

- [ ] **Audit logging** enabled for sensitive operations
- [ ] **Data residency** requirements met
- [ ] **Access reviews** scheduled for service permissions
- [ ] **Incident response** plan includes authorization failures

## Pre-Launch Verification

```bash
# Verify connectivity
cargo run --example health_check

# Verify authentication
cargo run --example auth_test

# Verify basic authorization
cargo run --example smoke_test

# Run load test
cargo run --example load_test -- --rps 1000 --duration 60s
```

## Post-Launch Monitoring

First 24 hours after deployment:

- [ ] Monitor error rates (should be < 0.1%)
- [ ] Check latency percentiles (p99 < 100ms)
- [ ] Verify cache hit rates (should stabilize > 50%)
- [ ] Review logs for unexpected errors
- [ ] Confirm metrics are flowing to dashboards
