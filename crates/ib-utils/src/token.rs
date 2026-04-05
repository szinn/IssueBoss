use std::{fmt, marker::PhantomData, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Marker trait for token prefixes. Implement via [`define_token_prefix!`].
pub trait TokenPrefix: fmt::Debug + Clone + PartialEq + Eq {
    const PREFIX: &'static str;
}

/// A typed, prefixed token ID.
///
/// `P` — zero-size prefix marker (see [`define_token_prefix!`])
/// `Id` — underlying unsigned integer (typically `u64`)
/// `MAX` — maximum legal value (enforced at construction)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token<P, Id, const MAX: u128> {
    id: Id,
    _phantom: PhantomData<P>,
}

impl<P: TokenPrefix, const MAX: u128> Token<P, u64, MAX> {
    /// Construct from a raw id. Panics if `id as u128 > MAX`.
    pub fn from_id(id: u64) -> Self {
        assert!(id as u128 <= MAX, "Token id {id} exceeds maximum {MAX} for prefix {}", P::PREFIX,);
        Self { id, _phantom: PhantomData }
    }

    /// Returns the underlying id.
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl<P: TokenPrefix, const MAX: u128> fmt::Display for Token<P, u64, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", P::PREFIX, self.id)
    }
}

/// Error returned when a token string cannot be parsed.
#[derive(Debug, Error)]
pub enum TokenParseError {
    #[error("expected prefix '{expected}', got '{actual}'")]
    WrongPrefix { expected: &'static str, actual: String },
    #[error("id portion is not a valid integer: {0}")]
    InvalidId(String),
    #[error("id exceeds maximum allowed value")]
    ExceedsMax,
}

impl<P: TokenPrefix, const MAX: u128> FromStr for Token<P, u64, MAX> {
    type Err = TokenParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let prefix = P::PREFIX;
        let id_str = s.strip_prefix(prefix).ok_or_else(|| TokenParseError::WrongPrefix {
            expected: prefix,
            actual: s.to_owned(),
        })?;
        let id: u64 = id_str.parse().map_err(|_| TokenParseError::InvalidId(id_str.to_owned()))?;
        if id as u128 > MAX {
            return Err(TokenParseError::ExceedsMax);
        }
        Ok(Self::from_id(id))
    }
}

impl<P: TokenPrefix, const MAX: u128> Serialize for Token<P, u64, MAX> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, P: TokenPrefix, const MAX: u128> Deserialize<'de> for Token<P, u64, MAX> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Define a zero-size token prefix type.
///
/// # Example
/// ```rust
/// use ib_utils::{define_token_prefix, Token};
/// define_token_prefix!(UserPrefix, "U_");
/// pub type UserId    = u64;
/// pub type UserToken = Token<UserPrefix, UserId, { i64::MAX as u128 }>;
/// ```
#[macro_export]
macro_rules! define_token_prefix {
    ($name:ident, $prefix:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name;

        impl $crate::token::TokenPrefix for $name {
            const PREFIX: &'static str = $prefix;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    define_token_prefix!(TestPrefix, "T_");
    type TestToken = Token<TestPrefix, u64, { i64::MAX as u128 }>;

    #[test]
    fn token_displays_with_prefix() {
        let t = TestToken::from_id(42);
        assert_eq!(t.to_string(), "T_42");
    }

    #[test]
    fn token_roundtrips_from_str() {
        let t: TestToken = "T_99".parse().unwrap();
        assert_eq!(t.id(), 99u64);
    }

    #[test]
    fn token_parse_wrong_prefix_fails() {
        let result: Result<TestToken, _> = "U_99".parse();
        assert!(result.is_err());
    }

    #[test]
    fn token_roundtrips_serde() {
        let t = TestToken::from_id(7);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#""T_7""#);
        let t2: TestToken = serde_json::from_str(&json).unwrap();
        assert_eq!(t2.id(), 7u64);
    }
}
