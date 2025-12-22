//! Context type for ABAC (Attribute-Based Access Control).

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A value that can be passed in an ABAC context.
///
/// Context values are used to evaluate attribute-based conditions in
/// permission checks. They support the common JSON-compatible types.
///
/// # Example
///
/// ```rust
/// use inferadb::ContextValue;
///
/// let string_val: ContextValue = "production".into();
/// let number_val: ContextValue = 42.into();
/// let bool_val: ContextValue = true.into();
/// let null_val = ContextValue::Null;
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ContextValue {
    /// Null value.
    #[default]
    Null,

    /// Boolean value.
    Bool(bool),

    /// Integer value (64-bit signed).
    Integer(i64),

    /// Floating-point value (64-bit).
    Float(f64),

    /// String value.
    String(String),

    /// Array of values.
    Array(Vec<ContextValue>),

    /// Nested object.
    Object(HashMap<String, ContextValue>),
}

impl ContextValue {
    /// Returns `true` if this is a null value.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, ContextValue::Null)
    }

    /// Returns the boolean value if this is a Bool variant.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the integer value if this is an Integer variant.
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ContextValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the float value if this is a Float variant.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ContextValue::Float(f) => Some(*f),
            ContextValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Returns the string value if this is a String variant.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ContextValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the array if this is an Array variant.
    #[inline]
    pub fn as_array(&self) -> Option<&[ContextValue]> {
        match self {
            ContextValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns the object if this is an Object variant.
    #[inline]
    pub fn as_object(&self) -> Option<&HashMap<String, ContextValue>> {
        match self {
            ContextValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
}


impl From<bool> for ContextValue {
    fn from(value: bool) -> Self {
        ContextValue::Bool(value)
    }
}

impl From<i32> for ContextValue {
    fn from(value: i32) -> Self {
        ContextValue::Integer(value as i64)
    }
}

impl From<i64> for ContextValue {
    fn from(value: i64) -> Self {
        ContextValue::Integer(value)
    }
}

impl From<f64> for ContextValue {
    fn from(value: f64) -> Self {
        ContextValue::Float(value)
    }
}

impl From<&str> for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue::String(value.to_owned())
    }
}

impl From<String> for ContextValue {
    fn from(value: String) -> Self {
        ContextValue::String(value)
    }
}

impl<T: Into<ContextValue>> From<Vec<T>> for ContextValue {
    fn from(value: Vec<T>) -> Self {
        ContextValue::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<ContextValue>> From<Option<T>> for ContextValue {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => ContextValue::Null,
        }
    }
}

impl fmt::Display for ContextValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextValue::Null => write!(f, "null"),
            ContextValue::Bool(b) => write!(f, "{}", b),
            ContextValue::Integer(i) => write!(f, "{}", i),
            ContextValue::Float(fl) => write!(f, "{}", fl),
            ContextValue::String(s) => write!(f, "\"{}\"", s),
            ContextValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            ContextValue::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// ABAC context for attribute-based authorization conditions.
///
/// Context provides dynamic attributes that can be evaluated in permission
/// checks. These attributes are evaluated against conditions defined in
/// the authorization schema.
///
/// ## Common Use Cases
///
/// - **Time-based access**: Check if current time is within business hours
/// - **Location-based access**: Verify user's IP is in allowed range
/// - **Environment checks**: Production vs. development access rules
/// - **Dynamic attributes**: User's current subscription tier
///
/// ## Example
///
/// ```rust
/// use inferadb::{Context, ContextValue};
///
/// // Build context for a permission check
/// let context = Context::new()
///     .with("environment", "production")
///     .with("user_tier", "premium")
///     .with("request_ip", "192.168.1.100")
///     .with("is_business_hours", true);
///
/// // Use with check
/// // vault.check("user:alice", "access", "resource:data")
/// //     .with_context(context)
/// //     .await?;
/// ```
///
/// ## Nested Values
///
/// Context supports nested structures:
///
/// ```rust
/// use inferadb::{Context, ContextValue};
/// use std::collections::HashMap;
///
/// let mut user_attrs = HashMap::new();
/// user_attrs.insert("department".to_string(), ContextValue::from("engineering"));
/// user_attrs.insert("level".to_string(), ContextValue::from(5));
///
/// let context = Context::new()
///     .with("user", ContextValue::Object(user_attrs));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Context {
    #[serde(flatten)]
    values: HashMap<String, ContextValue>,
}

impl Context {
    /// Creates an empty context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Context;
    ///
    /// let context = Context::new();
    /// assert!(context.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Creates a context with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: HashMap::with_capacity(capacity),
        }
    }

    /// Adds a key-value pair to the context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Context;
    ///
    /// let context = Context::new()
    ///     .with("environment", "production")
    ///     .with("debug", false)
    ///     .with("max_retries", 3);
    /// ```
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<ContextValue>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Inserts a key-value pair, mutating the context.
    ///
    /// Returns the previous value if the key was present.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<ContextValue>,
    ) -> Option<ContextValue> {
        self.values.insert(key.into(), value.into())
    }

    /// Gets a value by key.
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.values.get(key)
    }

    /// Removes a value by key.
    pub fn remove(&mut self, key: &str) -> Option<ContextValue> {
        self.values.remove(key)
    }

    /// Returns `true` if the context contains the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Returns `true` if the context is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the number of entries in the context.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns an iterator over the context entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ContextValue)> {
        self.values.iter()
    }

    /// Extends this context with entries from another context.
    ///
    /// Existing keys are overwritten.
    pub fn extend(&mut self, other: Context) {
        self.values.extend(other.values);
    }

    /// Merges another context into this one, returning a new context.
    ///
    /// Entries from `other` overwrite entries from `self`.
    #[must_use]
    pub fn merge(mut self, other: Context) -> Self {
        self.extend(other);
        self
    }
}

impl FromIterator<(String, ContextValue)> for Context {
    fn from_iter<T: IntoIterator<Item = (String, ContextValue)>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Context {
    type Item = (String, ContextValue);
    type IntoIter = std::collections::hash_map::IntoIter<String, ContextValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a Context {
    type Item = (&'a String, &'a ContextValue);
    type IntoIter = std::collections::hash_map::Iter<'a, String, ContextValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_value_types() {
        assert!(ContextValue::Null.is_null());
        assert_eq!(ContextValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ContextValue::Integer(42).as_i64(), Some(42));
        assert_eq!(ContextValue::Float(2.5).as_f64(), Some(2.5));
        assert_eq!(ContextValue::String("test".into()).as_str(), Some("test"));
    }

    #[test]
    fn test_context_value_conversions() {
        let b: ContextValue = true.into();
        assert_eq!(b.as_bool(), Some(true));

        let i: ContextValue = 42i32.into();
        assert_eq!(i.as_i64(), Some(42));

        let f: ContextValue = 2.5.into();
        assert_eq!(f.as_f64(), Some(2.5));

        let s: ContextValue = "hello".into();
        assert_eq!(s.as_str(), Some("hello"));

        let arr: ContextValue = vec![1i32, 2, 3].into();
        assert!(arr.as_array().is_some());
    }

    #[test]
    fn test_context_new() {
        let ctx = Context::new();
        assert!(ctx.is_empty());
        assert_eq!(ctx.len(), 0);
    }

    #[test]
    fn test_context_with() {
        let ctx = Context::new()
            .with("env", "prod")
            .with("debug", false)
            .with("count", 10);

        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx.get("env").and_then(|v| v.as_str()), Some("prod"));
        assert_eq!(ctx.get("debug").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(ctx.get("count").and_then(|v| v.as_i64()), Some(10));
    }

    #[test]
    fn test_context_insert() {
        let mut ctx = Context::new();
        assert!(ctx.insert("key", "value1").is_none());
        assert!(ctx.insert("key", "value2").is_some());
        assert_eq!(ctx.get("key").and_then(|v| v.as_str()), Some("value2"));
    }

    #[test]
    fn test_context_remove() {
        let mut ctx = Context::new().with("key", "value");
        assert!(ctx.remove("key").is_some());
        assert!(ctx.remove("key").is_none());
        assert!(!ctx.contains_key("key"));
    }

    #[test]
    fn test_context_merge() {
        let ctx1 = Context::new().with("a", 1).with("b", 2);
        let ctx2 = Context::new().with("b", 3).with("c", 4);

        let merged = ctx1.merge(ctx2);
        assert_eq!(merged.get("a").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(merged.get("b").and_then(|v| v.as_i64()), Some(3)); // Overwritten
        assert_eq!(merged.get("c").and_then(|v| v.as_i64()), Some(4));
    }

    #[test]
    fn test_context_iteration() {
        let ctx = Context::new().with("a", 1).with("b", 2);

        let keys: Vec<_> = ctx.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"a"));
        assert!(keys.contains(&"b"));
    }

    #[test]
    fn test_context_serialization() {
        let ctx = Context::new()
            .with("string", "hello")
            .with("number", 42)
            .with("bool", true);

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: Context = serde_json::from_str(&json).unwrap();

        assert_eq!(ctx, parsed);
    }

    #[test]
    fn test_context_value_display() {
        assert_eq!(ContextValue::Null.to_string(), "null");
        assert_eq!(ContextValue::Bool(true).to_string(), "true");
        assert_eq!(ContextValue::Integer(42).to_string(), "42");
        assert_eq!(ContextValue::String("test".into()).to_string(), "\"test\"");
    }

    #[test]
    fn test_nested_context() {
        let mut inner = HashMap::new();
        inner.insert("nested_key".to_string(), ContextValue::from("nested_value"));

        let ctx = Context::new()
            .with("outer", ContextValue::Object(inner));

        let obj = ctx.get("outer").and_then(|v| v.as_object()).unwrap();
        assert_eq!(obj.get("nested_key").and_then(|v| v.as_str()), Some("nested_value"));
    }

    #[test]
    fn test_array_context_value() {
        let arr = ContextValue::from(vec!["a", "b", "c"]);
        let values = arr.as_array().unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0].as_str(), Some("a"));
    }

    #[test]
    fn test_option_conversion() {
        let some: ContextValue = Some("value").into();
        assert_eq!(some.as_str(), Some("value"));

        let none: ContextValue = Option::<String>::None.into();
        assert!(none.is_null());
    }

    #[test]
    fn test_from_iterator() {
        let pairs = vec![
            ("a".to_string(), ContextValue::from(1)),
            ("b".to_string(), ContextValue::from(2)),
        ];
        let ctx: Context = pairs.into_iter().collect();
        assert_eq!(ctx.len(), 2);
    }
}
