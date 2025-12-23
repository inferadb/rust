//! Batch operations example.
//!
//! This example demonstrates batch authorization checks and relationship management.
//! Batch operations reduce network round-trips for better performance.
//!
//! # Running
//!
//! ```bash
//! export INFERADB_URL="https://api.inferadb.com"
//! export INFERADB_TOKEN="your-token-here"
//! export INFERADB_ORG_ID="org_..."
//! export INFERADB_VAULT_ID="vlt_..."
//!
//! cargo run --example batch_operations
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

    let client = Client::builder()
        .url(&url)
        .credentials(BearerCredentialsConfig::new(&token))
        .build()
        .await?;

    let vault = client.organization(&org_id).vault(&vault_id);

    // ─────────────────────────────────────────────────────────────────────────
    // Batch Relationship Writes
    // ─────────────────────────────────────────────────────────────────────────

    println!("Writing relationships...");

    // Write multiple relationships in a single request
    let relationships = [
        // Folder hierarchy
        Relationship::new("folder:engineering", "viewer", "team:engineering#member"),
        Relationship::new("folder:engineering", "editor", "team:engineering#lead"),
        // Document in folder
        Relationship::new("document:design-doc", "parent", "folder:engineering"),
        Relationship::new("document:design-doc", "owner", "user:alice"),
        // Direct user access
        Relationship::new("document:readme", "viewer", "user:bob"),
    ];

    vault.relationships().write_batch(relationships).await?;

    println!("  Wrote {} relationships", 5);

    // ─────────────────────────────────────────────────────────────────────────
    // Batch Authorization Checks
    // ─────────────────────────────────────────────────────────────────────────

    println!("\nRunning batch authorization checks...");

    // Check multiple permissions in a single round-trip
    let checks = [
        ("user:alice", "view", "document:design-doc"),
        ("user:alice", "edit", "document:design-doc"),
        ("user:alice", "delete", "document:design-doc"),
        ("user:bob", "view", "document:readme"),
        ("user:bob", "edit", "document:readme"),
    ];

    // BatchCheckRequest returns Vec<bool> in the same order as inputs
    let results: Vec<bool> = vault.check_batch(checks).await?;

    // Results preserve input order
    for (i, allowed) in results.iter().enumerate() {
        let (subject, permission, resource) = checks[i];
        let status = if *allowed { "✓" } else { "✗" };
        println!("  {status} {subject} {permission} {resource}");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Consistency Tokens
    // ─────────────────────────────────────────────────────────────────────────

    println!("\nDemonstrating consistency tokens...");

    // Write returns a consistency token for read-after-write consistency
    let token = vault
        .relationships()
        .write(Relationship::new(
            "document:new-doc",
            "viewer",
            "user:charlie",
        ))
        .await?;

    println!("  Write returned consistency token: {}", token.value());

    // After writing, the authorization check will reflect the new relationship
    let allowed = vault
        .check("user:charlie", "view", "document:new-doc")
        .await?;

    println!("  user:charlie can view document:new-doc: {allowed}");

    // ─────────────────────────────────────────────────────────────────────────
    // Cleanup
    // ─────────────────────────────────────────────────────────────────────────

    println!("\nCleaning up...");

    // Delete the test relationships
    vault
        .relationships()
        .delete(Relationship::new(
            "document:new-doc",
            "viewer",
            "user:charlie",
        ))
        .await?;

    println!("  Deleted test relationship");

    println!("\nExample complete!");
    Ok(())
}
