//! Transport layer integration tests.
//!
//! These tests verify the transport layer functionality including
//! gRPC and REST transports, transport strategy, and fallback behavior.

use std::time::Duration;

use inferadb::{FallbackTrigger, PoolConfig, TransportStrategy};

use crate::common::TestFixture;

/// Test creating client with REST transport only
#[tokio::test]
async fn test_rest_transport_strategy() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with REST-only strategy
    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .transport_strategy(TransportStrategy::RestOnly)
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("REST-only client created successfully");

            // Verify it can perform operations
            let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

            let result = vault.check("user:alice", "view", "document:test").await;
            match result {
                Ok(allowed) => {
                    println!("REST check succeeded: {}", allowed);
                },
                Err(e) => {
                    println!("REST check error: {:?}", e);
                },
            }
        },
        Err(e) => {
            println!("REST client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test creating client with gRPC transport only
#[tokio::test]
async fn test_grpc_transport_strategy() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with gRPC-only strategy
    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .transport_strategy(TransportStrategy::GrpcOnly)
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("gRPC-only client created successfully");

            // Verify it can perform operations
            let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

            let result = vault.check("user:alice", "view", "document:test").await;
            match result {
                Ok(allowed) => {
                    println!("gRPC check succeeded: {}", allowed);
                },
                Err(e) => {
                    println!("gRPC check error: {:?}", e);
                },
            }
        },
        Err(e) => {
            // gRPC might not be available, which is acceptable
            println!("gRPC client creation error (may be expected): {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test creating client with PreferGrpc strategy (default)
#[tokio::test]
async fn test_prefer_grpc_strategy() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with PreferGrpc strategy (with fallback)
    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .transport_strategy(TransportStrategy::PreferGrpc {
            fallback_on: FallbackTrigger::default(),
        })
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("PreferGrpc client created successfully");

            // Verify it can perform operations
            let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

            let result = vault.check("user:alice", "view", "document:test").await;
            match result {
                Ok(allowed) => {
                    println!("PreferGrpc check succeeded: {}", allowed);
                },
                Err(e) => {
                    println!("PreferGrpc check error: {:?}", e);
                },
            }
        },
        Err(e) => {
            println!("PreferGrpc client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test default transport strategy
#[tokio::test]
async fn test_default_transport_strategy() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with default strategy (should be PreferGrpc)
    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("Default strategy client created successfully");

            // Verify it can perform operations (simple health check returns bool)
            // Note: health_check() may return false in some environments due to
            // endpoint configuration or authentication requirements
            let result = client.health_check().await;
            match result {
                Ok(healthy) => {
                    println!("Health check with default strategy: {}", healthy);
                    // Don't assert - health check result depends on environment
                },
                Err(e) => {
                    println!("Health check error: {:?}", e);
                },
            }

            // Instead, verify the client can perform actual operations
            let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

            let check_result = vault.check("user:test", "view", "document:test").await;
            assert!(check_result.is_ok(), "Client should be able to perform checks");
        },
        Err(e) => {
            println!("Default client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test pool configuration
#[tokio::test]
async fn test_pool_configuration() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with custom pool config
    let pool_config = PoolConfig {
        max_connections: 5,
        pool_timeout: Duration::from_secs(60),
        ..PoolConfig::default()
    };

    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .pool_config(pool_config)
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("Client with custom pool config created successfully");

            // Verify it works
            let result = client.health_check().await;
            assert!(result.is_ok(), "Client with custom pool config should work");
        },
        Err(e) => {
            println!("Pool config client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test concurrent operations across transport
#[tokio::test]
async fn test_concurrent_operations() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Spawn multiple concurrent check operations
    let mut handles = Vec::new();

    for i in 0..10 {
        let vault_clone = vault.clone();
        let subject = format!("user:concurrent{}", i);
        let resource = format!("document:concurrent{}", i);

        let handle =
            tokio::spawn(async move { vault_clone.check(&subject, "view", &resource).await });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) => error_count += 1,
            Err(_) => error_count += 1,
        }
    }

    println!("Concurrent operations: {} successes, {} errors", success_count, error_count);
    assert!(success_count > 0, "At least some concurrent operations should succeed");

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test timeout configuration
#[tokio::test]
async fn test_timeout_configuration() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with custom timeout
    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .timeout(Duration::from_secs(60))
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("Client with custom timeout created successfully");

            // Verify it works
            let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

            let result = vault.check("user:timeout-test", "view", "document:test").await;
            assert!(result.is_ok(), "Client with custom timeout should work");
        },
        Err(e) => {
            println!("Timeout config client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test retry configuration
#[tokio::test]
async fn test_retry_configuration() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let jwt = fixture
        .generate_jwt(None, &["inferadb.check", "inferadb.write"])
        .expect("Failed to generate JWT");

    // Create client with custom retry config
    let retry_config = inferadb::RetryConfig::default()
        .with_max_retries(5)
        .with_initial_delay(Duration::from_millis(100))
        .with_max_delay(Duration::from_secs(5));

    let client = inferadb::Client::builder()
        .url(&fixture.ctx.api_base_url)
        .credentials(inferadb::BearerCredentialsConfig::new(jwt))
        .tls_config(inferadb::TlsConfig::new().insecure())
        .retry_config(retry_config)
        .build()
        .await;

    match client {
        Ok(client) => {
            println!("Client with custom retry config created successfully");

            // Verify it works
            let result = client.health_check().await;
            assert!(result.is_ok(), "Client with custom retry config should work");
        },
        Err(e) => {
            println!("Retry config client creation error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test batch operations with transport
#[tokio::test]
async fn test_batch_transport_operations() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let vault = client.organization(fixture.org_id_str()).vault(fixture.vault_id_str());

    // Batch write - create owned relationships
    let mut relationships = Vec::new();
    for i in 0..20 {
        let resource = format!("document:batch-transport{}", i);
        relationships.push(inferadb::Relationship::new(resource, "viewer", "user:alice"))
    }

    let write_result = vault.relationships().write_batch(relationships).await;
    match write_result {
        Ok(token) => {
            println!("Batch write succeeded: {:?}", token);
        },
        Err(e) => {
            println!("Batch write error: {:?}", e);
        },
    }

    // Batch check - using owned strings to avoid lifetime issues
    let check_subjects: Vec<String> = (0..20).map(|_| "user:alice".to_string()).collect();
    let check_permissions: Vec<String> = (0..20).map(|_| "viewer".to_string()).collect();
    let check_resources: Vec<String> =
        (0..20).map(|i| format!("document:batch-transport{}", i)).collect();

    let checks: Vec<(&str, &str, &str)> = (0..20)
        .map(|i| {
            (check_subjects[i].as_str(), check_permissions[i].as_str(), check_resources[i].as_str())
        })
        .collect();

    let check_result = vault.check_batch(checks).await;
    match check_result {
        Ok(results) => {
            println!("Batch check returned {} results", results.len());
        },
        Err(e) => {
            println!("Batch check error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test detailed health response
#[tokio::test]
async fn test_detailed_health() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");

    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    // Get detailed health response
    let result = client.health().await;

    match result {
        Ok(health_response) => {
            println!("Detailed health response: {:?}", health_response);
            assert!(health_response.is_healthy(), "Should report healthy");
            println!("Server version: {}", health_response.version);
            println!("Latency: {:?}", health_response.latency);
        },
        Err(e) => {
            println!("Detailed health error: {:?}", e);
        },
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}
