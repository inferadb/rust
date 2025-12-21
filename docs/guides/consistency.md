# Consistency & Real-Time Updates

Ensure read-after-write consistency and receive real-time relationship changes.

## Consistency Tokens

After writing a relationship, you may need to read immediately. Use consistency tokens to guarantee you see your own writes.

### Read-After-Write

```rust
// Write returns a consistency token
let result = vault
    .relationships()
    .write(Relationship::new("doc:1", "viewer", "user:alice"))
    .await?;

let token = result.consistency_token();

// Check immediately sees the write
let allowed = vault
    .check("user:alice", "view", "doc:1")
    .at_least_as_fresh_as(token)
    .await?;

assert!(allowed);  // Guaranteed to reflect the write
```

### How It Works

| Without Token         | With Token              |
| --------------------- | ----------------------- |
| May use cached result | Bypasses cache          |
| Eventual consistency  | Strong consistency      |
| Lower latency         | Slightly higher latency |
| Default behavior      | Opt-in when needed      |

### When to Use

```rust
// Use tokens when writes must be immediately visible
async fn grant_access_and_verify(
    vault: &VaultClient,
    subject: &str,
    resource: &str,
) -> Result<bool, Error> {
    let result = vault
        .relationships()
        .write(Relationship::new(resource, "viewer", subject))
        .await?;

    // This check MUST see the write we just made
    vault
        .check(subject, "view", resource)
        .at_least_as_fresh_as(result.consistency_token())
        .await
}
```

### Propagating Tokens

Pass tokens through your request lifecycle:

```rust
// In a write handler
async fn add_viewer(
    vault: &VaultClient,
    doc_id: &str,
    user_id: &str,
) -> Result<ConsistencyToken, Error> {
    let result = vault
        .relationships()
        .write(Relationship::new(
            format!("document:{}", doc_id),
            "viewer",
            format!("user:{}", user_id),
        ))
        .await?;

    Ok(result.consistency_token().clone())
}

// Return token in response header for client use
// X-Consistency-Token: ct_01JFQG...
```

### Batch Writes

Batch writes return a single token covering all relationships:

```rust
let result = vault
    .relationships()
    .write_batch([
        Relationship::new("doc:1", "viewer", "user:alice"),
        Relationship::new("doc:2", "viewer", "user:alice"),
        Relationship::new("doc:3", "viewer", "user:alice"),
    ])
    .await?;

// Token covers all three writes
let token = result.consistency_token();
```

## Watch for Changes

Subscribe to real-time relationship changes for cache invalidation, audit logging, or reactive updates.

### Basic Watch

```rust
use futures::StreamExt;

let mut stream = vault.watch().run().await?;

while let Some(event) = stream.next().await {
    let event = event?;
    println!("{:?}: {} -[{}]-> {}",
        event.operation,
        event.relationship.subject,
        event.relationship.relation,
        event.relationship.resource,
    );
}
```

### Filtered Watch

Reduce noise by filtering to relevant changes:

```rust
// Watch only document changes
let stream = vault
    .watch()
    .filter(WatchFilter::resource_type("document"))
    .run()
    .await?;

// Watch only viewer relation changes
let stream = vault
    .watch()
    .filter(WatchFilter::relation("viewer"))
    .run()
    .await?;

// Watch only creates (ignore deletes)
let stream = vault
    .watch()
    .filter(WatchFilter::operations([Operation::Create]))
    .run()
    .await?;

// Combine filters (AND logic)
let stream = vault
    .watch()
    .filter(WatchFilter::resource_type("document"))
    .filter(WatchFilter::relation("viewer"))
    .filter(WatchFilter::operations([Operation::Create]))
    .run()
    .await?;
```

### Available Filters

| Filter          | Example             | Description          |
| --------------- | ------------------- | -------------------- |
| `resource_type` | `"document"`        | Resource type prefix |
| `subject_type`  | `"user"`            | Subject type prefix  |
| `resource`      | `"document:readme"` | Specific resource    |
| `subject`       | `"user:alice"`      | Specific subject     |
| `relation`      | `"viewer"`          | Relation name        |
| `operations`    | `[Create, Delete]`  | Operation types      |

### Resumable Streams

Handle disconnections gracefully with automatic reconnection:

```rust
// Auto-reconnect on disconnect
let stream = vault
    .watch()
    .resumable()
    .run()
    .await?;

// Custom reconnection config
let stream = vault
    .watch()
    .resumable()
    .reconnect(ReconnectConfig {
        max_retries: Some(10),
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(30),
        backoff_multiplier: 2.0,
        jitter: 0.1,
    })
    .run()
    .await?;
```

### Crash Recovery with Checkpoints

Save your position for crash recovery:

```rust
let mut stream = vault
    .watch()
    .resumable()
    .run()
    .await?;

while let Some(event) = stream.next().await {
    let event = event?;

    // Process the change
    process_change(&event).await?;

    // Checkpoint to database
    save_checkpoint(event.revision).await?;
}

// On restart, resume from checkpoint
let checkpoint = load_checkpoint().await?;
let stream = vault
    .watch()
    .from_revision(checkpoint)
    .resumable()
    .run()
    .await?;
```

### Watch Event Structure

```rust
pub struct WatchEvent {
    /// Create or Delete
    pub operation: Operation,

    /// The changed relationship
    pub relationship: OwnedRelationship,

    /// Server revision (for resumption)
    pub revision: u64,

    /// When the change occurred
    pub timestamp: DateTime<Utc>,

    /// Who made the change (if audit enabled)
    pub actor: Option<String>,

    /// Original request ID
    pub request_id: Option<String>,
}
```

## Cache Invalidation Patterns

### Watch-Based Invalidation

Invalidate local cache when relationships change:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct AuthzCache {
    cache: moka::future::Cache<(String, String, String), bool>,
    vault: VaultClient,
}

impl AuthzCache {
    async fn start_invalidation(&self) {
        let cache = self.cache.clone();
        let mut stream = self.vault
            .watch()
            .resumable()
            .run()
            .await
            .expect("watch stream");

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                if let Ok(event) = event {
                    // Invalidate affected cache entries
                    // Real implementation would be more sophisticated
                    cache.invalidate_all();
                }
            }
        });
    }
}
```

### TTL + Watch Hybrid

Combine TTL for baseline freshness with watch for immediate invalidation:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .cache(CacheConfig::default()
        .permission_ttl(Duration::from_secs(60))  // Baseline TTL
        .negative_ttl(Duration::from_secs(10)))
    .build()
    .await?;

// Watch invalidates before TTL expires for critical changes
```

## Consistency Trade-offs

| Approach             | Latency | Consistency    | Use Case                        |
| -------------------- | ------- | -------------- | ------------------------------- |
| Default (cached)     | Lowest  | Eventual       | Read-heavy, tolerates staleness |
| Consistency token    | Medium  | Strong         | After writes, critical paths    |
| Watch + invalidation | Low     | Near-real-time | High consistency requirement    |
| No cache             | Highest | Strong         | Security-critical               |

## Best Practices

1. **Use tokens after writes** - Don't assume cache will be invalidated
2. **Keep tokens short-lived** - Don't store tokens long-term, they're request-scoped
3. **Use resumable watch** - Handle disconnects gracefully
4. **Checkpoint revisions** - Enable crash recovery for watch consumers
5. **Filter aggressively** - Reduce watch traffic to what you need
6. **Combine strategies** - Use TTL for baseline, watch for immediacy
