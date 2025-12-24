# Authentication

Configure authentication for your InferaDB SDK client.

## Authentication Methods

| Method                 | Use Case                     | Security Level            |
| ---------------------- | ---------------------------- | ------------------------- |
| **Client Credentials** | Service-to-service (M2M)     | High - Ed25519 signatures |
| **Bearer Token**       | User sessions, OAuth flows   | Medium - token-based      |
| **API Key**            | Simple integrations, testing | Basic                     |

## Client Credentials (Recommended)

For production service-to-service authentication using Ed25519 JWT assertions:

```rust
use inferadb::prelude::*;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "your_client_id".into(),
        private_key: Ed25519PrivateKey::from_pem_file("private-key.pem")?,
        certificate_id: Some("key-2024-01".into()),  // Optional key ID
    })
    .build()
    .await?;
```

### Key Management

```rust
// Load from PEM file
let key = Ed25519PrivateKey::from_pem_file("private-key.pem")?;

// Load from PEM bytes (e.g., from secrets manager)
let pem_bytes = std::env::var("PRIVATE_KEY")?.into_bytes();
let key = Ed25519PrivateKey::from_pem(&pem_bytes)?;

// Load from environment (production pattern)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: std::env::var("INFERADB_CLIENT_ID")?,
        private_key: Ed25519PrivateKey::from_pem(
            &std::env::var("INFERADB_PRIVATE_KEY")?.into_bytes()
        )?,
        certificate_id: std::env::var("INFERADB_KEY_ID").ok(),
    })
    .build()
    .await?;
```

### Key ID (kid) Derivation

The `certificate_id` field sets the JWT `kid` claim:

| Configuration      | Behavior                                                   |
| ------------------ | ---------------------------------------------------------- |
| `Some("key-2024")` | Uses explicit key ID                                       |
| `None`             | Derives from public key: `base64url(sha256(pubkey)[0..8])` |

**Recommendation**: Use explicit key IDs in production for easier rotation tracking.

## Bearer Token

For user sessions or OAuth-issued tokens:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: user_session.access_token.clone(),
    })
    .build()
    .await?;
```

### Token from OAuth Flow

```rust
// After OAuth callback
let tokens = oauth_client.exchange_code(code).await?;

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(BearerCredentialsConfig {
        token: tokens.access_token,
    })
    .build()
    .await?;
```

## API Key

For simple integrations and development:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(Credentials::api_key("key_..."))
    .build()
    .await?;
```

## Token Refresh

The SDK automatically manages token lifecycle:

```rust
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(creds)
    .refresh(RefreshConfig::default()
        .threshold_ratio(0.8)           // Refresh at 80% of lifetime
        .min_remaining(Duration::from_secs(300))  // Or 5 min before expiry
        .grace_period(Duration::from_secs(10)))   // Allow requests during refresh
    .build()
    .await?;
```

### Refresh Configuration

| Setting                 | Default | Description                                       |
| ----------------------- | ------- | ------------------------------------------------- |
| `threshold_ratio`       | 0.8     | Refresh when 80% of token lifetime elapsed        |
| `min_remaining`         | 5 min   | Fallback: refresh when < 5 min remaining          |
| `grace_period`          | 10s     | Allow requests with expiring token during refresh |
| `max_retries`           | 3       | Retry attempts for failed refresh                 |
| `retry_on_auth_failure` | false   | Don't retry 401/403 responses                     |

### Background Refresh

Token refresh happens in a background task:

```text
Token acquired (expires in 1 hour)
    ↓
[... 48 minutes pass ...]
    ↓
Background: Token at 80% lifetime → refresh
    ↓
New token acquired (no request blocked)
```

## Authentication Flow

```text
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│ SDK Client  │         │  Token API  │         │ Engine API  │
└──────┬──────┘         └──────┬──────┘         └──────┬──────┘
       │                       │                       │
       │ 1. Create JWT assertion                       │
       │ (client_id + private_key)                     │
       │──────────────────────►│                       │
       │                       │                       │
       │ 2. Exchange for access token                  │
       │◄──────────────────────│                       │
       │                       │                       │
       │ 3. API call with Bearer token                 │
       │───────────────────────────────────────────────►
       │                       │                       │
       │ [Background: Refresh before expiry]           │
       │──────────────────────►│                       │
```

## Custom Credential Providers

Implement `CredentialsProvider` for custom auth flows:

```rust
use inferadb::{CredentialsProvider, Credentials};

struct VaultCredentials {
    vault_client: vault::Client,
    secret_path: String,
}

#[async_trait]
impl CredentialsProvider for VaultCredentials {
    async fn get_credentials(&self) -> Result<Credentials, Error> {
        let secret = self.vault_client.read(&self.secret_path).await?;
        Ok(Credentials::client_credentials(ClientCredentialsConfig {
            client_id: secret.data["client_id"].clone(),
            private_key: Ed25519PrivateKey::from_pem(
                secret.data["private_key"].as_bytes()
            )?,
            certificate_id: secret.data.get("key_id").cloned(),
        }))
    }

    async fn refresh(&self) -> Result<Credentials, Error> {
        // Re-fetch from Vault (handles rotation)
        self.get_credentials().await
    }
}

let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials_provider(VaultCredentials { /* ... */ })
    .build()
    .await?;
```

## Security Best Practices

1. **Never commit private keys** - Use environment variables or secrets managers
2. **Rotate keys periodically** - Have a documented rotation procedure
3. **Use explicit key IDs** - Easier to track which key is in use
4. **Scope credentials narrowly** - Use least-privilege permissions

```rust
// Production pattern
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: std::env::var("INFERADB_CLIENT_ID")?,
        private_key: Ed25519PrivateKey::from_pem(
            &std::env::var("INFERADB_PRIVATE_KEY")?.into_bytes()
        )?,
        certificate_id: std::env::var("INFERADB_KEY_ID").ok(),
    })
    .build()
    .await?;
```

## Troubleshooting

### ErrorKind::Unauthorized (401)

- Invalid or expired credentials
- Wrong client_id or private key mismatch
- Key not registered with InferaDB

### ErrorKind::Forbidden (403)

- Valid credentials but insufficient permissions
- Client not authorized for the requested organization/vault

### Token Refresh Failures

```rust
// Check token status
match client.auth_status().await {
    AuthStatus::Valid { expires_at } => { /* OK */ }
    AuthStatus::Refreshing => { /* Refresh in progress */ }
    AuthStatus::Failed { error } => {
        tracing::error!("Auth failed: {}", error);
    }
}
```

## Related Guides

- [Production Checklist](production-checklist.md) - Security requirements
- [Error Handling](errors.md) - Auth error types
- [Management API](management-api.md) - Managing API clients and keys
