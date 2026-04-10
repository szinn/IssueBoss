use chrono::{DateTime, Utc};
use ib_utils::{Token, define_token_prefix};

use crate::issue::IssueId;

define_token_prefix!(ArtifactTokenPrefix, "A_");
pub type ArtifactId = u64;
pub type ArtifactToken = Token<ArtifactTokenPrefix, ArtifactId, { i64::MAX as u128 }>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactKind {
    TriageResult,
    Spec,
    Research,
    Plan,
    ResearchTopic,
    Comment,
    StatusTransition,
    Handoff,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::TriageResult => "TriageResult",
            Self::Spec => "Spec",
            Self::Research => "Research",
            Self::Plan => "Plan",
            Self::ResearchTopic => "ResearchTopic",
            Self::Comment => "Comment",
            Self::StatusTransition => "StatusTransition",
            Self::Handoff => "Handoff",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown artifact kind: {0}")]
pub struct ParseArtifactKindError(pub String);

impl std::str::FromStr for ArtifactKind {
    type Err = ParseArtifactKindError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TriageResult" => Ok(Self::TriageResult),
            "Spec" => Ok(Self::Spec),
            "Research" => Ok(Self::Research),
            "Plan" => Ok(Self::Plan),
            "ResearchTopic" => Ok(Self::ResearchTopic),
            "Comment" => Ok(Self::Comment),
            "StatusTransition" => Ok(Self::StatusTransition),
            "Handoff" => Ok(Self::Handoff),
            _ => Err(ParseArtifactKindError(s.to_owned())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IssueArtifact {
    pub id: ArtifactId,
    pub token: ArtifactToken,
    pub issue_id: IssueId,
    pub kind: ArtifactKind,
    pub slug: Option<String>,
    pub body: serde_json::Value,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewArtifact {
    pub issue_id: IssueId,
    pub kind: ArtifactKind,
    pub slug: Option<String>,
    pub body: serde_json::Value,
    pub created_by: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn artifact_kind_roundtrips() {
        for kind in [
            ArtifactKind::TriageResult,
            ArtifactKind::Spec,
            ArtifactKind::Research,
            ArtifactKind::Plan,
            ArtifactKind::ResearchTopic,
            ArtifactKind::Comment,
            ArtifactKind::StatusTransition,
            ArtifactKind::Handoff,
        ] {
            assert_eq!(ArtifactKind::from_str(&kind.to_string()).unwrap(), kind);
        }
    }
}
