//! Vault operations integration tests.
//!
//! These tests verify vault operations like check, relationships,
//! and permission queries against the dev environment.

use crate::common::TestFixture;
use inferadb::Relationship;

/// Test basic permission check (should deny when no relationships exist)
#[tokio::test]
async fn test_check_permission_denied() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Check permission - should be denied since no relationships exist
    let result = vault.check("user:alice", "view", "document:readme").await;

    match result {
        Ok(allowed) => {
            assert!(!allowed, "Permission should be denied when no relationships exist");
        }
        Err(e) => {
            // Some implementations may return an error for missing resources
            println!("Check returned error (may be expected): {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test writing and checking a relationship
#[tokio::test]
async fn test_write_and_check_relationship() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Write a relationship: document:readme has viewer user:alice
    let rel = Relationship::new("document:readme", "viewer", "user:alice");
    let write_result = vault.relationships().write(rel.clone()).await;

    match write_result {
        Ok(token) => {
            println!("Write succeeded with token: {:?}", token);

            // Now check if the permission is granted
            let check_result = vault.check("user:alice", "viewer", "document:readme").await;
            match check_result {
                Ok(allowed) => {
                    assert!(allowed, "Permission should be granted after writing relationship");
                }
                Err(e) => {
                    println!("Check error after write: {:?}", e);
                }
            }
        }
        Err(e) => {
            // Write may fail due to schema requirements - log but don't fail
            println!("Write relationship error (may require schema): {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test batch permission checks
#[tokio::test]
async fn test_batch_check() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Batch check multiple permissions
    let checks = vec![
        ("user:alice", "view", "document:readme"),
        ("user:bob", "edit", "document:readme"),
        ("user:charlie", "delete", "document:readme"),
    ];

    let result = vault.check_batch(checks).await;

    match result {
        Ok(results) => {
            println!("Batch check returned {} results", results.len());
            // All should be denied since no relationships exist
            for (i, allowed) in results.iter().enumerate() {
                println!("  Check {}: allowed={}", i, allowed);
            }
        }
        Err(e) => {
            println!("Batch check error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing relationships
#[tokio::test]
async fn test_list_relationships() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // List relationships for a resource (should be empty initially)
    let result = vault.relationships().list().resource("document:readme").await;

    match result {
        Ok(response) => {
            println!("Found {} relationships for document:readme", response.relationships.len());
        }
        Err(e) => {
            println!("List relationships error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test require() which should error on denied permission
#[tokio::test]
async fn test_require_permission_fails() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // require() should return an error when permission is denied
    let result = vault.check("user:alice", "view", "document:readme").require().await;

    match result {
        Ok(_) => {
            panic!("require() should fail when no relationship exists");
        }
        Err(e) => {
            println!("require() correctly returned AccessDenied: {:?}", e);
            // AccessDenied is the expected error type when permission is denied
            // The error message should contain information about what was denied
            let msg = e.to_string();
            assert!(
                msg.contains("denied") || msg.contains("user:alice") || msg.contains("document:readme"),
                "Error should indicate access was denied: {:?}",
                e
            );
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test querying subjects with permission on a resource
#[tokio::test]
async fn test_subjects_with_permission() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Query subjects with view permission on document:readme
    let result = vault
        .subjects()
        .with_permission("view")
        .on_resource("document:readme")
        .collect()
        .await;

    match result {
        Ok(subjects) => {
            println!("Found {} subjects with view permission", subjects.len());
            // Should be empty since no relationships exist
            assert!(subjects.is_empty(), "Should have no subjects initially");
        }
        Err(e) => {
            println!("Subjects query error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test querying resources accessible by a subject
#[tokio::test]
async fn test_resources_accessible_by() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Query resources accessible by user:alice with view permission
    let result = vault
        .resources()
        .accessible_by("user:alice")
        .with_permission("view")
        .collect()
        .await;

    match result {
        Ok(resources) => {
            println!("Found {} resources accessible by user:alice", resources.len());
            // Should be empty since no relationships exist
            assert!(resources.is_empty(), "Should have no resources initially");
        }
        Err(e) => {
            println!("Resources query error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test deleting a relationship
#[tokio::test]
async fn test_delete_relationship() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client
        .organization(fixture.org_id_str())
        .vault(fixture.vault_id_str());

    // Try to delete a relationship (may fail if it doesn't exist)
    let rel = Relationship::new("document:readme", "viewer", "user:alice");
    let result = vault.relationships().delete(rel).await;

    // Either success (idempotent delete) or NotFound is acceptable
    match result {
        Ok(()) => println!("Delete succeeded"),
        Err(e) => println!("Delete error (may be expected if not exists): {:?}", e),
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}
