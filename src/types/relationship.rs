//! Relationship type representing a tuple in the authorization graph.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

/// A relationship tuple representing an edge in the authorization graph.
///
/// Relationships follow the Zanzibar model: `(resource, relation, subject)`.
/// This reads as "resource has relation to subject" or "subject is relation of resource".
///
/// ## Argument Order
///
/// The constructor uses the order: `resource, relation, subject`
///
/// ```rust
/// use inferadb::Relationship;
///
/// // "document:readme has viewer user:alice"
/// let rel = Relationship::new("document:readme", "viewer", "user:alice");
///
/// // "folder:reports has parent folder:root"
/// let parent = Relationship::new("folder:reports", "parent", "folder:root");
/// ```
///
/// This differs from `check()` which uses `subject, permission, resource` because:
/// - `check()`: "Can subject do X to resource?" (question from subject's perspective)
/// - `Relationship`: "Resource has relation to subject" (statement about resource)
///
/// ## String Format
///
/// Relationships can be parsed from and formatted to the standard tuple format:
///
/// ```rust
/// use inferadb::Relationship;
///
/// // Parse from string
/// let rel: Relationship = "document:readme#viewer@user:alice".parse().unwrap();
/// assert_eq!(rel.resource(), "document:readme");
/// assert_eq!(rel.relation(), "viewer");
/// assert_eq!(rel.subject(), "user:alice");
///
/// // Format to string
/// assert_eq!(rel.to_string(), "document:readme#viewer@user:alice");
/// ```
///
/// ## Zero-Copy Efficiency
///
/// `Relationship` uses [`Cow<str>`](std::borrow::Cow) internally for optimal performance:
/// - Static strings: No allocation
/// - Owned strings: Takes ownership, no copy
/// - Borrowed strings: Zero-copy reference
///
/// ```rust
/// use inferadb::Relationship;
///
/// // Static strings - no allocation
/// let rel = Relationship::new("document:readme", "viewer", "user:alice");
///
/// // Owned strings - takes ownership
/// let resource = String::from("document:") + &doc_id();
/// let rel = Relationship::new(resource, "viewer", "user:alice");
///
/// fn doc_id() -> String { "123".to_string() }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relationship<'a> {
    /// The resource (object) in the relationship.
    /// Format: "type:id" (e.g., "document:readme")
    #[serde(borrow)]
    resource: Cow<'a, str>,

    /// The relation (edge label) connecting resource to subject.
    /// Examples: "viewer", "editor", "owner", "parent", "member"
    #[serde(borrow)]
    relation: Cow<'a, str>,

    /// The subject of the relationship.
    /// Format: "type:id" (e.g., "user:alice") or "type:id#relation" for subject sets
    #[serde(borrow)]
    subject: Cow<'a, str>,
}

impl<'a> Relationship<'a> {
    /// Creates a new relationship.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource/object (e.g., "document:readme")
    /// * `relation` - The relation type (e.g., "viewer")
    /// * `subject` - The subject (e.g., "user:alice" or "team:engineering#member")
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// // "document:readme has viewer user:alice"
    /// let rel = Relationship::new("document:readme", "viewer", "user:alice");
    ///
    /// // Subject set: "folder:reports has viewer anyone who is member of team:engineering"
    /// let rel = Relationship::new("folder:reports", "viewer", "team:engineering#member");
    /// ```
    pub fn new(
        resource: impl Into<Cow<'a, str>>,
        relation: impl Into<Cow<'a, str>>,
        subject: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            resource: resource.into(),
            relation: relation.into(),
            subject: subject.into(),
        }
    }

    /// Returns the resource (object) of the relationship.
    ///
    /// The resource is typically in the format "type:id".
    #[inline]
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Returns the relation (edge label) of the relationship.
    #[inline]
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Returns the subject of the relationship.
    ///
    /// The subject can be:
    /// - A direct entity: "user:alice"
    /// - A subject set: "team:engineering#member"
    #[inline]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the resource type (the part before the colon).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let rel = Relationship::new("document:readme", "viewer", "user:alice");
    /// assert_eq!(rel.resource_type(), Some("document"));
    /// ```
    pub fn resource_type(&self) -> Option<&str> {
        self.resource.split(':').next()
    }

    /// Returns the resource ID (the part after the colon).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let rel = Relationship::new("document:readme", "viewer", "user:alice");
    /// assert_eq!(rel.resource_id(), Some("readme"));
    /// ```
    pub fn resource_id(&self) -> Option<&str> {
        self.resource.split(':').nth(1)
    }

    /// Returns the subject type (the part before the colon or hash).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let rel = Relationship::new("doc:1", "viewer", "team:engineering#member");
    /// assert_eq!(rel.subject_type(), Some("team"));
    /// ```
    pub fn subject_type(&self) -> Option<&str> {
        // Handle both "type:id" and "type:id#relation" formats
        self.subject.split([':', '#']).next()
    }

    /// Returns the subject ID (the part after the colon, before any hash).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let rel = Relationship::new("doc:1", "viewer", "team:engineering#member");
    /// assert_eq!(rel.subject_id(), Some("engineering"));
    /// ```
    pub fn subject_id(&self) -> Option<&str> {
        // Find the part between : and # (or end of string)
        let after_colon = self.subject.split(':').nth(1)?;
        Some(after_colon.split('#').next().unwrap_or(after_colon))
    }

    /// Returns the subject relation if this is a subject set.
    ///
    /// Returns `None` for direct subjects like "user:alice".
    /// Returns `Some("member")` for subject sets like "team:engineering#member".
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let direct = Relationship::new("doc:1", "viewer", "user:alice");
    /// assert_eq!(direct.subject_relation(), None);
    ///
    /// let set = Relationship::new("doc:1", "viewer", "team:engineering#member");
    /// assert_eq!(set.subject_relation(), Some("member"));
    /// ```
    pub fn subject_relation(&self) -> Option<&str> {
        self.subject.split('#').nth(1)
    }

    /// Returns `true` if the subject is a subject set (contains #).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let direct = Relationship::new("doc:1", "viewer", "user:alice");
    /// assert!(!direct.is_subject_set());
    ///
    /// let set = Relationship::new("doc:1", "viewer", "team:engineering#member");
    /// assert!(set.is_subject_set());
    /// ```
    pub fn is_subject_set(&self) -> bool {
        self.subject.contains('#')
    }

    /// Converts to an owned `Relationship<'static>`.
    ///
    /// This is useful when you need to store a relationship or return it
    /// from a function that outlives the original data.
    pub fn into_owned(self) -> Relationship<'static> {
        Relationship {
            resource: Cow::Owned(self.resource.into_owned()),
            relation: Cow::Owned(self.relation.into_owned()),
            subject: Cow::Owned(self.subject.into_owned()),
        }
    }

    /// Creates a borrowed view of this relationship.
    pub fn as_borrowed(&self) -> Relationship<'_> {
        Relationship {
            resource: Cow::Borrowed(&self.resource),
            relation: Cow::Borrowed(&self.relation),
            subject: Cow::Borrowed(&self.subject),
        }
    }
}

impl fmt::Display for Relationship<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}@{}", self.resource, self.relation, self.subject)
    }
}

impl FromStr for Relationship<'static> {
    type Err = Error;

    /// Parses a relationship from the standard tuple format.
    ///
    /// Format: `resource#relation@subject`
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Relationship;
    ///
    /// let rel: Relationship = "document:readme#viewer@user:alice".parse().unwrap();
    /// assert_eq!(rel.resource(), "document:readme");
    /// assert_eq!(rel.relation(), "viewer");
    /// assert_eq!(rel.subject(), "user:alice");
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format: resource#relation@subject
        let (resource, rest) = s.split_once('#').ok_or_else(|| {
            Error::invalid_argument(format!(
                "invalid relationship format: missing '#' separator in '{}'",
                s
            ))
        })?;

        let (relation, subject) = rest.split_once('@').ok_or_else(|| {
            Error::invalid_argument(format!(
                "invalid relationship format: missing '@' separator in '{}'",
                s
            ))
        })?;

        if resource.is_empty() {
            return Err(Error::invalid_argument(
                "relationship resource cannot be empty",
            ));
        }
        if relation.is_empty() {
            return Err(Error::invalid_argument(
                "relationship relation cannot be empty",
            ));
        }
        if subject.is_empty() {
            return Err(Error::invalid_argument(
                "relationship subject cannot be empty",
            ));
        }

        Ok(Relationship::new(
            resource.to_owned(),
            relation.to_owned(),
            subject.to_owned(),
        ))
    }
}

/// Creates a relationship from a tuple of strings.
impl<'a, R, L, S> From<(R, L, S)> for Relationship<'a>
where
    R: Into<Cow<'a, str>>,
    L: Into<Cow<'a, str>>,
    S: Into<Cow<'a, str>>,
{
    fn from((resource, relation, subject): (R, L, S)) -> Self {
        Self::new(resource, relation, subject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_new() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        assert_eq!(rel.resource(), "document:readme");
        assert_eq!(rel.relation(), "viewer");
        assert_eq!(rel.subject(), "user:alice");
    }

    #[test]
    fn test_relationship_with_owned_strings() {
        let resource = String::from("document:123");
        let relation = String::from("editor");
        let subject = String::from("user:bob");

        let rel = Relationship::new(resource, relation, subject);
        assert_eq!(rel.resource(), "document:123");
        assert_eq!(rel.relation(), "editor");
        assert_eq!(rel.subject(), "user:bob");
    }

    #[test]
    fn test_relationship_parts() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        assert_eq!(rel.resource_type(), Some("document"));
        assert_eq!(rel.resource_id(), Some("readme"));
        assert_eq!(rel.subject_type(), Some("user"));
        assert_eq!(rel.subject_id(), Some("alice"));
        assert_eq!(rel.subject_relation(), None);
        assert!(!rel.is_subject_set());
    }

    #[test]
    fn test_subject_set() {
        let rel = Relationship::new("folder:reports", "viewer", "team:engineering#member");
        assert_eq!(rel.subject_type(), Some("team"));
        assert_eq!(rel.subject_id(), Some("engineering"));
        assert_eq!(rel.subject_relation(), Some("member"));
        assert!(rel.is_subject_set());
    }

    #[test]
    fn test_display() {
        let rel = Relationship::new("document:readme", "viewer", "user:alice");
        assert_eq!(rel.to_string(), "document:readme#viewer@user:alice");
    }

    #[test]
    fn test_from_str() {
        let rel: Relationship = "document:readme#viewer@user:alice".parse().unwrap();
        assert_eq!(rel.resource(), "document:readme");
        assert_eq!(rel.relation(), "viewer");
        assert_eq!(rel.subject(), "user:alice");
    }

    #[test]
    fn test_from_str_subject_set() {
        let rel: Relationship = "folder:reports#viewer@team:eng#member".parse().unwrap();
        assert_eq!(rel.resource(), "folder:reports");
        assert_eq!(rel.relation(), "viewer");
        assert_eq!(rel.subject(), "team:eng#member");
        assert!(rel.is_subject_set());
    }

    #[test]
    fn test_from_str_invalid() {
        // Missing #
        assert!("document:readme".parse::<Relationship>().is_err());
        // Missing @
        assert!("document:readme#viewer".parse::<Relationship>().is_err());
        // Empty parts
        assert!("#viewer@user:alice".parse::<Relationship>().is_err());
        assert!("doc:1#@user:alice".parse::<Relationship>().is_err());
        assert!("doc:1#viewer@".parse::<Relationship>().is_err());
    }

    #[test]
    fn test_from_tuple() {
        let rel: Relationship = ("doc:1", "viewer", "user:alice").into();
        assert_eq!(rel.resource(), "doc:1");
        assert_eq!(rel.relation(), "viewer");
        assert_eq!(rel.subject(), "user:alice");
    }

    #[test]
    fn test_into_owned() {
        let rel = Relationship::new("doc:1", "viewer", "user:alice");
        let owned: Relationship<'static> = rel.into_owned();
        assert_eq!(owned.resource(), "doc:1");
    }

    #[test]
    fn test_equality() {
        let rel1 = Relationship::new("doc:1", "viewer", "user:alice");
        let rel2 = Relationship::new("doc:1", "viewer", "user:alice");
        let rel3 = Relationship::new("doc:1", "editor", "user:alice");

        assert_eq!(rel1, rel2);
        assert_ne!(rel1, rel3);
    }

    #[test]
    fn test_serialization() {
        let rel = Relationship::new("doc:1", "viewer", "user:alice");
        let json = serde_json::to_string(&rel).unwrap();
        let parsed: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(rel, parsed);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        set.insert(Relationship::new("doc:1", "viewer", "user:alice").into_owned());
        set.insert(Relationship::new("doc:2", "viewer", "user:alice").into_owned());

        assert_eq!(set.len(), 2);
    }
}
