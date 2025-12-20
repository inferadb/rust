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

| Aspect | gRPC | REST |
|--------|------|------|
| Latency | ~5-10ms | ~10-20ms |
| Throughput | Higher | Lower |
| Streaming | Native | Polling |
| Binary size | Larger (+2MB) | Smaller |
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
    .pool_size(50)  // For ~30 concurrent requests
    .build()
    .await?;
```

| Workload | Recommended Pool Size |
|----------|----------------------|
| Low (< 10 RPS) | 10 (default) |
| Medium (10-100 RPS) | 20-50 |
| High (100-1000 RPS) | 50-100 |
| Very High (> 1000 RPS) | 100-200 |

### Connection Lifecycle

```rust
let client = Client::builder()
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
    .cache(CacheConfig::default()
        .max_entries(10_000)           // Adjust based on cardinality
        .ttl(Duration::from_secs(60))  // Balance freshness vs performance
        .negative_ttl(Duration::from_secs(10)))  // Cache denials shorter
    .build()
    .await?;
```

### Cache Sizing Guidelines

```
cache_size = unique_subjects * unique_resources * avg_permissions_per_check
```

| Use Case | Typical Cache Size |
|----------|-------------------|
| Single-tenant SaaS | 1,000 - 10,000 |
| Multi-tenant SaaS | 10,000 - 100,000 |
| High-cardinality | 100,000 - 1,000,000 |

### Cache Invalidation

For real-time consistency, combine caching with watch streams:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct CachedAuthz {
    client: Client,
    cache: Arc<RwLock<HashMap<CacheKey, bool>>>,
}

impl CachedAuthz {
    async fn start_invalidation(&self) {
        let cache = self.cache.clone();
        let mut stream = self.client.watch().run().await.unwrap();

        tokio::spawn(async move {
            while let Some(Ok(change)) = stream.next().await {
                let mut cache = cache.write().await;
                // Invalidate affected cache entries
                cache.retain(|k, _| !k.affected_by(&change));
            }
        });
    }
}
```

## Batch Operations

### Check Batching

Always batch multiple permission checks:

```rust
// Slow: 3 round-trips
let can_read = client.check(user, "read", doc).await?;
let can_write = client.check(user, "write", doc).await?;
let can_delete = client.check(user, "delete", doc).await?;

// Fast: 1 round-trip
let results = client
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
    client.write(Relationship::new(group, "member", member)).await?;
}

// Fast: 1 round-trip
let relationships: Vec<_> = new_members
    .iter()
    .map(|m| Relationship::new(group, "member", m))
    .collect();

client.write_batch(relationships).await?;
```

### Batch Size Limits

| Operation | Max Batch Size | Recommendation |
|-----------|---------------|----------------|
| check_batch | 1,000 | 100-500 |
| write_batch | 10,000 | 1,000-5,000 |
| delete_batch | 10,000 | 1,000-5,000 |

For larger batches, chunk your requests:

```rust
use futures::stream::{self, StreamExt};

async fn bulk_write(client: &Client, relationships: Vec<Relationship>) -> Result<(), Error> {
    let chunks: Vec<_> = relationships.chunks(1000).collect();

    stream::iter(chunks)
        .map(|chunk| client.write_batch(chunk.to_vec()))
        .buffer_unordered(4)  // 4 concurrent batches
        .try_collect()
        .await
}
```

## Request Coalescing

For high-frequency identical checks, enable request coalescing:

```rust
let client = Client::builder()
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

| Operation | Typical Latency | Recommended Timeout |
|-----------|-----------------|---------------------|
| check | 5-20ms | 100ms |
| check_batch | 10-50ms | 200ms |
| write | 10-30ms | 1s |
| write_batch | 50-200ms | 5s |
| list_resources | 20-100ms | 500ms |
| expand | 10-50ms | 200ms |

## Retry Configuration

### Exponential Backoff

```rust
let client = Client::builder()
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
// Memory-efficient: Stream processing
let mut stream = client
    .list_resources(user, "view")
    .resource_type("document")
    .stream();

while let Some(resource) = stream.next().await {
    let resource = resource?;
    process_resource(resource);
}

// Memory-heavy: Collect all
let resources: Vec<_> = client
    .list_resources(user, "view")
    .resource_type("document")
    .collect()
    .await?;  // All in memory at once
```

### String Interning for Hot Paths

For repeated entity references:

```rust
use std::sync::Arc;

// Instead of cloning strings repeatedly
let subject = Arc::from("user:alice");
let permission = Arc::from("view");

for resource in resources {
    client.check(&subject, &permission, resource).await?;
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
    let client = rt.block_on(test_client());

    c.bench_function("check", |b| {
        b.to_async(&rt).iter(|| {
            client.check("user:alice", "view", "document:readme")
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
    .with_tracing()
    .build()
    .await?;

// Analyze with tokio-console
// Run with: RUSTFLAGS="--cfg tokio_unstable" cargo run
```

### Latency Breakdown

```rust
use tracing::{instrument, info_span};

#[instrument(skip(client))]
async fn authorize_request(client: &Client, req: &Request) -> Result<bool, Error> {
    let span = info_span!("permission_check",
        subject = %req.user_id,
        resource = %req.resource_id,
    );

    client
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

| Metric | Target |
|--------|--------|
| Single check latency (p50) | < 10ms |
| Single check latency (p99) | < 50ms |
| Batch check throughput | 10,000+ checks/sec |
| Write throughput | 5,000+ writes/sec |
| Memory per connection | ~50KB |
| Cache memory (10K entries) | ~5MB |
