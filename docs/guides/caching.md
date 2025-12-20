# Caching & Invalidation

This guide covers caching strategies and invalidation patterns for optimizing authorization performance.

## Overview

The SDK provides intelligent caching with multiple invalidation strategies to balance performance with consistency.

## Basic Cache Configuration

```rust
use inferadb::{Client, CacheConfig};
use std::time::Duration;

let client = Client::builder()
    .endpoint("https://api.inferadb.io")
    .cache(CacheConfig::new()
        // Permission check results cached for 30 seconds
        .permission_ttl(Duration::from_secs(30))
        // Relationship data cached for 5 minutes
        .relationship_ttl(Duration::from_mins(5))
        // Schema cached for 1 hour (changes infrequently)
        .schema_ttl(Duration::from_hours(1)))
    .build()?;
```

## Invalidation Strategies

### Time-Based Invalidation

The simplest strategy - cache entries expire after a fixed duration:

```rust
let client = Client::builder()
    .cache(CacheConfig::new()
        .permission_ttl(Duration::from_secs(30))
        .relationship_ttl(Duration::from_mins(5)))
    .build()?;
```

**Trade-offs:**
- Simple to understand and configure
- May serve stale permissions for up to TTL duration
- Best for scenarios where eventual consistency is acceptable

### Event-Driven Invalidation

Subscribe to relationship changes for real-time cache invalidation:

```rust
use inferadb::{Client, CacheConfig, InvalidationStrategy};

let client = Client::builder()
    .endpoint("https://api.inferadb.io")
    .cache(CacheConfig::new()
        // Subscribe to relationship changes for real-time invalidation
        .invalidation(InvalidationStrategy::WatchBased)
        // Fallback TTL if watch connection drops
        .fallback_ttl(Duration::from_secs(60)))
    .build()?;

// Cache automatically invalidated when relationships change
// No stale permissions after revocation
```

**Trade-offs:**
- Near-instant invalidation on changes
- Requires persistent connection to server
- Higher resource usage
- Best for security-sensitive applications

### Targeted Invalidation

Manually invalidate specific cache entries when you know data has changed:

```rust
// Manually invalidate specific entries
client.cache().invalidate_permission("user:alice", "view", "doc:1");

// Invalidate all permissions for a subject
client.cache().invalidate_subject("user:alice");

// Invalidate all permissions for a resource
client.cache().invalidate_resource("doc:1");

// Invalidate by relation (e.g., after bulk membership change)
client.cache().invalidate_relation("group:admins", "member");

// Clear entire cache
client.cache().clear();
```

**Use cases:**
- After bulk relationship updates
- When you know specific data changed
- During deployment rollouts

### Hierarchical Invalidation

Automatically invalidate child resources when parent permissions change:

```rust
use inferadb::CacheConfig;

let client = Client::builder()
    .cache(CacheConfig::new()
        // When a folder's permissions change, invalidate all contained documents
        .hierarchical_invalidation(true)
        // Define parent-child relationships for cascade
        .hierarchy_relation("parent"))
    .build()?;

// Changing folder:root permissions automatically invalidates
// all documents with parent -> folder:root
```

## Cache Warming

Pre-populate the cache for expected access patterns:

```rust
// Pre-warm cache for expected access patterns
client.cache().warm(WarmConfig::new()
    .subjects(["user:alice", "user:bob"])
    .permissions(["view", "edit"])
    .resources(["doc:important", "folder:shared"])
).await?;

// Warm from access pattern analytics
client.cache().warm_from_analytics(
    Duration::from_hours(24),  // Look at last 24 hours
    100,                        // Top 100 access patterns
).await?;
```

**When to warm:**
- Application startup
- Before batch operations
- After cache clear

## Cache Metrics

Monitor cache effectiveness:

```rust
// Monitor cache effectiveness
let stats = client.cache().stats();
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Entries: {}", stats.entry_count());
println!("Memory: {} MB", stats.memory_bytes() / 1024 / 1024);
println!("Evictions: {}", stats.eviction_count());
```

Export metrics to your monitoring system:

```rust
// Prometheus metrics
client.cache().register_metrics(&prometheus_registry);

// OpenTelemetry metrics
client.cache().register_otel_metrics(&meter);
```

## Configuration Reference

```rust
pub struct CacheConfig {
    /// TTL for permission check results
    pub permission_ttl: Duration,

    /// TTL for relationship data
    pub relationship_ttl: Duration,

    /// TTL for schema data
    pub schema_ttl: Duration,

    /// Maximum cache entries
    pub max_entries: usize,

    /// Maximum memory usage
    pub max_memory_bytes: usize,

    /// Invalidation strategy
    pub invalidation: InvalidationStrategy,

    /// Fallback TTL when watch connection drops
    pub fallback_ttl: Duration,

    /// Enable hierarchical invalidation
    pub hierarchical_invalidation: bool,

    /// Relation name for hierarchy traversal
    pub hierarchy_relation: String,
}

pub enum InvalidationStrategy {
    /// Time-based expiration only
    TtlOnly,

    /// Subscribe to changes via watch stream
    WatchBased,

    /// Hybrid: watch with TTL fallback
    Hybrid,
}
```

## Best Practices

### 1. Choose the Right TTL

| Scenario | Recommended TTL |
|----------|-----------------|
| High-security (banking, healthcare) | 0-5 seconds or WatchBased |
| Standard web applications | 30-60 seconds |
| Read-heavy analytics | 5-15 minutes |
| Static permission models | 1+ hours |

### 2. Use Hierarchical Invalidation for Nested Resources

```rust
// Document inherits from folder
// Folder inherits from organization
client.cache().hierarchical_invalidation(true)
    .hierarchy_relations(["parent", "organization"]);
```

### 3. Warm Cache on Startup

```rust
// In your application startup
async fn init_app() -> Result<()> {
    let client = create_client().await?;

    // Warm cache for common access patterns
    client.cache().warm(WarmConfig::new()
        .from_fixture("common-access-patterns.json")
    ).await?;

    Ok(())
}
```

### 4. Monitor and Tune

```rust
// Log cache stats periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let stats = client.cache().stats();
        tracing::info!(
            hit_rate = %stats.hit_rate(),
            entries = %stats.entry_count(),
            "Cache stats"
        );
    }
});
```

### 5. Handle Watch Disconnections

```rust
let client = Client::builder()
    .cache(CacheConfig::new()
        .invalidation(InvalidationStrategy::WatchBased)
        // Fall back to short TTL if watch disconnects
        .fallback_ttl(Duration::from_secs(10))
        // Reconnect automatically
        .watch_reconnect(true)
        .watch_reconnect_delay(Duration::from_secs(1)))
    .build()?;
```

## Disabling Cache

For debugging or specific use cases:

```rust
// Disable caching entirely
let client = Client::builder()
    .cache(CacheConfig::disabled())
    .build()?;

// Bypass cache for specific check
let allowed = client.check("user:alice", "view", "doc:1")
    .bypass_cache()
    .await?;
```
