# Integration Patterns

This guide covers common patterns for integrating InferaDB authorization into different application architectures.

## Quick Start Integration

Before diving into specific frameworks, here's the general pattern for integrating InferaDB:

1. **Create a shared client** - Initialize once at startup, share via application state
2. **Extract user identity** - From JWT, session, or authentication middleware
3. **Build resource identifier** - Map request data to authorization resources
4. **Check permission** - Call the SDK before allowing the operation
5. **Handle denial** - Return appropriate HTTP status (403 Forbidden)

```rust
// General pattern (framework-agnostic)
async fn authorize<T>(
    client: &Client,
    user: &str,
    permission: &str,
    resource: &str,
) -> Result<T, AuthError>
where
    T: Default,
{
    match client.check(user, permission, resource).await {
        Ok(true) => Ok(T::default()),  // Proceed with operation
        Ok(false) => Err(AuthError::Forbidden),
        Err(e) => {
            tracing::error!(request_id = ?e.request_id(), "Authorization check failed");
            Err(AuthError::ServiceError)
        }
    }
}
```

## Web Framework Integration

### Axum Middleware

#### Complete Application Setup

```rust
use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use inferadb::Client;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    authz: Arc<Client>,
    // ... other services
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize InferaDB client ONCE at startup
    let authz = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(ClientCredentials::from_env()?)
        .default_vault(&std::env::var("INFERADB_VAULT_ID")?)
        .with_tracing()  // Integrates with tower-http tracing
        .build()
        .await?;

    let state = AppState {
        authz: Arc::new(authz),
    };

    let app = Router::new()
        // Public routes (no authorization)
        .route("/health", get(health_check))

        // Protected routes
        .nest("/api", api_routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/documents", get(list_documents).post(create_document))
        .route(
            "/documents/:id",
            get(get_document)
                .put(update_document)
                .delete(delete_document),
        )
}
```

#### Per-Route Authorization Middleware

```rust
use axum::extract::MatchedPath;

/// Middleware that checks permissions based on request method and path
async fn authz_middleware(
    State(state): State<AppState>,
    matched_path: MatchedPath,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = extract_user_from_request(&request)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Map HTTP method to permission
    let permission = match *request.method() {
        axum::http::Method::GET | axum::http::Method::HEAD => "view",
        axum::http::Method::POST => "create",
        axum::http::Method::PUT | axum::http::Method::PATCH => "edit",
        axum::http::Method::DELETE => "delete",
        _ => return Err(StatusCode::METHOD_NOT_ALLOWED),
    };

    // Extract resource from path params
    let resource = extract_resource_from_path(matched_path.as_str(), request.uri());

    // Perform authorization check
    match state.authz.check(&user, permission, &resource).await {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => {
            tracing::info!(
                user = %user,
                permission = %permission,
                resource = %resource,
                "Access denied"
            );
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            tracing::error!(
                request_id = ?e.request_id(),
                "Authorization service error"
            );
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

fn extract_user_from_request(request: &Request) -> Option<String> {
    // Extract from Authorization header (JWT), session, etc.
    request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|token| decode_jwt_subject(token))
}

fn extract_resource_from_path(pattern: &str, uri: &axum::http::Uri) -> String {
    // Map "/documents/:id" with "/documents/123" to "document:123"
    // Implementation depends on your resource naming convention
    let path = uri.path();
    if pattern.contains(":id") {
        let id = path.rsplit('/').next().unwrap_or("unknown");
        format!("document:{}", id)
    } else {
        "documents".to_string()
    }
}
```

#### Handler-Level Authorization

For fine-grained control, authorize within handlers:

```rust
use axum::extract::Extension;

// Extractor that provides authorization client
struct AuthzClient(Arc<Client>);

impl<S> axum::extract::FromRequestParts<S> for AuthzClient
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Arc<Client>>()
            .cloned()
            .map(AuthzClient)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,  // Your auth extractor
) -> Result<Json<Document>, StatusCode> {
    // Authorization check in handler
    let resource = format!("document:{}", doc_id);

    if !state.authz.check(&user.id, "view", &resource).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Fetch and return document
    let doc = fetch_document(&doc_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(doc))
}

// Batch authorization for listing
async fn list_documents(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Document>>, StatusCode> {
    // Fetch all documents user might have access to
    let all_docs = fetch_all_documents().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Batch check permissions
    let checks: Vec<_> = all_docs
        .iter()
        .map(|d| (user.id.as_str(), "view", format!("document:{}", d.id)))
        .map(|(u, p, r)| (u, p, r.as_str()))
        .collect();

    let permissions = state.authz
        .check_batch(checks)
        .collect()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    // Filter to accessible documents
    let accessible: Vec<_> = all_docs
        .into_iter()
        .zip(permissions)
        .filter_map(|(doc, allowed)| allowed.then_some(doc))
        .collect();

    Ok(Json(accessible))
}
```

### Actix-web Integration

#### Actix Application Setup

```rust
use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse, middleware};
use inferadb::Client;
use std::sync::Arc;

struct AppState {
    authz: Arc<Client>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize InferaDB client ONCE at startup
    let authz = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(ClientCredentials::from_env().unwrap())
        .default_vault(&std::env::var("INFERADB_VAULT_ID").unwrap())
        .build()
        .await
        .expect("Failed to create authorization client");

    let state = web::Data::new(AppState {
        authz: Arc::new(authz),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api")
                    .route("/documents", web::get().to(list_documents))
                    .route("/documents", web::post().to(create_document))
                    .route("/documents/{id}", web::get().to(get_document))
                    .route("/documents/{id}", web::put().to(update_document))
                    .route("/documents/{id}", web::delete().to(delete_document))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
```

#### Async Middleware (Recommended)

```rust
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

pub struct AuthzMiddleware {
    client: Arc<Client>,
}

impl AuthzMiddleware {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthzMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthzMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthzMiddlewareService {
            service: Rc::new(service),
            client: self.client.clone(),
        })
    }
}

pub struct AuthzMiddlewareService<S> {
    service: Rc<S>,
    client: Arc<Client>,
}

impl<S, B> Service<ServiceRequest> for AuthzMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let client = self.client.clone();

        Box::pin(async move {
            // Extract user from request
            let user = match extract_user_from_request(&req) {
                Some(u) => u,
                None => {
                    return Ok(req.into_response(
                        HttpResponse::Unauthorized().finish()
                    ).map_into_right_body());
                }
            };

            // Determine permission and resource
            let permission = match req.method().as_str() {
                "GET" | "HEAD" => "view",
                "POST" => "create",
                "PUT" | "PATCH" => "edit",
                "DELETE" => "delete",
                _ => "view",
            };

            let resource = extract_resource_from_request(&req);

            // Perform authorization check
            match client.check(&user, permission, &resource).await {
                Ok(true) => {
                    // Store user in request extensions for handlers
                    req.extensions_mut().insert(AuthenticatedUser { id: user });
                    service.call(req).await.map(|res| res.map_into_left_body())
                }
                Ok(false) => {
                    Ok(req.into_response(
                        HttpResponse::Forbidden().json(serde_json::json!({
                            "error": "Access denied"
                        }))
                    ).map_into_right_body())
                }
                Err(e) => {
                    tracing::error!(
                        request_id = ?e.request_id(),
                        "Authorization service error"
                    );
                    Ok(req.into_response(
                        HttpResponse::ServiceUnavailable().finish()
                    ).map_into_right_body())
                }
            }
        })
    }
}

fn extract_user_from_request(req: &ServiceRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|token| decode_jwt_subject(token))
}

fn extract_resource_from_request(req: &ServiceRequest) -> String {
    if let Some(id) = req.match_info().get("id") {
        format!("document:{}", id)
    } else {
        "documents".to_string()
    }
}
```

#### Actix Handler-Level Authorization

```rust
use actix_web::{web, HttpRequest, HttpResponse};

async fn get_document(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let doc_id = path.into_inner();
    let user = req.extensions()
        .get::<AuthenticatedUser>()
        .ok_or(actix_web::error::ErrorUnauthorized("Not authenticated"))?
        .id
        .clone();

    let resource = format!("document:{}", doc_id);

    // Authorization check
    let allowed = state.authz
        .check(&user, "view", &resource)
        .await
        .map_err(|e| {
            tracing::error!(request_id = ?e.request_id(), "Auth check failed");
            actix_web::error::ErrorInternalServerError("Authorization service unavailable")
        })?;

    if !allowed {
        return Err(actix_web::error::ErrorForbidden("Access denied"));
    }

    // Fetch and return document
    let doc = fetch_document(&doc_id)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("Document not found"))?;

    Ok(HttpResponse::Ok().json(doc))
}
```

### Tonic gRPC Interceptor

```rust
use inferadb::Client;
use tonic::{Request, Status};

fn authz_interceptor(
    client: Client,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |req: Request<()>| {
        let client = client.clone();
        let user = extract_user_from_metadata(req.metadata())?;
        let resource = extract_resource_from_request(&req)?;

        // For sync interceptor, use blocking client
        let allowed = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                client.check(&user, "access", &resource)
            )
        }).map_err(|e| Status::internal(e.to_string()))?;

        if allowed {
            Ok(req)
        } else {
            Err(Status::permission_denied("Access denied"))
        }
    }
}
```

### Tauri Desktop Application

Tauri applications have unique requirements: the Rust backend handles authorization while the frontend (JavaScript/TypeScript) makes requests through Tauri commands.

#### Tauri Application Setup

```rust
// src-tauri/src/main.rs
use inferadb::Client;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

struct AppState {
    authz: Arc<Client>,
    current_user: RwLock<Option<String>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize InferaDB client at app startup
    let authz = Client::builder()
        .url("https://api.inferadb.com")
        .client_credentials(ClientCredentials::from_env()?)
        .default_vault(&std::env::var("INFERADB_VAULT_ID")?)
        .build()
        .await?;

    let state = AppState {
        authz: Arc::new(authz),
        current_user: RwLock::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            check_permission,
            get_document,
            list_documents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
```

#### Tauri Commands with Authorization

```rust
use tauri::command;

#[command]
async fn login(state: State<'_, AppState>, user_id: String) -> Result<(), String> {
    // In production, validate JWT/OAuth token here
    let mut current_user = state.current_user.write().await;
    *current_user = Some(user_id);
    Ok(())
}

#[command]
async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut current_user = state.current_user.write().await;
    *current_user = None;
    Ok(())
}

/// Generic permission check callable from frontend
#[command]
async fn check_permission(
    state: State<'_, AppState>,
    permission: String,
    resource: String,
) -> Result<bool, String> {
    let current_user = state.current_user.read().await;
    let user = current_user.as_ref().ok_or("Not logged in")?;

    state.authz
        .check(user, &permission, &resource)
        .await
        .map_err(|e| format!("Authorization error: {}", e))
}

/// Protected command - authorization happens in Rust
#[command]
async fn get_document(
    state: State<'_, AppState>,
    doc_id: String,
) -> Result<Document, String> {
    let current_user = state.current_user.read().await;
    let user = current_user.as_ref().ok_or("Not logged in")?;

    let resource = format!("document:{}", doc_id);

    // Check permission before returning data
    let allowed = state.authz
        .check(user, "view", &resource)
        .await
        .map_err(|e| format!("Authorization error: {}", e))?;

    if !allowed {
        return Err("Access denied".to_string());
    }

    // Fetch from local storage or API
    fetch_document(&doc_id)
        .await
        .map_err(|e| format!("Failed to fetch document: {}", e))
}

/// Batch-filtered list with authorization
#[command]
async fn list_documents(
    state: State<'_, AppState>,
) -> Result<Vec<Document>, String> {
    let current_user = state.current_user.read().await;
    let user = current_user.as_ref().ok_or("Not logged in")?;

    // Fetch all documents
    let all_docs = fetch_all_documents()
        .await
        .map_err(|e| format!("Failed to fetch documents: {}", e))?;

    // Batch check permissions
    let checks: Vec<_> = all_docs
        .iter()
        .map(|d| (user.as_str(), "view", format!("document:{}", d.id)))
        .collect();

    let check_refs: Vec<_> = checks
        .iter()
        .map(|(u, p, r)| (*u, *p, r.as_str()))
        .collect();

    let permissions = state.authz
        .check_batch(check_refs)
        .collect()
        .await
        .map_err(|e| format!("Authorization error: {}", e))?;

    // Filter to accessible documents
    let accessible: Vec<_> = all_docs
        .into_iter()
        .zip(permissions)
        .filter_map(|(doc, allowed)| allowed.then_some(doc))
        .collect();

    Ok(accessible)
}
```

#### Frontend Integration (TypeScript)

```typescript
// src/lib/auth.ts
import { invoke } from '@tauri-apps/api/tauri';

export async function login(userId: string): Promise<void> {
  await invoke('login', { userId });
}

export async function logout(): Promise<void> {
  await invoke('logout');
}

export async function checkPermission(
  permission: string,
  resource: string
): Promise<boolean> {
  return invoke('check_permission', { permission, resource });
}

// src/lib/documents.ts
import { invoke } from '@tauri-apps/api/tauri';

export interface Document {
  id: string;
  title: string;
  content: string;
}

export async function getDocument(docId: string): Promise<Document> {
  // Authorization is handled in Rust backend
  return invoke('get_document', { docId });
}

export async function listDocuments(): Promise<Document[]> {
  // Returns only documents user has access to
  return invoke('list_documents');
}

// Usage in components
async function handleDocumentClick(docId: string) {
  try {
    const doc = await getDocument(docId);
    displayDocument(doc);
  } catch (error) {
    if (error === 'Access denied') {
      showAccessDeniedModal();
    } else {
      showErrorToast(error);
    }
  }
}
```

#### Offline-First with Cached Permissions

For desktop apps that need offline support:

```rust
use inferadb::Client;
use std::collections::HashMap;
use tokio::sync::RwLock;

struct OfflineAuthz {
    client: Arc<Client>,
    cache: Arc<RwLock<HashMap<(String, String, String), bool>>>,
}

impl OfflineAuthz {
    /// Check permission with fallback to cache
    pub async fn check(
        &self,
        user: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, String> {
        let cache_key = (user.to_string(), permission.to_string(), resource.to_string());

        // Try online check first
        match self.client.check(user, permission, resource).await {
            Ok(allowed) => {
                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(cache_key, allowed);
                Ok(allowed)
            }
            Err(e) if e.is_network_error() => {
                // Fall back to cache when offline
                let cache = self.cache.read().await;
                cache.get(&cache_key).copied().ok_or_else(|| {
                    "Offline and no cached permission".to_string()
                })
            }
            Err(e) => Err(format!("Authorization error: {}", e)),
        }
    }

    /// Pre-warm cache for known resources
    pub async fn warm_cache(&self, user: &str, resources: &[&str]) -> Result<(), String> {
        let permissions = ["view", "edit", "delete"];

        for resource in resources {
            let checks: Vec<_> = permissions
                .iter()
                .map(|p| (user, *p, *resource))
                .collect();

            let results = self.client
                .check_batch(checks.clone())
                .collect()
                .await
                .map_err(|e| format!("Cache warming failed: {}", e))?;

            let mut cache = self.cache.write().await;
            for ((u, p, r), allowed) in checks.into_iter().zip(results) {
                cache.insert((u.to_string(), p.to_string(), r.to_string()), allowed);
            }
        }

        Ok(())
    }
}
```

## Multi-Tenant SaaS Pattern

### Tenant-Scoped Client

```rust
use inferadb::{Client, Relationship};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct TenantManager {
    base_client: Client,
    tenant_vaults: Arc<RwLock<HashMap<String, String>>>,
}

impl TenantManager {
    /// Get tenant-scoped authorization client
    pub async fn for_tenant(&self, tenant_id: &str) -> Result<TenantClient, Error> {
        let vaults = self.tenant_vaults.read().await;
        let vault_id = vaults
            .get(tenant_id)
            .ok_or(Error::TenantNotFound)?
            .clone();

        Ok(TenantClient {
            client: self.base_client.clone(),
            vault_id,
            tenant_id: tenant_id.to_string(),
        })
    }

    /// Provision new tenant
    pub async fn provision_tenant(&self, tenant_id: &str) -> Result<(), Error> {
        // Create vault via control plane
        let vault = self.base_client
            .control()
            .create_vault(CreateVaultRequest {
                name: format!("tenant-{}", tenant_id),
                schema: include_str!("../schemas/saas.ipl").to_string(),
            })
            .await?;

        // Cache mapping
        let mut vaults = self.tenant_vaults.write().await;
        vaults.insert(tenant_id.to_string(), vault.id);

        Ok(())
    }
}

struct TenantClient {
    client: Client,
    vault_id: String,
    tenant_id: String,
}

impl TenantClient {
    pub async fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error> {
        self.client
            .vault(&self.vault_id)
            .check(subject, permission, resource)
            .await
    }

    pub async fn write(&self, relationship: Relationship) -> Result<(), Error> {
        self.client
            .vault(&self.vault_id)
            .write(relationship)
            .await
    }
}
```

### Request-Scoped Tenant Context

```rust
use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::request::Parts,
};

struct TenantContext {
    tenant_id: String,
    authz: TenantClient,
}

#[async_trait]
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract tenant from header, subdomain, or JWT claim
        let tenant_id = parts
            .headers
            .get("X-Tenant-ID")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_string();

        let tenant_manager = parts
            .extensions
            .get::<TenantManager>()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let authz = tenant_manager
            .for_tenant(&tenant_id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

        Ok(TenantContext { tenant_id, authz })
    }
}

// Usage in handler
async fn get_document(
    tenant: TenantContext,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
) -> Result<Json<Document>, StatusCode> {
    let allowed = tenant.authz
        .check(&user.id, "view", &format!("document:{}", doc_id))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    // Fetch and return document...
}
```

## API Gateway Pattern

### Centralized Authorization

```rust
use inferadb::{Client, Context};

struct ApiGateway {
    authz: Client,
    routes: Vec<RouteConfig>,
}

struct RouteConfig {
    path_pattern: String,
    method: String,
    permission: String,
    resource_template: String,
}

impl ApiGateway {
    pub async fn authorize_request(
        &self,
        method: &str,
        path: &str,
        user: &str,
        context: &RequestContext,
    ) -> Result<bool, Error> {
        // Find matching route
        let route = self.routes
            .iter()
            .find(|r| r.matches(method, path))
            .ok_or(Error::RouteNotFound)?;

        // Build resource identifier
        let resource = route.build_resource(path);

        // Build ABAC context
        let authz_context = Context::new()
            .insert("ip_address", &context.client_ip)
            .insert("user_agent", &context.user_agent)
            .insert("request_time", chrono::Utc::now());

        // Check permission
        self.authz
            .check(user, &route.permission, &resource)
            .with_context(authz_context)
            .await
    }
}

// Route configuration (typically from YAML/JSON)
let gateway = ApiGateway {
    authz: client,
    routes: vec![
        RouteConfig {
            path_pattern: "/api/documents/{id}".into(),
            method: "GET".into(),
            permission: "view".into(),
            resource_template: "document:{id}".into(),
        },
        RouteConfig {
            path_pattern: "/api/documents/{id}".into(),
            method: "PUT".into(),
            permission: "edit".into(),
            resource_template: "document:{id}".into(),
        },
        RouteConfig {
            path_pattern: "/api/documents/{id}".into(),
            method: "DELETE".into(),
            permission: "delete".into(),
            resource_template: "document:{id}".into(),
        },
    ],
};
```

### Pre-Authorization for Bulk Operations

```rust
impl ApiGateway {
    /// Pre-authorize multiple resources before bulk operation
    pub async fn authorize_bulk(
        &self,
        user: &str,
        permission: &str,
        resources: &[String],
    ) -> Result<Vec<(String, bool)>, Error> {
        let checks: Vec<_> = resources
            .iter()
            .map(|r| (user, permission, r.as_str()))
            .collect();

        let results = self.authz
            .check_batch(checks)
            .collect()
            .await?;

        Ok(resources.iter().cloned().zip(results).collect())
    }
}
```

## GraphQL Integration

### Async-graphql Field Guard

```rust
use async_graphql::{Context, Guard, Result};
use inferadb::Client;

struct RequirePermission {
    permission: String,
}

impl RequirePermission {
    fn new(permission: impl Into<String>) -> Self {
        Self { permission: permission.into() }
    }
}

#[async_trait::async_trait]
impl Guard for RequirePermission {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let authz = ctx.data::<Client>()?;
        let user = ctx.data::<AuthenticatedUser>()?;

        // Get resource from parent object or arguments
        let resource = ctx
            .parent_value
            .try_downcast_ref::<Document>()
            .map(|d| format!("document:{}", d.id))
            .ok_or("Missing resource context")?;

        let allowed = authz
            .check(&user.id, &self.permission, &resource)
            .await?;

        if allowed {
            Ok(())
        } else {
            Err("Access denied".into())
        }
    }
}

// Usage
#[Object]
impl Document {
    async fn id(&self) -> &str {
        &self.id
    }

    #[graphql(guard = "RequirePermission::new(\"view\")")]
    async fn content(&self) -> &str {
        &self.content
    }

    #[graphql(guard = "RequirePermission::new(\"view_metadata\")")]
    async fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}
```

### DataLoader for Batch Authorization

```rust
use async_graphql::dataloader::{DataLoader, Loader};
use std::collections::HashMap;

struct PermissionLoader {
    client: Client,
    user_id: String,
    permission: String,
}

#[async_trait::async_trait]
impl Loader<String> for PermissionLoader {
    type Value = bool;
    type Error = Error;

    async fn load(&self, resources: &[String]) -> Result<HashMap<String, bool>, Self::Error> {
        let checks: Vec<_> = resources
            .iter()
            .map(|r| (self.user_id.as_str(), self.permission.as_str(), r.as_str()))
            .collect();

        let results = self.client
            .check_batch(checks)
            .collect()
            .await?;

        Ok(resources.iter().cloned().zip(results).collect())
    }
}

// Usage in resolver
#[Object]
impl Query {
    async fn documents(&self, ctx: &Context<'_>) -> Result<Vec<Document>> {
        let docs = fetch_all_documents().await?;

        // Batch check permissions
        let loader = ctx.data::<DataLoader<PermissionLoader>>()?;
        let resources: Vec<_> = docs.iter().map(|d| format!("document:{}", d.id)).collect();
        let permissions = loader.load_many(resources).await?;

        // Filter to accessible documents
        Ok(docs
            .into_iter()
            .filter(|d| *permissions.get(&format!("document:{}", d.id)).unwrap_or(&false))
            .collect())
    }
}
```

## Background Job Pattern

### Job Authorization Context

```rust
use inferadb::Client;

struct JobContext {
    client: Client,
    service_identity: String,
}

impl JobContext {
    /// Check if the job can access a resource on behalf of a user
    pub async fn can_access_as(
        &self,
        user: &str,
        permission: &str,
        resource: &str,
    ) -> Result<bool, Error> {
        // Verify service can impersonate
        let can_impersonate = self.client
            .check(&self.service_identity, "impersonate", user)
            .await?;

        if !can_impersonate {
            return Ok(false);
        }

        // Check user's permission
        self.client
            .check(user, permission, resource)
            .await
    }
}

// Background job example
async fn process_export_job(ctx: &JobContext, job: ExportJob) -> Result<(), Error> {
    // Verify service can act on behalf of user
    if !ctx.can_access_as(&job.user_id, "export", &job.resource_id).await? {
        return Err(Error::Unauthorized);
    }

    // Process export...
    Ok(())
}
```

### Scheduled Permission Sync

```rust
use inferadb::Client;
use tokio::time::{interval, Duration};

async fn sync_permissions_from_external(
    client: &Client,
    external_system: &ExternalPermissionSystem,
) {
    let mut interval = interval(Duration::from_secs(300));  // Every 5 minutes

    loop {
        interval.tick().await;

        match external_system.get_changes_since_last_sync().await {
            Ok(changes) => {
                for change in changes {
                    let result = match change {
                        PermissionChange::Grant { user, role, resource } => {
                            client.write(Relationship::new(&resource, &role, &user)).await
                        }
                        PermissionChange::Revoke { user, role, resource } => {
                            client.delete(Relationship::new(&resource, &role, &user)).await
                        }
                    };

                    if let Err(e) = result {
                        tracing::error!("Failed to sync permission: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch changes: {}", e);
            }
        }
    }
}
```

## Event-Driven Pattern

### Permission Change Events

```rust
use futures::StreamExt;
use inferadb::{Client, WatchFilter};

async fn publish_permission_changes(
    client: &Client,
    event_bus: &EventBus,
) -> Result<(), Error> {
    let mut stream = client
        .watch()
        .filter(WatchFilter::operations([Operation::Create, Operation::Delete]))
        .resumable()
        .run()
        .await?;

    while let Some(change) = stream.next().await {
        let change = change?;

        let event = PermissionChangedEvent {
            operation: change.operation,
            subject: change.relationship.subject,
            relation: change.relationship.relation,
            resource: change.relationship.resource,
            timestamp: change.timestamp,
        };

        event_bus.publish("permissions.changed", event).await?;
    }

    Ok(())
}
```

### Cache Invalidation via Events

```rust
async fn invalidate_cache_on_changes(
    client: &Client,
    cache: Arc<RwLock<PermissionCache>>,
) -> Result<(), Error> {
    let mut stream = client.watch().run().await?;

    while let Some(change) = stream.next().await {
        let change = change?;

        let mut cache = cache.write().await;

        // Invalidate affected cache entries
        cache.invalidate_subject(&change.relationship.subject);
        cache.invalidate_resource(&change.relationship.resource);

        // For hierarchical permissions, invalidate parents too
        if let Some(parent) = get_parent_resource(&change.relationship.resource) {
            cache.invalidate_resource(&parent);
        }
    }

    Ok(())
}
```

## Testing Patterns

### Integration Test Setup

```rust
use inferadb::testing::{TestVault, TestClient};

struct TestFixture {
    client: TestClient,
    vault: TestVault,
}

impl TestFixture {
    async fn new() -> Self {
        let client = TestClient::from_env().await.unwrap();
        let vault = TestVault::create(&client).await.unwrap();

        // Set up common test relationships
        vault.write_batch([
            Relationship::new("org:acme", "member", "user:alice"),
            Relationship::new("org:acme", "admin", "user:bob"),
            Relationship::new("folder:root", "owner", "org:acme#admin"),
            Relationship::new("doc:readme", "parent", "folder:root"),
        ]).await.unwrap();

        Self { client, vault }
    }
}

#[tokio::test]
async fn test_inherited_permissions() {
    let fixture = TestFixture::new().await;

    // Bob is admin -> can access via folder ownership
    assert!(fixture.vault.check("user:bob", "view", "doc:readme").await.unwrap());

    // Alice is member -> no access
    assert!(!fixture.vault.check("user:alice", "view", "doc:readme").await.unwrap());
}
```

### Mock Client for Unit Tests

```rust
use inferadb::testing::MockClient;

#[tokio::test]
async fn test_authorization_middleware() {
    let mock = MockClient::builder()
        .check("user:alice", "view", "doc:1", true)
        .check("user:alice", "edit", "doc:1", false)
        .check("user:bob", "view", "doc:1", false)
        .build();

    let app = create_app(mock);

    // Alice can view
    let resp = app.get("/doc/1").header("X-User", "alice").send().await;
    assert_eq!(resp.status(), 200);

    // Alice cannot edit
    let resp = app.put("/doc/1").header("X-User", "alice").send().await;
    assert_eq!(resp.status(), 403);

    // Bob cannot view
    let resp = app.get("/doc/1").header("X-User", "bob").send().await;
    assert_eq!(resp.status(), 403);
}
```

## Framework Extractors

The SDK provides first-class authorization extractors for popular web frameworks.

### Axum Permission Extractors

```rust
use inferadb::axum::{Authorized, RequirePermission};

// Extractor that checks permission based on path params
async fn get_document(
    Authorized(user): Authorized,  // Extracts and validates user
    RequirePermission { .. }: RequirePermission<"view", DocumentId>,
    Path(doc_id): Path<String>,
) -> Json<Document> {
    // If we reach here, user is authorized
    let doc = fetch_document(&doc_id).await.unwrap();
    Json(doc)
}

// Define how to extract resource ID from request
#[derive(FromRequest)]
struct DocumentId(String);

impl ResourceId for DocumentId {
    fn resource_type() -> &'static str { "document" }
    fn resource_id(&self) -> &str { &self.0 }
}

impl<S> FromRequestParts<S> for DocumentId
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(id): Path<String> = Path::from_request_parts(parts, _state)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        Ok(DocumentId(id))
    }
}
```

### Permission Attribute Macro

```rust
use inferadb::axum::require_permission;

// Declarative permission checking
#[require_permission("view", resource = "document:{doc_id}")]
async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
) -> Json<Document> {
    // Authorization already checked by macro
    Json(fetch_document(&doc_id).await.unwrap())
}

#[require_permission("edit", resource = "document:{doc_id}")]
async fn update_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    user: AuthenticatedUser,
    Json(update): Json<DocumentUpdate>,
) -> StatusCode {
    // ...
    StatusCode::OK
}

// Multiple permissions (all must pass)
#[require_permission("view", "export", resource = "document:{doc_id}")]
async fn export_document(/* ... */) -> impl IntoResponse {
    // ...
}

// Any permission (at least one must pass)
#[require_any_permission("view", "admin", resource = "document:{doc_id}")]
async fn view_or_admin(/* ... */) -> impl IntoResponse {
    // ...
}
```

### Actix Permission Guard

```rust
use inferadb::actix::{Authorized, Permission, PermissionGuard};

async fn get_document(
    auth: Authorized,
    perm: Permission<"view", "document">,  // Checks view permission
    path: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let doc_id = path.into_inner();
    let doc = fetch_document(&doc_id).await?;
    Ok(HttpResponse::Ok().json(doc))
}

// Guard-based authorization
fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/documents/{id}")
            .guard(PermissionGuard::new("view", |req| {
                let id = req.match_info().get("id").unwrap();
                format!("document:{}", id)
            }))
            .route(web::get().to(get_document))
    );
}
```

### Extractor Configuration

```rust
/// Configuration for authorization extractors
#[derive(Clone)]
pub struct AuthzExtractorConfig {
    /// How to extract user ID from request
    pub user_extractor: UserExtractor,

    /// Client for authorization checks
    pub client: Arc<Client>,

    /// Default behavior on authz service errors
    pub on_error: OnError,
}

pub enum UserExtractor {
    /// Extract from Authorization header (Bearer token)
    BearerToken,

    /// Extract from specific header
    Header(&'static str),

    /// Extract from cookie
    Cookie(&'static str),

    /// Custom extraction function
    Custom(Arc<dyn Fn(&Request) -> Option<String> + Send + Sync>),
}
```

## Result Ergonomics

The SDK provides three core patterns for handling authorization results. Choose the simplest pattern that fits your use case.

### Quick Reference

| Pattern | Use When | Returns |
|---------|----------|---------|
| `require()` | Early-return on denial (most common) | `Result<(), Forbidden>` |
| `then(closure)` | Conditionally execute on success | `Result<Option<T>, Error>` |
| `filter_authorized()` | Filter collections by permission | `Result<Vec<T>, Error>` |

### Require Pattern (Early Return on Denial)

The most common pattern - fail fast when access is denied:

```rust
// Simple require - returns Forbidden error on denial
client.check("user:alice", "view", "doc:1")
    .require()  // Converts bool to Result<(), Forbidden>
    .await?;    // Early returns on denial

// Continue with authorized operation...
let doc = fetch_document("doc:1").await?;
```

For complex error handling, use standard Rust composition:

```rust
// Custom error mapping with standard Rust
let allowed = client.check("user:alice", "view", "doc:1").await?;
if !allowed {
    return Err(AppError::AccessDenied {
        user: "alice".into(),
        resource: "doc:1".into(),
    });
}
```

### Then Pattern (Conditional Execution)

Execute a closure only when authorized:

```rust
// Execute closure only if authorized
let document = client.check("user:alice", "view", "doc:1")
    .then(|| fetch_document(doc_id))
    .await?;  // Returns Option<Document>

match document {
    Some(doc) => Ok(Json(doc)),
    None => Err(StatusCode::FORBIDDEN),
}
```

### Filter Pattern (Batch Authorization)

Filter collections to only authorized items:

```rust
// Filter collection to only authorized items
let accessible_docs = client
    .filter_authorized("user:alice", "view", &documents, |doc| format!("document:{}", doc.id))
    .await?;
```

### Forbidden Error Type

```rust
/// Error returned when authorization is denied
pub struct Forbidden {
    pub subject: String,
    pub permission: String,
    pub resource: String,
    pub request_id: Option<String>,
}

impl Forbidden {
    /// Convert to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }
}

// Integrates with web frameworks
impl IntoResponse for Forbidden {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, Json(json!({
            "error": "forbidden",
            "message": format!("Access denied to {} on {}", self.permission, self.resource)
        }))).into_response()
    }
}
```

## Permission Aggregation

Utilities for common permission checking patterns.

### All/Any Permission Checks

```rust
use inferadb::{Client, PermissionSet};

// Check if user has ALL permissions (fails fast)
let can_manage = client
    .check_all("user:alice", ["view", "edit", "delete"], "doc:1")
    .await?;

// Check if user has ANY permission (succeeds fast)
let has_access = client
    .check_any("user:alice", ["view", "edit", "admin"], "doc:1")
    .await?;

// Get detailed results for permission set
let permissions: PermissionSet = client
    .check_permissions("user:alice", ["view", "edit", "delete", "share"], "doc:1")
    .await?;

assert!(permissions.has("view"));
assert!(!permissions.has("delete"));
println!("Allowed: {:?}", permissions.allowed());  // ["view", "edit"]
println!("Denied: {:?}", permissions.denied());    // ["delete", "share"]
```

### Cross-Resource Aggregation

```rust
// Check same permission across multiple resources
let access_map: HashMap<String, bool> = client
    .check_resources("user:alice", "view", ["doc:1", "doc:2", "doc:3", "folder:shared"])
    .await?;

// Filter to only accessible resources
let accessible: Vec<&str> = access_map
    .iter()
    .filter(|(_, allowed)| **allowed)
    .map(|(resource, _)| resource.as_str())
    .collect();
```

### Permission Matrix for UIs

```rust
use inferadb::aggregation::PermissionMatrix;

// Generate full permission matrix for UI display
let matrix: PermissionMatrix = client
    .permission_matrix()
    .subjects(["user:alice", "user:bob", "group:admins"])
    .permissions(["view", "edit", "delete"])
    .resources(["doc:1", "doc:2", "folder:root"])
    .await?;

// Access results
for subject in matrix.subjects() {
    for resource in matrix.resources() {
        let perms = matrix.get(subject, resource);
        println!("{} on {}: {:?}", subject, resource, perms.allowed());
    }
}

// Export for UI
let json = matrix.to_json();
```

## Structured Audit Context

Attach structured context to authorization checks for comprehensive audit logging.

### Basic Audit Context

```rust
use inferadb::{AuditContext, Client};

// Attach audit context to authorization checks
let allowed = client
    .check("user:alice", "view", "doc:sensitive")
    .with_audit(AuditContext::new()
        .request_id("req-abc-123")
        .correlation_id("order-flow-456")
        .ip_address("192.168.1.100")
        .user_agent("MyApp/1.0")
        .session_id("sess-xyz")
        .action_reason("User clicked download button")
        .custom("feature_flag", "new_permissions_v2")
        .custom("ab_test_group", "control"))
    .await?;

// Context flows to server for audit logging
// Server logs include all context fields for compliance
```

### Request-Scoped Context

```rust
use inferadb::{AuditContext, RequestScope};

// Create request-scoped context that applies to all operations
let scope = RequestScope::new()
    .audit_context(AuditContext::new()
        .request_id(&request_id)
        .ip_address(&client_ip)
        .user_agent(&user_agent));

// All operations within scope include audit context
scope.run(|| async {
    client.check("user:alice", "view", "doc:1").await?;
    client.check("user:alice", "edit", "doc:1").await?;
    client.write(Relationship::new("doc:1", "viewer", "user:bob")).await?;
    Ok(())
}).await?;
```

### Audit Context Middleware

```rust
// Axum middleware for automatic audit context
use inferadb::axum::AuditContextLayer;

let app = Router::new()
    .route("/documents/:id", get(get_document))
    .layer(AuditContextLayer::new()
        .request_id_header("X-Request-ID")
        .correlation_id_header("X-Correlation-ID")
        .extract_ip(true)
        .extract_user_agent(true));
```
