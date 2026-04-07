use chrono::{DateTime, Utc};
use ib_utils::{Token, define_token_prefix};

use crate::user::{Capabilities, UserId};

define_token_prefix!(ProjectTokenPrefix, "P_");

pub type ProjectId = u64;
pub type ProjectToken = Token<ProjectTokenPrefix, ProjectId, { i64::MAX as u128 }>;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: ProjectId,
    pub token: ProjectToken,
    pub name: String,
    pub slug: String,
    pub prefix: String,
    pub issue_counter: u32,
    pub description: Option<String>,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub slug: String,
    pub prefix: String,
    pub description: Option<String>,
}

impl NewProject {
    pub fn new(
        name: impl Into<String>,
        slug: impl Into<String>,
        prefix: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Result<Self, crate::Error> {
        let name = name.into();
        let slug = slug.into();
        let prefix = prefix.into();

        if name.trim().is_empty() {
            return Err(crate::Error::Validation("name must not be empty".into()));
        }
        if slug.trim().is_empty() {
            return Err(crate::Error::Validation("slug must not be empty".into()));
        }
        if !is_valid_prefix(&prefix) {
            return Err(crate::Error::Validation("prefix must be 1-4 uppercase ASCII letters".into()));
        }

        Ok(Self {
            name,
            slug,
            prefix,
            description: description.map(Into::into),
        })
    }
}

/// Validates that a prefix is 1-4 uppercase ASCII letters.
pub fn is_valid_prefix(prefix: &str) -> bool {
    !prefix.is_empty() && prefix.len() <= 4 && prefix.chars().all(|c| c.is_ascii_uppercase())
}

#[derive(Debug, Clone)]
pub struct ProjectMember {
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub capabilities: Capabilities,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewProjectMember {
    pub project_id: ProjectId,
    pub user_id: UserId,
    pub capabilities: Capabilities,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_token_displays_with_prefix_and_roundtrips() {
        let token = ProjectToken::new(1);
        let s = token.to_string();
        assert!(s.starts_with("P_"), "expected P_ prefix, got {s}");
        let parsed = ProjectToken::parse(&s).expect("should parse back");
        assert_eq!(parsed.id(), 1);
    }

    #[test]
    fn new_project_rejects_empty_name() {
        let err = NewProject::new("", "myapp", "MA", None::<String>).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_project_rejects_empty_slug() {
        let err = NewProject::new("MyApp", "", "MA", None::<String>).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_project_rejects_invalid_prefix_lowercase() {
        let err = NewProject::new("MyApp", "myapp", "ma", None::<String>).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_project_rejects_prefix_too_long() {
        let err = NewProject::new("MyApp", "myapp", "ABCDE", None::<String>).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_project_accepts_valid_input() {
        let p = NewProject::new("MyApp", "myapp", "MA", None::<String>).unwrap();
        assert_eq!(p.prefix, "MA");
    }

    #[test]
    fn is_valid_prefix_accepts_1_to_4_uppercase() {
        assert!(is_valid_prefix("A"));
        assert!(is_valid_prefix("AB"));
        assert!(is_valid_prefix("ABC"));
        assert!(is_valid_prefix("ABCD"));
    }

    #[test]
    fn is_valid_prefix_rejects_invalid() {
        assert!(!is_valid_prefix(""));
        assert!(!is_valid_prefix("ABCDE")); // 5 chars
        assert!(!is_valid_prefix("ab")); // lowercase
        assert!(!is_valid_prefix("A1")); // digit
    }
}
