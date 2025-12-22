# Performance Tuning Guide

This guide covers optimization strategies for high-performance applications using the InferaDB Rust SDK.

## Quick Wins

Before diving into advanced tuning, ensure you've implemented these basics:

1. **Reuse clients** - Create once, share across requests
2. **Use batch operations** - One request for multiple checks
3. **Enable caching** - Reduce network round-trips
4. **Use gRPC transport** - Lower latency than REST

## Transport Selection

### gRPC vs REST

| Aspect        | gRPC            | REST      |
| ------------- | --------------- | --------- |
| Latency       | ~5-10ms         | ~10-20ms  |
| Throughput    | Higher          | Lower     |
| Streaming     | Native          | Polling   |
| Binary size   | Larger (+2MB)   | Smaller   |
| Compatibility | HTTP/2 required | Universal |

**Recommendation:** Use gRPC for latency-sensitive paths, REST for broader compatibility.

```toml
# gRPC (default, best performance)
inferadb = { version = "0.1", features = ["grpc"] }

# REST (broader compatibility)
inferadb = { version = "0.1", default-features = false, features = ["rest"] }
```

## Connection Pool Tuning

### Pool Size Guidelines

The optimal pool size depends on your concurrency needs:

```rust
// Formula: pool_size = expected_concurrent_requests * 1.5
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .pool_size(50)  // For ~30 concurrent requests
    .build()
    .await?;

let vault = client.organization("org_...").vault("vlt_...");
```

| Workload               | Recommended Pool Size |
| ---------------------- | --------------------- |
| Low (< 10 RPS)         | 10 (default)          |
| Medium (10-100 RPS)    | 20-50                 |
| High (100-1000 RPS)    | 50-100                |
| Very High (> 1000 RPS) | 100-200               |

### Connection Lifecycle

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    // Eager connection - validates during build
    .eager_connect(true)

    // Keep-alive for long-lived connections
    .keep_alive_interval(Duration::from_secs(30))
    .keep_alive_timeout(Duration::from_secs(10))

    // Idle connection cleanup
    .idle_timeout(Duration::from_secs(300))

    .build()
    .await?;
```

## Caching Strategies

### Local Decision Cache

Enable caching for read-heavy workloads:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_secs(300))
        .schema_ttl(Duration::from_secs(3600))
        .negative_ttl(Duration::from_secs(10))
        .max_entries(10_000))
    .build()
    .await?;
```

### Cache Sizing Guidelines

```text
cache_size = unique_subjects * unique_resources * avg_permissions_per_check
```

| Use Case           | Typical Cache Size  |
| ------------------ | ------------------- |
| Single-tenant SaaS | 1,000 - 10,000      |
| Multi-tenant SaaS  | 10,000 - 100,000    |
| High-cardinality   | 100,000 - 1,000,000 |

### Cache Invalidation

For real-time consistency, use watch-based invalidation:

```rust
use inferadb::CacheInvalidation;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .cache(CacheConfig::default())
    .cache_invalidation(CacheInvalidation::Watch)
    .build()
    .await?;
```

## Batch Operations

### Check Batching

Always batch multiple permission checks:

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Slow: 3 round-trips
let can_read = vault.check(user, "read", doc).await?;
let can_write = vault.check(user, "write", doc).await?;
let can_delete = vault.check(user, "delete", doc).await?;

// Fast: 1 round-trip
let results = vault
    .check_batch([
        (user, "read", doc),
        (user, "write", doc),
        (user, "delete", doc),
    ])
    .collect()
    .await?;
```

### Write Batching

Batch relationship writes for bulk operations:

```rust
// Slow: N round-trips
for member in new_members {
    vault.relationships().write(Relationship::new(group, "member", member)).await?;
}

// Fast: 1 round-trip
let relationships: Vec<_> = new_members
    .iter()
    .map(|m| Relationship::new(group, "member", m))
    .collect();

vault.relationships().write_batch(relationships).await?;
```

### Batch Size Limits

| Operation    | Max Batch Size | Recommendation |
| ------------ | -------------- | -------------- |
| check_batch  | 1,000          | 100-500        |
| write_batch  | 10,000         | 1,000-5,000    |
| delete_batch | 10,000         | 1,000-5,000    |

For larger batches, chunk your requests:

```rust
use futures::stream::{self, StreamExt};

async fn bulk_write(vault: &VaultClient, relationships: Vec<Relationship>) -> Result<(), Error> {
    let chunks: Vec<_> = relationships.chunks(1000).collect();

    stream::iter(chunks)
        .map(|chunk| vault.relationships().write_batch(chunk.to_vec()))
        .buffer_unordered(4)  // 4 concurrent batches
        .try_collect()
        .await
}
```

## Request Coalescing

For high-frequency identical checks, enable request coalescing:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .coalesce_requests(true)  // Deduplicate in-flight requests
    .coalesce_window(Duration::from_millis(5))
    .build()
    .await?;
```

This is useful when:

- Multiple tasks check the same permission simultaneously
- You have a cache miss stampede scenario
- High request rates with repeated patterns

## Timeout Tuning

### Recommended Timeouts

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    // Connection establishment
    .connect_timeout(Duration::from_secs(5))

    // Individual request timeout
    .request_timeout(Duration::from_secs(30))

    // Per-operation timeouts (override defaults)
    .check_timeout(Duration::from_millis(100))
    .write_timeout(Duration::from_secs(5))

    .build()
    .await?;
```

### Timeout by Operation Type

| Operation      | Typical Latency | Recommended Timeout |
| -------------- | --------------- | ------------------- |
| check          | 5-20ms          | 100ms               |
| check_batch    | 10-50ms         | 200ms               |
| write          | 10-30ms         | 1s                  |
| write_batch    | 50-200ms        | 5s                  |
| list_resources | 20-100ms        | 500ms               |
| expand         | 10-50ms         | 200ms               |

## Retry Configuration

### Exponential Backoff

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .retry_config(RetryConfig::default()
        .max_retries(3)
        .initial_backoff(Duration::from_millis(50))
        .max_backoff(Duration::from_secs(2))
        .backoff_multiplier(2.0)
        .jitter(0.1))  // 10% jitter to prevent thundering herd
    .build()
    .await?;
```

### Retry Budget

For high-throughput systems, use a retry budget:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .retry_config(RetryConfig::default()
        .retry_budget(RetryBudget::new()
            .ttl(Duration::from_secs(10))
            .min_retries_per_second(10)
            .retry_ratio(0.1)))  // Max 10% of requests can be retries
    .build()
    .await?;
```

## Memory Optimization

### Streaming Large Results

For large list operations, use streaming to avoid memory spikes:

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Memory-efficient: Stream processing
let mut stream = vault
    .resources()
    .accessible_by(user)
    .with_permission("view")
    .resource_type("document")
    .stream();

while let Some(resource) = stream.next().await {
    let resource = resource?;
    process_resource(resource);
}

// Memory-heavy: Collect all
let resources: Vec<_> = vault
    .resources()
    .accessible_by(user)
    .with_permission("view")
    .resource_type("document")
    .collect()
    .await?;  // All in memory at once
```

### String Interning for Hot Paths

For repeated entity references:

```rust
use std::sync::Arc;

let vault = client.organization("org_...").vault("vlt_...");

// Instead of cloning strings repeatedly
let subject = Arc::from("user:alice");
let permission = Arc::from("view");

for resource in resources {
    vault.check(&subject, &permission, resource).await?;
}
```

## Benchmarking

### Built-in Benchmarks

```bash
# Run SDK benchmarks
cargo bench --features bench

# Profile specific operations
cargo bench --features bench -- check
cargo bench --features bench -- batch
```

### Custom Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_check(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (client, vault) = rt.block_on(async {
        let client = test_client().await;
        let vault = client.organization("org_...").vault("vlt_...");
        (client, vault)
    });

    c.bench_function("check", |b| {
        b.to_async(&rt).iter(|| {
            vault.check("user:alice", "view", "document:readme")
        });
    });
}

criterion_group!(benches, benchmark_check);
criterion_main!(benches);
```

## Profiling Tips

### Tracing Spans

Enable detailed tracing for performance analysis:

```rust
// Enable tracing
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .with_tracing()
    .build()
    .await?;

// Analyze with tokio-console
// Run with: RUSTFLAGS="--cfg tokio_unstable" cargo run
```

### Latency Breakdown

```rust
use tracing::{instrument, info_span};

#[instrument(skip(vault))]
async fn authorize_request(vault: &VaultClient, req: &Request) -> Result<bool, Error> {
    let span = info_span!("permission_check",
        subject = %req.user_id,
        resource = %req.resource_id,
    );

    vault
        .check(&req.user_id, "access", &req.resource_id)
        .instrument(span)
        .await
}
```

## Performance Checklist

Before optimizing, verify:

- [ ] Single client instance shared across requests
- [ ] Batch operations used where possible
- [ ] Caching enabled with appropriate TTL
- [ ] gRPC transport for latency-sensitive paths
- [ ] Connection pool sized for workload
- [ ] Timeouts configured appropriately
- [ ] Retries with exponential backoff
- [ ] Streaming used for large result sets
- [ ] Metrics exported for monitoring

## Expected Performance

With proper tuning on modern hardware:

| Metric                     | Target             |
| -------------------------- | ------------------ |
| Single check latency (p50) | < 10ms             |
| Single check latency (p99) | < 50ms             |
| Batch check throughput     | 10,000+ checks/sec |
| Write throughput           | 5,000+ writes/sec  |
| Memory per connection      | ~50KB              |
| Cache memory (10K entries) | ~5MB               |
