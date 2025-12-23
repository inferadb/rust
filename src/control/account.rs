//! Account management for the control plane.
//!
//! Provides operations for managing the current user's account,
//! including email addresses, sessions, and password management.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::control::Page;
use crate::Error;

/// Client for managing the current user's account.
///
/// Access via `client.account()`.
///
/// ## Example
///
/// ```rust,ignore
/// let account = client.account();
///
/// // Get current account info
/// let info = account.get().await?;
/// println!("Logged in as: {}", info.email);
///
/// // List sessions
/// let sessions = account.sessions().list().await?;
/// ```
#[derive(Clone)]
pub struct AccountClient {
    client: Client,
}

impl AccountClient {
    /// Creates a new account client.
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the current user's account information.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let account = client.account().get().await?;
    /// println!("Email: {}", account.email);
    /// println!("MFA enabled: {}", account.mfa_enabled);
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self) -> Result<Account, Error> {
        self.client.inner().control_get("/control/v1/account").await
    }

    #[cfg(not(feature = "rest"))]
    pub async fn get(&self) -> Result<Account, Error> {
        Err(Error::configuration(
            "REST feature is required for account API",
        ))
    }

    /// Updates the current user's account.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let updated = client.account()
    ///     .update(UpdateAccountRequest::new().with_name("Alice"))
    ///     .await?;
    /// ```
    pub async fn update(&self, request: UpdateAccountRequest) -> Result<Account, Error> {
        #[cfg(feature = "rest")]
        {
            return self
                .client
                .inner()
                .control_patch("/control/v1/users/me", &request)
                .await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = request;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Returns a client for managing email addresses.
    pub fn emails(&self) -> EmailsClient {
        EmailsClient {
            client: self.client.clone(),
        }
    }

    /// Returns a client for managing sessions.
    pub fn sessions(&self) -> SessionsClient {
        SessionsClient {
            client: self.client.clone(),
        }
    }

    /// Changes the account password.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.account()
    ///     .change_password(ChangePasswordRequest {
    ///         current_password: "old_password".into(),
    ///         new_password: "new_secure_password".into(),
    ///     })
    ///     .await?;
    /// ```
    pub async fn change_password(&self, request: ChangePasswordRequest) -> Result<(), Error> {
        #[cfg(feature = "rest")]
        {
            // Password changes may return an empty response or the updated account
            let _: serde_json::Value = self
                .client
                .inner()
                .control_post("/control/v1/users/me/password", &request)
                .await?;
            Ok(())
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = request;
            Err(Error::configuration("REST feature is required"))
        }
    }
}

impl std::fmt::Debug for AccountClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountClient").finish_non_exhaustive()
    }
}

/// Information about a user account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// The account ID (e.g., "usr_abc123").
    pub id: String,
    /// The primary email address.
    pub email: String,
    /// The user's display name.
    pub name: Option<String>,
    /// The account status.
    pub status: AccountStatus,
    /// When the account was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the account was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Whether MFA is enabled.
    pub mfa_enabled: bool,
}

/// Account status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    /// Account is active and can be used normally.
    Active,
    /// Account has been suspended by an administrator.
    Suspended,
    /// Account is pending email verification.
    PendingVerification,
}

impl AccountStatus {
    /// Returns `true` if the account is active.
    pub fn is_active(&self) -> bool {
        matches!(self, AccountStatus::Active)
    }

    /// Returns `true` if the account is suspended.
    pub fn is_suspended(&self) -> bool {
        matches!(self, AccountStatus::Suspended)
    }

    /// Returns `true` if the account is pending verification.
    pub fn is_pending_verification(&self) -> bool {
        matches!(self, AccountStatus::PendingVerification)
    }
}

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "active"),
            AccountStatus::Suspended => write!(f, "suspended"),
            AccountStatus::PendingVerification => write!(f, "pending_verification"),
        }
    }
}

/// Request to update an account.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateAccountRequest {
    /// New display name.
    pub name: Option<String>,
}

impl UpdateAccountRequest {
    /// Creates a new empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Request to change account password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    /// The current password.
    pub current_password: String,
    /// The new password.
    pub new_password: String,
}

impl ChangePasswordRequest {
    /// Creates a new password change request.
    pub fn new(current_password: impl Into<String>, new_password: impl Into<String>) -> Self {
        Self {
            current_password: current_password.into(),
            new_password: new_password.into(),
        }
    }
}

/// An email address associated with an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    /// The email address.
    pub address: String,
    /// Whether the email has been verified.
    pub verified: bool,
    /// Whether this is the primary email address.
    pub primary: bool,
    /// When the email was added.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Client for managing email addresses.
#[derive(Clone)]
pub struct EmailsClient {
    client: Client,
}

impl EmailsClient {
    /// Lists all email addresses on the account.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let emails = client.account().emails().list().await?;
    /// for email in emails.items {
    ///     println!("{}: verified={}, primary={}", email.address, email.verified, email.primary);
    /// }
    /// ```
    pub async fn list(&self) -> Result<Page<Email>, Error> {
        #[cfg(feature = "rest")]
        {
            return self
                .client
                .inner()
                .control_get("/control/v1/users/emails")
                .await;
        }
        #[cfg(not(feature = "rest"))]
        Err(Error::configuration("REST feature is required"))
    }

    /// Adds a new email address to the account.
    ///
    /// The email will need to be verified before it can be used.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let email = client.account().emails().add("newemail@example.com").await?;
    /// // Check your inbox for verification email
    /// ```
    pub async fn add(&self, address: impl Into<String>) -> Result<Email, Error> {
        let address = address.into();
        #[cfg(feature = "rest")]
        {
            #[derive(serde::Serialize)]
            struct AddEmailRequest {
                email: String,
            }
            return self
                .client
                .inner()
                .control_post(
                    "/control/v1/users/emails",
                    &AddEmailRequest { email: address },
                )
                .await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = address;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Removes an email address from the account.
    ///
    /// Cannot remove the primary email address.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.account().emails().remove("oldemail@example.com").await?;
    /// ```
    pub async fn remove(&self, address: impl Into<String>) -> Result<(), Error> {
        let address = address.into();
        #[cfg(feature = "rest")]
        {
            // URL-encode the email address for the path
            let encoded = urlencoding::encode(&address);
            let path = format!("/control/v1/users/emails/{}", encoded);
            return self.client.inner().control_delete(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = address;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Sets an email address as the primary email.
    ///
    /// The email must be verified first.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.account().emails().set_primary("newemail@example.com").await?;
    /// ```
    pub async fn set_primary(&self, address: impl Into<String>) -> Result<(), Error> {
        let address = address.into();
        #[cfg(feature = "rest")]
        {
            #[derive(serde::Serialize)]
            struct SetPrimaryRequest {
                primary: bool,
            }
            // URL-encode the email address for the path
            let encoded = urlencoding::encode(&address);
            let path = format!("/control/v1/users/emails/{}", encoded);
            let _: Email = self
                .client
                .inner()
                .control_patch(&path, &SetPrimaryRequest { primary: true })
                .await?;
            Ok(())
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = address;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Resends the verification email for an unverified address.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.account().emails().resend_verification("unverified@example.com").await?;
    /// ```
    pub async fn resend_verification(&self, address: impl Into<String>) -> Result<(), Error> {
        let address = address.into();
        #[cfg(feature = "rest")]
        {
            // URL-encode the email address for the path
            let encoded = urlencoding::encode(&address);
            let path = format!("/control/v1/users/emails/{}/resend-verification", encoded);
            let _: serde_json::Value = self.client.inner().control_post_empty(&path).await?;
            Ok(())
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = address;
            Err(Error::configuration("REST feature is required"))
        }
    }
}

impl std::fmt::Debug for EmailsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailsClient").finish_non_exhaustive()
    }
}

/// An active session for the account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The session ID.
    pub id: String,
    /// When the session was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// The IP address that created the session.
    pub ip_address: Option<String>,
    /// The user agent string.
    pub user_agent: Option<String>,
    /// Whether this is the current session.
    pub current: bool,
}

/// Client for managing sessions.
#[derive(Clone)]
pub struct SessionsClient {
    client: Client,
}

impl SessionsClient {
    /// Lists all active sessions.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let sessions = client.account().sessions().list().await?;
    /// for session in sessions.items {
    ///     let current = if session.current { " (current)" } else { "" };
    ///     println!("{}: {} {}{}", session.id, session.ip_address.unwrap_or_default(), session.user_agent.unwrap_or_default(), current);
    /// }
    /// ```
    pub async fn list(&self) -> Result<Page<Session>, Error> {
        #[cfg(feature = "rest")]
        {
            return self
                .client
                .inner()
                .control_get("/control/v1/users/sessions")
                .await;
        }
        #[cfg(not(feature = "rest"))]
        Err(Error::configuration("REST feature is required"))
    }

    /// Revokes a specific session.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// client.account().sessions().revoke("ses_abc123").await?;
    /// ```
    pub async fn revoke(&self, session_id: impl Into<String>) -> Result<(), Error> {
        let session_id = session_id.into();
        #[cfg(feature = "rest")]
        {
            let path = format!("/control/v1/users/sessions/{}", session_id);
            return self.client.inner().control_delete(&path).await;
        }
        #[cfg(not(feature = "rest"))]
        {
            let _ = session_id;
            Err(Error::configuration("REST feature is required"))
        }
    }

    /// Revokes all sessions except the current one.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // Log out of all other devices
    /// client.account().sessions().revoke_all_others().await?;
    /// ```
    pub async fn revoke_all_others(&self) -> Result<(), Error> {
        #[cfg(feature = "rest")]
        {
            let _: serde_json::Value = self
                .client
                .inner()
                .control_post_empty("/control/v1/users/sessions/revoke-others")
                .await?;
            Ok(())
        }
        #[cfg(not(feature = "rest"))]
        Err(Error::configuration("REST feature is required"))
    }

    /// Revokes all sessions including the current one.
    ///
    /// **Warning**: This will log out the current session as well.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // Log out everywhere
    /// client.account().sessions().revoke_all().await?;
    /// ```
    pub async fn revoke_all(&self) -> Result<(), Error> {
        #[cfg(feature = "rest")]
        {
            let _: serde_json::Value = self
                .client
                .inner()
                .control_post_empty("/control/v1/users/sessions/revoke-all")
                .await?;
            Ok(())
        }
        #[cfg(not(feature = "rest"))]
        Err(Error::configuration("REST feature is required"))
    }
}

impl std::fmt::Debug for SessionsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionsClient").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;

    async fn create_test_client() -> Client {
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
            .await
            .unwrap()
    }

    #[test]
    fn test_account_status() {
        assert!(AccountStatus::Active.is_active());
        assert!(!AccountStatus::Active.is_suspended());
        assert!(!AccountStatus::Active.is_pending_verification());

        assert!(!AccountStatus::Suspended.is_active());
        assert!(AccountStatus::Suspended.is_suspended());

        assert!(AccountStatus::PendingVerification.is_pending_verification());
    }

    #[test]
    fn test_account_status_display() {
        assert_eq!(AccountStatus::Active.to_string(), "active");
        assert_eq!(AccountStatus::Suspended.to_string(), "suspended");
        assert_eq!(
            AccountStatus::PendingVerification.to_string(),
            "pending_verification"
        );
    }

    #[test]
    fn test_update_account_request() {
        let req = UpdateAccountRequest::new().with_name("Alice");
        assert_eq!(req.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_change_password_request() {
        let req = ChangePasswordRequest::new("old", "new");
        assert_eq!(req.current_password, "old");
        assert_eq!(req.new_password, "new");
    }

    #[tokio::test]
    async fn test_debug_impls() {
        let client = create_test_client().await;
        let account = AccountClient::new(client.clone());

        assert!(format!("{:?}", account).contains("AccountClient"));
        assert!(format!("{:?}", account.emails()).contains("EmailsClient"));
        assert!(format!("{:?}", account.sessions()).contains("SessionsClient"));
    }

    // Additional tests for Clone implementations
    #[tokio::test]
    async fn test_account_client_clone() {
        let client = create_test_client().await;
        let account = AccountClient::new(client);
        let _cloned = account.clone();
    }

    #[tokio::test]
    async fn test_emails_client_clone() {
        let client = create_test_client().await;
        let account = AccountClient::new(client);
        let emails = account.emails();
        let _cloned = emails.clone();
    }

    #[tokio::test]
    async fn test_sessions_client_clone() {
        let client = create_test_client().await;
        let account = AccountClient::new(client);
        let sessions = account.sessions();
        let _cloned = sessions.clone();
    }

    // Test Account type serialization
    #[test]
    fn test_account_serde() {
        let json = r#"{
            "id": "usr_abc123",
            "email": "test@example.com",
            "name": "Test User",
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "mfa_enabled": false
        }"#;
        let account: Account = serde_json::from_str(json).unwrap();
        assert_eq!(account.id, "usr_abc123");
        assert_eq!(account.email, "test@example.com");
        assert_eq!(account.name, Some("Test User".to_string()));
        assert!(account.status.is_active());
        assert!(!account.mfa_enabled);
    }

    #[test]
    fn test_account_clone() {
        let account = Account {
            id: "usr_abc123".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test".to_string()),
            status: AccountStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            mfa_enabled: true,
        };
        let cloned = account.clone();
        assert_eq!(cloned.id, "usr_abc123");
        assert!(cloned.mfa_enabled);
    }

    // Test Email type
    #[test]
    fn test_email_serde() {
        let json = r#"{
            "address": "test@example.com",
            "verified": true,
            "primary": true,
            "created_at": "2024-01-01T00:00:00Z"
        }"#;
        let email: Email = serde_json::from_str(json).unwrap();
        assert_eq!(email.address, "test@example.com");
        assert!(email.verified);
        assert!(email.primary);
    }

    #[test]
    fn test_email_clone() {
        let email = Email {
            address: "test@example.com".to_string(),
            verified: false,
            primary: false,
            created_at: chrono::Utc::now(),
        };
        let cloned = email.clone();
        assert_eq!(cloned.address, "test@example.com");
        assert!(!cloned.verified);
    }

    // Test Session type
    #[test]
    fn test_session_serde() {
        let json = r#"{
            "id": "ses_abc123",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-02-01T00:00:00Z",
            "ip_address": "192.168.1.1",
            "user_agent": "Mozilla/5.0",
            "current": true
        }"#;
        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "ses_abc123");
        assert!(session.current);
        assert_eq!(session.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(session.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_session_clone() {
        let session = Session {
            id: "ses_abc".to_string(),
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now(),
            ip_address: None,
            user_agent: None,
            current: false,
        };
        let cloned = session.clone();
        assert_eq!(cloned.id, "ses_abc");
        assert!(!cloned.current);
    }

    // Test UpdateAccountRequest serialization
    #[test]
    fn test_update_account_request_serde() {
        let req = UpdateAccountRequest::new().with_name("New Name");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("New Name"));
    }

    // Test ChangePasswordRequest serialization
    #[test]
    fn test_change_password_request_serde() {
        let req = ChangePasswordRequest::new("old_pass", "new_pass");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("old_pass"));
        assert!(json.contains("new_pass"));
    }

    #[test]
    fn test_change_password_request_clone() {
        let req = ChangePasswordRequest::new("old", "new");
        let cloned = req.clone();
        assert_eq!(cloned.current_password, "old");
        assert_eq!(cloned.new_password, "new");
    }

    #[test]
    fn test_update_account_request_clone() {
        let req = UpdateAccountRequest::new().with_name("Test");
        let cloned = req.clone();
        assert_eq!(cloned.name, Some("Test".to_string()));
    }
}

#[cfg(test)]
#[cfg(feature = "rest")]
mod wiremock_tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;
    use crate::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn create_mock_client(server: &MockServer) -> Client {
        Client::builder()
            .url(server.uri())
            .credentials(BearerCredentialsConfig::new("test_token"))
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_account() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/account"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "usr_123",
                "email": "user@example.com",
                "name": "Test User",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z",
                "mfa_enabled": false
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().get().await;

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.id, "usr_123");
        assert_eq!(account.email, "user@example.com");
    }

    #[tokio::test]
    async fn test_get_account_unauthorized() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/account"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Unauthorized"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().get().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_account() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/control/v1/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "usr_123",
                "email": "user@example.com",
                "name": "Updated Name",
                "status": "active",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-02T00:00:00Z",
                "mfa_enabled": false
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = UpdateAccountRequest::new().with_name("Updated Name");
        let result = client.account().update(request).await;

        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.name, Some("Updated Name".to_string()));
    }

    #[tokio::test]
    async fn test_change_password() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/users/me/password"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let request = ChangePasswordRequest::new("old_pass", "new_pass");
        let result = client.account().change_password(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_emails() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/users/emails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "address": "primary@example.com",
                        "verified": true,
                        "primary": true,
                        "created_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "address": "secondary@example.com",
                        "verified": false,
                        "primary": false,
                        "created_at": "2024-01-02T00:00:00Z"
                    }
                ],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 2
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().emails().list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].address, "primary@example.com");
    }

    #[tokio::test]
    async fn test_add_email() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/users/emails"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "address": "new@example.com",
                "verified": false,
                "primary": false,
                "created_at": "2024-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().emails().add("new@example.com").await;

        assert!(result.is_ok());
        let email = result.unwrap();
        assert_eq!(email.address, "new@example.com");
        assert!(!email.verified);
    }

    #[tokio::test]
    async fn test_remove_email() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/users/emails/old%40example.com"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().emails().remove("old@example.com").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/control/v1/users/sessions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "sess_123",
                        "ip_address": "192.168.1.1",
                        "user_agent": "Mozilla/5.0",
                        "created_at": "2024-01-01T00:00:00Z",
                        "expires_at": "2024-02-01T00:00:00Z",
                        "current": true
                    }
                ],
                "page_info": {
                    "has_next": false,
                    "next_cursor": null,
                    "total_count": 1
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().sessions().list().await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].current);
    }

    #[tokio::test]
    async fn test_revoke_session() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/control/v1/users/sessions/sess_123"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().sessions().revoke("sess_123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_revoke_all_sessions() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/control/v1/users/sessions/revoke-all"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let client = create_mock_client(&server).await;
        let result = client.account().sessions().revoke_all().await;

        assert!(result.is_ok());
    }
}
