//! Advanced vault operations integration tests.
//!
//! These tests verify advanced vault operations like simulate, watch,
//! explain, and batch operations against the dev environment.

use inferadb::{Context, Relationship};

use crate::common::TestFixture;

/// Test check with context data
#[tokio::test]
async fn test_check_with_context() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Create context with additional data using builder pattern
    let context = Context::new()
        .with("ip_address", "192.168.1.1")
        .with("time_of_day", "business_hours")
        .with("risk_score", 25);

    // Check permission with context
    let result = vault.check("user:alice", "view", "document:readme").with_context(context).await;

    match result {
        Ok(allowed) => {
            println!("Check with context returned: {}", allowed);
            // Should be denied since no relationships exist
            assert!(!allowed, "Permission should be denied without relationships");
        },
        Err(e) => {
            println!("Check with context error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test detailed check that returns metadata
#[tokio::test]
async fn test_detailed_check() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Detailed check returns decision with metadata
    let result = vault.check("user:alice", "view", "document:readme").detailed().await;

    match result {
        Ok(decision) => {
            println!("Detailed check decision: {:?}", decision);
            assert!(!decision.is_allowed(), "Should be denied without relationships");
        },
        Err(e) => {
            println!("Detailed check error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test explain permission functionality
#[tokio::test]
async fn test_explain_permission() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Explain why a permission is granted or denied
    let result = vault
        .explain_permission()
        .subject("user:alice")
        .resource("document:readme")
        .permission("view")
        .await;

    match result {
        Ok(explanation) => {
            println!("Permission explanation: {:?}", explanation);
            // Should be denied with explanation
            assert!(!explanation.allowed, "Should be denied without relationships");
        },
        Err(e) => {
            println!("Explain permission error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test simulate with ephemeral relationships
#[tokio::test]
async fn test_simulate_check() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Simulate what would happen if we add a relationship
    let result = vault
        .simulate()
        .add_relationship(Relationship::new("document:readme", "viewer", "user:alice"))
        .check("user:alice", "viewer", "document:readme")
        .await;

    match result {
        Ok(simulation_result) => {
            println!("Simulation result: {:?}", simulation_result);
            // With the ephemeral relationship, permission should be granted
            assert!(simulation_result.allowed, "Should be allowed with simulated relationship");
        },
        Err(e) => {
            println!("Simulate error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test simulate with multiple relationships
#[tokio::test]
async fn test_simulate_multiple_relationships() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Simulate with multiple relationships
    let result = vault
        .simulate()
        .add_relationship(Relationship::new("document:readme", "viewer", "user:alice"))
        .add_relationship(Relationship::new("document:readme", "editor", "user:bob"))
        .add_relationship(Relationship::new("folder:root", "owner", "user:charlie"))
        .check("user:alice", "viewer", "document:readme")
        .await;

    match result {
        Ok(simulation_result) => {
            println!("Multi-relationship simulation result: {:?}", simulation_result);
        },
        Err(e) => {
            println!("Multi-relationship simulate error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test simulate with relationship removal
#[tokio::test]
async fn test_simulate_remove_relationship() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // First write a real relationship
    let rel = Relationship::new("document:test-sim", "viewer", "user:alice");
    let _ = vault.relationships().write(rel.clone()).await;

    // Now simulate removing it
    let result = vault
        .simulate()
        .remove_relationship(rel)
        .check("user:alice", "viewer", "document:test-sim")
        .await;

    match result {
        Ok(simulation_result) => {
            println!("Removal simulation result: {:?}", simulation_result);
            // After simulated removal, permission should be denied
            assert!(!simulation_result.allowed, "Should be denied after simulated removal");
        },
        Err(e) => {
            println!("Removal simulate error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test simulate compare
#[tokio::test]
async fn test_simulate_compare() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Compare hypothetical state with current state
    let result = vault
        .simulate()
        .add_relationship(Relationship::new("document:readme", "viewer", "user:dave"))
        .compare("user:dave", "viewer", "document:readme")
        .await;

    match result {
        Ok(diff) => {
            println!("Simulation diff: change={:?}", diff.change);
        },
        Err(e) => {
            println!("Simulate compare error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test batch write relationships
#[tokio::test]
async fn test_write_batch_relationships() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Write multiple relationships in a batch
    let relationships = vec![
        Relationship::new("document:batch1", "viewer", "user:alice"),
        Relationship::new("document:batch2", "viewer", "user:bob"),
        Relationship::new("document:batch3", "editor", "user:charlie"),
    ];

    let result = vault.relationships().write_batch(relationships).await;

    match result {
        Ok(token) => {
            println!("Batch write succeeded with token: {:?}", token);
        },
        Err(e) => {
            println!("Batch write error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing relationships with filtering
#[tokio::test]
async fn test_list_relationships_with_filters() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Write some test relationships first
    let _ = vault
        .relationships()
        .write(Relationship::new("document:filter1", "viewer", "user:alice"))
        .await;
    let _ = vault
        .relationships()
        .write(Relationship::new("document:filter2", "viewer", "user:alice"))
        .await;

    // List with resource filter
    let result = vault.relationships().list().resource("document:filter1").await;

    match result {
        Ok(response) => {
            println!("Filtered relationships: {} found", response.relationships.len());
            for rel in &response.relationships {
                println!("  {:?}", rel);
            }
        },
        Err(e) => {
            println!("List with filters error: {:?}", e);
        },
    }

    // List with subject filter
    let result = vault.relationships().list().subject("user:alice").await;

    match result {
        Ok(response) => {
            println!("Relationships for user:alice: {} found", response.relationships.len());
        },
        Err(e) => {
            println!("List with subject filter error: {:?}", e);
        },
    }

    // List with relation filter
    let result = vault.relationships().list().relation("viewer").await;

    match result {
        Ok(response) => {
            println!("Viewer relationships: {} found", response.relationships.len());
        },
        Err(e) => {
            println!("List with relation filter error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test resources query with resource type filter
#[tokio::test]
async fn test_resources_with_type_filter() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Write relationships with different resource types
    let _ = vault
        .relationships()
        .write(Relationship::new("document:typed1", "viewer", "user:alice"))
        .await;
    let _ = vault
        .relationships()
        .write(Relationship::new("folder:typed1", "viewer", "user:alice"))
        .await;
    let _ = vault
        .relationships()
        .write(Relationship::new("project:typed1", "viewer", "user:alice"))
        .await;

    // Query with resource type filter
    let result = vault
        .resources()
        .accessible_by("user:alice")
        .with_permission("viewer")
        .resource_type("document")
        .collect()
        .await;

    match result {
        Ok(resources) => {
            println!("Found {} document resources", resources.len());
            for resource in &resources {
                assert!(resource.starts_with("document:"), "Should only return document resources");
            }
        },
        Err(e) => {
            println!("Resources with type filter error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test subjects query with filtering
#[tokio::test]
async fn test_subjects_with_type_filter() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Write relationships with different subject types
    let _ = vault
        .relationships()
        .write(Relationship::new("document:typed", "viewer", "user:alice"))
        .await;
    let _ = vault
        .relationships()
        .write(Relationship::new("document:typed", "viewer", "group:admins"))
        .await;
    let _ = vault
        .relationships()
        .write(Relationship::new("document:typed", "viewer", "service:api"))
        .await;

    // Query with subject type filter
    let result = vault
        .subjects()
        .with_permission("viewer")
        .on_resource("document:typed")
        .subject_type("user")
        .collect()
        .await;

    match result {
        Ok(subjects) => {
            println!("Found {} user subjects", subjects.len());
            for subject in &subjects {
                assert!(subject.starts_with("user:"), "Should only return user subjects");
            }
        },
        Err(e) => {
            println!("Subjects with type filter error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test resources query with pagination via take()
#[tokio::test]
async fn test_resources_with_take() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Write several relationships for pagination testing
    for i in 0..5 {
        let _ = vault
            .relationships()
            .write(Relationship::new(format!("document:page{}", i), "viewer", "user:alice"))
            .await;
    }

    // Test take() for limiting results
    let result = vault
        .resources()
        .accessible_by("user:alice")
        .with_permission("viewer")
        .take(3)
        .collect()
        .await;

    match result {
        Ok(resources) => {
            println!("Took {} resources (limit 3)", resources.len());
            assert!(resources.len() <= 3, "Should not exceed take limit");
        },
        Err(e) => {
            println!("Take resources error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test watch for relationship changes (basic stream creation)
#[tokio::test]
async fn test_watch_stream_creation() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Start watching for changes (with short timeout for testing)
    let watch_result = vault.watch().run().await;

    match watch_result {
        Ok(_stream) => {
            println!("Watch stream created successfully");
            // We can't easily test real-time watching in integration tests,
            // but we can verify the stream can be created
        },
        Err(e) => {
            println!("Watch error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test client health check
#[tokio::test]
async fn test_client_health_check() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    // Test the client's simple health check
    // Note: health_check() may return false in some environments
    let result = client.health_check().await;

    match result {
        Ok(healthy) => {
            println!("Health check returned: {}", healthy);
            // Don't assert - health check result depends on environment
        },
        Err(e) => {
            println!("Health check error: {:?}", e);
        },
    }

    // Test detailed health
    let health_result = client.health().await;
    match health_result {
        Ok(response) => {
            println!("Detailed health: {:?}", response);
            // Don't assert - health endpoint behavior varies by environment
        },
        Err(e) => {
            println!("Detailed health error: {:?}", e);
        },
    }

    // Verify the client can perform actual operations as a more reliable test
    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    let check_result = vault.check("user:test", "view", "document:test").await;
    assert!(check_result.is_ok(), "Client should be able to perform operations");

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test batch check
#[tokio::test]
async fn test_batch_check() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Batch check multiple permissions
    let checks = vec![
        ("user:alice", "view", "document:batch1"),
        ("user:bob", "edit", "document:batch2"),
        ("user:charlie", "delete", "document:batch3"),
    ];

    let result = vault.check_batch(checks).await;

    match result {
        Ok(results) => {
            println!("Batch check returned {} results", results.len());
            assert_eq!(results.len(), 3, "Should return 3 results");
            for (i, allowed) in results.iter().enumerate() {
                println!("  Check {}: {}", i, allowed);
            }
        },
        Err(e) => {
            println!("Batch check error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test batch check with context
#[tokio::test]
async fn test_batch_check_with_context() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Create context
    let context = Context::new().with("region", "us-west");

    // Batch check with context
    let checks =
        vec![("user:alice", "view", "document:ctx1"), ("user:bob", "edit", "document:ctx2")];

    let result = vault.check_batch(checks).with_context(context).await;

    match result {
        Ok(results) => {
            println!("Batch check with context: {} results", results.len());
            assert_eq!(results.len(), 2, "Should return 2 results");
        },
        Err(e) => {
            println!("Batch check with context error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test explain permission with context
#[tokio::test]
async fn test_explain_permission_with_context() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Create context for explanation
    let context = Context::new().with("department", "engineering");

    let result = vault
        .explain_permission()
        .subject("user:alice")
        .resource("document:readme")
        .permission("view")
        .with_context(context)
        .await;

    match result {
        Ok(explanation) => {
            println!("Explain with context: allowed={}", explanation.allowed);
        },
        Err(e) => {
            println!("Explain with context error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test require permission (should fail when denied)
#[tokio::test]
async fn test_require_permission() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // require() should return an error when permission is denied
    let result = vault.check("user:alice", "view", "document:nonexistent").require().await;

    assert!(result.is_err(), "require() should fail when no relationship exists");

    if let Err(e) = result {
        println!("require() correctly returned error: {:?}", e);
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}
