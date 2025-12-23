//! Axum web framework integration example.
//!
//! This example demonstrates how to integrate InferaDB authorization
//! into an Axum web application using middleware and extractors.
//!
//! # Running
//!
//! ```bash
//! export INFERADB_URL="https://api.inferadb.com"
//! export INFERADB_TOKEN="your-token-here"
//! export INFERADB_ORG_ID="org_..."
//! export INFERADB_VAULT_ID="vlt_..."
//!
//! cargo run --example axum_middleware --features rest
//! ```
//!
//! Then test with:
//! ```bash
//! curl http://localhost:3000/documents/readme
//! curl -X DELETE http://localhost:3000/documents/readme
//! ```

use std::env;
use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Router,
};
use inferadb::prelude::*;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    vault: VaultClient,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Read configuration from environment
    let url = env::var("INFERADB_URL").unwrap_or_else(|_| "https://api.inferadb.com".to_string());
    let token = env::var("INFERADB_TOKEN").expect("INFERADB_TOKEN must be set");
    let org_id = env::var("INFERADB_ORG_ID").expect("INFERADB_ORG_ID must be set");
    let vault_id = env::var("INFERADB_VAULT_ID").expect("INFERADB_VAULT_ID must be set");

    // Create InferaDB client
    let client = Client::builder()
        .url(&url)
        .credentials(BearerCredentialsConfig::new(&token))
        .build()
        .await?;

    // Get vault context - VaultClient is Clone and cheap to share
    let vault = client.organization(&org_id).vault(&vault_id);

    let state = AppState { vault };

    // Build router with authorization-protected routes
    let app = Router::new()
        .route("/documents/:id", get(view_document))
        .route("/documents/:id", delete(delete_document))
        .route("/health", get(health_check))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// View a document - requires "view" permission
async fn view_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> std::result::Result<impl IntoResponse, AppError> {
    // In a real app, extract user ID from JWT/session
    let user_id = "user:alice";
    let resource = format!("document:{doc_id}");

    // Use require() pattern - converts denial to error
    state
        .vault
        .check(user_id, "view", &resource)
        .require()
        .await?;

    // User is authorized - return document content
    Ok(format!("Document content for: {doc_id}"))
}

/// Delete a document - requires "delete" permission
async fn delete_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> std::result::Result<impl IntoResponse, AppError> {
    let user_id = "user:alice";
    let resource = format!("document:{doc_id}");

    // Check authorization first
    state
        .vault
        .check(user_id, "delete", &resource)
        .require()
        .await?;

    // User is authorized - delete the document
    Ok(format!("Deleted document: {doc_id}"))
}

/// Health check endpoint (no authorization required)
async fn health_check() -> impl IntoResponse {
    "OK"
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Handling
// ─────────────────────────────────────────────────────────────────────────────

/// Application error type that converts InferaDB errors to HTTP responses
enum AppError {
    /// Authorization denied
    AccessDenied(AccessDenied),
    /// Other SDK errors
    InferaDb(Error),
}

impl From<AccessDenied> for AppError {
    fn from(err: AccessDenied) -> Self {
        AppError::AccessDenied(err)
    }
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        AppError::InferaDb(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::AccessDenied(denied) => {
                // 403 Forbidden for authorization denials
                (
                    StatusCode::FORBIDDEN,
                    format!(
                        "Access denied: {} cannot {} on {}",
                        denied.subject(),
                        denied.permission(),
                        denied.resource()
                    ),
                )
                    .into_response()
            }
            AppError::InferaDb(err) => {
                // Map SDK errors to HTTP status codes
                let status = match err.kind() {
                    ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
                    ErrorKind::RateLimited => StatusCode::TOO_MANY_REQUESTS,
                    ErrorKind::NotFound => StatusCode::NOT_FOUND,
                    ErrorKind::InvalidArgument => StatusCode::BAD_REQUEST,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };

                // Include request ID for debugging if available
                let message = if let Some(request_id) = err.request_id() {
                    format!("{} (request_id: {})", err, request_id)
                } else {
                    err.to_string()
                };

                (status, message).into_response()
            }
        }
    }
}
