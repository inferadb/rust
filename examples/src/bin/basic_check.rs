//! Basic authorization check example.
//!
//! This example demonstrates the minimal SDK usage pattern for authorization checks.
//!
//! # Running
//!
//! ```bash
//! # Set environment variables
//! export INFERADB_URL="https://api.inferadb.com"
//! export INFERADB_TOKEN="your-token-here"
//! export INFERADB_ORG_ID="org_..."
//! export INFERADB_VAULT_ID="vlt_..."
//!
//! # Run the example
//! cargo run --example basic_check
//! ```
//!
//! Or with local development:
//!
//! ```bash
//! cargo run --example basic_check --features insecure
//! ```

use std::env;

use inferadb::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Read configuration from environment
    let url = env::var("INFERADB_URL").unwrap_or_else(|_| "https://api.inferadb.com".to_string());
    let token = env::var("INFERADB_TOKEN").expect("INFERADB_TOKEN must be set");
    let org_id = env::var("INFERADB_ORG_ID").expect("INFERADB_ORG_ID must be set");
    let vault_id = env::var("INFERADB_VAULT_ID").expect("INFERADB_VAULT_ID must be set");

    // Create client with bearer token authentication
    let client = Client::builder()
        .url(&url)
        .credentials(BearerCredentialsConfig::new(&token))
        .build()
        .await?;

    // Get vault context (organization-first hierarchy)
    let vault = client.organization(&org_id).vault(&vault_id);

    // Simple authorization check - returns bool
    // "Can user:alice view document:readme?"
    let allowed = vault.check("user:alice", "view", "document:readme").await?;

    println!("user:alice can view document:readme: {allowed}");

    // Check with ABAC context
    // Policies can use context values for attribute-based decisions
    let allowed_with_context = vault
        .check("user:alice", "view", "document:confidential")
        .with_context(
            Context::new()
                .with("ip_address", "10.0.0.50")
                .with("mfa_verified", true),
        )
        .await?;

    println!("user:alice can view document:confidential (with MFA): {allowed_with_context}");

    // The require() pattern - recommended for HTTP handlers
    // Converts denial (false) into Err(AccessDenied), integrates with ?
    match vault
        .check("user:alice", "delete", "document:readme")
        .require()
        .await
    {
        Ok(()) => println!("user:alice can delete document:readme"),
        Err(AccessDenied { .. }) => {
            println!("user:alice cannot delete document:readme (access denied)")
        }
    }

    println!("\nExample complete!");
    Ok(())
}
