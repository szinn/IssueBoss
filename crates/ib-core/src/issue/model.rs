use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use ib_utils::{Token, define_token_prefix};

use crate::project::ProjectId;

define_token_prefix!(IssueTokenPrefix, "I_");

// ── Parse error
// ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ParseEnumError(pub String);

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown variant: {}", self.0)
    }
}

impl std::error::Error for ParseEnumError {}

pub type IssueId = u64;
pub type IssueToken = Token<IssueTokenPrefix, IssueId, { i64::MAX as u128 }>;

// ── Enumerations ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueStatus {
    Triage,
    SpecNeeded,
    ResearchNeeded,
    ResearchInProgress,
    ResearchInReview,
    ReadyForPlan,
    PlanInProgress,
    PlanInReview,
    ReadyForDev,
    InDev,
    CodeReview,
    Done,
    Backlog,
    Canceled,
}

impl fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl FromStr for IssueStatus {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Triage" => Ok(Self::Triage),
            "SpecNeeded" => Ok(Self::SpecNeeded),
            "ResearchNeeded" => Ok(Self::ResearchNeeded),
            "ResearchInProgress" => Ok(Self::ResearchInProgress),
            "ResearchInReview" => Ok(Self::ResearchInReview),
            "ReadyForPlan" => Ok(Self::ReadyForPlan),
            "PlanInProgress" => Ok(Self::PlanInProgress),
            "PlanInReview" => Ok(Self::PlanInReview),
            "ReadyForDev" => Ok(Self::ReadyForDev),
            "InDev" => Ok(Self::InDev),
            "CodeReview" => Ok(Self::CodeReview),
            "Done" => Ok(Self::Done),
            "Backlog" => Ok(Self::Backlog),
            "Canceled" => Ok(Self::Canceled),
            _ => Err(ParseEnumError(s.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssuePriority {
    Urgent,
    High,
    #[default]
    Medium,
    Low,
}

impl fmt::Display for IssuePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl FromStr for IssuePriority {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Urgent" => Ok(Self::Urgent),
            "High" => Ok(Self::High),
            "Medium" => Ok(Self::Medium),
            "Low" => Ok(Self::Low),
            _ => Err(ParseEnumError(s.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueSize {
    XS,
    Small,
    Medium,
    Large,
}

impl fmt::Display for IssueSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl FromStr for IssueSize {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "XS" => Ok(Self::XS),
            "Small" => Ok(Self::Small),
            "Medium" => Ok(Self::Medium),
            "Large" => Ok(Self::Large),
            _ => Err(ParseEnumError(s.to_owned())),
        }
    }
}

// ── Domain structs
// ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Issue {
    pub id: IssueId,
    pub token: IssueToken,
    pub number: u32,
    pub project_id: ProjectId,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    pub size: Option<IssueSize>,
    pub slug: String,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What callers pass to `IssueService::create_issue`.
/// The handler resolves the project token → id and prefix before calling the
/// service.
#[derive(Debug, Clone)]
pub struct NewIssue {
    pub project_id: ProjectId,
    pub project_prefix: String,
    pub title: String,
    pub description: String,
    pub priority: IssuePriority,
    pub size: Option<IssueSize>,
}

impl NewIssue {
    pub fn new(
        project_id: ProjectId,
        project_prefix: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        priority: IssuePriority,
        size: Option<IssueSize>,
    ) -> Result<Self, crate::Error> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(crate::Error::Validation("title must not be empty".into()));
        }
        Ok(Self {
            project_id,
            project_prefix: project_prefix.into(),
            title,
            description: description.into(),
            priority,
            size,
        })
    }
}

/// What the service passes to `IssueRepository::create` after resolving the
/// counter and slug.
#[derive(Debug, Clone)]
pub struct NewIssueRecord {
    pub project_id: ProjectId,
    pub number: u32,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub priority: IssuePriority,
    pub size: Option<IssueSize>,
    pub slug: String,
}

/// Filters for `IssueRepository::list`.
#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    pub status: Option<IssueStatus>,
    pub priority: Option<IssuePriority>,
    pub size: Option<IssueSize>,
    pub limit: Option<u64>,
}

/// Derives the immutable slug for an issue at creation time.
/// Format: `"{prefix}-{number}"`, e.g. `"IB-42"`.
/// This is also the canonical user-facing reference for the issue.
pub fn derive_issue_slug(prefix: &str, number: u32) -> String {
    format!("{}-{}", prefix, number)
}

impl IssueStatus {
    /// Returns the position of this status in the pipeline sequence.
    /// Backlog and Canceled are off-chain and return None.
    fn pipeline_index(&self) -> Option<usize> {
        match self {
            Self::Triage => Some(0),
            Self::SpecNeeded => Some(1),
            Self::ResearchNeeded => Some(2),
            Self::ResearchInProgress => Some(3),
            Self::ResearchInReview => Some(4),
            Self::ReadyForPlan => Some(5),
            Self::PlanInProgress => Some(6),
            Self::PlanInReview => Some(7),
            Self::ReadyForDev => Some(8),
            Self::InDev => Some(9),
            Self::CodeReview => Some(10),
            Self::Done => Some(11),
            Self::Backlog | Self::Canceled => None,
        }
    }

    /// Determines if this status can transition to the given `next` status.
    ///
    /// Rules:
    /// - Self-transition (same state → same state) is never allowed
    /// - Canceled is terminal (no outgoing transitions)
    /// - Any non-Canceled state → Backlog or Canceled is always allowed
    /// - From Backlog: any pipeline state is reachable (reactivation)
    /// - In-pipeline: forward moves are adjacent-only (index + 1); backward
    ///   moves are unrestricted (any earlier index)
    pub fn can_transition_to(&self, next: &Self) -> bool {
        // Self-transition is never valid
        if self == next {
            return false;
        }
        // Canceled is terminal
        if matches!(self, Self::Canceled) {
            return false;
        }
        // Any non-Canceled state can go to Backlog or Canceled
        if matches!(next, Self::Backlog | Self::Canceled) {
            return true;
        }
        // From Backlog: any pipeline state is reachable (next is not Backlog/Canceled
        // here)
        if matches!(self, Self::Backlog) {
            return next.pipeline_index().is_some();
        }
        // Triage can skip directly to ReadyForPlan (no spec/research needed)
        if matches!(self, Self::Triage) && matches!(next, Self::ReadyForPlan) {
            return true;
        }
        // Both in the pipeline: adjacent-forward or any-backward
        match (self.pipeline_index(), next.pipeline_index()) {
            (Some(from), Some(to)) => to == from + 1 || to < from,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_token_roundtrips() {
        let token = IssueToken::new(42);
        let s = token.to_string();
        assert!(s.starts_with("I_"), "expected I_ prefix, got {s}");
        let parsed = IssueToken::parse(&s).expect("should parse");
        assert_eq!(parsed.id(), 42);
    }

    #[test]
    fn new_issue_rejects_empty_title() {
        let err = NewIssue::new(1, "BB", "", "", IssuePriority::Medium, None).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_issue_accepts_valid_input() {
        let i = NewIssue::new(1, "BB", "Fix login", "", IssuePriority::High, None).unwrap();
        assert_eq!(i.title, "Fix login");
        assert_eq!(i.priority, IssuePriority::High);
    }

    #[test]
    fn derive_issue_slug_produces_expected_output() {
        assert_eq!(derive_issue_slug("BB", 42), "BB-42");
        assert_eq!(derive_issue_slug("IB", 1), "IB-1");
    }

    #[test]
    fn issue_status_roundtrips_via_display_and_fromstr() {
        for s in [
            IssueStatus::Triage,
            IssueStatus::SpecNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchInReview,
            IssueStatus::ReadyForPlan,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanInReview,
            IssueStatus::ReadyForDev,
            IssueStatus::InDev,
            IssueStatus::CodeReview,
            IssueStatus::Done,
            IssueStatus::Backlog,
            IssueStatus::Canceled,
        ] {
            let displayed = s.to_string();
            let parsed: IssueStatus = displayed.parse().expect("should parse");
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn issue_priority_roundtrips() {
        for p in [IssuePriority::Urgent, IssuePriority::High, IssuePriority::Medium, IssuePriority::Low] {
            let s = p.to_string();
            let parsed: IssuePriority = s.parse().expect("should parse");
            assert_eq!(parsed, p);
        }
    }

    #[test]
    fn issue_size_roundtrips() {
        for sz in [IssueSize::XS, IssueSize::Small, IssueSize::Medium, IssueSize::Large] {
            let s = sz.to_string();
            let parsed: IssueSize = s.parse().expect("should parse");
            assert_eq!(parsed, sz);
        }
    }

    #[test]
    fn slug_is_the_issue_reference() {
        assert_eq!(derive_issue_slug("BB", 42), "BB-42");
        // slug on Issue struct is the canonical user-facing reference
        let issue = Issue {
            id: 1,
            token: IssueToken::new(1),
            number: 42,
            project_id: 1,
            title: "t".into(),
            description: "".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: "BB-42".into(),
            version: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(issue.slug, "BB-42");
    }

    #[test]
    fn self_transition_is_always_denied() {
        for s in [
            IssueStatus::Triage,
            IssueStatus::InDev,
            IssueStatus::Done,
            IssueStatus::Backlog,
            IssueStatus::Canceled,
        ] {
            assert!(!s.can_transition_to(&s.clone()), "self-transition should be denied for {s:?}");
        }
    }

    #[test]
    fn canceled_is_terminal() {
        for next in [
            IssueStatus::Triage,
            IssueStatus::SpecNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchInReview,
            IssueStatus::ReadyForPlan,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanInReview,
            IssueStatus::ReadyForDev,
            IssueStatus::InDev,
            IssueStatus::CodeReview,
            IssueStatus::Done,
            IssueStatus::Backlog,
        ] {
            assert!(!IssueStatus::Canceled.can_transition_to(&next), "Canceled → {next:?} should be denied");
        }
    }

    #[test]
    fn any_non_canceled_state_can_go_to_backlog() {
        for s in [
            IssueStatus::Triage,
            IssueStatus::SpecNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::InDev,
            IssueStatus::Done,
        ] {
            assert!(s.can_transition_to(&IssueStatus::Backlog), "{s:?} → Backlog should be allowed");
        }
    }

    #[test]
    fn any_non_canceled_state_can_go_to_canceled() {
        for s in [IssueStatus::Triage, IssueStatus::InDev, IssueStatus::Done, IssueStatus::Backlog] {
            assert!(s.can_transition_to(&IssueStatus::Canceled), "{s:?} → Canceled should be allowed");
        }
    }

    #[test]
    fn backlog_can_go_to_any_pipeline_state() {
        for next in [
            IssueStatus::Triage,
            IssueStatus::SpecNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchInReview,
            IssueStatus::ReadyForPlan,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanInReview,
            IssueStatus::ReadyForDev,
            IssueStatus::InDev,
            IssueStatus::CodeReview,
            IssueStatus::Done,
        ] {
            assert!(IssueStatus::Backlog.can_transition_to(&next), "Backlog → {next:?} should be allowed");
        }
    }

    #[test]
    fn forward_adjacent_moves_are_allowed() {
        let chain = [
            IssueStatus::Triage,
            IssueStatus::SpecNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchInReview,
            IssueStatus::ReadyForPlan,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanInReview,
            IssueStatus::ReadyForDev,
            IssueStatus::InDev,
            IssueStatus::CodeReview,
            IssueStatus::Done,
        ];
        for window in chain.windows(2) {
            let from = &window[0];
            let to = &window[1];
            assert!(from.can_transition_to(to), "{from:?} → {to:?} (adjacent forward) should be allowed");
        }
    }

    #[test]
    fn triage_can_skip_to_ready_for_plan() {
        assert!(IssueStatus::Triage.can_transition_to(&IssueStatus::ReadyForPlan));
    }

    #[test]
    fn forward_skip_moves_are_denied() {
        assert!(!IssueStatus::Triage.can_transition_to(&IssueStatus::ResearchNeeded));
        assert!(!IssueStatus::Triage.can_transition_to(&IssueStatus::PlanInProgress));
        assert!(!IssueStatus::ReadyForDev.can_transition_to(&IssueStatus::CodeReview));
        assert!(!IssueStatus::PlanInProgress.can_transition_to(&IssueStatus::ReadyForDev));
    }

    #[test]
    fn backward_moves_to_any_earlier_state_are_allowed() {
        assert!(IssueStatus::ResearchInProgress.can_transition_to(&IssueStatus::SpecNeeded));
        assert!(IssueStatus::PlanInReview.can_transition_to(&IssueStatus::ReadyForPlan));
        assert!(IssueStatus::Done.can_transition_to(&IssueStatus::Triage));
        assert!(IssueStatus::InDev.can_transition_to(&IssueStatus::ResearchNeeded));
    }

    #[test]
    fn spec_examples_from_design_doc() {
        assert!(IssueStatus::SpecNeeded.can_transition_to(&IssueStatus::ResearchNeeded));
        assert!(IssueStatus::ResearchNeeded.can_transition_to(&IssueStatus::ResearchInProgress));
        assert!(IssueStatus::ResearchInProgress.can_transition_to(&IssueStatus::SpecNeeded));
        assert!(IssueStatus::ResearchInProgress.can_transition_to(&IssueStatus::ResearchInReview));
        assert!(IssueStatus::ResearchInReview.can_transition_to(&IssueStatus::ReadyForPlan));
    }
}
