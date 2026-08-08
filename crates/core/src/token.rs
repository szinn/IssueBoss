//! Token type for entity identifiers, backed by the `alderkit-token` crate.
//!
//! `TokenAlphabet` must stay byte-for-byte identical to the alphabet the
//! original `ib-utils` `Token` implementation used
//! (`Y4XK0N8AR3G6JM2VT9BS5WC1DPH7EUZF`) — changing it would make every
//! already-persisted token unparseable.

alderkit_token::define_alphabet!(TokenAlphabet, b"Y4XK0N8AR3G6JM2VT9BS5WC1DPH7EUZF");

pub use alderkit_token::{
    define_token_prefix,
    token::{Token, TokenError, TokenPrefix},
};

#[cfg(test)]
mod tests {
    use super::*;

    define_token_prefix!(TestPrefix, "T_");
    type TestToken = Token<TestPrefix, u64, TokenAlphabet>;

    define_token_prefix!(UserPrefix, "U_");
    type UserId = u64;
    type UserToken = Token<UserPrefix, UserId, TokenAlphabet>;

    #[test]
    fn round_trip() {
        for id in [0, 1, 42, 1000, 123_456_789, u64::MAX] {
            let token = TestToken::new(id);
            let s = token.to_string();
            let parsed = TestToken::parse(&s).unwrap();
            assert_eq!(parsed.id(), id);
        }
    }

    #[test]
    fn zero_encodes_to_all_first_char() {
        let token = TestToken::new(0);
        assert_eq!(token.to_string(), "T_YYYYYYYYYYYYY");
    }

    #[test]
    fn u64_max_round_trips() {
        let token = TestToken::new(u64::MAX);
        let s = token.to_string();
        let parsed = TestToken::parse(&s).unwrap();
        assert_eq!(parsed.id(), u64::MAX);
    }

    #[test]
    fn known_value_encoding() {
        let token = TestToken::new(1);
        let s = token.to_string();
        assert_eq!(s, "T_YYYYYYYYYYYY4");
    }

    #[test]
    fn wrong_prefix_error() {
        let err = UserToken::parse("T_AAAAAAAAAAAAA").unwrap_err();
        assert_eq!(
            err,
            TokenError::InvalidPrefix {
                expected: "U_",
                found: "T_".to_string(),
            }
        );
    }

    #[test]
    fn invalid_character_error() {
        // 'I' is not in the alphabet
        let err = TestToken::parse("T_AAAAAAAAAAAIA").unwrap_err();
        assert_eq!(err, TokenError::InvalidCharacter('I'));
    }

    #[test]
    fn excluded_characters_rejected() {
        for ch in ['I', 'L', 'O', 'Q'] {
            let s = format!("T_AAAAAAAAAAAA{ch}");
            let err = TestToken::parse(&s).unwrap_err();
            assert_eq!(err, TokenError::InvalidCharacter(ch));
        }
    }

    #[test]
    fn wrong_length_error() {
        let err = TestToken::parse("T_AAAA").unwrap_err();
        assert_eq!(err, TokenError::InvalidLength { expected: 15, found: 6 });
    }

    #[test]
    fn is_valid_returns_true_for_valid() {
        let s = TestToken::new(42).to_string();
        assert!(TestToken::is_valid(&s));
    }

    #[test]
    fn is_valid_returns_false_for_invalid() {
        assert!(!TestToken::is_valid("INVALID"));
        assert!(!TestToken::is_valid("T_SHORT"));
        assert!(!TestToken::is_valid("X_AAAAAAAAAAAAA"));
    }

    #[test]
    fn from_str_works() {
        let s = TestToken::new(99).to_string();
        let parsed: TestToken = s.parse().unwrap();
        assert_eq!(parsed.id(), 99);
    }

    #[test]
    fn encoded_id_round_trips() {
        let token = TestToken::new(42);
        let enc = token.encoded_id();
        assert_eq!(enc.len(), 13);
        assert!(!enc.starts_with("T_"));
        let parsed = TestToken::from_encoded_id(&enc).unwrap();
        assert_eq!(parsed.id(), 42);
    }

    #[test]
    fn from_encoded_id_rejects_wrong_length() {
        let err = TestToken::from_encoded_id("SHORT").unwrap_err();
        assert!(matches!(err, TokenError::InvalidLength { .. }));
    }

    #[test]
    fn from_encoded_id_rejects_invalid_char() {
        let err = TestToken::from_encoded_id("AAAAAAAAAAAAI").unwrap_err();
        assert_eq!(err, TokenError::InvalidCharacter('I'));
    }

    #[test]
    fn encoded_id_does_not_include_prefix() {
        let token = TestToken::new(1);
        assert_eq!(token.encoded_id(), "YYYYYYYYYYYY4");
    }

    #[test]
    fn different_prefix_types_are_distinct() {
        let test_s = TestToken::new(42).to_string();
        let user_s = UserToken::new(42).to_string();
        assert_ne!(test_s, user_s);
        assert!(test_s.starts_with("T_"));
        assert!(user_s.starts_with("U_"));
    }

    #[test]
    fn serde_round_trip() {
        let token = TestToken::new(123_456);
        let json = serde_json::to_string(&token).unwrap();
        let parsed: TestToken = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id(), 123_456);
    }

    #[test]
    fn serde_serializes_as_string() {
        let token = TestToken::new(0);
        let json = serde_json::to_string(&token).unwrap();
        assert_eq!(json, r#""T_YYYYYYYYYYYYY""#);
    }

    #[test]
    fn serde_rejects_invalid_token() {
        let result = serde_json::from_str::<TestToken>(r#""INVALID""#);
        result.unwrap_err();
    }

    #[test]
    fn debug_format() {
        let token = TestToken::new(0);
        let debug = format!("{token:?}");
        assert_eq!(debug, "Token(T_YYYYYYYYYYYYY)");
    }

    define_token_prefix!(BigPrefix, "B_");
    type BigToken = Token<BigPrefix, u128, TokenAlphabet>;

    #[test]
    fn u128_round_trip() {
        for id in [0u128, 1, u128::from(u64::MAX), u128::MAX] {
            let token = BigToken::new(id);
            let s = token.to_string();
            let parsed = BigToken::parse(&s).unwrap();
            assert_eq!(parsed.id(), id);
        }
    }

    #[test]
    fn u128_zero_encodes_to_26_as() {
        let token = BigToken::new(0);
        assert_eq!(token.to_string(), "B_YYYYYYYYYYYYYYYYYYYYYYYYYY");
    }

    #[test]
    fn u128_max_round_trips() {
        let token = BigToken::new(u128::MAX);
        let s = token.to_string();
        let parsed = BigToken::parse(&s).unwrap();
        assert_eq!(parsed.id(), u128::MAX);
    }

    #[test]
    fn u128_known_value_encoding() {
        let token = BigToken::new(1);
        let s = token.to_string();
        // 25 Y's + 4
        assert_eq!(s, "B_YYYYYYYYYYYYYYYYYYYYYYYYY4");
    }

    #[test]
    fn u128_wrong_length_error() {
        // prefix (2) + encoded (26) = 28
        let err = BigToken::parse("B_AAAA").unwrap_err();
        assert_eq!(err, TokenError::InvalidLength { expected: 28, found: 6 });
    }

    #[test]
    fn u128_serde_round_trip() {
        let token = BigToken::new(123_456);
        let json = serde_json::to_string(&token).unwrap();
        let parsed: BigToken = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id(), 123_456);
    }

    #[test]
    fn u128_debug_format() {
        let token = BigToken::new(0);
        let debug = format!("{token:?}");
        assert_eq!(debug, "Token(B_YYYYYYYYYYYYYYYYYYYYYYYYYY)");
    }

    define_token_prefix!(CappedPrefix, "C_");
    type CappedToken = Token<CappedPrefix, u64, TokenAlphabet, { i64::MAX as u128 }>;

    #[test]
    fn capped_generate_respects_max() {
        for _ in 0..1000 {
            let token = CappedToken::generate();
            assert!(token.id() >= 1);
            i64::try_from(token.id()).unwrap();
        }
    }
}
