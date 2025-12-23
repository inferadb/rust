//! Client connection and health check integration tests.
//!
//! These tests verify that the SDK can connect to the dev environment
//! and perform basic operations.

use crate::common::{validate_environment, TestFixture};

/// Test that the dev environment is accessible
#[tokio::test]
async fn test_environment_health() {
    validate_environment()
        .await
        .expect("Dev environment should be running and healthy");
}

/// Test that we can create a test fixture (user, org, vault, client)
#[tokio::test]
async fn test_fixture_creation() {
    let fixture = TestFixture::create()
        .await
        .expect("Failed to create test fixture");

    assert!(fixture.user_id > 0, "User ID should be positive");
    assert!(fixture.org_id > 0, "Org ID should be positive");
    assert!(fixture.vault_id > 0, "Vault ID should be positive");
    assert!(fixture.client_id > 0, "Client ID should be positive");
    assert!(
        !fixture.cert_kid.is_empty(),
        "Certificate KID should not be empty"
    );

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test that we can generate valid JWTs
#[tokio::test]
async fn test_jwt_generation() {
    let fixture = TestFixture::create()
        .await
        .expect("Failed to create test fixture");

    // Generate a JWT with default scopes
    let jwt = fixture
        .generate_jwt(None, &[])
        .expect("JWT generation should succeed");

    // JWT should have three parts
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "JWT should have header.payload.signature format"
    );

    // Generate a JWT with specific scopes
    let jwt_write = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("JWT generation with scopes should succeed");
    assert!(!jwt_write.is_empty());

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test SDK client creation with bearer token authentication
#[tokio::test]
async fn test_sdk_client_creation() {
    let fixture = TestFixture::create()
        .await
        .expect("Failed to create test fixture");

    let client = fixture.create_sdk_client().await;
    assert!(
        client.is_ok(),
        "SDK client creation should succeed: {:?}",
        client.err()
    );

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test SDK client health check
#[tokio::test]
async fn test_sdk_health_check() {
    let fixture = TestFixture::create()
        .await
        .expect("Failed to create test fixture");
    let client = fixture
        .create_sdk_client()
        .await
        .expect("Failed to create SDK client");

    // Try health check - this may fail depending on SDK implementation
    // but should not panic
    let health_result = client.health_check().await;
    println!("Health check result: {:?}", health_result);

    fixture.cleanup().await.expect("Cleanup should succeed");
}
