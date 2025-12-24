//! Decision types for authorization check results.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ConsistencyToken;

/// The reason for an authorization decision.
///
/// When using detailed checks, this provides insight into why
/// access was granted or denied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    /// Access granted through a direct relationship.
    ///
    /// The subject has an explicit relationship to the resource.
    DirectRelationship,

    /// Access granted through an inherited relationship.
    ///
    /// The subject has access through a parent resource or group membership.
    InheritedRelationship,

    /// Access granted through a computed permission.
    ///
    /// The permission was derived from other permissions (e.g., union, intersection).
    ComputedPermission,

    /// Access granted through an ABAC condition.
    ///
    /// An attribute-based condition was evaluated and passed.
    ConditionMet,

    /// Access denied: no matching relationship found.
    NoRelationship,

    /// Access denied: ABAC condition not met.
    ConditionNotMet,

    /// Access denied by explicit deny rule.
    ExplicitDeny,

    /// The reason is unknown or not provided.
    #[default]
    Unknown,
}

impl fmt::Display for DecisionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecisionReason::DirectRelationship => write!(f, "direct relationship"),
            DecisionReason::InheritedRelationship => write!(f, "inherited relationship"),
            DecisionReason::ComputedPermission => write!(f, "computed permission"),
            DecisionReason::ConditionMet => write!(f, "condition met"),
            DecisionReason::NoRelationship => write!(f, "no relationship"),
            DecisionReason::ConditionNotMet => write!(f, "condition not met"),
            DecisionReason::ExplicitDeny => write!(f, "explicit deny"),
            DecisionReason::Unknown => write!(f, "unknown"),
        }
    }
}

/// Metadata about an authorization decision.
///
/// This provides observability into the authorization check process,
/// useful for debugging and performance monitoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionMetadata {
    /// Time taken to evaluate the authorization check.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "duration_millis",
        default
    )]
    pub evaluation_time: Option<Duration>,

    /// The reason for the decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<DecisionReason>,

    /// Depth of the graph traversal.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub depth: Option<u32>,

    /// Number of relationships evaluated.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub relationships_evaluated: Option<u32>,

    /// Whether the result was served from cache.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cached: Option<bool>,

    /// The request ID for this check.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub request_id: Option<String>,

    /// Debug trace path (only in debug mode).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trace_path: Option<Vec<String>>,
}

impl DecisionMetadata {
    /// Creates new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the evaluation time.
    #[must_use]
    pub fn with_evaluation_time(mut self, duration: Duration) -> Self {
        self.evaluation_time = Some(duration);
        self
    }

    /// Sets the decision reason.
    #[must_use]
    pub fn with_reason(mut self, reason: DecisionReason) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Sets the traversal depth.
    #[must_use]
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    /// Sets the request ID.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Sets whether the result was cached.
    #[must_use]
    pub fn with_cached(mut self, cached: bool) -> Self {
        self.cached = Some(cached);
        self
    }
}

/// An authorization decision with optional metadata.
///
/// `Decision` wraps the boolean result of an authorization check with
/// additional metadata that can be useful for debugging and observability.
///
/// ## Basic Usage
///
/// For simple checks, `Decision` behaves like a boolean:
///
/// ```rust
/// use inferadb::Decision;
///
/// let decision = Decision::allowed();
/// if decision.is_allowed() {
///     println!("Access granted");
/// }
///
/// // Can also use bool coercion
/// let allowed: bool = decision.into();
/// ```
///
/// ## Detailed Checks
///
/// When using `check_detailed()`, the decision includes metadata:
///
/// ```rust,ignore
/// let decision = vault.check("user:alice", "view", "doc:1")
///     .detailed()
///     .await?;
///
/// if decision.is_allowed() {
///     if let Some(meta) = decision.metadata() {
///         println!("Evaluation took: {:?}", meta.evaluation_time);
///         if meta.cached == Some(true) {
///             println!("Result was cached");
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Whether access is allowed.
    allowed: bool,

    /// Optional metadata about the decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    metadata: Option<DecisionMetadata>,

    /// Consistency token for this decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    consistency_token: Option<ConsistencyToken>,
}

impl Decision {
    /// Creates a new decision with the given result.
    pub fn new(allowed: bool) -> Self {
        Self {
            allowed,
            metadata: None,
            consistency_token: None,
        }
    }

    /// Creates an "allowed" decision.
    pub fn allowed() -> Self {
        Self::new(true)
    }

    /// Creates a "denied" decision.
    pub fn denied() -> Self {
        Self::new(false)
    }

    /// Returns `true` if access is allowed.
    #[inline]
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Returns `true` if access is denied.
    #[inline]
    pub fn is_denied(&self) -> bool {
        !self.allowed
    }

    /// Returns the decision metadata, if available.
    pub fn metadata(&self) -> Option<&DecisionMetadata> {
        self.metadata.as_ref()
    }

    /// Returns the consistency token for this decision.
    pub fn consistency_token(&self) -> Option<&ConsistencyToken> {
        self.consistency_token.as_ref()
    }

    /// Returns the decision reason, if available.
    pub fn reason(&self) -> Option<&DecisionReason> {
        self.metadata.as_ref().and_then(|m| m.reason.as_ref())
    }

    /// Returns the evaluation time, if available.
    pub fn evaluation_time(&self) -> Option<Duration> {
        self.metadata.as_ref().and_then(|m| m.evaluation_time)
    }

    /// Returns `true` if the result was served from cache.
    pub fn was_cached(&self) -> Option<bool> {
        self.metadata.as_ref().and_then(|m| m.cached)
    }

    /// Returns the request ID, if available.
    pub fn request_id(&self) -> Option<&str> {
        self.metadata.as_ref().and_then(|m| m.request_id.as_deref())
    }

    /// Sets the metadata for this decision.
    #[must_use]
    pub fn with_metadata(mut self, metadata: DecisionMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the consistency token for this decision.
    #[must_use]
    pub fn with_consistency_token(mut self, token: ConsistencyToken) -> Self {
        self.consistency_token = Some(token);
        self
    }
}

impl From<bool> for Decision {
    fn from(allowed: bool) -> Self {
        Decision::new(allowed)
    }
}

impl From<Decision> for bool {
    fn from(decision: Decision) -> Self {
        decision.allowed
    }
}

impl PartialEq<bool> for Decision {
    fn eq(&self, other: &bool) -> bool {
        self.allowed == *other
    }
}

impl PartialEq<Decision> for bool {
    fn eq(&self, other: &Decision) -> bool {
        *self == other.allowed
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.allowed {
            write!(f, "allowed")?;
        } else {
            write!(f, "denied")?;
        }

        if let Some(reason) = self.reason() {
            write!(f, " ({})", reason)?;
        }

        Ok(())
    }
}

// Custom serialization for Duration as milliseconds
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_u64(d.as_millis() as u64),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis: Option<u64> = Option::deserialize(deserializer)?;
        Ok(millis.map(Duration::from_millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_new() {
        let allowed = Decision::new(true);
        assert!(allowed.is_allowed());
        assert!(!allowed.is_denied());

        let denied = Decision::new(false);
        assert!(denied.is_denied());
        assert!(!denied.is_allowed());
    }

    #[test]
    fn test_decision_constructors() {
        assert!(Decision::allowed().is_allowed());
        assert!(Decision::denied().is_denied());
    }

    #[test]
    fn test_decision_from_bool() {
        let decision: Decision = true.into();
        assert!(decision.is_allowed());

        let decision: Decision = false.into();
        assert!(decision.is_denied());
    }

    #[test]
    fn test_decision_into_bool() {
        let allowed: bool = Decision::allowed().into();
        assert!(allowed);

        let denied: bool = Decision::denied().into();
        assert!(!denied);
    }

    #[test]
    fn test_decision_equality_with_bool() {
        assert!(Decision::allowed() == true);
        assert!(Decision::denied() == false);
        assert!(true == Decision::allowed());
        assert!(false == Decision::denied());
    }

    #[test]
    fn test_decision_with_metadata() {
        let metadata = DecisionMetadata::new()
            .with_reason(DecisionReason::DirectRelationship)
            .with_depth(1)
            .with_cached(true);

        let decision = Decision::allowed().with_metadata(metadata);

        assert!(decision.metadata().is_some());
        assert_eq!(decision.reason(), Some(&DecisionReason::DirectRelationship));
        assert_eq!(decision.was_cached(), Some(true));
    }

    #[test]
    fn test_decision_with_consistency_token() {
        let token = ConsistencyToken::new("test_token");
        let decision = Decision::allowed().with_consistency_token(token.clone());

        assert_eq!(decision.consistency_token(), Some(&token));
    }

    #[test]
    fn test_decision_display() {
        assert_eq!(Decision::allowed().to_string(), "allowed");
        assert_eq!(Decision::denied().to_string(), "denied");

        let allowed_with_reason = Decision::allowed()
            .with_metadata(DecisionMetadata::new().with_reason(DecisionReason::DirectRelationship));
        assert_eq!(
            allowed_with_reason.to_string(),
            "allowed (direct relationship)"
        );

        let denied_with_reason = Decision::denied()
            .with_metadata(DecisionMetadata::new().with_reason(DecisionReason::NoRelationship));
        assert_eq!(denied_with_reason.to_string(), "denied (no relationship)");
    }

    #[test]
    fn test_decision_reason_display() {
        assert_eq!(
            DecisionReason::DirectRelationship.to_string(),
            "direct relationship"
        );
        assert_eq!(
            DecisionReason::InheritedRelationship.to_string(),
            "inherited relationship"
        );
        assert_eq!(
            DecisionReason::ComputedPermission.to_string(),
            "computed permission"
        );
        assert_eq!(DecisionReason::ConditionMet.to_string(), "condition met");
        assert_eq!(
            DecisionReason::NoRelationship.to_string(),
            "no relationship"
        );
        assert_eq!(
            DecisionReason::ConditionNotMet.to_string(),
            "condition not met"
        );
        assert_eq!(DecisionReason::ExplicitDeny.to_string(), "explicit deny");
        assert_eq!(DecisionReason::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = DecisionMetadata::new()
            .with_evaluation_time(Duration::from_millis(10))
            .with_reason(DecisionReason::InheritedRelationship)
            .with_depth(3)
            .with_request_id("req_123")
            .with_cached(false);

        assert_eq!(metadata.evaluation_time, Some(Duration::from_millis(10)));
        assert_eq!(metadata.reason, Some(DecisionReason::InheritedRelationship));
        assert_eq!(metadata.depth, Some(3));
        assert_eq!(metadata.request_id, Some("req_123".to_string()));
        assert_eq!(metadata.cached, Some(false));
    }

    #[test]
    fn test_decision_serialization() {
        let decision = Decision::allowed();
        let json = serde_json::to_string(&decision).unwrap();
        let parsed: Decision = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_allowed());
    }

    #[test]
    fn test_decision_with_metadata_serialization() {
        let decision = Decision::allowed().with_metadata(
            DecisionMetadata::new()
                .with_reason(DecisionReason::DirectRelationship)
                .with_evaluation_time(Duration::from_millis(5)),
        );

        let json = serde_json::to_string(&decision).unwrap();
        let parsed: Decision = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_allowed());
        assert!(parsed.metadata().is_some());
        assert_eq!(parsed.reason(), Some(&DecisionReason::DirectRelationship));
        assert_eq!(parsed.evaluation_time(), Some(Duration::from_millis(5)));
    }

    #[test]
    fn test_reason_default() {
        assert_eq!(DecisionReason::default(), DecisionReason::Unknown);
    }

    #[test]
    fn test_decision_request_id_none() {
        let decision = Decision::allowed();
        assert!(decision.request_id().is_none());
    }

    #[test]
    fn test_decision_request_id_some() {
        let metadata = DecisionMetadata {
            request_id: Some("req_123".to_string()),
            ..Default::default()
        };
        let decision = Decision::allowed().with_metadata(metadata);
        assert_eq!(decision.request_id(), Some("req_123"));
    }

    #[test]
    fn test_decision_request_id_metadata_without_id() {
        let metadata = DecisionMetadata {
            request_id: None,
            ..Default::default()
        };
        let decision = Decision::allowed().with_metadata(metadata);
        assert!(decision.request_id().is_none());
    }
}
