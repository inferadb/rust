//! Permission explanation types for debugging authorization decisions.
//!
//! This module provides types for understanding why a permission was allowed
//! or denied, including the paths through the relationship graph and
//! suggestions for granting access.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::types::Context;

/// Explains why a permission check resulted in allow or deny.
///
/// The explanation includes:
/// - The paths through the relationship graph that grant access (if any)
/// - Reasons why access was denied (if denied)
/// - Suggestions for how to grant access
///
/// ## Example
///
/// ```rust,ignore
/// let explanation = vault
///     .explain_permission()
///     .subject("user:alice")
///     .permission("edit")
///     .resource("doc:readme")
///     .await?;
///
/// if explanation.allowed {
///     println!("Access granted via {} path(s)", explanation.paths.len());
///     for path in &explanation.paths {
///         println!("  Path: {}", explanation.format_path(path));
///     }
/// } else {
///     println!("Access denied:");
///     for reason in &explanation.denial_reasons {
///         println!("  - {}", reason);
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionExplanation {
    /// Whether the permission check was allowed.
    pub allowed: bool,
    /// The permission that was checked.
    pub permission: String,
    /// The subject that was checked.
    pub subject: String,
    /// The resource that was checked.
    pub resource: String,
    /// Paths through the relationship graph that grant access.
    ///
    /// Each inner `Vec<PathNode>` represents one path from the subject to
    /// the resource. Multiple paths may exist if there are multiple ways
    /// to grant the permission.
    pub paths: Vec<Vec<PathNode>>,
    /// Reasons why access was denied (empty if allowed).
    pub denial_reasons: Vec<DenialReason>,
    /// Suggestions for how to grant access.
    pub suggestions: Vec<AccessSuggestion>,
    /// Time taken to evaluate the permission.
    #[serde(with = "duration_millis")]
    pub evaluation_time: Duration,
    /// Whether the result was served from cache.
    pub cached: bool,
}

impl PermissionExplanation {
    /// Creates a new explanation for an allowed permission.
    pub fn allowed(
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            allowed: true,
            permission: permission.into(),
            subject: subject.into(),
            resource: resource.into(),
            paths: vec![],
            denial_reasons: vec![],
            suggestions: vec![],
            evaluation_time: Duration::ZERO,
            cached: false,
        }
    }

    /// Creates a new explanation for a denied permission.
    pub fn denied(
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        Self {
            allowed: false,
            permission: permission.into(),
            subject: subject.into(),
            resource: resource.into(),
            paths: vec![],
            denial_reasons: vec![],
            suggestions: vec![],
            evaluation_time: Duration::ZERO,
            cached: false,
        }
    }

    /// Adds a path to the explanation.
    #[must_use]
    pub fn with_path(mut self, path: Vec<PathNode>) -> Self {
        self.paths.push(path);
        self
    }

    /// Adds a denial reason to the explanation.
    #[must_use]
    pub fn with_denial_reason(mut self, reason: DenialReason) -> Self {
        self.denial_reasons.push(reason);
        self
    }

    /// Adds a suggestion to the explanation.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: AccessSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Sets the evaluation time.
    #[must_use]
    pub fn with_evaluation_time(mut self, time: Duration) -> Self {
        self.evaluation_time = time;
        self
    }

    /// Marks the result as cached.
    #[must_use]
    pub fn with_cached(mut self, cached: bool) -> Self {
        self.cached = cached;
        self
    }

    /// Formats a path as a human-readable string.
    ///
    /// ## Example Output
    ///
    /// ```text
    /// user:alice -> viewer -> team:engineering -> member -> doc:readme
    /// ```
    pub fn format_path(&self, path: &[PathNode]) -> String {
        if path.is_empty() {
            return String::new();
        }

        path.iter()
            .enumerate()
            .fold(String::new(), |mut acc, (i, node)| {
                if i > 0 {
                    acc.push_str(" -> ");
                }
                acc.push_str(&node.to_string());
                acc
            })
    }

    /// Returns a human-readable summary of the explanation.
    pub fn summary(&self) -> String {
        if self.allowed {
            if self.paths.is_empty() {
                format!(
                    "{} has {} on {} (no path details available)",
                    self.subject, self.permission, self.resource
                )
            } else {
                format!(
                    "{} has {} on {} via {} path(s)",
                    self.subject,
                    self.permission,
                    self.resource,
                    self.paths.len()
                )
            }
        } else if self.denial_reasons.is_empty() {
            format!(
                "{} does not have {} on {}",
                self.subject, self.permission, self.resource
            )
        } else {
            format!(
                "{} does not have {} on {}: {}",
                self.subject,
                self.permission,
                self.resource,
                self.denial_reasons
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl std::fmt::Display for PermissionExplanation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Permission Explanation")?;
        writeln!(f, "======================")?;
        writeln!(f, "Subject:    {}", self.subject)?;
        writeln!(f, "Permission: {}", self.permission)?;
        writeln!(f, "Resource:   {}", self.resource)?;
        writeln!(
            f,
            "Result:     {}",
            if self.allowed { "ALLOWED" } else { "DENIED" }
        )?;
        writeln!(f, "Cached:     {}", self.cached)?;
        writeln!(f, "Time:       {:?}", self.evaluation_time)?;

        if !self.paths.is_empty() {
            writeln!(f, "\nPaths:")?;
            for (i, path) in self.paths.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, self.format_path(path))?;
            }
        }

        if !self.denial_reasons.is_empty() {
            writeln!(f, "\nDenial Reasons:")?;
            for reason in &self.denial_reasons {
                writeln!(f, "  - {}", reason)?;
            }
        }

        if !self.suggestions.is_empty() {
            writeln!(f, "\nSuggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "  - {}", suggestion)?;
            }
        }

        Ok(())
    }
}

/// A node in the permission path.
///
/// Represents a step in the path from subject to resource through
/// the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathNode {
    /// The entity at this node (e.g., "user:alice", "team:engineering").
    pub entity: String,
    /// The relation used to reach this node (e.g., "member", "viewer").
    pub relation: Option<String>,
    /// How this node was derived (for computed relations).
    pub derived_from: Option<String>,
}

impl PathNode {
    /// Creates a new path node.
    pub fn new(entity: impl Into<String>) -> Self {
        Self {
            entity: entity.into(),
            relation: None,
            derived_from: None,
        }
    }

    /// Sets the relation.
    #[must_use]
    pub fn with_relation(mut self, relation: impl Into<String>) -> Self {
        self.relation = Some(relation.into());
        self
    }

    /// Sets the derived_from field.
    #[must_use]
    pub fn with_derived_from(mut self, derived_from: impl Into<String>) -> Self {
        self.derived_from = Some(derived_from.into());
        self
    }
}

impl std::fmt::Display for PathNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref relation) = self.relation {
            write!(f, "{}#{}", self.entity, relation)
        } else {
            write!(f, "{}", self.entity)
        }
    }
}

/// Reason why access was denied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DenialReason {
    /// No path exists from subject to resource.
    NoPath {
        /// Description of why no path exists.
        details: Option<String>,
    },
    /// A condition was not satisfied.
    ConditionFailed {
        /// The condition expression that failed.
        condition: String,
        /// Why the condition failed.
        reason: Option<String>,
    },
    /// Access was explicitly denied.
    ExplicitDeny {
        /// The relationship that caused the deny.
        relationship: Option<String>,
    },
    /// The relationship or permission has expired.
    Expired {
        /// When the access expired.
        expired_at: Option<String>,
    },
    /// The subject or resource does not exist.
    NotFound {
        /// What was not found.
        what: String,
    },
}

impl DenialReason {
    /// Creates a "no path" denial reason.
    pub fn no_path() -> Self {
        DenialReason::NoPath { details: None }
    }

    /// Creates a "no path" denial reason with details.
    pub fn no_path_with_details(details: impl Into<String>) -> Self {
        DenialReason::NoPath {
            details: Some(details.into()),
        }
    }

    /// Creates a "condition failed" denial reason.
    pub fn condition_failed(condition: impl Into<String>) -> Self {
        DenialReason::ConditionFailed {
            condition: condition.into(),
            reason: None,
        }
    }

    /// Creates a "condition failed" denial reason with a reason.
    pub fn condition_failed_with_reason(
        condition: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        DenialReason::ConditionFailed {
            condition: condition.into(),
            reason: Some(reason.into()),
        }
    }

    /// Creates an "explicit deny" denial reason.
    pub fn explicit_deny() -> Self {
        DenialReason::ExplicitDeny { relationship: None }
    }

    /// Creates an "expired" denial reason.
    pub fn expired() -> Self {
        DenialReason::Expired { expired_at: None }
    }

    /// Creates a "not found" denial reason.
    pub fn not_found(what: impl Into<String>) -> Self {
        DenialReason::NotFound { what: what.into() }
    }
}

impl std::fmt::Display for DenialReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DenialReason::NoPath { details } => {
                if let Some(details) = details {
                    write!(f, "no path: {}", details)
                } else {
                    write!(f, "no path from subject to resource")
                }
            }
            DenialReason::ConditionFailed { condition, reason } => {
                if let Some(reason) = reason {
                    write!(f, "condition '{}' failed: {}", condition, reason)
                } else {
                    write!(f, "condition '{}' failed", condition)
                }
            }
            DenialReason::ExplicitDeny { relationship } => {
                if let Some(rel) = relationship {
                    write!(f, "explicitly denied by {}", rel)
                } else {
                    write!(f, "explicitly denied")
                }
            }
            DenialReason::Expired { expired_at } => {
                if let Some(at) = expired_at {
                    write!(f, "expired at {}", at)
                } else {
                    write!(f, "access has expired")
                }
            }
            DenialReason::NotFound { what } => {
                write!(f, "{} not found", what)
            }
        }
    }
}

/// A suggestion for how to grant access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessSuggestion {
    /// The relationship to add.
    pub relationship: String,
    /// Human-readable description of the suggestion.
    pub description: String,
    /// Impact of adding this relationship (e.g., "low", "medium", "high").
    pub impact: Option<String>,
}

impl AccessSuggestion {
    /// Creates a new access suggestion.
    pub fn new(relationship: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            relationship: relationship.into(),
            description: description.into(),
            impact: None,
        }
    }

    /// Sets the impact level.
    #[must_use]
    pub fn with_impact(mut self, impact: impl Into<String>) -> Self {
        self.impact = Some(impact.into());
        self
    }
}

impl std::fmt::Display for AccessSuggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref impact) = self.impact {
            write!(
                f,
                "{} (impact: {}): {}",
                self.relationship, impact, self.description
            )
        } else {
            write!(f, "{}: {}", self.relationship, self.description)
        }
    }
}

/// Builder for explain permission requests.
///
/// ## Example
///
/// ```rust,ignore
/// let explanation = vault
///     .explain_permission()
///     .subject("user:alice")
///     .permission("edit")
///     .resource("doc:readme")
///     .await?;
/// ```
pub struct ExplainBuilder {
    pub(crate) subject: Option<String>,
    pub(crate) permission: Option<String>,
    pub(crate) resource: Option<String>,
    pub(crate) context: Option<Context>,
}

impl ExplainBuilder {
    /// Creates a new explain builder.
    pub fn new() -> Self {
        Self {
            subject: None,
            permission: None,
            resource: None,
            context: None,
        }
    }

    /// Sets the subject to check.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Sets the permission to check.
    #[must_use]
    pub fn permission(mut self, permission: impl Into<String>) -> Self {
        self.permission = Some(permission.into());
        self
    }

    /// Sets the resource to check.
    #[must_use]
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Sets the ABAC context for condition evaluation.
    #[must_use]
    pub fn context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Validates the builder configuration.
    #[allow(dead_code)] // Will be used when transport is wired
    pub(crate) fn validate(&self) -> Result<(), crate::Error> {
        if self.subject.is_none() {
            return Err(crate::Error::invalid_argument("subject is required"));
        }
        if self.permission.is_none() {
            return Err(crate::Error::invalid_argument("permission is required"));
        }
        if self.resource.is_none() {
            return Err(crate::Error::invalid_argument("resource is required"));
        }
        Ok(())
    }
}

impl Default for ExplainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialization helper for Duration as milliseconds.
mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_explanation_allowed() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1");
        assert!(exp.allowed);
        assert_eq!(exp.subject, "user:alice");
        assert_eq!(exp.permission, "view");
        assert_eq!(exp.resource, "doc:1");
    }

    #[test]
    fn test_permission_explanation_denied() {
        let exp = PermissionExplanation::denied("user:alice", "edit", "doc:1");
        assert!(!exp.allowed);
    }

    #[test]
    fn test_permission_explanation_with_path() {
        let path = vec![
            PathNode::new("user:alice"),
            PathNode::new("team:engineering").with_relation("member"),
            PathNode::new("doc:1").with_relation("viewer"),
        ];

        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1").with_path(path);

        assert_eq!(exp.paths.len(), 1);
        assert_eq!(exp.paths[0].len(), 3);
    }

    #[test]
    fn test_permission_explanation_format_path() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1");
        let path = vec![
            PathNode::new("user:alice"),
            PathNode::new("team:eng").with_relation("member"),
            PathNode::new("doc:1").with_relation("viewer"),
        ];

        let formatted = exp.format_path(&path);
        assert!(formatted.contains("user:alice"));
        assert!(formatted.contains("team:eng#member"));
        assert!(formatted.contains("doc:1#viewer"));
    }

    #[test]
    fn test_permission_explanation_summary() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1");
        let summary = exp.summary();
        assert!(summary.contains("user:alice"));
        assert!(summary.contains("view"));
        assert!(summary.contains("doc:1"));
    }

    #[test]
    fn test_permission_explanation_display() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1")
            .with_path(vec![PathNode::new("user:alice"), PathNode::new("doc:1")]);

        let display = format!("{}", exp);
        assert!(display.contains("Permission Explanation"));
        assert!(display.contains("ALLOWED"));
        assert!(display.contains("user:alice"));
    }

    #[test]
    fn test_path_node() {
        let node = PathNode::new("user:alice")
            .with_relation("member")
            .with_derived_from("team_membership");

        assert_eq!(node.entity, "user:alice");
        assert_eq!(node.relation, Some("member".to_string()));
        assert_eq!(node.derived_from, Some("team_membership".to_string()));
    }

    #[test]
    fn test_path_node_display() {
        let node1 = PathNode::new("user:alice");
        assert_eq!(format!("{}", node1), "user:alice");

        let node2 = PathNode::new("team:eng").with_relation("member");
        assert_eq!(format!("{}", node2), "team:eng#member");
    }

    #[test]
    fn test_denial_reason_no_path() {
        let reason = DenialReason::no_path();
        assert!(format!("{}", reason).contains("no path"));

        let reason2 = DenialReason::no_path_with_details("user not in any team");
        assert!(format!("{}", reason2).contains("user not in any team"));
    }

    #[test]
    fn test_denial_reason_condition_failed() {
        let reason = DenialReason::condition_failed("time < 18:00");
        assert!(format!("{}", reason).contains("time < 18:00"));

        let reason2 =
            DenialReason::condition_failed_with_reason("time < 18:00", "current time is 20:00");
        assert!(format!("{}", reason2).contains("current time is 20:00"));
    }

    #[test]
    fn test_denial_reason_explicit_deny() {
        let reason = DenialReason::explicit_deny();
        assert!(format!("{}", reason).contains("explicitly denied"));
    }

    #[test]
    fn test_denial_reason_expired() {
        let reason = DenialReason::expired();
        assert!(format!("{}", reason).contains("expired"));
    }

    #[test]
    fn test_denial_reason_not_found() {
        let reason = DenialReason::not_found("resource");
        assert!(format!("{}", reason).contains("resource not found"));
    }

    #[test]
    fn test_access_suggestion() {
        let suggestion =
            AccessSuggestion::new("doc:1#viewer@user:alice", "Grant viewer access directly");
        assert_eq!(suggestion.relationship, "doc:1#viewer@user:alice");
        assert!(suggestion.impact.is_none());

        let suggestion2 = suggestion.with_impact("low");
        assert_eq!(suggestion2.impact, Some("low".to_string()));
    }

    #[test]
    fn test_access_suggestion_display() {
        let suggestion = AccessSuggestion::new("doc:1#viewer@user:alice", "Grant viewer access");
        let display = format!("{}", suggestion);
        assert!(display.contains("doc:1#viewer@user:alice"));
        assert!(display.contains("Grant viewer access"));

        let suggestion2 = suggestion.with_impact("low");
        let display2 = format!("{}", suggestion2);
        assert!(display2.contains("(impact: low)"));
    }

    #[test]
    fn test_explain_builder() {
        let builder = ExplainBuilder::new()
            .subject("user:alice")
            .permission("view")
            .resource("doc:1");

        assert_eq!(builder.subject, Some("user:alice".to_string()));
        assert_eq!(builder.permission, Some("view".to_string()));
        assert_eq!(builder.resource, Some("doc:1".to_string()));
    }

    #[test]
    fn test_explain_builder_validate() {
        let builder = ExplainBuilder::new();
        assert!(builder.validate().is_err());

        let builder2 = ExplainBuilder::new().subject("user:alice");
        assert!(builder2.validate().is_err());

        let builder3 = ExplainBuilder::new()
            .subject("user:alice")
            .permission("view")
            .resource("doc:1");
        assert!(builder3.validate().is_ok());
    }

    #[test]
    fn test_permission_explanation_serialization() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1")
            .with_evaluation_time(Duration::from_millis(50))
            .with_cached(true);

        let json = serde_json::to_string(&exp).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("\"evaluation_time\":50"));
        assert!(json.contains("\"cached\":true"));

        let parsed: PermissionExplanation = serde_json::from_str(&json).unwrap();
        assert!(parsed.allowed);
        assert_eq!(parsed.evaluation_time, Duration::from_millis(50));
    }

    #[test]
    fn test_denial_reason_serialization() {
        let reason = DenialReason::condition_failed_with_reason("time < 18:00", "too late");
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("\"type\":\"condition_failed\""));
        assert!(json.contains("\"condition\":\"time < 18:00\""));

        let parsed: DenialReason = serde_json::from_str(&json).unwrap();
        match parsed {
            DenialReason::ConditionFailed { condition, reason } => {
                assert_eq!(condition, "time < 18:00");
                assert_eq!(reason, Some("too late".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_permission_explanation_with_denial_reason() {
        let exp = PermissionExplanation::denied("user:alice", "edit", "doc:secret")
            .with_denial_reason(DenialReason::no_path())
            .with_suggestion(AccessSuggestion::new(
                "doc:secret#editor@user:alice",
                "Add alice as editor",
            ));

        assert!(!exp.allowed);
        assert_eq!(exp.denial_reasons.len(), 1);
        assert_eq!(exp.suggestions.len(), 1);
    }

    #[test]
    fn test_permission_explanation_summary_with_paths() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1")
            .with_path(vec![PathNode::new("user:alice"), PathNode::new("doc:1")]);
        let summary = exp.summary();
        assert!(summary.contains("via 1 path(s)"));
    }

    #[test]
    fn test_permission_explanation_summary_denied_with_reasons() {
        let exp = PermissionExplanation::denied("user:alice", "edit", "doc:1")
            .with_denial_reason(DenialReason::no_path());
        let summary = exp.summary();
        assert!(summary.contains("does not have"));
        assert!(summary.contains("no path"));
    }

    #[test]
    fn test_permission_explanation_format_empty_path() {
        let exp = PermissionExplanation::allowed("user:alice", "view", "doc:1");
        let formatted = exp.format_path(&[]);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_permission_explanation_display_denied_with_reasons() {
        let exp = PermissionExplanation::denied("user:alice", "edit", "doc:1")
            .with_denial_reason(DenialReason::no_path())
            .with_suggestion(AccessSuggestion::new("add relation", "Grant access"));

        let display = format!("{}", exp);
        assert!(display.contains("DENIED"));
        assert!(display.contains("Denial Reasons"));
        assert!(display.contains("Suggestions"));
    }

    #[test]
    fn test_denial_reason_explicit_deny_with_relationship() {
        let reason = DenialReason::ExplicitDeny {
            relationship: Some("doc:1#blocked@user:alice".to_string()),
        };
        let display = format!("{}", reason);
        assert!(display.contains("doc:1#blocked@user:alice"));
    }

    #[test]
    fn test_denial_reason_expired_with_time() {
        let reason = DenialReason::Expired {
            expired_at: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let display = format!("{}", reason);
        assert!(display.contains("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_explain_builder_default() {
        let builder = ExplainBuilder::default();
        assert!(builder.subject.is_none());
        assert!(builder.permission.is_none());
        assert!(builder.resource.is_none());
        assert!(builder.context.is_none());
    }

    #[test]
    fn test_explain_builder_with_context() {
        let builder = ExplainBuilder::new()
            .subject("user:alice")
            .permission("view")
            .resource("doc:1")
            .context(Context::new());

        assert!(builder.context.is_some());
    }

    #[test]
    fn test_path_node_equality() {
        let node1 = PathNode::new("user:alice").with_relation("member");
        let node2 = PathNode::new("user:alice").with_relation("member");
        let node3 = PathNode::new("user:bob").with_relation("member");

        assert_eq!(node1, node2);
        assert_ne!(node1, node3);
    }

    #[test]
    fn test_denial_reason_equality() {
        let reason1 = DenialReason::no_path();
        let reason2 = DenialReason::no_path();
        let reason3 = DenialReason::not_found("user");

        assert_eq!(reason1, reason2);
        assert_ne!(reason1, reason3);
    }

    #[test]
    fn test_access_suggestion_equality() {
        let sugg1 = AccessSuggestion::new("rel1", "desc1");
        let sugg2 = AccessSuggestion::new("rel1", "desc1");
        let sugg3 = AccessSuggestion::new("rel2", "desc2");

        assert_eq!(sugg1, sugg2);
        assert_ne!(sugg1, sugg3);
    }

    #[test]
    fn test_path_node_clone() {
        let node = PathNode::new("user:alice")
            .with_relation("member")
            .with_derived_from("via_team");
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    #[test]
    fn test_denial_reason_clone() {
        let reason = DenialReason::condition_failed_with_reason("cond", "failed");
        let cloned = reason.clone();
        assert_eq!(reason, cloned);
    }

    #[test]
    fn test_access_suggestion_clone() {
        let sugg = AccessSuggestion::new("rel", "desc").with_impact("low");
        let cloned = sugg.clone();
        assert_eq!(sugg, cloned);
    }
}
