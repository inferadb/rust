//! Simulate/What-If analysis for authorization decisions.
//!
//! This module provides types for testing hypothetical changes to the
//! relationship graph without actually modifying the data.

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};

use super::explain::PermissionExplanation;
use crate::{Error, types::Relationship};

/// Builder for what-if/simulation queries.
///
/// Simulations allow you to test hypothetical changes to the relationship
/// graph without actually modifying the data. This is useful for:
///
/// - Testing policy changes before deployment
/// - Understanding the impact of adding/removing relationships
/// - Debugging authorization issues
///
/// ## Example
///
/// ```rust,ignore
/// let result = vault
///     .simulate()
///     .add_relationship(Relationship::new("doc:1", "viewer", "user:bob"))
///     .check("user:bob", "view", "doc:1")
///     .await?;
///
/// if result.allowed {
///     println!("If we add this relationship, Bob will have access");
/// }
/// ```
pub struct SimulateBuilder {
    vault: super::VaultClient,
    additions: Vec<Relationship<'static>>,
    removals: Vec<Relationship<'static>>,
}

impl SimulateBuilder {
    /// Creates a new simulation builder.
    pub(crate) fn new(vault: super::VaultClient) -> Self {
        Self { vault, additions: vec![], removals: vec![] }
    }

    /// Adds a hypothetical relationship.
    ///
    /// This relationship will be considered as existing for the simulation,
    /// even though it hasn't been written to the vault.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let result = vault
    ///     .simulate()
    ///     .add_relationship(Relationship::new("doc:1", "viewer", "user:bob"))
    ///     .check("user:bob", "view", "doc:1")
    ///     .await?;
    /// ```
    #[must_use]
    pub fn add_relationship(mut self, relationship: Relationship<'_>) -> Self {
        self.additions.push(relationship.into_owned());
        self
    }

    /// Adds multiple hypothetical relationships.
    #[must_use]
    pub fn add_all<'a>(
        mut self,
        relationships: impl IntoIterator<Item = Relationship<'a>>,
    ) -> Self {
        self.additions.extend(relationships.into_iter().map(|r| r.into_owned()));
        self
    }

    /// Removes a hypothetical relationship.
    ///
    /// This relationship will be considered as not existing for the simulation,
    /// even though it may actually exist in the vault.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// // What if we remove Alice's editor access?
    /// let result = vault
    ///     .simulate()
    ///     .remove_relationship(Relationship::new("doc:1", "editor", "user:alice"))
    ///     .check("user:alice", "edit", "doc:1")
    ///     .await?;
    /// ```
    #[must_use]
    pub fn remove_relationship(mut self, relationship: Relationship<'_>) -> Self {
        self.removals.push(relationship.into_owned());
        self
    }

    /// Removes multiple hypothetical relationships.
    #[must_use]
    pub fn remove_all<'a>(
        mut self,
        relationships: impl IntoIterator<Item = Relationship<'a>>,
    ) -> Self {
        self.removals.extend(relationships.into_iter().map(|r| r.into_owned()));
        self
    }

    /// Performs a simulated permission check.
    ///
    /// Returns a `SimulateCheckBuilder` that can be awaited to get the result.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let result = vault
    ///     .simulate()
    ///     .add(Relationship::new("doc:1", "viewer", "user:bob"))
    ///     .check("user:bob", "view", "doc:1")
    ///     .await?;
    /// ```
    pub fn check(
        self,
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> SimulateCheckBuilder {
        SimulateCheckBuilder {
            vault: self.vault,
            additions: self.additions,
            removals: self.removals,
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
        }
    }

    /// Compares the hypothetical state with the current state.
    ///
    /// This runs both a regular check and a simulated check, returning
    /// a diff that shows how the outcome would change.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let diff = vault
    ///     .simulate()
    ///     .add_relationship(Relationship::new("doc:1", "viewer", "user:bob"))
    ///     .compare("user:bob", "view", "doc:1")
    ///     .await?;
    ///
    /// match diff.change {
    ///     SimulationChange::NoChange => println!("No impact"),
    ///     SimulationChange::NowAllowed => println!("Access would be granted"),
    ///     SimulationChange::NowDenied => println!("Access would be revoked"),
    /// }
    /// ```
    pub fn compare(
        self,
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> SimulateCompareBuilder {
        SimulateCompareBuilder {
            vault: self.vault,
            additions: self.additions,
            removals: self.removals,
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
        }
    }
}

/// Builder for simulated permission checks.
pub struct SimulateCheckBuilder {
    #[cfg_attr(not(feature = "rest"), allow(dead_code))]
    vault: super::VaultClient,
    additions: Vec<Relationship<'static>>,
    removals: Vec<Relationship<'static>>,
    subject: String,
    permission: String,
    resource: String,
}

impl SimulateCheckBuilder {
    async fn execute(self) -> Result<SimulationResult, Error> {
        #[cfg(feature = "rest")]
        if let Some(transport) = self.vault.transport() {
            use crate::transport::TransportSimulateRequest;

            let request = TransportSimulateRequest {
                subject: self.subject.clone(),
                permission: self.permission.clone(),
                resource: self.resource.clone(),
                context: None,
                additions: self.additions.clone(),
                removals: self.removals.clone(),
            };

            let response = transport.simulate(request).await?;

            return Ok(SimulationResult {
                allowed: response.allowed,
                subject: self.subject,
                permission: self.permission,
                resource: self.resource,
                hypothetical_additions: self.additions.iter().map(|r| r.to_string()).collect(),
                hypothetical_removals: self.removals.iter().map(|r| r.to_string()).collect(),
                explanation: None,
            });
        }

        // Fallback when transport is not available
        Ok(SimulationResult {
            allowed: false,
            subject: self.subject,
            permission: self.permission,
            resource: self.resource,
            hypothetical_additions: self.additions.iter().map(|r| r.to_string()).collect(),
            hypothetical_removals: self.removals.iter().map(|r| r.to_string()).collect(),
            explanation: None,
        })
    }
}

/// Enables ergonomic `.await` without explicit `.build()`.
///
/// This `IntoFuture` implementation is intentionally manual (not derived via `bon`)
/// to preserve the ergonomic async API: `vault.simulate()...check(...).await`
/// instead of `vault.simulate()...check(...).build().await`.
impl std::future::IntoFuture for SimulateCheckBuilder {
    type Output = Result<SimulationResult, Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Builder for comparing simulated state with current state.
pub struct SimulateCompareBuilder {
    #[cfg_attr(not(feature = "rest"), allow(dead_code))]
    vault: super::VaultClient,
    additions: Vec<Relationship<'static>>,
    removals: Vec<Relationship<'static>>,
    subject: String,
    permission: String,
    resource: String,
}

impl SimulateCompareBuilder {
    async fn execute(self) -> Result<SimulationDiff, Error> {
        #[cfg(feature = "rest")]
        if let Some(transport) = self.vault.transport() {
            use crate::transport::{TransportCheckRequest, TransportSimulateRequest};

            // Run current check (without hypothetical changes)
            let current_request = TransportCheckRequest {
                subject: self.subject.clone(),
                permission: self.permission.clone(),
                resource: self.resource.clone(),
                context: None,
                consistency: None,
                trace: false,
            };
            let current_response = transport.check(current_request).await?;
            let current_allowed = current_response.allowed;

            // Run simulated check (with hypothetical changes)
            let simulate_request = TransportSimulateRequest {
                subject: self.subject.clone(),
                permission: self.permission.clone(),
                resource: self.resource.clone(),
                context: None,
                additions: self.additions.clone(),
                removals: self.removals.clone(),
            };
            let simulated_response = transport.simulate(simulate_request).await?;
            let simulated_allowed = simulated_response.allowed;

            let change = match (current_allowed, simulated_allowed) {
                (true, true) | (false, false) => SimulationChange::NoChange,
                (false, true) => SimulationChange::NowAllowed,
                (true, false) => SimulationChange::NowDenied,
            };

            return Ok(SimulationDiff {
                subject: self.subject,
                permission: self.permission,
                resource: self.resource,
                current_allowed,
                simulated_allowed,
                change,
                hypothetical_additions: self.additions.iter().map(|r| r.to_string()).collect(),
                hypothetical_removals: self.removals.iter().map(|r| r.to_string()).collect(),
            });
        }

        // Fallback when transport is not available
        Ok(SimulationDiff {
            subject: self.subject,
            permission: self.permission,
            resource: self.resource,
            current_allowed: false,
            simulated_allowed: false,
            change: SimulationChange::NoChange,
            hypothetical_additions: self.additions.iter().map(|r| r.to_string()).collect(),
            hypothetical_removals: self.removals.iter().map(|r| r.to_string()).collect(),
        })
    }
}

/// Enables ergonomic `.await` without explicit `.build()`.
///
/// This `IntoFuture` implementation is intentionally manual (not derived via `bon`)
/// to preserve the ergonomic async API: `vault.simulate()...compare(...).await`
/// instead of `vault.simulate()...compare(...).build().await`.
impl std::future::IntoFuture for SimulateCompareBuilder {
    type Output = Result<SimulationDiff, Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Result of a simulated permission check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Whether the permission would be allowed in the simulated state.
    pub allowed: bool,
    /// The subject that was checked.
    pub subject: String,
    /// The permission that was checked.
    pub permission: String,
    /// The resource that was checked.
    pub resource: String,
    /// Relationships that were hypothetically added (as strings).
    pub hypothetical_additions: Vec<String>,
    /// Relationships that were hypothetically removed (as strings).
    pub hypothetical_removals: Vec<String>,
    /// Optional detailed explanation of the decision.
    pub explanation: Option<PermissionExplanation>,
}

impl SimulationResult {
    /// Returns a summary of the simulation result.
    pub fn summary(&self) -> String {
        let changes =
            if self.hypothetical_additions.is_empty() && self.hypothetical_removals.is_empty() {
                "with no changes".to_string()
            } else {
                let mut parts = vec![];
                if !self.hypothetical_additions.is_empty() {
                    parts.push(format!("+{} relationships", self.hypothetical_additions.len()));
                }
                if !self.hypothetical_removals.is_empty() {
                    parts.push(format!("-{} relationships", self.hypothetical_removals.len()));
                }
                format!("with {}", parts.join(", "))
            };

        if self.allowed {
            format!(
                "{} would have {} on {} {}",
                self.subject, self.permission, self.resource, changes
            )
        } else {
            format!(
                "{} would NOT have {} on {} {}",
                self.subject, self.permission, self.resource, changes
            )
        }
    }
}

impl std::fmt::Display for SimulationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation Result")?;
        writeln!(f, "=================")?;
        writeln!(f, "Subject:    {}", self.subject)?;
        writeln!(f, "Permission: {}", self.permission)?;
        writeln!(f, "Resource:   {}", self.resource)?;
        writeln!(f, "Result:     {}", if self.allowed { "ALLOWED" } else { "DENIED" })?;

        if !self.hypothetical_additions.is_empty() {
            writeln!(f, "\nHypothetical Additions:")?;
            for rel in &self.hypothetical_additions {
                writeln!(f, "  + {}", rel)?;
            }
        }

        if !self.hypothetical_removals.is_empty() {
            writeln!(f, "\nHypothetical Removals:")?;
            for rel in &self.hypothetical_removals {
                writeln!(f, "  - {}", rel)?;
            }
        }

        Ok(())
    }
}

/// Comparison between current and simulated states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationDiff {
    /// The subject that was checked.
    pub subject: String,
    /// The permission that was checked.
    pub permission: String,
    /// The resource that was checked.
    pub resource: String,
    /// Whether the permission is currently allowed.
    pub current_allowed: bool,
    /// Whether the permission would be allowed in the simulated state.
    pub simulated_allowed: bool,
    /// The change in authorization status.
    pub change: SimulationChange,
    /// Relationships that were hypothetically added (as strings).
    pub hypothetical_additions: Vec<String>,
    /// Relationships that were hypothetically removed (as strings).
    pub hypothetical_removals: Vec<String>,
}

impl SimulationDiff {
    /// Returns `true` if the simulation would result in a change.
    pub fn has_change(&self) -> bool {
        !matches!(self.change, SimulationChange::NoChange)
    }

    /// Returns a summary of the diff.
    pub fn summary(&self) -> String {
        match self.change {
            SimulationChange::NoChange => format!(
                "No change: {} {} {} on {}",
                self.subject,
                if self.current_allowed { "can" } else { "cannot" },
                self.permission,
                self.resource
            ),
            SimulationChange::NowAllowed => format!(
                "Change: {} would GAIN {} on {}",
                self.subject, self.permission, self.resource
            ),
            SimulationChange::NowDenied => format!(
                "Change: {} would LOSE {} on {}",
                self.subject, self.permission, self.resource
            ),
        }
    }
}

impl std::fmt::Display for SimulationDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Simulation Comparison")?;
        writeln!(f, "=====================")?;
        writeln!(f, "Subject:    {}", self.subject)?;
        writeln!(f, "Permission: {}", self.permission)?;
        writeln!(f, "Resource:   {}", self.resource)?;
        writeln!(f, "Current:    {}", if self.current_allowed { "ALLOWED" } else { "DENIED" })?;
        writeln!(f, "Simulated:  {}", if self.simulated_allowed { "ALLOWED" } else { "DENIED" })?;
        writeln!(f, "Change:     {}", self.change)?;

        if !self.hypothetical_additions.is_empty() {
            writeln!(f, "\nHypothetical Additions:")?;
            for rel in &self.hypothetical_additions {
                writeln!(f, "  + {}", rel)?;
            }
        }

        if !self.hypothetical_removals.is_empty() {
            writeln!(f, "\nHypothetical Removals:")?;
            for rel in &self.hypothetical_removals {
                writeln!(f, "  - {}", rel)?;
            }
        }

        Ok(())
    }
}

/// The change in authorization status between current and simulated states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationChange {
    /// No change in authorization status.
    NoChange,
    /// The subject would now be allowed (currently denied).
    NowAllowed,
    /// The subject would now be denied (currently allowed).
    NowDenied,
}

impl std::fmt::Display for SimulationChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulationChange::NoChange => write!(f, "no change"),
            SimulationChange::NowAllowed => write!(f, "now allowed"),
            SimulationChange::NowDenied => write!(f, "now denied"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_result_summary_no_changes() {
        let result = SimulationResult {
            allowed: true,
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
            explanation: None,
        };

        let summary = result.summary();
        assert!(summary.contains("user:alice"));
        assert!(summary.contains("view"));
        assert!(summary.contains("doc:1"));
        assert!(summary.contains("with no changes"));
    }

    #[test]
    fn test_simulation_result_summary_with_additions() {
        let result = SimulationResult {
            allowed: true,
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec!["doc:1#viewer@user:alice".to_string()],
            hypothetical_removals: vec![],
            explanation: None,
        };

        let summary = result.summary();
        assert!(summary.contains("+1 relationships"));
    }

    #[test]
    fn test_simulation_result_display() {
        let result = SimulationResult {
            allowed: false,
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec!["doc:1#viewer@user:alice".to_string()],
            hypothetical_removals: vec![],
            explanation: None,
        };

        let display = format!("{}", result);
        assert!(display.contains("Simulation Result"));
        assert!(display.contains("DENIED"));
        assert!(display.contains("Hypothetical Additions"));
    }

    #[test]
    fn test_simulation_diff_has_change() {
        let no_change = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: true,
            simulated_allowed: true,
            change: SimulationChange::NoChange,
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
        };
        assert!(!no_change.has_change());

        let now_allowed = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: false,
            simulated_allowed: true,
            change: SimulationChange::NowAllowed,
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
        };
        assert!(now_allowed.has_change());
    }

    #[test]
    fn test_simulation_diff_summary() {
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: false,
            simulated_allowed: true,
            change: SimulationChange::NowAllowed,
            hypothetical_additions: vec!["doc:1#editor@user:alice".to_string()],
            hypothetical_removals: vec![],
        };

        let summary = diff.summary();
        assert!(summary.contains("GAIN"));
        assert!(summary.contains("user:alice"));
        assert!(summary.contains("edit"));
    }

    #[test]
    fn test_simulation_change_display() {
        assert_eq!(SimulationChange::NoChange.to_string(), "no change");
        assert_eq!(SimulationChange::NowAllowed.to_string(), "now allowed");
        assert_eq!(SimulationChange::NowDenied.to_string(), "now denied");
    }

    #[test]
    fn test_simulation_change_serialization() {
        let json = serde_json::to_string(&SimulationChange::NowAllowed).unwrap();
        assert_eq!(json, "\"now_allowed\"");

        let parsed: SimulationChange = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SimulationChange::NowAllowed);
    }

    #[test]
    fn test_simulation_result_summary_with_removals() {
        let result = SimulationResult {
            allowed: false,
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec![],
            hypothetical_removals: vec!["doc:1#editor@user:alice".to_string()],
            explanation: None,
        };

        let summary = result.summary();
        assert!(summary.contains("-1 relationships"));
    }

    #[test]
    fn test_simulation_result_summary_with_both() {
        let result = SimulationResult {
            allowed: true,
            subject: "user:bob".to_string(),
            permission: "view".to_string(),
            resource: "doc:2".to_string(),
            hypothetical_additions: vec!["doc:2#viewer@user:bob".to_string()],
            hypothetical_removals: vec!["doc:2#editor@user:alice".to_string()],
            explanation: None,
        };

        let summary = result.summary();
        assert!(summary.contains("+1 relationships"));
        assert!(summary.contains("-1 relationships"));
    }

    #[test]
    fn test_simulation_result_display_allowed() {
        let result = SimulationResult {
            allowed: true,
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
            explanation: None,
        };

        let display = format!("{}", result);
        assert!(display.contains("ALLOWED"));
    }

    #[test]
    fn test_simulation_result_display_with_removals() {
        let result = SimulationResult {
            allowed: false,
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec![],
            hypothetical_removals: vec![
                "doc:1#editor@user:alice".to_string(),
                "doc:1#owner@user:alice".to_string(),
            ],
            explanation: None,
        };

        let display = format!("{}", result);
        assert!(display.contains("Hypothetical Removals"));
    }

    #[test]
    fn test_simulation_diff_now_denied() {
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: true,
            simulated_allowed: false,
            change: SimulationChange::NowDenied,
            hypothetical_additions: vec![],
            hypothetical_removals: vec!["doc:1#editor@user:alice".to_string()],
        };

        assert!(diff.has_change());
        let summary = diff.summary();
        assert!(summary.contains("LOSE"));
    }

    #[test]
    fn test_simulation_diff_display() {
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: false,
            simulated_allowed: true,
            change: SimulationChange::NowAllowed,
            hypothetical_additions: vec!["doc:1#viewer@user:alice".to_string()],
            hypothetical_removals: vec![],
        };

        let display = format!("{}", diff);
        assert!(display.contains("Simulation Comparison"));
        assert!(display.contains("Current:"));
        assert!(display.contains("Simulated:"));
        assert!(display.contains("Change:"));
    }

    #[test]
    fn test_simulation_diff_display_with_removals() {
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: true,
            simulated_allowed: false,
            change: SimulationChange::NowDenied,
            hypothetical_additions: vec![],
            hypothetical_removals: vec!["doc:1#editor@user:alice".to_string()],
        };

        let display = format!("{}", diff);
        assert!(display.contains("Hypothetical Removals"));
    }

    #[test]
    fn test_simulation_result_serialization() {
        let result = SimulationResult {
            allowed: true,
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            hypothetical_additions: vec!["doc:1#viewer@user:alice".to_string()],
            hypothetical_removals: vec![],
            explanation: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"subject\":\"user:alice\""));

        let parsed: SimulationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.allowed, result.allowed);
        assert_eq!(parsed.subject, result.subject);
    }

    #[test]
    fn test_simulation_diff_serialization() {
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: false,
            simulated_allowed: true,
            change: SimulationChange::NowAllowed,
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
        };

        let json = serde_json::to_string(&diff).unwrap();
        assert!(json.contains("\"change\":\"now_allowed\""));

        let parsed: SimulationDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.change, SimulationChange::NowAllowed);
    }

    #[test]
    fn test_simulation_change_all_variants() {
        // Test all three variants
        let changes =
            [SimulationChange::NoChange, SimulationChange::NowAllowed, SimulationChange::NowDenied];

        for change in changes {
            let json = serde_json::to_string(&change).unwrap();
            let parsed: SimulationChange = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, change);
        }
    }

    // Tests for SimulateBuilder
    use std::sync::Arc;

    use crate::{auth::BearerCredentialsConfig, client::Client, transport::mock::MockTransport};

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_builder_add_relationship() {
        let mock_transport = Arc::new(MockTransport::new().into_any());
        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");
        let result = vault
            .simulate()
            .add_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .check("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        // Adding the relationship should grant access
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_simulate_builder_remove_relationship() {
        // Add existing relationship before wrapping
        let mock = MockTransport::new();
        mock.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        let mock_transport = Arc::new(mock.into_any());

        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");
        let result = vault
            .simulate()
            .remove_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .check("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        // Removing the relationship should deny access
        assert!(!result.allowed);
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_builder_add_all() {
        let mock_transport = Arc::new(MockTransport::new().into_any());
        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");
        let relationships = vec![
            Relationship::new("doc:1", "viewer", "user:alice"),
            Relationship::new("doc:2", "viewer", "user:bob"),
        ];

        let result = vault
            .simulate()
            .add_all(relationships)
            .check("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_simulate_builder_remove_all() {
        let mock = MockTransport::new();
        mock.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        mock.add_relationship(Relationship::new("doc:2", "viewer", "user:bob").into_owned());
        let mock_transport = Arc::new(mock.into_any());

        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");
        let relationships = vec![
            Relationship::new("doc:1", "viewer", "user:alice"),
            Relationship::new("doc:2", "viewer", "user:bob"),
        ];

        let result = vault
            .simulate()
            .remove_all(relationships)
            .check("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        assert!(!result.allowed);
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_builder_compare() {
        let mock_transport = Arc::new(MockTransport::new().into_any());
        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Compare: what if we add viewer relationship
        let diff = vault
            .simulate()
            .add_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .compare("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        // Should show that access would be granted
        assert_eq!(diff.change, SimulationChange::NowAllowed);
        assert!(!diff.current_allowed);
        assert!(diff.simulated_allowed);
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_builder_chaining() {
        // Add some initial relationships before wrapping
        let mock = MockTransport::new();
        mock.add_relationship(Relationship::new("doc:2", "viewer", "user:charlie").into_owned());
        let mock_transport = Arc::new(mock.into_any());

        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Chain multiple operations
        let result = vault
            .simulate()
            .add_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .add_relationship(Relationship::new("doc:1", "editor", "user:bob"))
            .remove_relationship(Relationship::new("doc:2", "viewer", "user:charlie"))
            .check("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        assert!(result.allowed);
        assert!(!result.hypothetical_additions.is_empty());
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_compare_now_denied() {
        // Add existing relationship before wrapping
        let mock = MockTransport::new();
        mock.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        let mock_transport = Arc::new(mock.into_any());

        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Compare: what if we remove viewer relationship
        let diff = vault
            .simulate()
            .remove_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .compare("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        // Should show that access would be revoked
        assert_eq!(diff.change, SimulationChange::NowDenied);
        assert!(diff.current_allowed);
        assert!(!diff.simulated_allowed);
    }

    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_simulate_compare_no_change() {
        // Add existing relationship before wrapping
        let mock = MockTransport::new();
        mock.add_relationship(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        let mock_transport = Arc::new(mock.into_any());

        let client = Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build_with_transport(mock_transport)
            .await
            .unwrap();

        let vault = client.organization("org_test").vault("vlt_test");

        // Compare: adding the same relationship that already exists
        let diff = vault
            .simulate()
            .add_relationship(Relationship::new("doc:1", "viewer", "user:alice"))
            .compare("user:alice", "viewer", "doc:1")
            .await
            .unwrap();

        // Should show no change (already allowed)
        assert_eq!(diff.change, SimulationChange::NoChange);
        assert!(diff.current_allowed);
        assert!(diff.simulated_allowed);
    }

    #[test]
    fn test_simulation_diff_summary_no_change_allowed() {
        // Test NoChange summary when current_allowed is true
        let diff = SimulationDiff {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            current_allowed: true,
            simulated_allowed: true,
            change: SimulationChange::NoChange,
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
        };

        let summary = diff.summary();
        assert!(summary.contains("No change"));
        assert!(summary.contains("can"));
        assert!(summary.contains("view"));
    }

    #[test]
    fn test_simulation_diff_summary_no_change_denied() {
        // Test NoChange summary when current_allowed is false
        let diff = SimulationDiff {
            subject: "user:bob".to_string(),
            permission: "edit".to_string(),
            resource: "doc:2".to_string(),
            current_allowed: false,
            simulated_allowed: false,
            change: SimulationChange::NoChange,
            hypothetical_additions: vec![],
            hypothetical_removals: vec![],
        };

        let summary = diff.summary();
        assert!(summary.contains("No change"));
        assert!(summary.contains("cannot"));
        assert!(summary.contains("edit"));
    }
}
