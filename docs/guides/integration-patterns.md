# Integration Patterns

Common patterns for integrating InferaDB authorization into applications.

## Core Pattern

```rust
use inferadb::prelude::*;

// 1. Create client (once at startup)
let client = Client::builder()
    .url("https://api.inferadb.com")
    .credentials(ClientCredentialsConfig {
        client_id: "service-account-id".into(),
        private_key: Ed25519PrivateKey::from_pem_file("private_key.pem")?,
        certificate_id: None,
    })
    .build()
    .await?;

// 2. Get vault context
let vault = client
    .organization("org_8675309...")
    .vault("vlt_01JFQGK...");

// 3. Check permissions
let allowed = vault.check("user:alice", "view", "document:readme").await?;
```

## Web Framework Integration

### Axum

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use inferadb::{Client, VaultClient};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    vault: Arc<VaultClient>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(Credentials::from_env()?)
        .build()
        .await?;

    let vault = client
        .organization(&std::env::var("INFERADB_ORG_ID")?)
        .vault(&std::env::var("INFERADB_VAULT_ID")?);

    let state = AppState {
        vault: Arc::new(vault),
    };

    let app = Router::new()
        .route("/documents/:id", get(get_document))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,  // Your auth extractor
) -> Result<Json<Document>, StatusCode> {
    let resource = format!("document:{}", doc_id);

    // Authorization check
    let allowed = state.vault
        .check(&user.id, "view", &resource)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    let doc = fetch_document(&doc_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(doc))
}
```

### Axum Middleware

```rust
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};

async fn authz_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = extract_user(&request)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let permission = match *request.method() {
        axum::http::Method::GET => "view",
        axum::http::Method::POST => "create",
        axum::http::Method::PUT => "edit",
        axum::http::Method::DELETE => "delete",
        _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
    };

    let resource = extract_resource_from_path(request.uri());

    match state.vault.check(&user, permission, &resource).await {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err(StatusCode::FORBIDDEN),
        Err(e) => {
            tracing::error!(request_id = ?e.request_id(), "Authorization error");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
```

### Actix-web

```rust
use actix_web::{web, App, HttpServer, HttpResponse};
use inferadb::{Client, VaultClient};
use std::sync::Arc;

struct AppState {
    vault: Arc<VaultClient>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client = Client::builder()
        .url("https://api.inferadb.com")
        .credentials(Credentials::from_env().unwrap())
        .build()
        .await
        .expect("Failed to create client");

    let vault = client
        .organization(&std::env::var("INFERADB_ORG_ID").unwrap())
        .vault(&std::env::var("INFERADB_VAULT_ID").unwrap());

    let state = web::Data::new(AppState {
        vault: Arc::new(vault),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/documents/{id}", web::get().to(get_document))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

async fn get_document(
    state: web::Data<AppState>,
    path: web::Path<String>,
    user: AuthenticatedUser,
) -> actix_web::Result<HttpResponse> {
    let doc_id = path.into_inner();
    let resource = format!("document:{}", doc_id);

    let allowed = state.vault
        .check(&user.id, "view", &resource)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    if !allowed {
        return Err(actix_web::error::ErrorForbidden("Access denied"));
    }

    let doc = fetch_document(&doc_id).await?;
    Ok(HttpResponse::Ok().json(doc))
}
```

## Batch Authorization

Filter collections to only accessible items:

```rust
async fn list_documents(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Document>>, StatusCode> {
    let all_docs = fetch_all_documents().await?;

    // Build check tuples
    let checks: Vec<_> = all_docs
        .iter()
        .map(|d| (user.id.as_str(), "view", format!("document:{}", d.id)))
        .collect();

    // Batch check - returns Vec<(Check, bool)>
    let results = state.vault
        .check_batch(&checks)
        .collect()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Filter to accessible - results are (check, allowed) tuples
    let accessible: Vec<_> = all_docs
        .into_iter()
        .zip(results)
        .filter_map(|(doc, (_check, allowed))| allowed.then_some(doc))
        .collect();

    Ok(Json(accessible))
}
```

## Require Pattern

Use `.require()` for early-return on denial:

```rust
use inferadb::AccessDenied;

async fn update_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
    Json(update): Json<DocumentUpdate>,
) -> Result<StatusCode, AccessDenied> {
    let resource = format!("document:{}", doc_id);

    // Returns Err(AccessDenied) if denied
    state.vault
        .check(&user.id, "edit", &resource)
        .require()
        .await?;

    // Authorized - proceed
    apply_update(&doc_id, update).await?;
    Ok(StatusCode::OK)
}
```

## GraphQL (async-graphql)

```rust
use async_graphql::{Context, Guard, Result};
use inferadb::VaultClient;

struct RequirePermission {
    permission: String,
}

#[async_trait::async_trait]
impl Guard for RequirePermission {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let vault = ctx.data::<VaultClient>()?;
        let user = ctx.data::<AuthenticatedUser>()?;
        let resource = ctx.parent_value
            .try_downcast_ref::<Document>()
            .map(|d| format!("document:{}", d.id))
            .ok_or("Missing resource")?;

        let allowed = vault.check(&user.id, &self.permission, &resource).await?;

        if allowed { Ok(()) } else { Err("Access denied".into()) }
    }
}

#[Object]
impl Document {
    #[graphql(guard = "RequirePermission { permission: \"view\".into() }")]
    async fn content(&self) -> &str {
        &self.content
    }
}
```

## gRPC (Tonic)

```rust
use tonic::{Request, Status};

async fn authz_interceptor(
    vault: VaultClient,
    mut req: Request<()>,
) -> Result<Request<()>, Status> {
    let user = extract_user_from_metadata(req.metadata())?;
    let resource = extract_resource(&req)?;

    let allowed = vault
        .check(&user, "access", &resource)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

    if allowed {
        Ok(req)
    } else {
        Err(Status::permission_denied("Access denied"))
    }
}
```

## Background Jobs

```rust
async fn process_job(vault: &VaultClient, job: Job) -> Result<(), Error> {
    // Verify service can act on behalf of user
    let resource = format!("document:{}", job.resource_id);

    let allowed = vault
        .check(&job.user_id, &job.required_permission, &resource)
        .await?;

    if !allowed {
        return Err(Error::Unauthorized);
    }

    // Process job...
    Ok(())
}
```

## ABAC Context

Pass runtime attributes for attribute-based access control:

```rust
use inferadb::Context;

// Check with ABAC context
vault.check("user:alice", "view", "document:confidential")
    .with_context(Context::new()
        .insert("ip_address", "10.0.0.50")
        .insert("mfa_verified", true)
        .insert("department", "engineering"))
    .await?;

// In HTTP handler - pass request context
async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<Document>, StatusCode> {
    let context = Context::new()
        .insert("ip_address", addr.ip().to_string())
        .insert("mfa_verified", user.mfa_verified);

    state.vault
        .check(&user.id, "view", &format!("document:{}", doc_id))
        .with_context(context)
        .require()
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let doc = fetch_document(&doc_id).await?;
    Ok(Json(doc))
}
```

## Convenience Helpers

### Then Pattern

Combine auth check with conditional execution:

```rust
// Execute action only if authorized
let document = vault.check("user:alice", "view", "doc:1")
    .then(|| fetch_document(doc_id))
    .await?;  // Returns Option<Document>

match document {
    Some(doc) => Ok(Json(doc)),
    None => Err(StatusCode::FORBIDDEN),
}
```

### Filter Authorized

Filter a collection to only authorized items:

```rust
let accessible_docs = vault
    .filter_authorized("user:alice", "view", &documents, |doc| format!("document:{}", doc.id))
    .await?;
```

## Best Practices

1. **Create client once** - Share across requests via application state
2. **Store vault reference** - Get `VaultClient` once, reuse throughout request lifecycle
3. **Use batch operations** - `check_batch()` for multiple checks in one round-trip
4. **Handle errors gracefully** - Log `request_id` for debugging, return appropriate status
5. **Use `.require()`** - For guard clauses that should fail fast on denial
6. **Pass ABAC context** - Include runtime attributes for attribute-based policies
