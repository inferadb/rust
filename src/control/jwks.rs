//! JWKS operations for the control plane.
//!
//! Provides operations for retrieving JSON Web Key Sets (JWKS) from
//! InferaDB for verifying tokens issued by the service.

use serde::{Deserialize, Serialize};

use crate::client::Client;
use crate::Error;

/// Client for JWKS operations.
///
/// Access via `client.jwks()`.
///
/// ## Example
///
/// ```rust,ignore
/// let jwks = client.jwks().get().await?;
/// if let Some(key) = jwks.find_key("key_id_123") {
///     // Use key for token verification
/// }
/// ```
#[derive(Clone)]
pub struct JwksClient {
    client: Client,
}

impl JwksClient {
    /// Creates a new JWKS client.
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Gets the JWKS for the current organization.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let jwks = client.jwks().get().await?;
    /// println!("Found {} keys", jwks.keys.len());
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get(&self) -> Result<Jwks, Error> {
        self.client.inner().control_get("/control/v1/jwks").await
    }

    /// Gets the JWKS for the current organization.
    #[cfg(not(feature = "rest"))]
    pub async fn get(&self) -> Result<Jwks, Error> {
        Err(Error::configuration("REST feature is required for JWKS"))
    }

    /// Gets the JWKS from the well-known endpoint.
    ///
    /// This fetches from `/.well-known/jwks.json`.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let jwks = client.jwks().get_well_known().await?;
    /// ```
    #[cfg(feature = "rest")]
    pub async fn get_well_known(&self) -> Result<Jwks, Error> {
        self.client
            .inner()
            .control_get("/.well-known/jwks.json")
            .await
    }

    /// Gets the JWKS from the well-known endpoint.
    #[cfg(not(feature = "rest"))]
    pub async fn get_well_known(&self) -> Result<Jwks, Error> {
        Err(Error::configuration("REST feature is required for JWKS"))
    }

    /// Gets a specific key by ID.
    ///
    /// This is a convenience method that fetches the JWKS and finds
    /// the key with the given ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// if let Some(key) = client.jwks().get_key("key_abc123").await? {
    ///     println!("Found key: {:?}", key);
    /// }
    /// ```
    pub async fn get_key(&self, kid: impl Into<String>) -> Result<Option<Jwk>, Error> {
        let kid = kid.into();
        let jwks = self.get().await?;
        Ok(jwks.find_key(&kid).cloned())
    }
}

impl std::fmt::Debug for JwksClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksClient").finish_non_exhaustive()
    }
}

/// A JSON Web Key Set (JWKS).
///
/// Contains a collection of public keys that can be used to verify
/// tokens issued by InferaDB.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Jwks {
    /// The keys in the set.
    pub keys: Vec<Jwk>,
}

impl Jwks {
    /// Creates a new empty JWKS.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a JWKS with the given keys.
    pub fn with_keys(keys: Vec<Jwk>) -> Self {
        Self { keys }
    }

    /// Finds a key by its ID.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// if let Some(key) = jwks.find_key("key_abc123") {
    ///     // Use the key for verification
    /// }
    /// ```
    pub fn find_key(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|k| k.kid.as_deref() == Some(kid))
    }

    /// Finds keys by algorithm.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let ed25519_keys = jwks.find_by_algorithm("EdDSA");
    /// ```
    pub fn find_by_algorithm(&self, alg: &str) -> Vec<&Jwk> {
        self.keys
            .iter()
            .filter(|k| k.alg.as_deref() == Some(alg))
            .collect()
    }

    /// Finds keys by use.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// let signing_keys = jwks.find_by_use("sig");
    /// ```
    pub fn find_by_use(&self, use_: &str) -> Vec<&Jwk> {
        self.keys
            .iter()
            .filter(|k| k.use_.as_deref() == Some(use_))
            .collect()
    }

    /// Returns the number of keys in the set.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns `true` if the key set is empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Returns an iterator over the keys.
    pub fn iter(&self) -> impl Iterator<Item = &Jwk> {
        self.keys.iter()
    }
}

impl IntoIterator for Jwks {
    type Item = Jwk;
    type IntoIter = std::vec::IntoIter<Jwk>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter()
    }
}

impl<'a> IntoIterator for &'a Jwks {
    type Item = &'a Jwk;
    type IntoIter = std::slice::Iter<'a, Jwk>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.iter()
    }
}

/// A JSON Web Key (JWK).
///
/// Represents a cryptographic key in JWK format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    /// Key type (e.g., "OKP" for Ed25519, "RSA").
    pub kty: String,

    /// Public key use (e.g., "sig" for signature).
    #[serde(rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// Key operations (e.g., ["sign", "verify"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_ops: Option<Vec<String>>,

    /// Algorithm (e.g., "EdDSA", "RS256").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,

    /// Key ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,

    /// X.509 URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5u: Option<String>,

    /// X.509 certificate chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5c: Option<Vec<String>>,

    /// X.509 certificate SHA-1 thumbprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5t: Option<String>,

    /// X.509 certificate SHA-256 thumbprint.
    #[serde(rename = "x5t#S256", skip_serializing_if = "Option::is_none")]
    pub x5t_s256: Option<String>,

    // RSA parameters
    /// RSA public key modulus (base64url).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,

    /// RSA public key exponent (base64url).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,

    // EC/OKP parameters
    /// Curve name (e.g., "Ed25519", "P-256").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,

    /// X coordinate (base64url).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,

    /// Y coordinate (base64url, for EC keys).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
}

impl Jwk {
    /// Creates a new JWK with the given key type.
    pub fn new(kty: impl Into<String>) -> Self {
        Self {
            kty: kty.into(),
            use_: None,
            key_ops: None,
            alg: None,
            kid: None,
            x5u: None,
            x5c: None,
            x5t: None,
            x5t_s256: None,
            n: None,
            e: None,
            crv: None,
            x: None,
            y: None,
        }
    }

    /// Creates a new Ed25519 JWK.
    pub fn ed25519(x: impl Into<String>) -> Self {
        Self {
            kty: "OKP".to_string(),
            crv: Some("Ed25519".to_string()),
            x: Some(x.into()),
            alg: Some("EdDSA".to_string()),
            use_: Some("sig".to_string()),
            ..Self::new("OKP")
        }
    }

    /// Creates a new RSA JWK.
    pub fn rsa(n: impl Into<String>, e: impl Into<String>) -> Self {
        Self {
            kty: "RSA".to_string(),
            n: Some(n.into()),
            e: Some(e.into()),
            alg: Some("RS256".to_string()),
            use_: Some("sig".to_string()),
            ..Self::new("RSA")
        }
    }

    /// Sets the key ID.
    #[must_use]
    pub fn with_kid(mut self, kid: impl Into<String>) -> Self {
        self.kid = Some(kid.into());
        self
    }

    /// Sets the algorithm.
    #[must_use]
    pub fn with_alg(mut self, alg: impl Into<String>) -> Self {
        self.alg = Some(alg.into());
        self
    }

    /// Sets the use.
    #[must_use]
    pub fn with_use(mut self, use_: impl Into<String>) -> Self {
        self.use_ = Some(use_.into());
        self
    }

    /// Returns `true` if this is an Ed25519 key.
    pub fn is_ed25519(&self) -> bool {
        self.kty == "OKP" && self.crv.as_deref() == Some("Ed25519")
    }

    /// Returns `true` if this is an RSA key.
    pub fn is_rsa(&self) -> bool {
        self.kty == "RSA"
    }

    /// Returns `true` if this is an EC key.
    pub fn is_ec(&self) -> bool {
        self.kty == "EC"
    }

    /// Returns `true` if this key is for signing.
    pub fn is_signing_key(&self) -> bool {
        self.use_.as_deref() == Some("sig")
            || self
                .key_ops
                .as_ref()
                .is_some_and(|ops| ops.iter().any(|op| op == "sign" || op == "verify"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BearerCredentialsConfig;

    async fn create_test_client() -> Client {
        Client::builder()
            .url("https://api.example.com")
            .credentials(BearerCredentialsConfig::new("test"))
            .build()
            .await
            .unwrap()
    }

    #[test]
    fn test_jwks_empty() {
        let jwks = Jwks::new();
        assert!(jwks.is_empty());
        assert_eq!(jwks.len(), 0);
        assert!(jwks.find_key("any").is_none());
    }

    #[test]
    fn test_jwks_with_keys() {
        let jwks = Jwks::with_keys(vec![
            Jwk::ed25519("x_value").with_kid("key1"),
            Jwk::rsa("n_value", "e_value")
                .with_kid("key2")
                .with_alg("RS256"),
        ]);

        assert!(!jwks.is_empty());
        assert_eq!(jwks.len(), 2);

        let key1 = jwks.find_key("key1").unwrap();
        assert!(key1.is_ed25519());

        let key2 = jwks.find_key("key2").unwrap();
        assert!(key2.is_rsa());

        assert!(jwks.find_key("nonexistent").is_none());
    }

    #[test]
    fn test_jwks_find_by_algorithm() {
        let jwks = Jwks::with_keys(vec![
            Jwk::ed25519("x1").with_kid("key1").with_alg("EdDSA"),
            Jwk::ed25519("x2").with_kid("key2").with_alg("EdDSA"),
            Jwk::rsa("n", "e").with_kid("key3").with_alg("RS256"),
        ]);

        let eddsa_keys = jwks.find_by_algorithm("EdDSA");
        assert_eq!(eddsa_keys.len(), 2);

        let rsa_keys = jwks.find_by_algorithm("RS256");
        assert_eq!(rsa_keys.len(), 1);

        let ps256_keys = jwks.find_by_algorithm("PS256");
        assert!(ps256_keys.is_empty());
    }

    #[test]
    fn test_jwks_find_by_use() {
        let jwks = Jwks::with_keys(vec![
            Jwk::ed25519("x1").with_kid("key1").with_use("sig"),
            Jwk::ed25519("x2").with_kid("key2").with_use("enc"),
        ]);

        let sig_keys = jwks.find_by_use("sig");
        assert_eq!(sig_keys.len(), 1);

        let enc_keys = jwks.find_by_use("enc");
        assert_eq!(enc_keys.len(), 1);
    }

    #[test]
    fn test_jwks_iteration() {
        let jwks = Jwks::with_keys(vec![
            Jwk::ed25519("x1").with_kid("key1"),
            Jwk::ed25519("x2").with_kid("key2"),
        ]);

        let mut count = 0;
        for key in &jwks {
            assert!(key.is_ed25519());
            count += 1;
        }
        assert_eq!(count, 2);

        // Into iterator
        let keys: Vec<Jwk> = jwks.into_iter().collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_jwk_ed25519() {
        let jwk = Jwk::ed25519("base64url_x").with_kid("my_key");

        assert!(jwk.is_ed25519());
        assert!(!jwk.is_rsa());
        assert!(!jwk.is_ec());
        assert!(jwk.is_signing_key());
        assert_eq!(jwk.kty, "OKP");
        assert_eq!(jwk.crv, Some("Ed25519".to_string()));
        assert_eq!(jwk.x, Some("base64url_x".to_string()));
        assert_eq!(jwk.kid, Some("my_key".to_string()));
    }

    #[test]
    fn test_jwk_rsa() {
        let jwk = Jwk::rsa("base64url_n", "AQAB").with_kid("rsa_key");

        assert!(!jwk.is_ed25519());
        assert!(jwk.is_rsa());
        assert!(!jwk.is_ec());
        assert!(jwk.is_signing_key());
        assert_eq!(jwk.kty, "RSA");
        assert_eq!(jwk.n, Some("base64url_n".to_string()));
        assert_eq!(jwk.e, Some("AQAB".to_string()));
    }

    #[test]
    fn test_jwk_serialization() {
        let jwk = Jwk::ed25519("x_value").with_kid("key1").with_alg("EdDSA");

        let json = serde_json::to_string(&jwk).unwrap();
        assert!(json.contains("\"kty\":\"OKP\""));
        assert!(json.contains("\"crv\":\"Ed25519\""));
        assert!(json.contains("\"kid\":\"key1\""));
        assert!(json.contains("\"alg\":\"EdDSA\""));

        let parsed: Jwk = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_ed25519());
        assert_eq!(parsed.kid, Some("key1".to_string()));
    }

    #[test]
    fn test_jwks_serialization() {
        let jwks = Jwks::with_keys(vec![Jwk::ed25519("x").with_kid("key1")]);

        let json = serde_json::to_string(&jwks).unwrap();
        assert!(json.contains("\"keys\""));

        let parsed: Jwks = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[tokio::test]
    async fn test_jwks_client_debug() {
        let client = create_test_client().await;
        let jwks_client = JwksClient::new(client);
        assert!(format!("{:?}", jwks_client).contains("JwksClient"));
    }
}
