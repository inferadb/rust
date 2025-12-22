//! VaultClient implementation.

// Allow dead code for request types that aren't fully integrated yet
#![allow(dead_code)]

use std::borrow::Cow;

use crate::client::Client;
use crate::types::{ConsistencyToken, Context, Decision, Relationship};
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

    /// Returns a client for querying resources accessible by a subject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Find all documents Alice can view
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .resource_type("document")
    ///     .collect()
    ///     .await?;
    ///
    /// // Stream results for large result sets
    /// let mut stream = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .stream();
    ///
    /// while let Some(resource) = stream.try_next().await? {
    ///     println!("Resource: {}", resource);
    /// }
    /// ```
    pub fn resources(&self) -> ResourcesClient<'_> {
        ResourcesClient::new(self)
    }

    /// Returns a client for querying subjects with access to a resource.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Find all users who can edit this document
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .subject_type("user")
    ///     .collect()
    ///     .await?;
    ///
    /// // Stream results for large result sets
    /// let mut stream = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .stream();
    ///
    /// while let Some(subject) = stream.try_next().await? {
    ///     println!("Subject: {}", subject);
    /// }
    /// ```
    pub fn subjects(&self) -> SubjectsClient<'_> {
        SubjectsClient::new(self)
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
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

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
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// An individual check item in a batch request.
#[derive(Debug, Clone)]
pub struct BatchCheckItem<'a> {
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
}

impl<'a> BatchCheckItem<'a> {
    /// Creates a new batch check item.
    pub fn new(
        subject: impl Into<Cow<'a, str>>,
        permission: impl Into<Cow<'a, str>>,
        resource: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
        }
    }

    /// Returns the subject.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the permission.
    pub fn permission(&self) -> &str {
        &self.permission
    }

    /// Returns the resource.
    pub fn resource(&self) -> &str {
        &self.resource
    }
}

/// A builder for batch authorization check requests.
///
/// Created by [`VaultClient::check_batch()`]. Use method chaining to add
/// context or consistency requirements, then `.await` to execute.
pub struct BatchCheckRequest<'a> {
    vault: VaultClient,
    items: Vec<BatchCheckItem<'a>>,
    context: Option<Context>,
    consistency: Option<ConsistencyToken>,
}

impl<'a> BatchCheckRequest<'a> {
    /// Adds ABAC context to all checks in the batch.
    ///
    /// The same context is applied to every check in the batch.
    #[must_use]
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Specifies a consistency requirement for all checks.
    #[must_use]
    pub fn at_least_as_fresh(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Returns the number of checks in this batch.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Executes the batch check and returns results.
    async fn execute(self) -> Result<Vec<bool>, Error> {
        // TODO: Implement actual API call
        // For now, return all true
        Ok(vec![true; self.items.len()])
    }
}

impl<'a> std::future::IntoFuture for BatchCheckRequest<'a> {
    type Output = Result<Vec<bool>, Error>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// The result of a batch check, with detailed information per item.
#[derive(Debug, Clone)]
pub struct BatchCheckResult {
    /// Results in the same order as the input checks.
    pub results: Vec<bool>,
    /// Per-item decisions with metadata (if detailed mode was used).
    pub decisions: Option<Vec<Decision>>,
    /// Consistency token from this operation.
    pub consistency_token: Option<ConsistencyToken>,
}

impl BatchCheckResult {
    /// Returns the results as a slice.
    pub fn as_slice(&self) -> &[bool] {
        &self.results
    }

    /// Returns an iterator over the results.
    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.results.iter().copied()
    }

    /// Returns the number of results.
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns `true` if there are no results.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns `true` if all checks were allowed.
    pub fn all_allowed(&self) -> bool {
        self.results.iter().all(|&r| r)
    }

    /// Returns `true` if any check was allowed.
    pub fn any_allowed(&self) -> bool {
        self.results.iter().any(|&r| r)
    }

    /// Returns the indices of denied checks.
    pub fn denied_indices(&self) -> Vec<usize> {
        self.results
            .iter()
            .enumerate()
            .filter_map(|(i, &allowed)| if !allowed { Some(i) } else { None })
            .collect()
    }
}

/// Client for managing relationships in a vault.
///
/// Obtained via [`VaultClient::relationships()`].
#[derive(Clone)]
pub struct RelationshipsClient {
    vault: VaultClient,
}

impl RelationshipsClient {
    pub(crate) fn new(vault: VaultClient) -> Self {
        Self { vault }
    }

    /// Writes a relationship to the vault.
    ///
    /// Returns a consistency token that can be used to ensure subsequent
    /// reads see this write.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Relationship;
    ///
    /// let token = vault.relationships()
    ///     .write(Relationship::new("doc:1", "viewer", "user:alice"))
    ///     .await?;
    ///
    /// // Use token for consistent reads
    /// let allowed = vault.check("user:alice", "view", "doc:1")
    ///     .at_least_as_fresh(token)
    ///     .await?;
    /// ```
    pub fn write<'a>(&self, relationship: Relationship<'a>) -> WriteRelationshipRequest<'a> {
        WriteRelationshipRequest {
            client: self.clone(),
            relationship,
        }
    }

    /// Writes multiple relationships in a single batch.
    ///
    /// This is more efficient than individual writes when you need to
    /// create many relationships at once.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Relationship;
    ///
    /// let relationships = vec![
    ///     Relationship::new("doc:1", "viewer", "user:alice"),
    ///     Relationship::new("doc:1", "editor", "user:bob"),
    /// ];
    ///
    /// let token = vault.relationships()
    ///     .write_batch(relationships)
    ///     .await?;
    /// ```
    pub fn write_batch<'a, I>(&self, relationships: I) -> WriteBatchRequest<'a>
    where
        I: IntoIterator<Item = Relationship<'a>>,
    {
        WriteBatchRequest {
            client: self.clone(),
            relationships: relationships.into_iter().collect(),
        }
    }

    /// Deletes a relationship from the vault.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Relationship;
    ///
    /// vault.relationships()
    ///     .delete(Relationship::new("doc:1", "viewer", "user:alice"))
    ///     .await?;
    /// ```
    pub fn delete<'a>(&self, relationship: Relationship<'a>) -> DeleteRelationshipRequest<'a> {
        DeleteRelationshipRequest {
            client: self.clone(),
            relationship,
        }
    }

    /// Lists relationships in the vault with optional filters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // List all relationships for a resource
    /// let rels = vault.relationships()
    ///     .list()
    ///     .resource("doc:1")
    ///     .await?;
    ///
    /// // List all relationships with a specific relation
    /// let viewers = vault.relationships()
    ///     .list()
    ///     .resource("doc:1")
    ///     .relation("viewer")
    ///     .await?;
    /// ```
    pub fn list(&self) -> ListRelationshipsRequest {
        ListRelationshipsRequest {
            client: self.clone(),
            resource: None,
            relation: None,
            subject: None,
            limit: None,
            cursor: None,
        }
    }
}

impl std::fmt::Debug for RelationshipsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelationshipsClient")
            .field("vault_id", &self.vault.vault_id)
            .finish_non_exhaustive()
    }
}

/// Request to write a single relationship.
pub struct WriteRelationshipRequest<'a> {
    client: RelationshipsClient,
    relationship: Relationship<'a>,
}

impl<'a> WriteRelationshipRequest<'a> {
    async fn execute(self) -> Result<ConsistencyToken, Error> {
        // TODO: Implement actual API call
        Ok(ConsistencyToken::new(format!(
            "token_{}",
            uuid::Uuid::new_v4()
        )))
    }
}

impl<'a> std::future::IntoFuture for WriteRelationshipRequest<'a> {
    type Output = Result<ConsistencyToken, Error>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to write multiple relationships.
pub struct WriteBatchRequest<'a> {
    client: RelationshipsClient,
    relationships: Vec<Relationship<'a>>,
}

impl<'a> WriteBatchRequest<'a> {
    /// Returns the number of relationships in this batch.
    pub fn len(&self) -> usize {
        self.relationships.len()
    }

    /// Returns `true` if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }

    async fn execute(self) -> Result<ConsistencyToken, Error> {
        // TODO: Implement actual API call
        Ok(ConsistencyToken::new(format!(
            "token_{}",
            uuid::Uuid::new_v4()
        )))
    }
}

impl<'a> std::future::IntoFuture for WriteBatchRequest<'a> {
    type Output = Result<ConsistencyToken, Error>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to delete a relationship.
pub struct DeleteRelationshipRequest<'a> {
    client: RelationshipsClient,
    relationship: Relationship<'a>,
}

impl<'a> DeleteRelationshipRequest<'a> {
    async fn execute(self) -> Result<(), Error> {
        // TODO: Implement actual API call
        Ok(())
    }
}

impl<'a> std::future::IntoFuture for DeleteRelationshipRequest<'a> {
    type Output = Result<(), Error>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Request to list relationships.
pub struct ListRelationshipsRequest {
    client: RelationshipsClient,
    resource: Option<String>,
    relation: Option<String>,
    subject: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

impl ListRelationshipsRequest {
    /// Filters by resource.
    #[must_use]
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Filters by relation.
    #[must_use]
    pub fn relation(mut self, relation: impl Into<String>) -> Self {
        self.relation = Some(relation.into());
        self
    }

    /// Filters by subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Sets the maximum number of results to return.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the pagination cursor for fetching the next page.
    #[must_use]
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    async fn execute(self) -> Result<ListRelationshipsResponse, Error> {
        // TODO: Implement actual API call
        Ok(ListRelationshipsResponse {
            relationships: vec![],
            next_cursor: None,
        })
    }
}

impl std::future::IntoFuture for ListRelationshipsRequest {
    type Output = Result<ListRelationshipsResponse, Error>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Response from listing relationships.
#[derive(Debug, Clone)]
pub struct ListRelationshipsResponse {
    /// The relationships matching the query.
    pub relationships: Vec<Relationship<'static>>,
    /// Cursor for fetching the next page, if any.
    pub next_cursor: Option<String>,
}

impl ListRelationshipsResponse {
    /// Returns `true` if there are more results.
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }

    /// Returns an iterator over the relationships.
    pub fn iter(&self) -> impl Iterator<Item = &Relationship<'static>> {
        self.relationships.iter()
    }
}

impl IntoIterator for ListRelationshipsResponse {
    type Item = Relationship<'static>;
    type IntoIter = std::vec::IntoIter<Relationship<'static>>;

    fn into_iter(self) -> Self::IntoIter {
        self.relationships.into_iter()
    }
}

// ============================================================================
// ResourcesClient
// ============================================================================

/// Sub-client for resource queries.
///
/// Obtained via [`VaultClient::resources()`].
///
/// # Example
///
/// ```rust,ignore
/// // Find all documents Alice can view
/// let docs = vault.resources()
///     .accessible_by("user:alice")
///     .with_permission("view")
///     .resource_type("document")
///     .collect()
///     .await?;
/// ```
pub struct ResourcesClient<'a> {
    vault: &'a VaultClient,
}

impl<'a> ResourcesClient<'a> {
    /// Creates a new ResourcesClient.
    fn new(vault: &'a VaultClient) -> Self {
        Self { vault }
    }

    /// Start a query for resources accessible by a subject.
    ///
    /// Returns a builder that must be further configured with `.with_permission()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Find all documents Alice can view
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .resource_type("document")
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn accessible_by(self, subject: impl Into<Cow<'a, str>>) -> ResourcesQueryBuilder<'a> {
        ResourcesQueryBuilder {
            vault: self.vault,
            subject: subject.into(),
        }
    }
}

/// Builder for resource queries - requires subject and permission.
///
/// Created by [`ResourcesClient::accessible_by()`].
pub struct ResourcesQueryBuilder<'a> {
    vault: &'a VaultClient,
    subject: Cow<'a, str>,
}

impl<'a> ResourcesQueryBuilder<'a> {
    /// Specify the permission to check (required).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")  // Required
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn with_permission(self, permission: impl Into<Cow<'a, str>>) -> ResourcesListBuilder<'a> {
        ResourcesListBuilder {
            vault: self.vault,
            subject: self.subject,
            permission: permission.into(),
            resource_type: None,
            consistency: None,
            page_size: None,
        }
    }
}

/// Builder for resource list queries (after subject and permission are set).
///
/// Created by [`ResourcesQueryBuilder::with_permission()`].
pub struct ResourcesListBuilder<'a> {
    vault: &'a VaultClient,
    subject: Cow<'a, str>,
    permission: Cow<'a, str>,
    resource_type: Option<Cow<'a, str>>,
    consistency: Option<ConsistencyToken>,
    page_size: Option<u32>,
}

impl<'a> ResourcesListBuilder<'a> {
    /// Filter by resource type (e.g., "document", "folder").
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .resource_type("document")  // Only documents
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn resource_type(mut self, resource_type: impl Into<Cow<'a, str>>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Ensure read consistency with a previously obtained token.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .at_least_as_fresh_as(token)
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Set page size for pagination.
    #[must_use]
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Limit results to first N items.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .take(100)
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn take(self, n: usize) -> ResourcesListTake<'a> {
        ResourcesListTake {
            inner: self,
            limit: n,
        }
    }

    /// Collect all results into a Vec.
    ///
    /// Use with caution for large result sets - consider using `.stream()` or `.take()` instead.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let docs = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .collect()
    ///     .await?;
    /// ```
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        // TODO: Implement actual API call via transport layer
        let _ = (
            self.vault,
            self.subject,
            self.permission,
            self.resource_type,
            self.consistency,
            self.page_size,
        );
        Ok(Vec::new())
    }

    /// Get paginated results with cursor.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let page = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .cursor(None)
    ///     .await?;
    ///
    /// // Get next page
    /// let next_page = vault.resources()
    ///     .accessible_by("user:alice")
    ///     .with_permission("view")
    ///     .cursor(page.next_cursor.as_deref())
    ///     .await?;
    /// ```
    pub async fn cursor(self, _cursor: Option<&str>) -> Result<ResourcesPage, Error> {
        // TODO: Implement actual API call via transport layer
        let _ = (
            self.vault,
            self.subject,
            self.permission,
            self.resource_type,
            self.consistency,
            self.page_size,
        );
        Ok(ResourcesPage {
            resources: Vec::new(),
            next_cursor: None,
        })
    }
}

/// Builder wrapper that limits results to first N items.
///
/// Created by calling `.take(n)` on a [`ResourcesListBuilder`].
pub struct ResourcesListTake<'a> {
    inner: ResourcesListBuilder<'a>,
    limit: usize,
}

impl<'a> ResourcesListTake<'a> {
    /// Collect limited results into a Vec.
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        let results = self.inner.collect().await?;
        Ok(results.into_iter().take(self.limit).collect())
    }
}

/// A page of resource query results.
#[derive(Debug, Clone)]
pub struct ResourcesPage {
    /// The resources in this page.
    pub resources: Vec<String>,
    /// Cursor for fetching the next page, if any.
    pub next_cursor: Option<String>,
}

impl ResourcesPage {
    /// Returns `true` if there are more results.
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }

    /// Returns an iterator over the resources.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.resources.iter().map(|s| s.as_str())
    }
}

impl IntoIterator for ResourcesPage {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.resources.into_iter()
    }
}

// ============================================================================
// SubjectsClient
// ============================================================================

/// Sub-client for subject queries.
///
/// Obtained via [`VaultClient::subjects()`].
///
/// # Example
///
/// ```rust,ignore
/// // Find all users who can edit this document
/// let editors = vault.subjects()
///     .with_permission("edit")
///     .on_resource("document:readme")
///     .subject_type("user")
///     .collect()
///     .await?;
/// ```
pub struct SubjectsClient<'a> {
    vault: &'a VaultClient,
}

impl<'a> SubjectsClient<'a> {
    /// Creates a new SubjectsClient.
    fn new(vault: &'a VaultClient) -> Self {
        Self { vault }
    }

    /// Start a query for subjects with a given permission.
    ///
    /// Returns a builder that must be further configured with `.on_resource()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Find all users who can edit this document
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .subject_type("user")
    ///     .collect()
    ///     .await?;
    /// ```
    pub fn with_permission(self, permission: impl Into<Cow<'a, str>>) -> SubjectsQueryBuilder<'a> {
        SubjectsQueryBuilder {
            vault: self.vault,
            permission: permission.into(),
        }
    }
}

/// Builder for subject queries - requires permission and resource.
///
/// Created by [`SubjectsClient::with_permission()`].
pub struct SubjectsQueryBuilder<'a> {
    vault: &'a VaultClient,
    permission: Cow<'a, str>,
}

impl<'a> SubjectsQueryBuilder<'a> {
    /// Specify the resource to check (required).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")  // Required
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn on_resource(self, resource: impl Into<Cow<'a, str>>) -> SubjectsListBuilder<'a> {
        SubjectsListBuilder {
            vault: self.vault,
            permission: self.permission,
            resource: resource.into(),
            subject_type: None,
            consistency: None,
            page_size: None,
        }
    }
}

/// Builder for subject list queries (after permission and resource are set).
///
/// Created by [`SubjectsQueryBuilder::on_resource()`].
pub struct SubjectsListBuilder<'a> {
    vault: &'a VaultClient,
    permission: Cow<'a, str>,
    resource: Cow<'a, str>,
    subject_type: Option<Cow<'a, str>>,
    consistency: Option<ConsistencyToken>,
    page_size: Option<u32>,
}

impl<'a> SubjectsListBuilder<'a> {
    /// Filter by subject type (e.g., "user", "group", "service").
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .subject_type("user")  // Only users
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn subject_type(mut self, subject_type: impl Into<Cow<'a, str>>) -> Self {
        self.subject_type = Some(subject_type.into());
        self
    }

    /// Ensure read consistency with a previously obtained token.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .at_least_as_fresh_as(token)
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn at_least_as_fresh_as(mut self, token: ConsistencyToken) -> Self {
        self.consistency = Some(token);
        self
    }

    /// Set page size for pagination.
    #[must_use]
    pub fn page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Limit results to first N items.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .take(100)
    ///     .collect()
    ///     .await?;
    /// ```
    #[must_use]
    pub fn take(self, n: usize) -> SubjectsListTake<'a> {
        SubjectsListTake {
            inner: self,
            limit: n,
        }
    }

    /// Collect all results into a Vec.
    ///
    /// Use with caution for large result sets - consider using `.stream()` or `.take()` instead.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let editors = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .collect()
    ///     .await?;
    /// ```
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        // TODO: Implement actual API call via transport layer
        let _ = (
            self.vault,
            self.permission,
            self.resource,
            self.subject_type,
            self.consistency,
            self.page_size,
        );
        Ok(Vec::new())
    }

    /// Get paginated results with cursor.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let page = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .cursor(None)
    ///     .await?;
    ///
    /// // Get next page
    /// let next_page = vault.subjects()
    ///     .with_permission("edit")
    ///     .on_resource("document:readme")
    ///     .cursor(page.next_cursor.as_deref())
    ///     .await?;
    /// ```
    pub async fn cursor(self, _cursor: Option<&str>) -> Result<SubjectsPage, Error> {
        // TODO: Implement actual API call via transport layer
        let _ = (
            self.vault,
            self.permission,
            self.resource,
            self.subject_type,
            self.consistency,
            self.page_size,
        );
        Ok(SubjectsPage {
            subjects: Vec::new(),
            next_cursor: None,
        })
    }
}

/// Builder wrapper that limits results to first N items.
///
/// Created by calling `.take(n)` on a [`SubjectsListBuilder`].
pub struct SubjectsListTake<'a> {
    inner: SubjectsListBuilder<'a>,
    limit: usize,
}

impl<'a> SubjectsListTake<'a> {
    /// Collect limited results into a Vec.
    pub async fn collect(self) -> Result<Vec<String>, Error> {
        let results = self.inner.collect().await?;
        Ok(results.into_iter().take(self.limit).collect())
    }
}

/// A page of subject query results.
#[derive(Debug, Clone)]
pub struct SubjectsPage {
    /// The subjects in this page.
    pub subjects: Vec<String>,
    /// Cursor for fetching the next page, if any.
    pub next_cursor: Option<String>,
}

impl SubjectsPage {
    /// Returns `true` if there are more results.
    pub fn has_more(&self) -> bool {
        self.next_cursor.is_some()
    }

    /// Returns an iterator over the subjects.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.subjects.iter().map(|s| s.as_str())
    }
}

impl IntoIterator for SubjectsPage {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.subjects.into_iter()
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
        let result = vault.check("user:alice", "view", "doc:1").require().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_with_context() {
        let vault = create_test_vault().await;
        let result = vault
            .check("user:alice", "view", "doc:1")
            .require()
            .with_context(Context::new().with("env", "prod"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_with_consistency() {
        let vault = create_test_vault().await;
        let token = ConsistencyToken::new("test_token");
        let result = vault
            .check("user:alice", "view", "doc:1")
            .require()
            .at_least_as_fresh(token)
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

    #[tokio::test]
    async fn test_vault_client_accessors() {
        let vault = create_test_vault().await;
        assert_eq!(vault.organization_id(), "org_test");
        assert_eq!(vault.vault_id(), "vlt_test");
        let _ = vault.client();
    }

    // BatchCheckItem tests
    #[test]
    fn test_batch_check_item_new() {
        let item = BatchCheckItem::new("user:alice", "view", "doc:1");
        assert_eq!(item.subject(), "user:alice");
        assert_eq!(item.permission(), "view");
        assert_eq!(item.resource(), "doc:1");
    }

    #[test]
    fn test_batch_check_item_debug() {
        let item = BatchCheckItem::new("user:alice", "view", "doc:1");
        let debug = format!("{:?}", item);
        assert!(debug.contains("user:alice"));
    }

    // BatchCheckRequest tests
    #[tokio::test]
    async fn test_check_batch_basic() {
        let vault = create_test_vault().await;
        let checks = vec![
            ("user:alice", "view", "doc:1"),
            ("user:bob", "edit", "doc:2"),
        ];
        let results = vault.check_batch(checks).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_check_batch_with_context() {
        let vault = create_test_vault().await;
        let checks = vec![("user:alice", "view", "doc:1")];
        let results = vault
            .check_batch(checks)
            .with_context(Context::new().with("env", "prod"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_check_batch_with_consistency() {
        let vault = create_test_vault().await;
        let token = ConsistencyToken::new("test_token");
        let checks = vec![("user:alice", "view", "doc:1")];
        let results = vault
            .check_batch(checks)
            .at_least_as_fresh(token)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_check_batch_len_is_empty() {
        let vault = create_test_vault().await;
        let batch = vault.check_batch(vec![("user:alice", "view", "doc:1")]);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        let empty_batch = vault.check_batch(Vec::<(&str, &str, &str)>::new());
        assert_eq!(empty_batch.len(), 0);
        assert!(empty_batch.is_empty());
    }

    // BatchCheckResult tests
    #[test]
    fn test_batch_check_result() {
        let result = BatchCheckResult {
            results: vec![true, false, true],
            decisions: None,
            consistency_token: Some(ConsistencyToken::new("token")),
        };

        assert_eq!(result.as_slice(), &[true, false, true]);
        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());
        assert!(!result.all_allowed());
        assert!(result.any_allowed());
        assert_eq!(result.denied_indices(), vec![1]);

        let items: Vec<_> = result.iter().collect();
        assert_eq!(items, vec![true, false, true]);
    }

    #[test]
    fn test_batch_check_result_all_allowed() {
        let result = BatchCheckResult {
            results: vec![true, true, true],
            decisions: None,
            consistency_token: None,
        };
        assert!(result.all_allowed());
        assert!(result.any_allowed());
        assert!(result.denied_indices().is_empty());
    }

    #[test]
    fn test_batch_check_result_all_denied() {
        let result = BatchCheckResult {
            results: vec![false, false],
            decisions: None,
            consistency_token: None,
        };
        assert!(!result.all_allowed());
        assert!(!result.any_allowed());
        assert_eq!(result.denied_indices(), vec![0, 1]);
    }

    #[test]
    fn test_batch_check_result_empty() {
        let result = BatchCheckResult {
            results: vec![],
            decisions: None,
            consistency_token: None,
        };
        assert!(result.is_empty());
        assert!(result.all_allowed()); // vacuously true
        assert!(!result.any_allowed());
    }

    // RelationshipsClient tests
    #[tokio::test]
    async fn test_relationships_client_debug() {
        let vault = create_test_vault().await;
        let rels = vault.relationships();
        let debug = format!("{:?}", rels);
        assert!(debug.contains("RelationshipsClient"));
    }

    #[tokio::test]
    async fn test_relationships_write() {
        let vault = create_test_vault().await;
        let rel = Relationship::new("doc:1", "viewer", "user:alice");
        let token = vault.relationships().write(rel).await.unwrap();
        assert!(!token.value().is_empty());
    }

    #[tokio::test]
    async fn test_relationships_write_batch() {
        let vault = create_test_vault().await;
        let rels = vec![
            Relationship::new("doc:1", "viewer", "user:alice"),
            Relationship::new("doc:1", "editor", "user:bob"),
        ];
        let batch = vault.relationships().write_batch(rels);
        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
        let token = batch.await.unwrap();
        assert!(!token.value().is_empty());
    }

    #[tokio::test]
    async fn test_relationships_write_batch_empty() {
        let vault = create_test_vault().await;
        let batch = vault
            .relationships()
            .write_batch(Vec::<Relationship>::new());
        assert!(batch.is_empty());
    }

    #[tokio::test]
    async fn test_relationships_delete() {
        let vault = create_test_vault().await;
        let rel = Relationship::new("doc:1", "viewer", "user:alice");
        let result = vault.relationships().delete(rel).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_relationships_list() {
        let vault = create_test_vault().await;
        let response = vault.relationships().list().await.unwrap();
        assert!(response.relationships.is_empty());
    }

    #[tokio::test]
    async fn test_relationships_list_with_filters() {
        let vault = create_test_vault().await;
        let response = vault
            .relationships()
            .list()
            .resource("doc:1")
            .relation("viewer")
            .subject("user:alice")
            .limit(100)
            .cursor("cursor123")
            .await
            .unwrap();
        assert!(response.relationships.is_empty());
    }

    // ListRelationshipsResponse tests
    #[test]
    fn test_list_relationships_response() {
        let response = ListRelationshipsResponse {
            relationships: vec![Relationship::new("doc:1", "viewer", "user:alice").into_owned()],
            next_cursor: Some("cursor123".to_string()),
        };

        assert!(response.has_more());
        assert_eq!(response.iter().count(), 1);

        let items: Vec<_> = response.into_iter().collect();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_list_relationships_response_no_more() {
        let response = ListRelationshipsResponse {
            relationships: vec![],
            next_cursor: None,
        };
        assert!(!response.has_more());
    }

    // ResourcesClient tests
    #[tokio::test]
    async fn test_resources_accessible_by() {
        let vault = create_test_vault().await;
        let resources = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .collect()
            .await
            .unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_resources_with_type() {
        let vault = create_test_vault().await;
        let resources = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .resource_type("document")
            .collect()
            .await
            .unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_resources_with_consistency() {
        let vault = create_test_vault().await;
        let token = ConsistencyToken::new("test_token");
        let resources = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .at_least_as_fresh_as(token)
            .collect()
            .await
            .unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_resources_with_page_size() {
        let vault = create_test_vault().await;
        let resources = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .page_size(50)
            .collect()
            .await
            .unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_resources_take() {
        let vault = create_test_vault().await;
        let resources = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .take(10)
            .collect()
            .await
            .unwrap();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_resources_cursor() {
        let vault = create_test_vault().await;
        let page = vault
            .resources()
            .accessible_by("user:alice")
            .with_permission("view")
            .cursor(None)
            .await
            .unwrap();
        assert!(!page.has_more());
    }

    // ResourcesPage tests
    #[test]
    fn test_resources_page() {
        let page = ResourcesPage {
            resources: vec!["doc:1".to_string(), "doc:2".to_string()],
            next_cursor: Some("cursor".to_string()),
        };

        assert!(page.has_more());
        let items: Vec<_> = page.iter().collect();
        assert_eq!(items, vec!["doc:1", "doc:2"]);

        let owned: Vec<_> = page.into_iter().collect();
        assert_eq!(owned.len(), 2);
    }

    #[test]
    fn test_resources_page_no_more() {
        let page = ResourcesPage {
            resources: vec![],
            next_cursor: None,
        };
        assert!(!page.has_more());
    }

    // SubjectsClient tests
    #[tokio::test]
    async fn test_subjects_with_permission() {
        let vault = create_test_vault().await;
        let subjects = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .collect()
            .await
            .unwrap();
        assert!(subjects.is_empty());
    }

    #[tokio::test]
    async fn test_subjects_with_type() {
        let vault = create_test_vault().await;
        let subjects = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .subject_type("user")
            .collect()
            .await
            .unwrap();
        assert!(subjects.is_empty());
    }

    #[tokio::test]
    async fn test_subjects_with_consistency() {
        let vault = create_test_vault().await;
        let token = ConsistencyToken::new("test_token");
        let subjects = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .at_least_as_fresh_as(token)
            .collect()
            .await
            .unwrap();
        assert!(subjects.is_empty());
    }

    #[tokio::test]
    async fn test_subjects_with_page_size() {
        let vault = create_test_vault().await;
        let subjects = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .page_size(50)
            .collect()
            .await
            .unwrap();
        assert!(subjects.is_empty());
    }

    #[tokio::test]
    async fn test_subjects_take() {
        let vault = create_test_vault().await;
        let subjects = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .take(10)
            .collect()
            .await
            .unwrap();
        assert!(subjects.is_empty());
    }

    #[tokio::test]
    async fn test_subjects_cursor() {
        let vault = create_test_vault().await;
        let page = vault
            .subjects()
            .with_permission("edit")
            .on_resource("doc:1")
            .cursor(None)
            .await
            .unwrap();
        assert!(!page.has_more());
    }

    // SubjectsPage tests
    #[test]
    fn test_subjects_page() {
        let page = SubjectsPage {
            subjects: vec!["user:alice".to_string(), "user:bob".to_string()],
            next_cursor: Some("cursor".to_string()),
        };

        assert!(page.has_more());
        let items: Vec<_> = page.iter().collect();
        assert_eq!(items, vec!["user:alice", "user:bob"]);

        let owned: Vec<_> = page.into_iter().collect();
        assert_eq!(owned.len(), 2);
    }

    #[test]
    fn test_subjects_page_no_more() {
        let page = SubjectsPage {
            subjects: vec![],
            next_cursor: None,
        };
        assert!(!page.has_more());
    }
}
