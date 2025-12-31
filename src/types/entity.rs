//! Type-safe entity traits for resources and subjects.
//!
//! These traits enable compile-time type safety for authorization operations
//! by ensuring that only valid entity types are used in relationship operations.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use inferadb::types::{Resource, Subject};
//!
//! // Implement Resource for your domain types
//! struct Document {
//!     id: String,
//! }
//!
//! impl Resource for Document {
//!     fn resource_type() -> &'static str { "document" }
//!     fn resource_id(&self) -> &str { &self.id }
//! }
//!
//! // Implement Subject for your user types
//! struct User {
//!     id: String,
//! }
//!
//! impl Subject for User {
//!     fn subject_type() -> &'static str { "user" }
//!     fn subject_id(&self) -> &str { &self.id }
//! }
//!
//! // Type-safe relationship creation
//! let doc = Document { id: "readme".into() };
//! let user = User { id: "alice".into() };
//!
//! // Using the typed API
//! vault.check(&user, "view", &doc).await?;
//! ```
//!
//! ## Derive Macros
//!
//! With the `derive` feature enabled, you can use derive macros:
//!
//! ```rust,ignore
//! use inferadb::derive::{Resource, Subject};
//!
//! #[derive(Resource)]
//! #[resource(type = "document")]
//! struct Document {
//!     #[resource(id)]
//!     id: String,
//! }
//!
//! #[derive(Subject)]
//! #[subject(type = "user")]
//! struct User {
//!     #[subject(id)]
//!     id: String,
//! }
//! ```

use std::{borrow::Cow, fmt};

/// A trait for types that can be used as resources in authorization checks.
///
/// Resources are the objects being accessed (e.g., documents, folders, projects).
///
/// ## Example
///
/// ```rust
/// use inferadb::types::Resource;
///
/// struct Document {
///     id: String,
///     title: String,
/// }
///
/// impl Resource for Document {
///     fn resource_type() -> &'static str { "document" }
///     fn resource_id(&self) -> &str { &self.id }
/// }
///
/// let doc = Document { id: "readme".into(), title: "README".into() };
/// assert_eq!(doc.as_resource_ref(), "document:readme");
/// ```
pub trait Resource {
    /// Returns the type name for this resource (e.g., "document", "folder").
    fn resource_type() -> &'static str;

    /// Returns the unique identifier for this resource instance.
    fn resource_id(&self) -> &str;

    /// Returns the full resource reference in "type:id" format.
    fn as_resource_ref(&self) -> String {
        format!("{}:{}", Self::resource_type(), self.resource_id())
    }
}

/// A trait for types that can be used as subjects in authorization checks.
///
/// Subjects are the actors performing actions (e.g., users, groups, service accounts).
///
/// ## Example
///
/// ```rust
/// use inferadb::types::Subject;
///
/// struct User {
///     id: String,
///     name: String,
/// }
///
/// impl Subject for User {
///     fn subject_type() -> &'static str { "user" }
///     fn subject_id(&self) -> &str { &self.id }
/// }
///
/// let user = User { id: "alice".into(), name: "Alice".into() };
/// assert_eq!(user.as_subject_ref(), "user:alice");
/// ```
pub trait Subject {
    /// Returns the type name for this subject (e.g., "user", "group", "service_account").
    fn subject_type() -> &'static str;

    /// Returns the unique identifier for this subject instance.
    fn subject_id(&self) -> &str;

    /// Returns the full subject reference in "type:id" format.
    fn as_subject_ref(&self) -> String {
        format!("{}:{}", Self::subject_type(), self.subject_id())
    }

    /// Returns a userset reference in "type:id#relation" format.
    ///
    /// This is used for group membership checks where subjects are
    /// defined through a relation (e.g., "group:admins#member").
    fn as_userset_ref(&self, relation: &str) -> String {
        format!("{}:{}#{}", Self::subject_type(), self.subject_id(), relation)
    }
}

/// A parsed entity reference in "type:id" format.
///
/// This struct provides zero-copy parsing and manipulation of entity references
/// used in authorization operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityRef<'a> {
    entity_type: Cow<'a, str>,
    entity_id: Cow<'a, str>,
}

/// Error parsing an entity reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Missing colon separator between type and ID.
    MissingColon,
    /// Empty entity type.
    EmptyType,
    /// Empty entity ID.
    EmptyId,
    /// Invalid characters in entity type.
    InvalidTypeChars(String),
    /// Invalid characters in entity ID.
    InvalidIdChars(String),
    /// Invalid userset format (for SubjectRef with #relation).
    InvalidUserset(String),
}

impl std::error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingColon => write!(f, "missing colon separator in entity reference"),
            ParseError::EmptyType => write!(f, "empty entity type"),
            ParseError::EmptyId => write!(f, "empty entity ID"),
            ParseError::InvalidTypeChars(s) => {
                write!(f, "invalid characters in entity type: {}", s)
            },
            ParseError::InvalidIdChars(s) => write!(f, "invalid characters in entity ID: {}", s),
            ParseError::InvalidUserset(s) => write!(f, "invalid userset format: {}", s),
        }
    }
}

impl<'a> EntityRef<'a> {
    /// Parse an entity reference from "type:id" format.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::types::EntityRef;
    ///
    /// let entity = EntityRef::parse("document:readme").unwrap();
    /// assert_eq!(entity.entity_type(), "document");
    /// assert_eq!(entity.entity_id(), "readme");
    /// ```
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        let (entity_type, entity_id) = s.split_once(':').ok_or(ParseError::MissingColon)?;

        if entity_type.is_empty() {
            return Err(ParseError::EmptyType);
        }
        if entity_id.is_empty() {
            return Err(ParseError::EmptyId);
        }

        Ok(Self { entity_type: Cow::Borrowed(entity_type), entity_id: Cow::Borrowed(entity_id) })
    }

    /// Create an entity reference from type and ID components.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::types::EntityRef;
    ///
    /// let entity = EntityRef::new("user", "alice");
    /// assert_eq!(entity.to_string(), "user:alice");
    /// ```
    pub fn new(entity_type: impl Into<Cow<'a, str>>, entity_id: impl Into<Cow<'a, str>>) -> Self {
        Self { entity_type: entity_type.into(), entity_id: entity_id.into() }
    }

    /// Returns the entity type.
    pub fn entity_type(&self) -> &str {
        &self.entity_type
    }

    /// Returns the entity ID.
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    /// Convert to an owned version with `'static` lifetime.
    pub fn into_owned(self) -> EntityRef<'static> {
        EntityRef {
            entity_type: Cow::Owned(self.entity_type.into_owned()),
            entity_id: Cow::Owned(self.entity_id.into_owned()),
        }
    }

    /// Create from a Resource implementation.
    pub fn from_resource<R: Resource>(resource: &R) -> EntityRef<'static> {
        EntityRef {
            entity_type: Cow::Borrowed(R::resource_type()),
            entity_id: Cow::Owned(resource.resource_id().to_owned()),
        }
    }

    /// Create from a Subject implementation.
    pub fn from_subject<S: Subject>(subject: &S) -> EntityRef<'static> {
        EntityRef {
            entity_type: Cow::Borrowed(S::subject_type()),
            entity_id: Cow::Owned(subject.subject_id().to_owned()),
        }
    }
}

impl fmt::Display for EntityRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.entity_type, self.entity_id)
    }
}

/// A subject reference that can include an optional relation for usersets.
///
/// Supports both simple subjects ("user:alice") and usersets ("group:admins#member").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubjectRef<'a> {
    entity: EntityRef<'a>,
    relation: Option<Cow<'a, str>>,
}

impl<'a> SubjectRef<'a> {
    /// Parse a subject reference from "type:id" or "type:id#relation" format.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use inferadb::types::SubjectRef;
    ///
    /// // Simple subject
    /// let subject = SubjectRef::parse("user:alice").unwrap();
    /// assert_eq!(subject.entity().entity_type(), "user");
    /// assert_eq!(subject.relation(), None);
    ///
    /// // Userset (group membership)
    /// let userset = SubjectRef::parse("group:admins#member").unwrap();
    /// assert_eq!(userset.entity().entity_type(), "group");
    /// assert_eq!(userset.relation(), Some("member"));
    /// ```
    pub fn parse(s: &'a str) -> Result<Self, ParseError> {
        if let Some((entity_part, relation)) = s.split_once('#') {
            let entity = EntityRef::parse(entity_part)?;
            if relation.is_empty() {
                return Err(ParseError::InvalidUserset("empty relation in userset".to_string()));
            }
            Ok(Self { entity, relation: Some(Cow::Borrowed(relation)) })
        } else {
            Ok(Self { entity: EntityRef::parse(s)?, relation: None })
        }
    }

    /// Create a simple subject reference without a relation.
    pub fn simple(
        entity_type: impl Into<Cow<'a, str>>,
        entity_id: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self { entity: EntityRef::new(entity_type, entity_id), relation: None }
    }

    /// Create a userset reference with a relation.
    pub fn userset(
        entity_type: impl Into<Cow<'a, str>>,
        entity_id: impl Into<Cow<'a, str>>,
        relation: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self { entity: EntityRef::new(entity_type, entity_id), relation: Some(relation.into()) }
    }

    /// Returns the entity part of this subject reference.
    pub fn entity(&self) -> &EntityRef<'a> {
        &self.entity
    }

    /// Returns the relation if this is a userset reference.
    pub fn relation(&self) -> Option<&str> {
        self.relation.as_deref()
    }

    /// Returns true if this is a userset reference (has a relation).
    pub fn is_userset(&self) -> bool {
        self.relation.is_some()
    }

    /// Convert to an owned version with `'static` lifetime.
    pub fn into_owned(self) -> SubjectRef<'static> {
        SubjectRef {
            entity: self.entity.into_owned(),
            relation: self.relation.map(|r| Cow::Owned(r.into_owned())),
        }
    }

    /// Create from a Subject implementation.
    pub fn from_subject<S: Subject>(subject: &S) -> SubjectRef<'static> {
        SubjectRef { entity: EntityRef::from_subject(subject), relation: None }
    }

    /// Create a userset from a Subject implementation.
    pub fn from_subject_userset<S: Subject>(subject: &S, relation: &str) -> SubjectRef<'static> {
        SubjectRef {
            entity: EntityRef::from_subject(subject),
            relation: Some(Cow::Owned(relation.to_owned())),
        }
    }
}

impl fmt::Display for SubjectRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref relation) = self.relation {
            write!(f, "{}#{}", self.entity, relation)
        } else {
            write!(f, "{}", self.entity)
        }
    }
}

// Implement Resource for string-like types for convenience
impl Resource for str {
    fn resource_type() -> &'static str {
        ""
    }

    fn resource_id(&self) -> &str {
        // For raw strings, the whole string is the reference (e.g., "document:readme")
        self
    }

    fn as_resource_ref(&self) -> String {
        self.to_string()
    }
}

impl Resource for String {
    fn resource_type() -> &'static str {
        ""
    }

    fn resource_id(&self) -> &str {
        self
    }

    fn as_resource_ref(&self) -> String {
        self.clone()
    }
}

impl Subject for str {
    fn subject_type() -> &'static str {
        ""
    }

    fn subject_id(&self) -> &str {
        self
    }

    fn as_subject_ref(&self) -> String {
        self.to_string()
    }
}

impl Subject for String {
    fn subject_type() -> &'static str {
        ""
    }

    fn subject_id(&self) -> &str {
        self
    }

    fn as_subject_ref(&self) -> String {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test types
    struct Document {
        id: String,
    }

    impl Resource for Document {
        fn resource_type() -> &'static str {
            "document"
        }
        fn resource_id(&self) -> &str {
            &self.id
        }
    }

    struct User {
        id: String,
    }

    impl Subject for User {
        fn subject_type() -> &'static str {
            "user"
        }
        fn subject_id(&self) -> &str {
            &self.id
        }
    }

    struct Group {
        id: String,
    }

    impl Subject for Group {
        fn subject_type() -> &'static str {
            "group"
        }
        fn subject_id(&self) -> &str {
            &self.id
        }
    }

    #[test]
    fn test_resource_trait() {
        let doc = Document { id: "readme".into() };
        assert_eq!(Document::resource_type(), "document");
        assert_eq!(doc.resource_id(), "readme");
        assert_eq!(doc.as_resource_ref(), "document:readme");
    }

    #[test]
    fn test_subject_trait() {
        let user = User { id: "alice".into() };
        assert_eq!(User::subject_type(), "user");
        assert_eq!(user.subject_id(), "alice");
        assert_eq!(user.as_subject_ref(), "user:alice");
    }

    #[test]
    fn test_subject_userset() {
        let group = Group { id: "admins".into() };
        assert_eq!(group.as_userset_ref("member"), "group:admins#member");
    }

    #[test]
    fn test_entity_ref_parse() {
        let entity = EntityRef::parse("document:readme").unwrap();
        assert_eq!(entity.entity_type(), "document");
        assert_eq!(entity.entity_id(), "readme");
    }

    #[test]
    fn test_entity_ref_new() {
        let entity = EntityRef::new("user", "alice");
        assert_eq!(entity.entity_type(), "user");
        assert_eq!(entity.entity_id(), "alice");
        assert_eq!(entity.to_string(), "user:alice");
    }

    #[test]
    fn test_entity_ref_parse_errors() {
        assert!(matches!(EntityRef::parse("no-colon"), Err(ParseError::MissingColon)));
        assert!(matches!(EntityRef::parse(":id"), Err(ParseError::EmptyType)));
        assert!(matches!(EntityRef::parse("type:"), Err(ParseError::EmptyId)));
    }

    #[test]
    fn test_entity_ref_into_owned() {
        let s = "document:readme".to_string();
        let entity = EntityRef::parse(&s).unwrap();
        let owned = entity.into_owned();
        assert_eq!(owned.entity_type(), "document");
        assert_eq!(owned.entity_id(), "readme");
    }

    #[test]
    fn test_entity_ref_from_resource() {
        let doc = Document { id: "readme".into() };
        let entity = EntityRef::from_resource(&doc);
        assert_eq!(entity.entity_type(), "document");
        assert_eq!(entity.entity_id(), "readme");
    }

    #[test]
    fn test_entity_ref_from_subject() {
        let user = User { id: "alice".into() };
        let entity = EntityRef::from_subject(&user);
        assert_eq!(entity.entity_type(), "user");
        assert_eq!(entity.entity_id(), "alice");
    }

    #[test]
    fn test_subject_ref_parse_simple() {
        let subject = SubjectRef::parse("user:alice").unwrap();
        assert_eq!(subject.entity().entity_type(), "user");
        assert_eq!(subject.entity().entity_id(), "alice");
        assert_eq!(subject.relation(), None);
        assert!(!subject.is_userset());
    }

    #[test]
    fn test_subject_ref_parse_userset() {
        let subject = SubjectRef::parse("group:admins#member").unwrap();
        assert_eq!(subject.entity().entity_type(), "group");
        assert_eq!(subject.entity().entity_id(), "admins");
        assert_eq!(subject.relation(), Some("member"));
        assert!(subject.is_userset());
    }

    #[test]
    fn test_subject_ref_simple() {
        let subject = SubjectRef::simple("user", "alice");
        assert_eq!(subject.to_string(), "user:alice");
    }

    #[test]
    fn test_subject_ref_userset() {
        let subject = SubjectRef::userset("group", "admins", "member");
        assert_eq!(subject.to_string(), "group:admins#member");
    }

    #[test]
    fn test_subject_ref_from_subject() {
        let user = User { id: "alice".into() };
        let subject = SubjectRef::from_subject(&user);
        assert_eq!(subject.to_string(), "user:alice");
    }

    #[test]
    fn test_subject_ref_from_subject_userset() {
        let group = Group { id: "admins".into() };
        let subject = SubjectRef::from_subject_userset(&group, "member");
        assert_eq!(subject.to_string(), "group:admins#member");
    }

    #[test]
    fn test_parse_error_display() {
        assert!(ParseError::MissingColon.to_string().contains("colon"));
        assert!(ParseError::EmptyType.to_string().contains("type"));
        assert!(ParseError::EmptyId.to_string().contains("ID"));
    }

    #[test]
    fn test_string_resource() {
        let s = "document:readme";
        assert_eq!(s.as_resource_ref(), "document:readme");
    }

    #[test]
    fn test_string_subject() {
        let s = "user:alice";
        assert_eq!(s.as_subject_ref(), "user:alice");
    }

    #[test]
    fn test_parse_error_all_variants_display() {
        // Test all ParseError variants have meaningful display
        assert!(ParseError::MissingColon.to_string().contains("colon"));
        assert!(ParseError::EmptyType.to_string().contains("empty"));
        assert!(ParseError::EmptyId.to_string().contains("empty"));
        assert!(ParseError::InvalidTypeChars("bad".to_string()).to_string().contains("bad"));
        assert!(ParseError::InvalidIdChars("invalid".to_string()).to_string().contains("invalid"));
        assert!(ParseError::InvalidUserset("format".to_string()).to_string().contains("format"));
    }

    #[test]
    fn test_parse_error_is_error_impl() {
        // Test that ParseError implements std::error::Error
        let err: &dyn std::error::Error = &ParseError::MissingColon;
        assert!(err.source().is_none()); // ParseError has no source
    }

    #[test]
    fn test_entity_ref_equality() {
        let a = EntityRef::parse("user:alice").unwrap();
        let b = EntityRef::parse("user:alice").unwrap();
        let c = EntityRef::parse("user:bob").unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_entity_ref_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EntityRef::parse("user:alice").unwrap());
        set.insert(EntityRef::parse("user:bob").unwrap());
        set.insert(EntityRef::parse("user:alice").unwrap()); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_subject_ref_equality() {
        let a = SubjectRef::parse("user:alice").unwrap();
        let b = SubjectRef::parse("user:alice").unwrap();
        let c = SubjectRef::parse("group:admins#member").unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_subject_ref_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SubjectRef::parse("user:alice").unwrap());
        set.insert(SubjectRef::parse("group:admins#member").unwrap());
        set.insert(SubjectRef::parse("user:alice").unwrap()); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_subject_ref_into_owned() {
        let s = "group:admins#member".to_string();
        let subject = SubjectRef::parse(&s).unwrap();
        let owned = subject.into_owned();

        assert_eq!(owned.entity().entity_type(), "group");
        assert_eq!(owned.entity().entity_id(), "admins");
        assert_eq!(owned.relation(), Some("member"));
    }

    #[test]
    fn test_subject_ref_parse_empty_relation() {
        // Empty relation after # should fail
        let result = SubjectRef::parse("group:admins#");
        assert!(matches!(result, Err(ParseError::InvalidUserset(_))));
    }

    #[test]
    fn test_owned_string_resource() {
        let s = String::from("document:readme");
        assert_eq!(String::resource_type(), "");
        assert_eq!(s.resource_id(), "document:readme");
        assert_eq!(s.as_resource_ref(), "document:readme");
    }

    #[test]
    fn test_owned_string_subject() {
        let s = String::from("user:alice");
        assert_eq!(String::subject_type(), "");
        assert_eq!(s.subject_id(), "user:alice");
        assert_eq!(s.as_subject_ref(), "user:alice");
    }

    #[test]
    fn test_entity_ref_clone() {
        let entity = EntityRef::parse("user:alice").unwrap();
        let cloned = entity.clone();
        assert_eq!(entity, cloned);
    }

    #[test]
    fn test_subject_ref_clone() {
        let subject = SubjectRef::parse("group:admins#member").unwrap();
        let cloned = subject.clone();
        assert_eq!(subject, cloned);
    }

    #[test]
    fn test_entity_ref_debug() {
        let entity = EntityRef::parse("user:alice").unwrap();
        let debug = format!("{:?}", entity);
        assert!(debug.contains("EntityRef"));
        assert!(debug.contains("user"));
        assert!(debug.contains("alice"));
    }

    #[test]
    fn test_subject_ref_debug() {
        let subject = SubjectRef::parse("group:admins#member").unwrap();
        let debug = format!("{:?}", subject);
        assert!(debug.contains("SubjectRef"));
    }

    #[test]
    fn test_parse_error_clone() {
        let err = ParseError::InvalidTypeChars("bad".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_entity_ref_new_with_cow() {
        use std::borrow::Cow;

        // Test with borrowed strings
        let entity1 = EntityRef::new("user", "alice");
        assert_eq!(entity1.to_string(), "user:alice");

        // Test with owned strings
        let entity2 =
            EntityRef::new(Cow::Owned("group".to_string()), Cow::Owned("admins".to_string()));
        assert_eq!(entity2.to_string(), "group:admins");
    }

    #[test]
    fn test_subject_ref_userset_with_cow() {
        use std::borrow::Cow;

        let subject = SubjectRef::userset(
            Cow::Owned("group".to_string()),
            Cow::Owned("admins".to_string()),
            Cow::Owned("member".to_string()),
        );
        assert_eq!(subject.to_string(), "group:admins#member");
        assert!(subject.is_userset());
    }

    #[test]
    fn test_str_resource_type_is_empty() {
        assert_eq!(<str as Resource>::resource_type(), "");
    }

    #[test]
    fn test_str_subject_type_is_empty() {
        assert_eq!(<str as Subject>::subject_type(), "");
    }

    #[test]
    fn test_entity_ref_display() {
        let entity = EntityRef::new("document", "secret-file");
        assert_eq!(format!("{}", entity), "document:secret-file");
    }

    #[test]
    fn test_subject_ref_simple_display() {
        let subject = SubjectRef::simple("user", "bob");
        assert_eq!(format!("{}", subject), "user:bob");
        assert!(!subject.is_userset());
    }

    #[test]
    fn test_subject_as_userset_ref() {
        let user = User { id: "alice".into() };
        let userset = user.as_userset_ref("member");
        assert_eq!(userset, "user:alice#member");
    }

    #[test]
    fn test_str_resource_id() {
        let s: &str = "document:readme";
        assert_eq!(s.resource_id(), "document:readme");
        assert_eq!(s.as_resource_ref(), "document:readme");
    }

    #[test]
    fn test_str_subject_id() {
        let s: &str = "user:alice";
        assert_eq!(s.subject_id(), "user:alice");
        assert_eq!(s.as_subject_ref(), "user:alice");
    }

    #[test]
    fn test_string_resource_id() {
        let s = String::from("document:readme");
        assert_eq!(s.resource_id(), "document:readme");
        assert_eq!(s.as_resource_ref(), "document:readme");
    }

    #[test]
    fn test_string_subject_id() {
        let s = String::from("user:alice");
        assert_eq!(s.subject_id(), "user:alice");
        assert_eq!(s.as_subject_ref(), "user:alice");
    }
}
