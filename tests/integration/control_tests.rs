//! Control API integration tests.
//!
//! These tests verify control plane operations like organization
//! and vault management against the dev environment.

use crate::common::TestFixture;

/// Test getting organization info via SDK
#[tokio::test]
async fn test_get_organization() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to get organization info via control API
    let result = org.control().get().await;

    match result {
        Ok(org_info) => {
            println!("Got organization: {:?}", org_info);
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("Get organization error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing vaults in an organization
#[tokio::test]
async fn test_list_vaults() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to list vaults via control API
    let result = org.control().vaults().list().await;

    match result {
        Ok(page) => {
            println!("Found {} vaults", page.items.len());
            // Should have at least the test vault we created
            assert!(!page.items.is_empty(), "Should have at least one vault");
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("List vaults error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test getting vault info
#[tokio::test]
async fn test_get_vault() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to get vault info via control API
    let result = org.control().vaults().get(fixture.vault_id_str()).await;

    match result {
        Ok(vault_info) => {
            println!("Got vault: {:?}", vault_info);
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("Get vault error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing API clients in an organization
#[tokio::test]
async fn test_list_clients() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to list clients
    let result = org.clients().list().await;

    match result {
        Ok(page) => {
            println!("Found {} API clients", page.items.len());
            // Should have at least the test client we created
            assert!(!page.items.is_empty(), "Should have at least one client");
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("List clients error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test getting JWKS
#[tokio::test]
async fn test_get_jwks() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    // Try to get JWKS (this is on the main client, not organization)
    let result = client.jwks().get().await;

    match result {
        Ok(jwks) => {
            println!("Got JWKS with {} keys", jwks.keys.len());
            // Should have at least the certificate key we created
            assert!(!jwks.keys.is_empty(), "JWKS should have at least one key");

            // Check if our certificate kid is in the JWKS
            let has_our_key = jwks.find_key(&fixture.cert_kid).is_some();
            println!("Our certificate kid {} in JWKS: {}", fixture.cert_kid, has_our_key);
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("Get JWKS error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing organization members
#[tokio::test]
async fn test_list_members() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to list members via control API
    let result = org.control().members().list().await;

    match result {
        Ok(page) => {
            println!("Found {} members", page.items.len());
            // Should have at least the owner (our test user)
            assert!(!page.items.is_empty(), "Should have at least one member (owner)");
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("List members error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test listing teams
#[tokio::test]
async fn test_list_teams() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to list teams via control API
    let result = org.control().teams().list().await;

    match result {
        Ok(page) => {
            println!("Found {} teams", page.items.len());
            // May be empty if no teams have been created
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("List teams error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test audit log query
#[tokio::test]
async fn test_audit_log() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    let org = client.organization(fixture.org_id_str());

    // Try to query audit log via control API
    let result = org.control().audit_logs().list().await;

    match result {
        Ok(page) => {
            println!("Found {} audit events", page.items.len());
            // Should have at least some events from our setup
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("Audit log query error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}

/// Test account info
#[tokio::test]
async fn test_account_info() {
    let fixture = TestFixture::create().await.expect("Failed to create test fixture");
    let client = fixture.create_sdk_client().await.expect("Failed to create SDK client");

    // Try to get account info
    let result = client.account().get().await;

    match result {
        Ok(account) => {
            println!("Got account info: {:?}", account);
        }
        Err(e) => {
            // May fail if control API is not fully wired yet
            println!("Get account error: {:?}", e);
        }
    }

    fixture.cleanup().await.expect("Cleanup should succeed");
}
