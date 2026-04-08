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
            || (!matches!(cap, Capability::SuperAdmin | Capability::Admin) && (self.0.contains(&Capability::Admin) || self.0.contains(&Capability::SuperAdmin)))
    }

    pub fn is_super_admin(&self) -> bool {
        self.has(Capability::SuperAdmin)
    }

    /// Returns a new `Capabilities` containing all capabilities from both sets,
    /// with duplicates removed.
    pub fn merge(&self, other: &Self) -> Self {
        let mut combined = self.0.clone();
        for cap in &other.0 {
            if !combined.contains(cap) {
                combined.push(*cap);
            }
        }
        Self(combined)
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

    #[test]
    fn admin_grants_all_non_admin_capabilities() {
        let caps = Capabilities(vec![Capability::Admin]);
        assert!(caps.has(Capability::ViewIssues));
        assert!(caps.has(Capability::CreateIssues));
        assert!(caps.has(Capability::UpdateIssues));
        assert!(caps.has(Capability::TransitionStatus));
        assert!(caps.has(Capability::ManageLabels));
        assert!(caps.has(Capability::ManageMembers));
        // Admin does not implicitly grant SuperAdmin
        assert!(!caps.has(Capability::SuperAdmin));
    }

    #[test]
    fn super_admin_grants_all_non_admin_capabilities() {
        let caps = Capabilities(vec![Capability::SuperAdmin]);
        assert!(caps.has(Capability::ViewIssues));
        assert!(caps.has(Capability::CreateIssues));
        assert!(caps.has(Capability::UpdateIssues));
        assert!(caps.has(Capability::TransitionStatus));
        assert!(caps.has(Capability::ManageLabels));
        assert!(caps.has(Capability::ManageMembers));
        // SuperAdmin does not implicitly grant Admin
        assert!(!caps.has(Capability::Admin));
    }

    #[test]
    fn capabilities_merge_combines_both_sets() {
        let a = Capabilities(vec![Capability::Admin]);
        let b = Capabilities(vec![Capability::ViewIssues, Capability::CreateIssues]);
        let merged = a.merge(&b);
        assert!(merged.has(Capability::Admin));
        assert!(merged.has(Capability::ViewIssues));
        assert!(merged.has(Capability::CreateIssues));
    }

    #[test]
    fn capabilities_merge_deduplicates() {
        let a = Capabilities(vec![Capability::ViewIssues]);
        let b = Capabilities(vec![Capability::ViewIssues, Capability::UpdateIssues]);
        let merged = a.merge(&b);
        assert_eq!(merged.0.iter().filter(|&&c| c == Capability::ViewIssues).count(), 1);
        assert!(merged.has(Capability::UpdateIssues));
    }

    #[test]
    fn capabilities_merge_with_empty() {
        let a = Capabilities(vec![Capability::Admin]);
        let empty = Capabilities::default();
        assert_eq!(a.merge(&empty).0, a.0);
        assert_eq!(empty.merge(&a).0, a.0);
    }
}
