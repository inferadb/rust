# Observability

Integrate InferaDB SDK with your monitoring stack using tracing, metrics, and distributed tracing.

## Quick Setup

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .observability(ObservabilityConfig::default()
        .tracing(true)
        .metrics(true))
    .build()
    .await?;
```

## Tracing Integration

The SDK emits structured spans compatible with the `tracing` ecosystem.

### Enable Tracing

```toml
[dependencies]
inferadb = { version = "0.1", features = ["tracing"] }
tracing-subscriber = "0.3"
```

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .init();

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .observability(ObservabilityConfig::default().tracing(true))
    .build()
    .await?;
```

### Span Attributes

All SDK operations emit spans with consistent attributes:

| Attribute             | Example           | Description              |
| --------------------- | ----------------- | ------------------------ |
| `inferadb.operation`  | `check`           | Operation type           |
| `inferadb.subject`    | `user:alice`      | Subject being checked    |
| `inferadb.permission` | `view`            | Permission being checked |
| `inferadb.resource`   | `document:readme` | Target resource          |
| `inferadb.vault_id`   | `vlt_01JFQGK...`  | Vault context            |
| `inferadb.allowed`    | `true`            | Authorization result     |
| `inferadb.latency_ms` | `12`              | Operation latency        |

### Example Output

```text
2024-01-15T10:30:00Z INFO inferadb::check: check
    inferadb.subject="user:alice"
    inferadb.permission="view"
    inferadb.resource="document:readme"
    inferadb.allowed=true
    inferadb.latency_ms=8
```

## Metrics

The SDK exports metrics in Prometheus format.

### Enable Metrics

```toml
[dependencies]
inferadb = { version = "0.1", features = ["metrics"] }
```

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .observability(ObservabilityConfig::default().metrics(true))
    .build()
    .await?;
```

### Available Metrics

| Metric                               | Type      | Labels                 | Description                   |
| ------------------------------------ | --------- | ---------------------- | ----------------------------- |
| `inferadb_check_total`               | Counter   | `result`, `cached`     | Total permission checks       |
| `inferadb_check_latency_seconds`     | Histogram | `operation`            | Check latency distribution    |
| `inferadb_write_total`               | Counter   | `operation`            | Relationship write operations |
| `inferadb_connection_pool_size`      | Gauge     |                        | Current pool size             |
| `inferadb_connection_pool_available` | Gauge     |                        | Available connections         |
| `inferadb_cache_hit_total`           | Counter   | `cache_type`           | Cache hits                    |
| `inferadb_cache_miss_total`          | Counter   | `cache_type`           | Cache misses                  |
| `inferadb_token_refresh_total`       | Counter   | `result`               | Token refresh attempts        |
| `inferadb_retry_total`               | Counter   | `operation`, `attempt` | Retry attempts                |

### Key Metrics to Monitor

| Metric                                            | Alert Threshold | Description               |
| ------------------------------------------------- | --------------- | ------------------------- |
| `inferadb_check_latency_seconds{quantile="0.99"}` | > 100ms         | P99 authorization latency |
| `rate(inferadb_check_total{result="error"}[5m])`  | > 1%            | Error rate                |
| `inferadb_connection_pool_available`              | < 5             | Pool exhaustion risk      |
| `inferadb_token_refresh_total{result="failure"}`  | > 0             | Auth issues               |
| `inferadb_cache_hit_total / (hit + miss)`         | < 50%           | Cache ineffective         |

## OpenTelemetry Integration

For distributed tracing with OpenTelemetry:

```toml
[dependencies]
inferadb = { version = "0.1", features = ["opentelemetry"] }
opentelemetry = "0.21"
tracing-opentelemetry = "0.22"
```

```rust
use opentelemetry::sdk::trace::TracerProvider;
use tracing_opentelemetry::OpenTelemetryLayer;

let tracer = TracerProvider::builder()
    .with_simple_exporter(opentelemetry_jaeger::new_agent_pipeline().build_simple()?)
    .build()
    .tracer("inferadb-app");

tracing_subscriber::registry()
    .with(OpenTelemetryLayer::new(tracer))
    .init();

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .observability(ObservabilityConfig::default()
        .tracing(true)
        .sampling(SamplingConfig::default().ratio(0.1)))  // 10% sampling
    .build()
    .await?;
```

### Sampling Configuration

```rust
SamplingConfig::default()
    .ratio(0.1)                    // Sample 10% of traces
    .always_sample_errors(true)    // Always sample errors
    .always_sample_slow(Duration::from_millis(100))  // Sample slow ops
```

## W3C Trace Context Propagation

The SDK automatically propagates W3C Trace Context headers:

```rust
// Trace context flows through SDK calls
let trace_ctx = TraceContext::new();

vault.check("user:alice", "view", "doc:1")
    .with_trace_context(trace_ctx)
    .await?;

// Headers propagated:
// traceparent: 00-{trace_id}-{span_id}-{flags}
// tracestate: inferadb=...
```

### Extract from Incoming Request

```rust
use inferadb::tracing::TraceContext;

async fn handler(headers: HeaderMap) -> Result<Response, Error> {
    let trace_ctx = TraceContext::extract_from_headers(&headers)?;

    vault.check("user:alice", "view", "doc:1")
        .with_trace_context(trace_ctx)
        .await?;

    Ok(Response::ok())
}
```

### Framework Integration

```rust
// Axum middleware
use inferadb::integrations::axum::InferaDbTraceLayer;

let app = Router::new()
    .route("/api/documents", get(list_documents))
    .layer(InferaDbTraceLayer::new(client.clone()));

// Actix middleware
use inferadb::integrations::actix::TracingMiddleware;

App::new()
    .wrap(TracingMiddleware::new(client.clone()))
    .service(web::resource("/documents").to(list_documents))
```

## Health Checks

Monitor SDK health in your application:

```rust
// Lightweight connectivity check
vault.health().await?;

// Detailed diagnostics
let report = client.diagnostics().await?;
println!("Transport: {:?}", report.transport);
println!("Pool stats: {:?}", report.pool);
println!("Auth status: {:?}", report.auth);
```

### Diagnostics Report

```rust
pub struct DiagnosticsReport {
    pub transport: Transport,
    pub pool: PoolStats,
    pub auth: AuthStatus,
    pub checks: Vec<DiagnosticCheck>,
}

pub struct PoolStats {
    pub size: u32,
    pub available: u32,
    pub in_use: u32,
    pub idle_timeout: Duration,
}
```

## Latency Breakdown

Enable detailed latency tracing:

```rust
let result = vault.check("user:alice", "view", "doc:1")
    .trace(true)
    .await?;

// Access timing breakdown
if let Some(trace) = result.trace() {
    println!("Total: {:?}", trace.total_duration);
    println!("Network: {:?}", trace.network_duration);
    println!("Server: {:?}", trace.server_duration);
}
```

## Transport Statistics

Monitor protocol-level stats:

```rust
let stats = client.transport_stats();

println!("Active transport: {:?}", stats.active_transport);
println!("Fallback count: {}", stats.fallback_count);

// gRPC stats
if let Some(grpc) = stats.grpc {
    println!("gRPC requests: {}", grpc.requests_sent);
    println!("Active streams: {}", grpc.streams_active);
}

// REST stats
if let Some(rest) = stats.rest {
    println!("REST requests: {}", rest.requests_sent);
    println!("SSE connections: {}", rest.sse_active);
}
```

### Transport Events

```rust
let mut events = client.transport_events();

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        match event {
            TransportEvent::FallbackTriggered { from, to, reason } => {
                tracing::warn!("Fallback: {:?} -> {:?}: {:?}", from, to, reason);
            }
            TransportEvent::Restored { transport } => {
                tracing::info!("Restored: {:?}", transport);
            }
        }
    }
});
```

## Dashboard Recommendations

### Grafana Dashboard Panels

1. **Authorization Latency** - P50, P95, P99 of `inferadb_check_latency_seconds`
2. **Request Rate** - `rate(inferadb_check_total[1m])`
3. **Error Rate** - `rate(inferadb_check_total{result="error"}[5m]) / rate(inferadb_check_total[5m])`
4. **Cache Hit Rate** - `inferadb_cache_hit_total / (hit + miss)`
5. **Connection Pool** - `inferadb_connection_pool_available` vs `size`
6. **Token Health** - `inferadb_token_refresh_total{result="failure"}`

### Alerting Rules

```yaml
groups:
  - name: inferadb
    rules:
      - alert: InferaDBHighLatency
        expr: histogram_quantile(0.99, inferadb_check_latency_seconds) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "InferaDB P99 latency above 100ms"

      - alert: InferaDBHighErrorRate
        expr: rate(inferadb_check_total{result="error"}[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "InferaDB error rate above 1%"

      - alert: InferaDBPoolExhausted
        expr: inferadb_connection_pool_available < 5
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "InferaDB connection pool nearly exhausted"
```

## Best Practices

1. **Sample in production** - Use 1-10% sampling for high-traffic services
2. **Always sample errors** - Ensure error traces are captured
3. **Monitor cache effectiveness** - Low hit rates indicate configuration issues
4. **Set latency alerts** - P99 > 100ms usually indicates problems
5. **Include request IDs** - All errors include `request_id()` for debugging

## Related Guides

- [Performance Tuning](performance-tuning.md) - Optimize latency
- [Production Checklist](production-checklist.md) - Monitoring requirements
- [Debugging](debugging.md) - Using traces for troubleshooting
