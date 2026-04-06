use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{DateTime, Utc};
use ib_utils::{Token, define_token_prefix};
use sha2::{Digest, Sha256};

use crate::user::UserId;

define_token_prefix!(ApiKeyTokenPrefix, "AK_");

pub type ApiKeyId = u64;
pub type ApiKeyToken = Token<ApiKeyTokenPrefix, ApiKeyId, { i64::MAX as u128 }>;

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub token: ApiKeyToken,
    pub user_id: UserId,
    pub key_type: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewApiKey {
    pub user_id: UserId,
    pub key_type: String,
    pub name: Option<String>,
}

/// A freshly generated API key — all representations needed for storage.
pub struct GeneratedApiKey {
    /// The plaintext key — return to caller once, then discard.
    pub plaintext: String,
    /// SHA-256 hex of `plaintext` — stored in `api_keys.key_hash`.
    pub hash: String,
    /// First 12 chars of `plaintext` — stored in `api_keys.key_prefix` for
    /// display.
    pub prefix: String,
}

/// Generate a new `{key_type}_{32 hex chars}` API key.
pub fn generate_api_key(key_type: &str) -> GeneratedApiKey {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let random_part: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let plaintext = format!("{key_type}_{random_part}");
    let hash = sha256_hex(&plaintext);
    let prefix = plaintext[..12.min(plaintext.len())].to_owned();
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
        let key = generate_api_key("ib_live");
        assert!(key.plaintext.starts_with("ib_live_"), "expected ib_live_ prefix");
        assert_eq!(key.plaintext.len(), 40, "expected 40 chars total");
        assert_eq!(key.hash.len(), 64, "expected 64-char SHA-256 hex");
        assert_eq!(key.prefix, &key.plaintext[..12]);
    }

    #[test]
    fn key_type_is_configurable() {
        let key = generate_api_key("myapp_test");
        assert!(key.plaintext.starts_with("myapp_test_"));
    }

    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }
}
