# Caching

Local caching reduces latency and network load for authorization checks.

## Configuration

```rust
use inferadb::{Client, CacheConfig};
use std::time::Duration;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(30))     // Check results
        .relationship_ttl(Duration::from_secs(300))  // Relationship data
        .schema_ttl(Duration::from_secs(3600))       // Schema metadata
        .negative_ttl(Duration::from_secs(10))       // Denial results
        .max_entries(10_000))
    .build()
    .await?;
```

## TTL Guidelines

| Scenario                | Permission TTL | Notes                      |
| ----------------------- | -------------- | -------------------------- |
| High-security (banking) | 0-5s           | Near real-time consistency |
| Standard web apps       | 30-60s         | Balanced performance       |
| Read-heavy analytics    | 5-15min        | Maximize cache hits        |
| Static permissions      | 1h+            | Rarely changing access     |

**Negative TTL**: Cache denials for shorter periods than grants. A denied user who receives access should see it quickly.

## Defaults

```rust
CacheConfig::default()
// permission_ttl: 30s
// relationship_ttl: 5min
// schema_ttl: 1h
// negative_ttl: 10s
// max_entries: 10,000
```

## Disabling Cache

```rust
// Disable entirely (all requests hit server)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .cache(CacheConfig::disabled())
    .build()
    .await?;
```

## Cache Sizing

```text
entries ≈ unique_subjects × unique_resources × permissions_per_check
```

| Use Case           | Typical Size   |
| ------------------ | -------------- |
| Single-tenant SaaS | 1,000-10,000   |
| Multi-tenant SaaS  | 10,000-100,000 |
| High-cardinality   | 100,000+       |

## Invalidation Strategies

The SDK supports multiple cache invalidation approaches:

### TTL-Only (Default)

Entries expire based on configured TTLs. Simple and predictable.

### Watch-Based

Subscribe to relationship changes for real-time invalidation:

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

### Consistency Token

Invalidate when a newer consistency token is observed:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(credentials)
    .cache_invalidation(CacheInvalidation::ConsistencyToken)
    .build()
    .await?;
```

## Bypassing Cache with Consistency Tokens

For read-after-write consistency, use `.at_least_as_fresh_as()` to bypass the cache:

```rust
// Write returns a consistency token
let result = vault.relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

// This check bypasses cache, hits server directly
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .at_least_as_fresh_as(result.consistency_token())
    .await?;
```

See [Consistency & Watch](consistency.md) for full documentation on consistency tokens.

## Best Practices

1. **Start with defaults** - They work well for most applications
2. **Tune based on metrics** - Monitor hit rates before adjusting
3. **Shorter negative TTL** - Grants should propagate faster than revocations initially cached
4. **Size for peak** - Estimate entries during peak usage
5. **Use Watch for security-critical** - Real-time invalidation for sensitive resources
6. **Use consistency tokens after writes** - Guarantee visibility of your own writes
