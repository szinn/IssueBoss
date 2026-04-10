use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use crate::issue::IssueId;

pub type IssueRelationshipId = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipKind {
    DependsOn,
    RelatedTo,
}

impl fmt::Display for RelationshipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DependsOn => write!(f, "DependsOn"),
            Self::RelatedTo => write!(f, "RelatedTo"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRelationshipKindError(String);

impl fmt::Display for ParseRelationshipKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown relationship kind: {}", self.0)
    }
}

impl FromStr for RelationshipKind {
    type Err = ParseRelationshipKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DependsOn" => Ok(Self::DependsOn),
            "RelatedTo" => Ok(Self::RelatedTo),
            other => Err(ParseRelationshipKindError(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IssueRelationship {
    pub id: IssueRelationshipId,
    pub from_issue_id: IssueId,
    pub to_issue_id: IssueId,
    pub kind: RelationshipKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewIssueRelationship {
    pub from_issue_id: IssueId,
    pub to_issue_id: IssueId,
    pub kind: RelationshipKind,
}

#[derive(Debug, Clone)]
pub struct RelatedIssueSummary {
    pub id: IssueId,
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Default)]
pub struct IssueRelationships {
    pub depends_on: Vec<RelatedIssueSummary>,
    pub blocks: Vec<RelatedIssueSummary>,
    pub related_to: Vec<RelatedIssueSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_kind_roundtrips() {
        for kind in [RelationshipKind::DependsOn, RelationshipKind::RelatedTo] {
            let s = kind.to_string();
            let parsed: RelationshipKind = s.parse().expect("should parse");
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn relationship_kind_rejects_unknown() {
        assert!("Blocks".parse::<RelationshipKind>().is_err());
        assert!("".parse::<RelationshipKind>().is_err());
    }
}
