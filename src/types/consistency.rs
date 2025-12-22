//! ConsistencyToken for read-after-write consistency.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

/// A token representing a point-in-time snapshot of the authorization graph.
///
/// Consistency tokens enable read-after-write consistency in distributed
/// authorization checks. After a write operation (creating/deleting relationships),
/// the returned token can be used to ensure subsequent reads see those changes.
///
/// ## Obtaining Tokens
///
/// Tokens are returned from write operations:
///
/// ```rust,ignore
/// // Write returns a consistency token
/// let token = vault.relationships()
///     .write(Relationship::new("doc:1", "viewer", "user:alice"))
///     .await?;
///
/// // Use token for consistent read
/// let allowed = vault.check("user:alice", "view", "doc:1")
///     .at_least_as_fresh(token)
///     .await?;
/// ```
///
/// ## Token Lifetime
///
/// Tokens are typically valid for a limited time (usually minutes to hours).
/// They should be used promptly after write operations.
///
/// ## Serialization
///
/// Tokens can be serialized for transmission between services:
///
/// ```rust
/// use inferadb::ConsistencyToken;
///
/// let token = ConsistencyToken::new("abc123xyz");
///
/// // Serialize to string
/// let serialized = token.to_string();
///
/// // Parse from string
/// let parsed: ConsistencyToken = serialized.parse().unwrap();
/// assert_eq!(token, parsed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsistencyToken {
    /// The opaque token value.
    #[serde(rename = "token")]
    value: String,
}

impl ConsistencyToken {
    /// Creates a new consistency token from a string value.
    ///
    /// # Arguments
    ///
    /// * `value` - The opaque token value (typically from server response)
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::ConsistencyToken;
    ///
    /// let token = ConsistencyToken::new("MXxhYmMxMjM=");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Returns the raw token value.
    ///
    /// This is the opaque string that should be passed to the server.
    #[inline]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Consumes the token and returns the inner value.
    #[inline]
    pub fn into_value(self) -> String {
        self.value
    }

    /// Returns `true` if the token value is empty.
    ///
    /// Empty tokens are typically invalid.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Returns the length of the token value in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.value.len()
    }
}

impl fmt::Display for ConsistencyToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for ConsistencyToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::invalid_argument("consistency token cannot be empty"));
        }
        Ok(ConsistencyToken::new(s))
    }
}

impl From<String> for ConsistencyToken {
    fn from(value: String) -> Self {
        ConsistencyToken::new(value)
    }
}

impl From<&str> for ConsistencyToken {
    fn from(value: &str) -> Self {
        ConsistencyToken::new(value)
    }
}

impl AsRef<str> for ConsistencyToken {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

/// Represents the desired consistency level for a read operation.
///
/// This is used internally to specify how fresh the data should be.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConsistencyRequirement {
    /// Use eventual consistency (fastest, but may see stale data).
    #[default]
    Eventual,

    /// Ensure data is at least as fresh as the given token.
    AtLeastAsFresh(ConsistencyToken),

    /// Use the strongest consistency guarantee available.
    Full,
}


impl ConsistencyRequirement {
    /// Returns the token if this is an `AtLeastAsFresh` requirement.
    pub fn token(&self) -> Option<&ConsistencyToken> {
        match self {
            ConsistencyRequirement::AtLeastAsFresh(token) => Some(token),
            _ => None,
        }
    }

    /// Returns `true` if this is `Eventual` consistency.
    pub fn is_eventual(&self) -> bool {
        matches!(self, ConsistencyRequirement::Eventual)
    }

    /// Returns `true` if this requires full consistency.
    pub fn is_full(&self) -> bool {
        matches!(self, ConsistencyRequirement::Full)
    }
}

impl From<ConsistencyToken> for ConsistencyRequirement {
    fn from(token: ConsistencyToken) -> Self {
        ConsistencyRequirement::AtLeastAsFresh(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistency_token_new() {
        let token = ConsistencyToken::new("abc123");
        assert_eq!(token.value(), "abc123");
        assert!(!token.is_empty());
        assert_eq!(token.len(), 6);
    }

    #[test]
    fn test_consistency_token_display() {
        let token = ConsistencyToken::new("xyz789");
        assert_eq!(token.to_string(), "xyz789");
    }

    #[test]
    fn test_consistency_token_from_str() {
        let token: ConsistencyToken = "test_token".parse().unwrap();
        assert_eq!(token.value(), "test_token");
    }

    #[test]
    fn test_consistency_token_from_str_empty() {
        let result = "".parse::<ConsistencyToken>();
        assert!(result.is_err());
    }

    #[test]
    fn test_consistency_token_from_string() {
        let token: ConsistencyToken = String::from("from_string").into();
        assert_eq!(token.value(), "from_string");
    }

    #[test]
    fn test_consistency_token_equality() {
        let t1 = ConsistencyToken::new("same");
        let t2 = ConsistencyToken::new("same");
        let t3 = ConsistencyToken::new("different");

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }

    #[test]
    fn test_consistency_token_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ConsistencyToken::new("token1"));
        set.insert(ConsistencyToken::new("token1"));
        set.insert(ConsistencyToken::new("token2"));

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_consistency_token_serialization() {
        let token = ConsistencyToken::new("serialized");
        let json = serde_json::to_string(&token).unwrap();
        let parsed: ConsistencyToken = serde_json::from_str(&json).unwrap();
        assert_eq!(token, parsed);
    }

    #[test]
    fn test_consistency_token_into_value() {
        let token = ConsistencyToken::new("owned_value");
        let value = token.into_value();
        assert_eq!(value, "owned_value");
    }

    #[test]
    fn test_consistency_requirement_default() {
        let req = ConsistencyRequirement::default();
        assert!(req.is_eventual());
        assert!(!req.is_full());
        assert!(req.token().is_none());
    }

    #[test]
    fn test_consistency_requirement_token() {
        let token = ConsistencyToken::new("my_token");
        let req = ConsistencyRequirement::AtLeastAsFresh(token.clone());
        assert_eq!(req.token(), Some(&token));
        assert!(!req.is_eventual());
        assert!(!req.is_full());
    }

    #[test]
    fn test_consistency_requirement_full() {
        let req = ConsistencyRequirement::Full;
        assert!(req.is_full());
        assert!(!req.is_eventual());
        assert!(req.token().is_none());
    }

    #[test]
    fn test_consistency_requirement_from_token() {
        let token = ConsistencyToken::new("test");
        let req: ConsistencyRequirement = token.clone().into();
        assert_eq!(req.token(), Some(&token));
    }

    #[test]
    fn test_as_ref() {
        let token = ConsistencyToken::new("ref_test");
        let s: &str = token.as_ref();
        assert_eq!(s, "ref_test");
    }
}
