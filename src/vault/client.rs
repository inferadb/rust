//! VaultClient implementation.

use std::borrow::Cow;

use crate::client::Client;
use crate::types::{ConsistencyToken, Context, Decision};
use crate::{AccessDenied, Error};

/// A vault-scoped client for authorization operations.
///
/// `VaultClient` provides the main authorization API:
///
/// - [`check()`](VaultClient::check): Check permissions
/// - [`relationships()`](VaultClient::relationships): Manage relationships
/// - [`resources()`](VaultClient::resources): Query resources
/// - [`subjects()`](VaultClient::subjects): Query subjects
///
/// ## Creating a VaultClient
///
/// Obtain a `VaultClient` through the client hierarchy:
///
/// ```rust,ignore
/// let vault = client.organization("org_123").vault("vlt_456");
/// ```
///
/// ## Thread Safety
///
/// `VaultClient` is `Clone` and thread-safe.
#[derive(Clone)]
pub struct VaultClient {
    client: Client,
    organization_id: String,
    vault_id: String,
}

impl VaultClient {
    /// Creates a new VaultClient.
    pub(crate) fn new(client: Client, organization_id: String, vault_id: String) -> Self {
        Self {
            client,
            organization_id,
            vault_id,
        }
    }

    /// Returns the organization ID.
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }

    /// Returns the vault ID.
    pub fn vault_id(&self) -> &str {
        &self.vault_id
    }

    /// Returns the underlying client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Checks if a subject has a permission on a resource.
    ///
    /// # Argument Order
    ///
    /// The argument order follows the question "Can subject do permission to resource?":
    /// - `subject`: Who is requesting access (e.g., "user:alice")
    /// - `permission`: What action they want to do (e.g., "view")
    /// - `resource`: What they want to access (e.g., "document:readme")
    ///
    /// # Return Value
    ///
    /// Returns `Ok(true)` if allowed, `Ok(false)` if denied.
    /// Only returns `Err` for actual errors (network, auth, etc.).
    ///
    /// **Important**: Denial is NOT an error. Use `require()` if you want
    /// denial to be an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Simple check
    /// let allowed = vault.check("user:alice", "view", "doc:readme").await?;
    /// if allowed {
    ///     println!("Access granted");
    /// }
    ///
    /// // With context (ABAC)
    /// let allowed = vault.check("user:alice", "view", "doc:sensitive")
    ///     .with_context(Context::new().with("environment", "production"))
    ///     .await?;
    ///
    /// // With consistency token (read-after-write)
    /// let allowed = vault.check("user:alice", "view", "doc:1")
    ///     .at_least_as_fresh(token)
    ///     .await?;
    ///
    /// // Require access (denial is an error)
    /// vault.check("user:alice", "view", "doc:1")
    ///     .require()
    ///     .await?;
    /// ```
    pub fn check<'a>(
        &self,
        subject: impl Into<Cow<'a, str>>,
        permission: impl Into<Cow<'a, str>>,
        resource: impl Into<Cow<'a, str>>,
    ) -> CheckRequest<'a> {
        CheckRequest {
            vault: self.clone(),
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            context: None,
            consistency: None,
        }
    }

    /// Checks multiple authorization requests in a single batch.
    ///
    /// Batch checks are more efficient than individual checks when you need
    /// to verify multiple permissions at once. Results are returned in the
    /// same order as the input requests.
    ///
    /// # Arguments
    ///
    /// * `checks` - An iterator of (subject, permission, resource) tuples
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let checks = vec![
    ///     ("user:alice", "view", "doc:1"),
    ///     ("user:alice", "edit", "doc:1"),
    ///     ("user:bob", "view", "doc:2"),
    /// ];
    ///
    /// let results = vault.check_batch(checks).await?;
    /// for (check, allowed) in checks.iter().zip(results.iter()) {
    ///     println!("{:?} -> {}", check, allowed);
    /// }
    /// ```
    ///
    /// # Ordering Guarantee
    ///
    /// Results are **always** in the same order as the input. If you pass
    /// checks A, B, C, you get results for A, B, C in that order.
    pub fn check_batch<'a, I, S, P, R>(&self, checks: I) -> BatchCheckRequest<'a>
    where
        I: IntoIterator<Item = (S, P, R)>,
        S: Into<Cow<'a, str>>,
        P: Into<Cow<'a, str>>,
        R: Into<Cow<'a, str>>,
    {
        let items: Vec<_> = checks
            .into_iter()
            .map(|(s, p, r)| BatchCheckItem {
                subject: s.into(),
                permission: p.into(),
                resource: r.into(),
            })
            .collect();

        BatchCheckRequest {
            vault: self.clone(),
            items,
            context: None,
            consistency: None,
        }
    }

    /// Returns a client for managing relationships in this vault.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Write a relationship
    /// let token = vault.relationships()
    ///     .write(Relationship::new("doc:1", "viewer", "user:alice"))
    ///     .await?;
    ///
    /// // Delete a relationship
    /// vault.relationships()
    ///     .delete(Relationship::new("doc:1", "viewer", "user:alice"))
    ///     .await?;
    ///
    /// // List relationships
    /// let rels = vault.relationships()
    ///     .list()
    ///     .resource("doc:1")
    ///     .await?;
    /// ```
    pub fn relationships(&self) -> RelationshipsClient {
        RelationshipsClient::new(self.clone())
    }
}

impl std::fmt::Debug for VaultClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultClient")
            .field("organization_id", &self.organization_id)
            .field("vault_id", &self.vault_id)
            .finish_non_exhaustive()
    }
}

/// A builder for authorization check requests.
///
/// Created by [`VaultClient::check()`]. Use method chaining to add
/// context or consistency requirements, then `.await` to execute.
pub struct CheckRequest<'a> {
    vault: VaultClient,
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
    context: Option<Context>,
    consistency: Option<ConsistencyToken>,
}

impl<'a> CheckRequest<'a> {
    /// Adds ABAC context to the check.
    ///
    /// Context values are evaluated against conditions in the authorization schema.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let allowed = vault.check("user:alice", "access", "resource:data")
    ///     .with_context(Context::new()
    ///         .with("environment", "production")
    ///         .with("time_of_day", "business_hours"))
    ///     .await?;
    /// ```
    #[must_use]
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Specifies a consistency requirement.
    ///
    /// Ensures the check sees data at least as fresh as the given token.
    /// Use this for read-after-write consistency.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // After writing a relationship
    /// let token = vault.relationships()
    ///     .write(relationship)
    ///     .await?;
    ///
    /// // Check with consistency guarantee
    /// let allowed = vault.check("user:alice", "view", "doc:1")
    ///     .at_least_as_fresh(token)
    ///     .await?;
    /// ```
    #[must_use]
    pub fn at_least_as_fresh(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Converts this to a requiring check that returns an error on denial.
    ///
    /// Instead of returning `Ok(false)` for denial, `require()` returns
    /// `Err(AccessDenied)`. This is useful when you want to use `?` to
    /// propagate denial as an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // This returns Err(AccessDenied) if denied
    /// vault.check("user:alice", "delete", "doc:important")
    ///     .require()
    ///     .await?;
    ///
    /// // If we get here, access was granted
    /// delete_document("doc:important").await;
    /// ```
    pub fn require(self) -> RequireCheckRequest<'a> {
        RequireCheckRequest { inner: self }
    }

    /// Executes the check and returns a detailed decision.
    ///
    /// The detailed decision includes metadata like evaluation time,
    /// reason for the decision, and trace information.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let decision = vault.check("user:alice", "view", "doc:1")
    ///     .detailed()
    ///     .await?;
    ///
    /// if decision.is_allowed() {
    ///     if let Some(meta) = decision.metadata() {
    ///         println!("Evaluation took: {:?}", meta.evaluation_time);
    ///     }
    /// }
    /// ```
    pub async fn detailed(self) -> Result<Decision, Error> {
        // TODO: Implement actual API call
        Ok(Decision::allowed())
    }

    /// Executes the check and returns a boolean result.
    async fn execute(self) -> Result<bool, Error> {
        // TODO: Implement actual API call
        Ok(true)
    }
}

impl<'a> std::future::IntoFuture for CheckRequest<'a> {
    type Output = Result<bool, Error>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// A check request that returns an error on denial.
///
/// Created by [`CheckRequest::require()`].
pub struct RequireCheckRequest<'a> {
    inner: CheckRequest<'a>,
}

impl<'a> RequireCheckRequest<'a> {
    /// Adds ABAC context to the check.
    #[must_use]
    pub fn with_context(mut self, context: Context) -> Self {
        self.inner.context = Some(context);
        self
    }

    /// Specifies a consistency requirement.
    #[must_use]
    pub fn at_least_as_fresh(mut self, token: ConsistencyToken) -> Self {
        self.inner.consistency = Some(token);
        self
    }

    /// Executes the check and returns an error on denial.
    async fn execute(self) -> Result<(), AccessDenied> {
        // TODO: Implement actual API call
        // For now, simulate success
        let _allowed = true;

        if _allowed {
            Ok(())
        } else {
            Err(AccessDenied::new(
                self.inner.subject.into_owned(),
                self.inner.permission.into_owned(),
                self.inner.resource.into_owned(),
            ))
        }
    }
}

impl<'a> std::future::IntoFuture for RequireCheckRequest<'a> {
    type Output = Result<(), AccessDenied>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;

    async fn create_test_vault() -> VaultClient {
        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
            .await
            .unwrap();

        client.organization("org_test").vault("vlt_test")
    }

    #[tokio::test]
    async fn test_check_basic() {
        let vault = create_test_vault().await;
        let result = vault.check("user:alice", "view", "doc:1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_with_context() {
        let vault = create_test_vault().await;
        let result = vault
            .check("user:alice", "view", "doc:1")
            .with_context(Context::new().with("env", "prod"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_with_consistency() {
        let vault = create_test_vault().await;
        let token = ConsistencyToken::new("test_token");
        let result = vault
            .check("user:alice", "view", "doc:1")
            .at_least_as_fresh(token)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require() {
        let vault = create_test_vault().await;
        let result = vault
            .check("user:alice", "view", "doc:1")
            .require()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detailed() {
        let vault = create_test_vault().await;
        let decision = vault
            .check("user:alice", "view", "doc:1")
            .detailed()
            .await
            .unwrap();
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_vault_client_debug() {
        let vault = create_test_vault().await;
        let debug = format!("{:?}", vault);
        assert!(debug.contains("VaultClient"));
        assert!(debug.contains("org_test"));
        assert!(debug.contains("vlt_test"));
    }
}
