use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
