//! API key generation and hashing utilities.
//!
//! Keys have the format `ib_live_<32 lowercase hex chars>` (40 chars total).
//! Only the SHA-256 hash is stored in the database; the plaintext is returned
//! once and never stored.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

/// A freshly generated API key — all representations needed for storage.
pub struct GeneratedApiKey {
    /// The plaintext key — return to caller once, then discard.
    pub plaintext: String,
    /// SHA-256 hex of `plaintext` — stored in `users.api_key_hash`.
    pub hash: String,
    /// First 12 chars of `plaintext` — stored in `users.api_key_prefix` for UI
    /// display.
    pub prefix: String,
}

/// Generate a new `ib_live_<32 hex chars>` API key.
pub fn generate_api_key() -> GeneratedApiKey {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let random_part: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let plaintext = format!("ib_live_{random_part}");
    let hash = sha256_hex(&plaintext);
    let prefix = plaintext[..12].to_owned(); // "ib_live_XXXX"
    GeneratedApiKey { plaintext, hash, prefix }
}

/// Compute the SHA-256 hex digest of `input`.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_correct_format() {
        let key = generate_api_key();
        assert!(key.plaintext.starts_with("ib_live_"), "expected ib_live_ prefix");
        assert_eq!(key.plaintext.len(), 40, "expected 40 chars total");
        assert_eq!(key.hash.len(), 64, "expected 64-char SHA-256 hex");
        assert_eq!(key.prefix.len(), 12);
        assert_eq!(&key.prefix, &key.plaintext[..12]);
    }

    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    #[test]
    fn sha256_hex_known_value() {
        // SHA-256("") =
        // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(sha256_hex(""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }
}
