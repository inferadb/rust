//! MockClient for testing with expectations.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{Error, testing::AuthorizationClient, types::Context};

/// A mock authorization client for testing.
///
/// `MockClient` allows you to set up expectations for authorization checks
/// and verify that they were called as expected.
///
/// ## Example
///
/// ```rust
/// use inferadb::testing::MockClient;
///
/// let mock = MockClient::new()
///     .expect_check("user:alice", "view", "doc:1", true)
///     .expect_check("user:bob", "edit", "doc:1", false);
///
/// // Use the mock in your tests...
/// // mock.verify() at the end to ensure all expectations were met
/// ```
#[derive(Clone)]
pub struct MockClient {
    expectations: Arc<Mutex<Vec<Expectation>>>,
    calls: Arc<Mutex<Vec<Call>>>,
    default_allow: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Expectation {
    subject: String,
    permission: String,
    resource: String,
    result: bool,
    times: Option<usize>,
}

#[derive(Debug, Clone)]
struct Call {
    subject: String,
    permission: String,
    resource: String,
}

impl MockClient {
    /// Creates a new mock client.
    pub fn new() -> Self {
        Self {
            expectations: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            default_allow: false,
        }
    }

    /// Creates a mock client that allows all by default.
    pub fn allow_all() -> Self {
        Self { default_allow: true, ..Self::new() }
    }

    /// Creates a mock client that denies all by default.
    pub fn deny_all() -> Self {
        Self { default_allow: false, ..Self::new() }
    }

    /// Adds an expectation for a check call.
    #[must_use]
    pub fn expect_check(
        self,
        subject: impl Into<String>,
        permission: impl Into<String>,
        resource: impl Into<String>,
        result: bool,
    ) -> Self {
        let expectation = Expectation {
            subject: subject.into(),
            permission: permission.into(),
            resource: resource.into(),
            result,
            times: None,
        };
        self.expectations.lock().unwrap().push(expectation);
        self
    }

    /// Verifies that all expectations were met.
    ///
    /// Call this at the end of your test to ensure all expected
    /// calls were made.
    ///
    /// # Panics
    ///
    /// Panics if any expectations were not met.
    pub fn verify(&self) {
        let expectations = self.expectations.lock().unwrap();
        let calls = self.calls.lock().unwrap();

        for expectation in expectations.iter() {
            let matching_calls = calls
                .iter()
                .filter(|c| {
                    c.subject == expectation.subject
                        && c.permission == expectation.permission
                        && c.resource == expectation.resource
                })
                .count();

            if matching_calls == 0 {
                panic!(
                    "Expected check({}, {}, {}) was never called",
                    expectation.subject, expectation.permission, expectation.resource
                );
            }
        }
    }

    /// Returns the number of check calls made.
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Clears all expectations and recorded calls.
    pub fn reset(&self) {
        self.expectations.lock().unwrap().clear();
        self.calls.lock().unwrap().clear();
    }

    fn find_result(&self, subject: &str, permission: &str, resource: &str) -> bool {
        let expectations = self.expectations.lock().unwrap();
        for expectation in expectations.iter() {
            if expectation.subject == subject
                && expectation.permission == permission
                && expectation.resource == resource
            {
                return expectation.result;
            }
        }
        self.default_allow
    }

    fn record_call(&self, subject: &str, permission: &str, resource: &str) {
        self.calls.lock().unwrap().push(Call {
            subject: subject.to_string(),
            permission: permission.to_string(),
            resource: resource.to_string(),
        });
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorizationClient for MockClient {
    fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
        self.record_call(subject, permission, resource);
        let result = self.find_result(subject, permission, resource);
        Box::pin(async move { Ok(result) })
    }

    fn check_with_context(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
        _context: &Context,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
        // For mock, we ignore context and use the same expectations
        self.check(subject, permission, resource)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_expectations() {
        let mock = MockClient::new()
            .expect_check("user:alice", "view", "doc:1", true)
            .expect_check("user:bob", "view", "doc:1", false);

        let result = mock.check("user:alice", "view", "doc:1").await.unwrap();
        assert!(result);

        let result = mock.check("user:bob", "view", "doc:1").await.unwrap();
        assert!(!result);

        mock.verify();
    }

    #[tokio::test]
    async fn test_mock_client_default_allow() {
        let mock = MockClient::allow_all();
        let result = mock.check("anyone", "anything", "anywhere").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_mock_client_default_deny() {
        let mock = MockClient::deny_all();
        let result = mock.check("anyone", "anything", "anywhere").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_mock_client_call_count() {
        let mock = MockClient::new();
        assert_eq!(mock.call_count(), 0);

        let _ = mock.check("user:a", "view", "doc:1").await;
        assert_eq!(mock.call_count(), 1);

        let _ = mock.check("user:b", "view", "doc:1").await;
        assert_eq!(mock.call_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_client_reset() {
        let mock = MockClient::new().expect_check("user:a", "view", "doc:1", true);
        let _ = mock.check("user:a", "view", "doc:1").await;
        assert_eq!(mock.call_count(), 1);

        mock.reset();
        assert_eq!(mock.call_count(), 0);
    }

    #[test]
    #[should_panic(expected = "Expected check")]
    fn test_mock_client_verify_fails() {
        let mock = MockClient::new().expect_check("user:alice", "view", "doc:1", true);

        // Never call check, so verify should panic
        mock.verify();
    }

    #[tokio::test]
    async fn test_mock_client_check_with_context() {
        use crate::types::Context;

        let mock = MockClient::allow_all();
        let context = Context::new().with("env", "test");

        let result =
            mock.check_with_context("user:alice", "view", "doc:1", &context).await.unwrap();
        assert!(result);
    }
}
