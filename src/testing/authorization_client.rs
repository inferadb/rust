//! AuthorizationClient trait for dependency injection.

use std::future::Future;
use std::pin::Pin;

use crate::types::Context;
use crate::Error;

/// Object-safe trait for authorization operations.
///
/// This trait allows you to abstract over different authorization clients
/// (real, mock, in-memory) for testing and dependency injection.
///
/// ## Example
///
/// ```rust
/// use inferadb::testing::AuthorizationClient;
/// use inferadb::Error;
///
/// // Function that works with any authorization client
/// async fn check_access(
///     client: &dyn AuthorizationClient,
///     user_id: &str,
/// ) -> Result<bool, Error> {
///     client.check(
///         &format!("user:{}", user_id),
///         "view",
///         "dashboard:main",
///     ).await
/// }
///
/// // In production, use the real client
/// // In tests, use MockClient or InMemoryClient
/// ```
///
/// ## Object Safety
///
/// This trait is object-safe, so you can use `&dyn AuthorizationClient`
/// or `Box<dyn AuthorizationClient>` for dynamic dispatch.
pub trait AuthorizationClient: Send + Sync {
    /// Checks if a subject has a permission on a resource.
    ///
    /// Returns `Ok(true)` if allowed, `Ok(false)` if denied.
    fn check(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>>;

    /// Checks with ABAC context.
    fn check_with_context(
        &self,
        subject: &str,
        permission: &str,
        resource: &str,
        context: &Context,
    ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestClient {
        allow_all: bool,
    }

    impl AuthorizationClient for TestClient {
        fn check(
            &self,
            _subject: &str,
            _permission: &str,
            _resource: &str,
        ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
            let result = self.allow_all;
            Box::pin(async move { Ok(result) })
        }

        fn check_with_context(
            &self,
            _subject: &str,
            _permission: &str,
            _resource: &str,
            _context: &Context,
        ) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send + '_>> {
            let result = self.allow_all;
            Box::pin(async move { Ok(result) })
        }
    }

    #[tokio::test]
    async fn test_trait_object() {
        let client: Box<dyn AuthorizationClient> = Box::new(TestClient { allow_all: true });
        let result = client.check("user:alice", "view", "doc:1").await;
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_deny() {
        let client = TestClient { allow_all: false };
        let result = client.check("user:alice", "view", "doc:1").await;
        assert!(!result.unwrap());
    }
}
