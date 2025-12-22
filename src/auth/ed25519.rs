//! Ed25519 private key handling for JWT signing.

use std::fmt;
use std::path::Path;

use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};
use zeroize::Zeroizing;

use crate::Error;

/// An Ed25519 private key for signing JWTs.
///
/// This type wraps an Ed25519 signing key with secure memory handling:
/// - Key material is zeroized on drop
/// - Debug output hides key contents
/// - Clone is disabled to prevent accidental key duplication
///
/// ## Loading Keys
///
/// Keys can be loaded from PEM files, raw bytes, or hex strings:
///
/// ```rust,ignore
/// use inferadb::Ed25519PrivateKey;
///
/// // From PEM file (recommended for production)
/// let key = Ed25519PrivateKey::from_pem_file("private-key.pem")?;
///
/// // From PEM string
/// let pem = std::fs::read_to_string("private-key.pem")?;
/// let key = Ed25519PrivateKey::from_pem(&pem)?;
///
/// // From raw bytes (32 bytes)
/// let key = Ed25519PrivateKey::from_bytes(&key_bytes)?;
///
/// // From hex string
/// let key = Ed25519PrivateKey::from_hex("deadbeef...")?;
/// ```
///
/// ## Generating Keys
///
/// For development, you can generate a new random key:
///
/// ```rust
/// use inferadb::Ed25519PrivateKey;
///
/// let key = Ed25519PrivateKey::generate();
/// ```
///
/// ## Security Notes
///
/// - Never log or serialize private keys
/// - Store keys securely (encrypted at rest, environment variables, or HSM)
/// - Rotate keys periodically
/// - Use certificate binding when available
pub struct Ed25519PrivateKey {
    /// The Ed25519 signing key.
    key: SigningKey,
}

impl Ed25519PrivateKey {
    /// Generates a new random Ed25519 private key.
    ///
    /// This is useful for development and testing. For production,
    /// use keys generated through secure key management.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let key = Ed25519PrivateKey::generate();
    /// let public_key = key.public_key_bytes();
    /// println!("Public key: {}", hex::encode(public_key));
    /// ```
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        Self {
            key: SigningKey::generate(&mut csprng),
        }
    }

    /// Creates a key from raw bytes.
    ///
    /// The bytes must be exactly 32 bytes (256 bits).
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not exactly 32 bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let bytes = [0u8; 32]; // In practice, use real key material
    /// let key = Ed25519PrivateKey::from_bytes(&bytes)?;
    /// # Ok::<(), inferadb::Error>(())
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(Error::configuration(format!(
                "Ed25519 private key must be {} bytes, got {}",
                SECRET_KEY_LENGTH,
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; SECRET_KEY_LENGTH];
        key_bytes.copy_from_slice(bytes);

        // Wrap in Zeroizing for secure cleanup
        let zeroizing_bytes = Zeroizing::new(key_bytes);

        Ok(Self {
            key: SigningKey::from_bytes(&zeroizing_bytes),
        })
    }

    /// Creates a key from a hex-encoded string.
    ///
    /// # Errors
    ///
    /// Returns an error if the hex is invalid or wrong length.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let key = Ed25519PrivateKey::from_hex("0102030405...")?;
    /// ```
    pub fn from_hex(hex_str: &str) -> Result<Self, Error> {
        let bytes = hex::decode(hex_str).map_err(|e| {
            Error::configuration(format!("invalid hex string for Ed25519 key: {}", e))
        })?;

        Self::from_bytes(&bytes)
    }

    /// Loads a key from a PEM-encoded string.
    ///
    /// Supports PKCS#8 format:
    /// ```text
    /// -----BEGIN PRIVATE KEY-----
    /// ...base64 encoded key...
    /// -----END PRIVATE KEY-----
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the PEM format is invalid.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let pem = std::fs::read_to_string("private-key.pem")?;
    /// let key = Ed25519PrivateKey::from_pem(&pem)?;
    /// ```
    pub fn from_pem(pem: &str) -> Result<Self, Error> {
        use ed25519_dalek::pkcs8::DecodePrivateKey;

        let key = SigningKey::from_pkcs8_pem(pem)
            .map_err(|e| Error::configuration(format!("failed to parse Ed25519 PEM: {}", e)))?;

        Ok(Self { key })
    }

    /// Loads a key from a PEM file.
    ///
    /// This is the recommended way to load production keys.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid PEM.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let key = Ed25519PrivateKey::from_pem_file("keys/private.pem")?;
    /// ```
    pub fn from_pem_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let pem = std::fs::read_to_string(path).map_err(|e| {
            Error::configuration(format!(
                "failed to read Ed25519 key file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Wrap in Zeroizing for secure cleanup of the PEM string
        let pem = Zeroizing::new(pem);

        Self::from_pem(&pem)
    }

    /// Returns the public key bytes (32 bytes).
    ///
    /// This can be shared publicly and is used for signature verification.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    /// Returns the public key as a hex string.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }

    /// Signs a message and returns the signature bytes (64 bytes).
    ///
    /// # Example
    ///
    /// ```rust
    /// use inferadb::Ed25519PrivateKey;
    ///
    /// let key = Ed25519PrivateKey::generate();
    /// let signature = key.sign(b"message to sign");
    /// assert_eq!(signature.len(), 64);
    /// ```
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        use ed25519_dalek::Signer;
        self.key.sign(message).to_bytes()
    }

    /// Signs a message and returns the signature as a hex string.
    pub fn sign_hex(&self, message: &[u8]) -> String {
        hex::encode(self.sign(message))
    }

    /// Signs a message and returns the signature as a base64url string (for JWT).
    pub fn sign_base64url(&self, message: &[u8]) -> String {
        use base64::prelude::*;
        BASE64_URL_SAFE_NO_PAD.encode(self.sign(message))
    }

    /// Returns a reference to the internal signing key.
    ///
    /// This is provided for advanced use cases that need direct access
    /// to the ed25519-dalek key type.
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.key
    }
}

// Explicitly implement Drop to ensure key is zeroized
// Note: SigningKey already implements ZeroizeOnDrop via ed25519-dalek's zeroize feature

impl fmt::Debug for Ed25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ed25519PrivateKey")
            .field("public_key", &self.public_key_hex())
            .finish_non_exhaustive()
    }
}

// Clone is intentionally NOT implemented to prevent accidental key duplication

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate() {
        let key = Ed25519PrivateKey::generate();
        let public = key.public_key_bytes();
        assert_eq!(public.len(), 32);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = [42u8; 32];
        let key = Ed25519PrivateKey::from_bytes(&bytes).unwrap();
        assert_eq!(key.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_from_bytes_wrong_length() {
        let bytes = [0u8; 16];
        let result = Ed25519PrivateKey::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_hex() {
        let hex_key = "00".repeat(32);
        let key = Ed25519PrivateKey::from_hex(&hex_key).unwrap();
        assert_eq!(key.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_from_hex_invalid() {
        let result = Ed25519PrivateKey::from_hex("not_hex");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign() {
        let key = Ed25519PrivateKey::generate();
        let signature = key.sign(b"test message");
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_sign_deterministic() {
        let bytes = [1u8; 32];
        let key = Ed25519PrivateKey::from_bytes(&bytes).unwrap();
        let sig1 = key.sign(b"message");
        let sig2 = key.sign(b"message");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_different_messages() {
        let key = Ed25519PrivateKey::generate();
        let sig1 = key.sign(b"message1");
        let sig2 = key.sign(b"message2");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_public_key_hex() {
        let key = Ed25519PrivateKey::generate();
        let hex = key.public_key_hex();
        assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_debug_hides_key() {
        let key = Ed25519PrivateKey::generate();
        let debug = format!("{:?}", key);
        assert!(debug.contains("Ed25519PrivateKey"));
        assert!(debug.contains("public_key"));
        // Should not contain raw key bytes
        assert!(!debug.contains("[0"));
    }

    #[test]
    fn test_sign_base64url() {
        let key = Ed25519PrivateKey::generate();
        let sig = key.sign_base64url(b"test");
        // Base64url signature should be 86 chars (64 bytes = 86 base64url chars without padding)
        assert_eq!(sig.len(), 86);
        // Should not contain padding or standard base64 chars
        assert!(!sig.contains('='));
        assert!(!sig.contains('+'));
        assert!(!sig.contains('/'));
    }
}
