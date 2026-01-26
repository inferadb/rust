//! Common test harness for InferaDB Rust SDK integration tests.
//!
//! Provides test fixtures and utilities for testing against the dev environment.

use std::{process::Command, sync::OnceLock};

use anyhow::{Context, Result};
use base64::Engine;
use chrono::{Duration, Utc};
use ed25519_dalek::SigningKey;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Required JWT audience for InferaDB Server API
pub const REQUIRED_AUDIENCE: &str = "https://api.inferadb.com";

/// Cached API base URL discovered from Tailscale
static API_BASE_URL: OnceLock<String> = OnceLock::new();

/// Generate a random Ed25519 signing key
#[allow(dead_code)]
pub fn generate_signing_key() -> SigningKey {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes for signing key");
    SigningKey::from_bytes(&bytes)
}

/// Convert raw Ed25519 private key bytes (32 bytes) to PKCS#8 PEM format
fn ed25519_to_pem(private_key: &[u8; 32]) -> Vec<u8> {
    // PKCS#8 v1 structure for Ed25519
    let mut pkcs8_der = vec![
        0x30, 0x2e, // SEQUENCE (46 bytes)
        0x02, 0x01, 0x00, // INTEGER 0 (version)
        0x30, 0x05, // SEQUENCE (algorithm)
        0x06, 0x03, 0x2b, 0x65, 0x70, // OID 1.3.101.112
        0x04, 0x22, // OCTET STRING (34 bytes)
        0x04, 0x20, // OCTET STRING (32 bytes)
    ];
    pkcs8_der.extend_from_slice(private_key);

    let pem = format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(&pkcs8_der)
    );

    pem.into_bytes()
}

/// Discover the tailnet domain from the local Tailscale CLI
fn discover_tailnet() -> Result<String> {
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .context("Failed to run 'tailscale status --json'. Is Tailscale installed and running?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tailscale status failed: {}", stderr);
    }

    let status: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse Tailscale status JSON")?;

    // Extract DNS name from Self.DNSName (e.g., "hostname.tail27bf77.ts.net.")
    let dns_name = status
        .get("Self")
        .and_then(|s| s.get("DNSName"))
        .and_then(|d| d.as_str())
        .context("Could not find DNSName in Tailscale status")?;

    // Extract tailnet domain (everything after first dot, removing trailing dot)
    let tailnet = dns_name.trim_end_matches('.').split('.').skip(1).collect::<Vec<_>>().join(".");

    if tailnet.is_empty() {
        anyhow::bail!("Could not extract tailnet from DNSName: {}", dns_name);
    }

    Ok(tailnet)
}

/// Get the API base URL (discovers from Tailscale or uses environment override)
pub fn api_base_url() -> String {
    API_BASE_URL
        .get_or_init(|| {
            // Allow environment override for CI/testing
            if let Ok(url) = std::env::var("INFERADB_API_URL") {
                return url;
            }

            // Discover from Tailscale
            match discover_tailnet() {
                Ok(tailnet) => format!("https://inferadb-api.{}", tailnet),
                Err(e) => {
                    eprintln!("Warning: Could not discover Tailscale tailnet: {}", e);
                    eprintln!("Falling back to localhost. Set INFERADB_API_URL to override.");
                    "http://localhost:9090".to_string()
                },
            }
        })
        .clone()
}

/// Validate that the dev environment is running and accessible
pub async fn validate_environment() -> Result<()> {
    let base_url = api_base_url();
    let client = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()?;

    let health_url = format!("{}/healthz", base_url);
    let response = client.get(&health_url).send().await.context(format!(
        "Failed to connect to API at {}. Is the dev environment running? Run: inferadb dev start",
        health_url
    ))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Health check failed with status {}. Is the dev environment healthy?",
            response.status()
        );
    }

    println!("Environment validated: {}", base_url);
    Ok(())
}

/// Test context containing HTTP client for setup operations
#[derive(Clone)]
pub struct TestContext {
    pub http_client: HttpClient,
    pub api_base_url: String,
}

impl Default for TestContext {
    fn default() -> Self {
        Self {
            http_client: HttpClient::builder()
                .timeout(std::time::Duration::from_secs(30))
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Failed to create HTTP client"),
            api_base_url: api_base_url(),
        }
    }
}

impl TestContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get Control API URL
    pub fn control_url(&self, path: &str) -> String {
        format!("{}/control/v1{}", self.api_base_url, path)
    }

    /// Get Engine (Access) API URL
    #[allow(dead_code)]
    pub fn engine_url(&self, path: &str) -> String {
        format!("{}/access/v1{}", self.api_base_url, path)
    }
}

// Request/Response types for setup operations

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub accept_tos: bool,
}

#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub user_id: i64,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub email: String,
    #[allow(dead_code)]
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    #[allow(dead_code)]
    pub user_id: i64,
    #[allow(dead_code)]
    pub name: String,
    pub session_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct OrganizationResponse {
    pub id: i64,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub tier: String,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ListOrganizationsResponse {
    pub organizations: Vec<OrganizationResponse>,
    #[allow(dead_code)]
    pub pagination: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub organization_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct VaultInfo {
    pub id: i64,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub organization_id: i64,
    #[allow(dead_code)]
    pub sync_status: String,
    #[allow(dead_code)]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateVaultResponse {
    pub vault: VaultInfo,
}

#[derive(Debug, Serialize)]
pub struct CreateClientRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientInfo {
    pub id: i64,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub is_active: bool,
    #[allow(dead_code)]
    pub organization_id: i64,
    #[allow(dead_code)]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateClientResponse {
    pub client: ClientInfo,
}

#[derive(Debug, Serialize)]
pub struct CreateCertificateRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CertificateResponse {
    pub certificate: CertificateInfo,
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CertificateInfo {
    pub id: i64,
    pub kid: String,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub public_key: String,
    #[allow(dead_code)]
    pub is_active: bool,
    #[allow(dead_code)]
    pub created_at: String,
}

/// JWT claims for client authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub vault_id: String,
    pub org_id: String,
    pub scope: String,
    pub vault_role: String,
}

/// Test fixture for creating a complete test environment.
///
/// Creates a user, organization, vault, client, and certificate for testing.
/// Provides methods to generate JWTs and create SDK clients.
pub struct TestFixture {
    pub ctx: TestContext,
    pub user_id: i64,
    pub session_id: i64,
    pub org_id: i64,
    pub vault_id: i64,
    pub client_id: i64,
    #[allow(dead_code)]
    pub cert_id: i64,
    pub cert_kid: String,
    pub signing_key: SigningKey,
    #[allow(dead_code)]
    pub private_key_pem: String,
}

impl TestFixture {
    /// Create a complete test fixture with user, org, vault, and client
    pub async fn create() -> Result<Self> {
        let ctx = TestContext::new();

        // Register user
        let email = format!("sdk-test-{}@example.com", Uuid::new_v4());
        let register_req = RegisterRequest {
            name: "SDK Test User".to_string(),
            email: email.clone(),
            password: "SecurePassword123!".to_string(),
            accept_tos: true,
        };

        let response = ctx
            .http_client
            .post(ctx.control_url("/auth/register"))
            .json(&register_req)
            .send()
            .await
            .context("Failed to register user")?;

        let status = response.status();
        if !status.is_success() {
            let error_body =
                response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            anyhow::bail!("Registration failed with status {}: {}", status, error_body);
        }

        let register_resp: RegisterResponse =
            response.json().await.context("Failed to parse registration response")?;

        let user_id = register_resp.user_id;

        // Login to get session
        let login_req = LoginRequest { email, password: "SecurePassword123!".to_string() };

        let login_response = ctx
            .http_client
            .post(ctx.control_url("/auth/login/password"))
            .json(&login_req)
            .send()
            .await
            .context("Failed to login")?;

        let login_status = login_response.status();
        if !login_status.is_success() {
            let error_body = login_response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            anyhow::bail!("Login failed with status {}: {}", login_status, error_body);
        }

        let login_resp: LoginResponse =
            login_response.json().await.context("Failed to parse login response")?;

        let session_id = login_resp.session_id;

        // Get default organization (created during registration)
        let orgs_response: ListOrganizationsResponse = ctx
            .http_client
            .get(ctx.control_url("/organizations"))
            .header("Authorization", format!("Bearer {}", session_id))
            .send()
            .await
            .context("Failed to list organizations")?
            .error_for_status()
            .context("List organizations failed")?
            .json()
            .await
            .context("Failed to parse organizations response")?;

        let org_id =
            orgs_response.organizations.first().context("No default organization found")?.id;

        // Create vault
        let vault_req = CreateVaultRequest {
            name: format!("SDK Test Vault {}", Uuid::new_v4()),
            organization_id: org_id,
        };

        let create_vault_resp: CreateVaultResponse = ctx
            .http_client
            .post(ctx.control_url(&format!("/organizations/{}/vaults", org_id)))
            .header("Authorization", format!("Bearer {}", session_id))
            .json(&vault_req)
            .send()
            .await
            .context("Failed to create vault")?
            .error_for_status()
            .context("Vault creation failed")?
            .json()
            .await
            .context("Failed to parse vault response")?;

        let vault_id = create_vault_resp.vault.id;

        // Create client
        let client_req =
            CreateClientRequest { name: format!("SDK Test Client {}", Uuid::new_v4()) };

        let create_client_resp: CreateClientResponse = ctx
            .http_client
            .post(ctx.control_url(&format!("/organizations/{}/clients", org_id)))
            .header("Authorization", format!("Bearer {}", session_id))
            .json(&client_req)
            .send()
            .await
            .context("Failed to create client")?
            .error_for_status()
            .context("Client creation failed")?
            .json()
            .await
            .context("Failed to parse client response")?;

        let client_id = create_client_resp.client.id;

        // Create certificate (server generates the keypair)
        let cert_req =
            CreateCertificateRequest { name: format!("SDK Test Certificate {}", Uuid::new_v4()) };

        let cert_resp: CertificateResponse = ctx
            .http_client
            .post(ctx.control_url(&format!(
                "/organizations/{}/clients/{}/certificates",
                org_id, client_id
            )))
            .header("Authorization", format!("Bearer {}", session_id))
            .json(&cert_req)
            .send()
            .await
            .context("Failed to create certificate")?
            .error_for_status()
            .context("Certificate creation failed")?
            .json()
            .await
            .context("Failed to parse certificate response")?;

        let cert_id = cert_resp.certificate.id;
        let cert_kid = cert_resp.certificate.kid;

        // Parse the server-generated private key (base64 encoded)
        let private_key_bytes = base64::engine::general_purpose::STANDARD
            .decode(&cert_resp.private_key)
            .context("Failed to decode private key")?;
        let private_key_array: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
        let signing_key = SigningKey::from_bytes(&private_key_array);

        // Convert to PEM for SDK usage
        let pem_bytes = ed25519_to_pem(&private_key_array);
        let private_key_pem = String::from_utf8(pem_bytes).context("Failed to create PEM")?;

        Ok(Self {
            ctx,
            user_id,
            session_id,
            org_id,
            vault_id,
            client_id,
            cert_id,
            cert_kid,
            signing_key,
            private_key_pem,
        })
    }

    /// Generate a JWT token for the client with specified vault and scopes
    pub fn generate_jwt(&self, vault_id: Option<i64>, scopes: &[&str]) -> Result<String> {
        let now = Utc::now();

        let vault_role = if scopes.contains(&"inferadb.admin") {
            "admin"
        } else if scopes.contains(&"inferadb.vault.manage") {
            "manage"
        } else if scopes.contains(&"inferadb.write") {
            "write"
        } else {
            "read"
        };

        let scope_str = if scopes.is_empty() {
            "inferadb.check inferadb.read inferadb.expand inferadb.list inferadb.list-relationships inferadb.list-subjects inferadb.list-resources".to_string()
        } else {
            scopes.join(" ")
        };

        let claims = ClientClaims {
            iss: self.ctx.api_base_url.clone(),
            sub: format!("client:{}", self.client_id),
            aud: REQUIRED_AUDIENCE.to_string(),
            exp: (now + Duration::minutes(5)).timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            vault_id: vault_id.unwrap_or(self.vault_id).to_string(),
            org_id: self.org_id.to_string(),
            scope: scope_str,
            vault_role: vault_role.to_string(),
        };

        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.cert_kid.clone());

        let secret_bytes = self.signing_key.to_bytes();
        let pem = ed25519_to_pem(&secret_bytes);
        let encoding_key =
            EncodingKey::from_ed_pem(&pem).context("Failed to create encoding key")?;

        encode(&header, &claims, &encoding_key).context("Failed to encode JWT")
    }

    /// Create an InferaDB SDK client for this fixture
    pub async fn create_sdk_client(&self) -> Result<inferadb::Client> {
        let jwt = self.generate_jwt(None, &["inferadb.check", "inferadb.write"])?;

        let client = inferadb::Client::builder()
            .url(&self.ctx.api_base_url)
            .credentials(inferadb::BearerCredentialsConfig::new(jwt))
            .tls_config(inferadb::TlsConfig::insecure())
            .build()
            .await
            .context("Failed to create SDK client")?;

        Ok(client)
    }

    /// Get the vault ID as a string (for SDK methods that need it)
    pub fn vault_id_str(&self) -> String {
        format!("vlt_{}", self.vault_id)
    }

    /// Get the organization ID as a string
    pub fn org_id_str(&self) -> String {
        format!("org_{}", self.org_id)
    }

    /// Cleanup test resources
    pub async fn cleanup(&self) -> Result<()> {
        // Delete vault
        let _ =
            self.ctx
                .http_client
                .delete(self.ctx.control_url(&format!(
                    "/organizations/{}/vaults/{}",
                    self.org_id, self.vault_id
                )))
                .header("Authorization", format!("Bearer {}", self.session_id))
                .send()
                .await;

        // Delete client
        let _ =
            self.ctx
                .http_client
                .delete(self.ctx.control_url(&format!(
                    "/organizations/{}/clients/{}",
                    self.org_id, self.client_id
                )))
                .header("Authorization", format!("Bearer {}", self.session_id))
                .send()
                .await;

        // Delete organization
        let _ = self
            .ctx
            .http_client
            .delete(self.ctx.control_url(&format!("/organizations/{}", self.org_id)))
            .header("Authorization", format!("Bearer {}", self.session_id))
            .send()
            .await;

        // Delete user
        let _ = self
            .ctx
            .http_client
            .delete(self.ctx.control_url(&format!("/users/{}", self.user_id)))
            .header("Authorization", format!("Bearer {}", self.session_id))
            .send()
            .await;

        Ok(())
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Best-effort cleanup on drop
        let ctx = self.ctx.clone();
        let session_id = self.session_id;
        let vault_id = self.vault_id;
        let org_id = self.org_id;
        let client_id = self.client_id;
        let user_id = self.user_id;

        tokio::spawn(async move {
            let _ = ctx
                .http_client
                .delete(ctx.control_url(&format!("/organizations/{}/vaults/{}", org_id, vault_id)))
                .header("Authorization", format!("Bearer {}", session_id))
                .send()
                .await;

            let _ = ctx
                .http_client
                .delete(
                    ctx.control_url(&format!("/organizations/{}/clients/{}", org_id, client_id)),
                )
                .header("Authorization", format!("Bearer {}", session_id))
                .send()
                .await;

            let _ = ctx
                .http_client
                .delete(ctx.control_url(&format!("/organizations/{}", org_id)))
                .header("Authorization", format!("Bearer {}", session_id))
                .send()
                .await;

            let _ = ctx
                .http_client
                .delete(ctx.control_url(&format!("/users/{}", user_id)))
                .header("Authorization", format!("Bearer {}", session_id))
                .send()
                .await;
        });
    }
}
