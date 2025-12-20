# Troubleshooting Guide

This guide covers common issues and their solutions when using the InferaDB Rust SDK.

## Connection Issues

### Cannot Connect to Server

**Symptoms:**
- `ConnectionRefused` error
- Timeout errors during client creation

**Solutions:**

1. Verify the server is running and accessible:

   ```bash
   curl -v https://api.inferadb.com/health
   ```

2. Check network connectivity and firewall rules

3. For local development, ensure you're using the correct URL:

   ```rust
   let client = Client::builder()
       .url("http://localhost:8080")  // Not https for local
       .insecure()                     // Required for non-TLS
       .build()
       .await?;
   ```

### TLS Certificate Errors

**Symptoms:**
- `InvalidCertificate` error
- `CertificateRequired` error

**Solutions:**

1. Ensure you're using the correct TLS feature:

   ```toml
   # For most environments (pure Rust)
   inferadb = { version = "0.1", features = ["rustls"] }

   # For environments requiring system certificates
   inferadb = { version = "0.1", features = ["native-tls"] }
   ```

2. For self-signed certificates in development:

   ```rust
   let client = Client::builder()
       .url("https://dev.internal")
       .add_root_certificate(Certificate::from_pem(include_bytes!("ca.pem"))?)
       .build()
       .await?;
   ```

3. Never use `.insecure()` in production

### Connection Pool Exhaustion

**Symptoms:**
- Requests hang indefinitely
- `PoolTimeout` error

**Solutions:**

1. Increase pool size for high-throughput applications:

   ```rust
   let client = Client::builder()
       .url("https://api.inferadb.com")
       .pool_size(50)  // Default is 20
       .build()
       .await?;
   ```

2. Ensure you're reusing the client (don't create new clients per request)

3. Check for connection leaks - streams must be fully consumed or dropped

## Authentication Issues

### Token Refresh Failures

**Symptoms:**
- `Unauthorized` errors after initial success
- Errors mentioning "token expired"

**Solutions:**

1. Verify your private key hasn't been rotated:

   ```rust
   // Check key fingerprint
   let key = Ed25519PrivateKey::from_pem_file("private_key.pem")?;
   println!("Key ID: {}", key.key_id());
   ```

2. Ensure system time is synchronized (JWT validation is time-sensitive):

   ```bash
   # Check system time drift
   date -u
   ```

3. Check that the certificate is still active in your tenant settings

### Invalid Client Credentials

**Symptoms:**
- `Unauthorized` error on first request
- "Invalid client assertion" message

**Solutions:**

1. Verify client ID matches the registered service:

   ```rust
   let creds = ClientCredentials {
       client_id: "my_service".into(),  // Must match registration
       private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
       certificate_id: None,
   };
   ```

2. Ensure the private key corresponds to a registered public key

3. Check that the key is Ed25519 (not RSA or other formats):

   ```bash
   # Verify key type
   openssl ec -in private_key.pem -text -noout 2>/dev/null || \
   echo "Not an EC key - checking Ed25519..."
   head -1 private_key.pem  # Should show "-----BEGIN PRIVATE KEY-----"
   ```

### Forbidden Errors

**Symptoms:**
- `Forbidden` error (403)
- "Insufficient permissions" message

**Solutions:**

1. Verify the service has access to the vault:

   ```rust
   // List accessible vaults
   let vaults = client.control().list_vaults().await?;
   for vault in vaults {
       println!("{}: {:?}", vault.id, vault.permissions);
   }
   ```

2. Check vault-level permissions in the control plane

3. Ensure you're using the correct vault ID:

   ```rust
   let client = Client::builder()
       .default_vault("correct-vault-id")  // Verify this
       .build()
       .await?;
   ```

## Authorization Check Issues

### Unexpected Denied Results

**Symptoms:**
- `check()` returns `false` when you expect `true`
- Permissions work for some users but not others

**Debugging Steps:**

1. Use `expand()` to see the permission resolution path:

   ```rust
   let expansion = client
       .expand("user:alice", "view", "document:readme")
       .await?;

   println!("Resolution tree:");
   print_tree(&expansion, 0);

   fn print_tree(node: &ExpansionNode, depth: usize) {
       let indent = "  ".repeat(depth);
       println!("{}{:?}: {}", indent, node.operation, node.description);
       for child in &node.children {
           print_tree(child, depth + 1);
       }
   }
   ```

2. Verify relationships exist:

   ```rust
   let relations = client
       .read("document:readme")
       .collect()
       .await?;

   for rel in relations {
       println!("{} -[{}]-> {}", rel.resource, rel.relation, rel.subject);
   }
   ```

3. Check for typos in entity IDs (they're case-sensitive):

   ```rust
   // These are different entities!
   "user:Alice"  // Wrong
   "user:alice"  // Correct (if registered as lowercase)
   ```

### Check Latency Issues

**Symptoms:**
- Authorization checks taking >100ms
- Latency spikes during peak traffic

**Solutions:**

1. Use batch checks for multiple permissions:

   ```rust
   // Slow: Sequential checks
   for (subject, permission, resource) in checks {
       client.check(subject, permission, resource).await?;
   }

   // Fast: Batch check
   let results = client
       .check_batch(checks)
       .collect()
       .await?;
   ```

2. Enable local decision caching:

   ```rust
   let client = Client::builder()
       .cache(CacheConfig::default()
           .max_entries(10_000)
           .ttl(Duration::from_secs(60)))
       .build()
       .await?;
   ```

3. Consider using the gRPC transport for lower latency:

   ```toml
   inferadb = { version = "0.1", features = ["grpc"] }
   ```

### Schema Mismatch Errors

**Symptoms:**
- `SchemaViolation` error
- "Unknown relation" or "Unknown permission" errors

**Solutions:**

1. Verify your schema is deployed:

   ```rust
   let schema = client.control().get_schema(vault_id).await?;
   println!("{}", schema.ipl);
   ```

2. Check that relation names match exactly:

   ```ipl
   // Schema defines "viewer", not "view"
   entity Document {
       relations {
           viewer: User  // Use "viewer" not "view"
       }
   }
   ```

3. Ensure you're checking permissions, not relations:

   ```rust
   // Wrong: "viewer" is a relation, not a permission
   client.check("user:alice", "viewer", "doc:1").await?;

   // Correct: "view" is the permission
   client.check("user:alice", "view", "doc:1").await?;
   ```

## Streaming Issues

### Watch Stream Disconnects

**Symptoms:**
- Watch stream stops receiving events
- `StreamReset` or `ConnectionClosed` errors

**Solutions:**

1. Implement automatic reconnection:

   ```rust
   use futures::StreamExt;

   loop {
       let mut stream = client
           .watch()
           .from_revision(last_revision)
           .run()
           .await?;

       while let Some(result) = stream.next().await {
           match result {
               Ok(change) => {
                   last_revision = change.revision;
                   process_change(change);
               }
               Err(e) if e.is_retriable() => {
                   eprintln!("Stream error, reconnecting: {}", e);
                   break;  // Reconnect
               }
               Err(e) => return Err(e.into()),
           }
       }

       tokio::time::sleep(Duration::from_secs(1)).await;
   }
   ```

2. Use the resumable stream helper:

   ```rust
   let stream = client
       .watch()
       .resumable()  // Automatically handles reconnection
       .run()
       .await?;
   ```

### Backpressure and Slow Consumers

**Symptoms:**
- Memory usage grows unbounded
- `BufferFull` errors

**Solutions:**

1. Process events promptly or use bounded channels:

   ```rust
   let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

   // Producer task
   tokio::spawn(async move {
       let mut stream = client.watch().run().await?;
       while let Some(change) = stream.next().await {
           if tx.send(change?).await.is_err() {
               break;  // Consumer dropped
           }
       }
       Ok::<_, Error>(())
   });

   // Consumer with backpressure
   while let Some(change) = rx.recv().await {
       process_change(change).await;
   }
   ```

2. Apply server-side filtering:

   ```rust
   let stream = client
       .watch()
       .filter(WatchFilter::resource_type("document"))
       .filter(WatchFilter::relation("viewer"))
       .run()
       .await?;
   ```

## Error Recovery Patterns

### Handling Transient Failures

```rust
use inferadb::{Error, ErrorKind};

async fn check_with_retry(
    client: &Client,
    subject: &str,
    permission: &str,
    resource: &str,
) -> Result<bool, Error> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match client.check(subject, permission, resource).await {
            Ok(allowed) => return Ok(allowed),
            Err(e) if e.is_retriable() && attempts < max_attempts => {
                attempts += 1;
                let delay = e.retry_after()
                    .unwrap_or(Duration::from_millis(100 * 2_u64.pow(attempts)));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Circuit Breaker Pattern

```rust
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

struct CircuitBreaker {
    failures: AtomicU32,
    last_failure: AtomicU64,
    threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    fn is_open(&self) -> bool {
        let failures = self.failures.load(Ordering::Relaxed);
        if failures < self.threshold {
            return false;
        }

        let last = self.last_failure.load(Ordering::Relaxed);
        let elapsed = Duration::from_millis(
            Instant::now().elapsed().as_millis() as u64 - last
        );
        elapsed < self.reset_timeout
    }

    fn record_failure(&self) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.last_failure.store(
            Instant::now().elapsed().as_millis() as u64,
            Ordering::Relaxed,
        );
    }

    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
    }
}
```

## Debugging Tips

### Enable Debug Logging

```rust
// With tracing feature
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(tracing_subscriber::EnvFilter::new("inferadb=debug"))
    .init();
```

### Inspect Request IDs

Every error includes a request ID for support:

```rust
match client.check("user:alice", "view", "doc:1").await {
    Err(e) => {
        eprintln!("Error: {}", e);
        if let Some(request_id) = e.request_id() {
            eprintln!("Request ID for support: {}", request_id);
        }
    }
    Ok(allowed) => println!("Allowed: {}", allowed),
}
```

### Common Environment Issues

| Issue | Check | Fix |
|-------|-------|-----|
| Missing env vars | `echo $INFERADB_URL` | Set required environment variables |
| Wrong key format | `file private_key.pem` | Ensure PEM format, Ed25519 algorithm |
| DNS resolution | `nslookup api.inferadb.com` | Check DNS settings |
| Firewall | `nc -zv api.inferadb.com 443` | Open outbound port 443 |

## Getting Help

If you're still stuck:

1. Check the [GitHub Issues](https://github.com/inferadb/rust-sdk/issues) for similar problems
2. Open a new issue with:
   - SDK version (`cargo pkgid inferadb`)
   - Rust version (`rustc --version`)
   - Minimal reproduction code
   - Full error message with request ID
3. For urgent production issues, contact support@inferadb.com
