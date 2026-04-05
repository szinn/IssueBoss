use chrono::{DateTime, Utc};
use ib_utils::{Token, define_token_prefix};
use serde::{Deserialize, Serialize};

define_token_prefix!(UserTokenPrefix, "U_");

pub type UserId = u64;
pub type UserToken = Token<UserTokenPrefix, UserId, { i64::MAX as u128 }>;

/// A capability that can be granted to a user.
///
/// Global capabilities (SuperAdmin, Admin) are stored on `User.capabilities`.
/// Per-project capabilities (ViewIssues, etc.) are stored on
/// `ProjectMember.capabilities`. Both use the same enum for a unified
/// permission model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Capability {
    // Global
    SuperAdmin,
    Admin,
    // Per-project
    ViewIssues,
    CreateIssues,
    UpdateIssues,
    TransitionStatus,
    ManageLabels,
    ManageMembers,
}

/// A set of capabilities stored as a JSON array.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Capabilities(pub Vec<Capability>);

impl Capabilities {
    pub fn has(&self, cap: Capability) -> bool {
        self.0.contains(&cap)
    }

    pub fn is_super_admin(&self) -> bool {
        self.has(Capability::SuperAdmin)
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub token: UserToken,
    pub username: String,
    pub full_name: String,
    pub password_hash: String,
    pub email_address: String,
    pub api_key_hash: Option<String>,
    pub api_key_prefix: Option<String>,
    pub api_key_created_at: Option<DateTime<Utc>>,
    pub api_key_last_used_at: Option<DateTime<Utc>>,
    pub capabilities: Capabilities,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_token_displays_correctly() {
        let token = UserToken::from_id(1);
        assert_eq!(token.to_string(), "U_1");
    }

    #[test]
    fn capabilities_roundtrip_serde() {
        let caps = Capabilities(vec![Capability::SuperAdmin, Capability::Admin]);
        let json = serde_json::to_string(&caps).unwrap();
        assert_eq!(json, r#"["SuperAdmin","Admin"]"#);
        let caps2: Capabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps2.0, vec![Capability::SuperAdmin, Capability::Admin]);
    }

    #[test]
    fn capabilities_has_works() {
        let caps = Capabilities(vec![Capability::SuperAdmin]);
        assert!(caps.has(Capability::SuperAdmin));
        assert!(!caps.has(Capability::Admin));
    }
}
