use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use crate::{
    project::ProjectId,
    token::{Token, TokenAlphabet, define_token_prefix},
    user::UserId,
};

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
pub type IssueToken = Token<IssueTokenPrefix, IssueId, TokenAlphabet, { i64::MAX as u128 }>;

// ── Enumerations ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueStatus {
    // Triage category
    TriageNeeded,
    TriageInProgress,
    TriageReview,
    // Research category
    ResearchNeeded,
    ResearchInProgress,
    ResearchReview,
    // Spec category
    SpecNeeded,
    SpecInProgress,
    SpecReview,
    // Plan category
    PlanNeeded,
    PlanInProgress,
    PlanReview,
    // Dev category
    DevNeeded,
    DevInProgress,
    DevReview,
    // Off-pipeline
    Backlog,
    Canceled,
    Done,
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
            "TriageNeeded" => Ok(Self::TriageNeeded),
            "TriageInProgress" => Ok(Self::TriageInProgress),
            "TriageReview" => Ok(Self::TriageReview),
            "ResearchNeeded" => Ok(Self::ResearchNeeded),
            "ResearchInProgress" => Ok(Self::ResearchInProgress),
            "ResearchReview" => Ok(Self::ResearchReview),
            "SpecNeeded" => Ok(Self::SpecNeeded),
            "SpecInProgress" => Ok(Self::SpecInProgress),
            "SpecReview" => Ok(Self::SpecReview),
            "PlanNeeded" => Ok(Self::PlanNeeded),
            "PlanInProgress" => Ok(Self::PlanInProgress),
            "PlanReview" => Ok(Self::PlanReview),
            "DevNeeded" => Ok(Self::DevNeeded),
            "DevInProgress" => Ok(Self::DevInProgress),
            "DevReview" => Ok(Self::DevReview),
            "Backlog" => Ok(Self::Backlog),
            "Canceled" => Ok(Self::Canceled),
            "Done" => Ok(Self::Done),
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
    pub submitter: UserId,
    pub assigned: Option<UserId>,
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
    pub submitted_by: UserId,
}

impl NewIssue {
    pub fn new(
        project_id: ProjectId,
        project_prefix: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        priority: IssuePriority,
        size: Option<IssueSize>,
        submitted_by: UserId,
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
            submitted_by,
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
    pub submitted_by: UserId,
}

/// Filters for `IssueRepository::list`.
#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    pub status: Option<IssueStatus>,
    pub priority: Option<IssuePriority>,
    pub size: Option<IssueSize>,
    pub limit: Option<u64>,
    pub exclude_blocked: Option<bool>,
    pub submitted_by: Option<UserId>,
    pub assigned_to: Option<UserId>,
    /// Exclude issues whose status is in this set (empty = no exclusion).
    /// Used to express "open" (exclude Done/Canceled/Backlog) server-side so
    /// `limit` and DB paging stay correct.
    pub status_not_in: Vec<IssueStatus>,
}

/// Derives the immutable slug for an issue at creation time.
/// Format: `"{prefix}-{number}"`, e.g. `"IB-42"`.
/// This is also the canonical user-facing reference for the issue.
pub fn derive_issue_slug(prefix: &str, number: u32) -> String {
    format!("{prefix}-{number}")
}

impl IssueStatus {
    /// Returns `true` if transitioning from `self` to `next` is a legal move.
    ///
    /// Rules encoded as an explicit allow-list:
    /// - Self-transition is never valid.
    /// - `Done` is truly terminal — no transitions out.
    /// - `Canceled` → `Backlog` or `TriageNeeded` only (reactivation).
    /// - Any non-Done, non-Canceled state → `Backlog` or `Canceled` always
    ///   allowed.
    /// - `Backlog` → any Needed state (reactivation).
    /// - Within categories: Needed → InProgress → Review.
    /// - Any `*Needed` → `Done` (skip all remaining phases).
    /// - From Review: can route to own Needed or any later Needed state (no
    ///   Review routes to an earlier category), or directly to `Done`.
    ///   TriageReview → any Needed or Done; ResearchReview →
    ///   Research/Spec/Plan/Dev Needed or Done; SpecReview →
    ///   Research/Spec/Plan/Dev Needed or Done; PlanReview → Plan/Dev Needed or
    ///   Done; DevReview → Dev Needed or Done.
    pub fn can_transition_to(&self, next: &Self) -> bool {
        if self == next {
            return false;
        }
        if matches!(self, Self::Done) {
            return false;
        }
        if matches!(self, Self::Canceled) {
            return matches!(next, Self::Backlog | Self::TriageNeeded);
        }
        if matches!(next, Self::Backlog | Self::Canceled) {
            return true;
        }
        if matches!(self, Self::Backlog) {
            return matches!(
                next,
                Self::TriageNeeded | Self::ResearchNeeded | Self::SpecNeeded | Self::PlanNeeded | Self::DevNeeded
            );
        }
        matches!(
            (self, next),
            // Needed → InProgress
            (Self::TriageNeeded, Self::TriageInProgress | Self::Done) |
(Self::ResearchNeeded, Self::ResearchInProgress | Self::Done) |
(Self::SpecNeeded, Self::SpecInProgress | Self::Done) |
(Self::PlanNeeded, Self::PlanInProgress | Self::Done) |
(Self::DevNeeded, Self::DevInProgress | Self::Done) |
(Self::TriageInProgress, Self::TriageReview) |
(Self::ResearchInProgress, Self::ResearchReview) |
(Self::SpecInProgress, Self::SpecReview) |
(Self::PlanInProgress, Self::PlanReview) |
(Self::DevInProgress, Self::DevReview) |
(Self::TriageReview,
Self::TriageNeeded | Self::ResearchNeeded | Self::SpecNeeded |
Self::PlanNeeded | Self::DevNeeded | Self::Done) |
(Self::ResearchReview | Self::SpecReview,
Self::ResearchNeeded | Self::SpecNeeded) |
(Self::ResearchReview | Self::SpecReview | Self::PlanReview, Self::PlanNeeded)
|
(Self::ResearchReview | Self::SpecReview | Self::PlanReview | Self::DevReview,
Self::DevNeeded | Self::Done)
        )
    }

    pub fn is_in_progress(self) -> bool {
        matches!(
            self,
            Self::TriageInProgress | Self::ResearchInProgress | Self::SpecInProgress | Self::PlanInProgress | Self::DevInProgress
        )
    }

    /// Returns `true` for every in-pipeline state and `false` for the three
    /// terminal/off-pipeline states (`Done`, `Canceled`, `Backlog`). This is
    /// the single source of truth for the "open issue" definition.
    pub fn is_open(self) -> bool {
        !matches!(self, Self::Done | Self::Canceled | Self::Backlog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_open_excludes_only_terminal_states() {
        use IssueStatus::*;
        for s in [
            TriageNeeded,
            TriageInProgress,
            TriageReview,
            ResearchNeeded,
            ResearchInProgress,
            ResearchReview,
            SpecNeeded,
            SpecInProgress,
            SpecReview,
            PlanNeeded,
            PlanInProgress,
            PlanReview,
            DevNeeded,
            DevInProgress,
            DevReview,
        ] {
            assert!(s.is_open(), "{s} should be open");
        }
        for s in [Backlog, Canceled, Done] {
            assert!(!s.is_open(), "{s} should not be open");
        }
    }

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
        let err = NewIssue::new(1, "BB", "", "", IssuePriority::Medium, None, 1).unwrap_err();
        assert!(matches!(err, crate::Error::Validation(_)));
    }

    #[test]
    fn new_issue_accepts_valid_input() {
        let i = NewIssue::new(1, "BB", "Fix login", "", IssuePriority::High, None, 1).unwrap();
        assert_eq!(i.title, "Fix login");
        assert_eq!(i.priority, IssuePriority::High);
        assert_eq!(i.submitted_by, 1);
    }

    #[test]
    fn derive_issue_slug_produces_expected_output() {
        assert_eq!(derive_issue_slug("BB", 42), "BB-42");
        assert_eq!(derive_issue_slug("IB", 1), "IB-1");
    }

    #[test]
    fn issue_status_roundtrips_via_display_and_fromstr() {
        for s in [
            IssueStatus::TriageNeeded,
            IssueStatus::TriageInProgress,
            IssueStatus::TriageReview,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchReview,
            IssueStatus::SpecNeeded,
            IssueStatus::SpecInProgress,
            IssueStatus::SpecReview,
            IssueStatus::PlanNeeded,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanReview,
            IssueStatus::DevNeeded,
            IssueStatus::DevInProgress,
            IssueStatus::DevReview,
            IssueStatus::Backlog,
            IssueStatus::Canceled,
            IssueStatus::Done,
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
            description: String::new(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: "BB-42".into(),
            version: 0,
            submitter: 1,
            assigned: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(issue.slug, "BB-42");
    }

    #[test]
    fn self_transition_is_always_denied() {
        for s in [
            IssueStatus::TriageNeeded,
            IssueStatus::TriageInProgress,
            IssueStatus::TriageReview,
            IssueStatus::ResearchNeeded,
            IssueStatus::ResearchInProgress,
            IssueStatus::ResearchReview,
            IssueStatus::SpecNeeded,
            IssueStatus::SpecInProgress,
            IssueStatus::SpecReview,
            IssueStatus::PlanNeeded,
            IssueStatus::PlanInProgress,
            IssueStatus::PlanReview,
            IssueStatus::DevNeeded,
            IssueStatus::DevInProgress,
            IssueStatus::DevReview,
            IssueStatus::Backlog,
            IssueStatus::Canceled,
            IssueStatus::Done,
        ] {
            assert!(!s.can_transition_to(&s.clone()), "{s:?} → {s:?} should be denied");
        }
    }

    #[test]
    fn done_is_truly_terminal() {
        for next in [
            IssueStatus::TriageNeeded,
            IssueStatus::Backlog,
            IssueStatus::Canceled,
            IssueStatus::DevNeeded,
            IssueStatus::ResearchNeeded,
        ] {
            assert!(!IssueStatus::Done.can_transition_to(&next), "Done → {next:?} should be denied");
        }
    }

    #[test]
    fn canceled_can_only_reactivate() {
        assert!(IssueStatus::Canceled.can_transition_to(&IssueStatus::Backlog));
        assert!(IssueStatus::Canceled.can_transition_to(&IssueStatus::TriageNeeded));
        assert!(!IssueStatus::Canceled.can_transition_to(&IssueStatus::ResearchNeeded));
        assert!(!IssueStatus::Canceled.can_transition_to(&IssueStatus::Done));
        assert!(!IssueStatus::Canceled.can_transition_to(&IssueStatus::SpecNeeded));
    }

    #[test]
    fn any_active_state_can_go_to_backlog_or_canceled() {
        // Pipeline states can always go to Backlog or Canceled
        for from in [
            IssueStatus::TriageNeeded,
            IssueStatus::TriageInProgress,
            IssueStatus::TriageReview,
            IssueStatus::ResearchNeeded,
            IssueStatus::PlanInProgress,
            IssueStatus::DevReview,
        ] {
            assert!(from.can_transition_to(&IssueStatus::Backlog), "{from:?} → Backlog should be allowed");
            assert!(from.can_transition_to(&IssueStatus::Canceled), "{from:?} → Canceled should be allowed");
        }
        // Backlog can go to Canceled (but not itself — self-transition is
        // denied)
        assert!(IssueStatus::Backlog.can_transition_to(&IssueStatus::Canceled));
    }

    #[test]
    fn backlog_can_reach_any_needed_state() {
        for needed in [
            IssueStatus::TriageNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::SpecNeeded,
            IssueStatus::PlanNeeded,
            IssueStatus::DevNeeded,
        ] {
            assert!(IssueStatus::Backlog.can_transition_to(&needed), "Backlog → {needed:?} should be allowed");
        }
        assert!(!IssueStatus::Backlog.can_transition_to(&IssueStatus::TriageInProgress));
        assert!(!IssueStatus::Backlog.can_transition_to(&IssueStatus::DevReview));
        assert!(!IssueStatus::Backlog.can_transition_to(&IssueStatus::Done));
    }

    #[test]
    fn needed_transitions_only_to_own_inprogress() {
        assert!(IssueStatus::TriageNeeded.can_transition_to(&IssueStatus::TriageInProgress));
        assert!(!IssueStatus::TriageNeeded.can_transition_to(&IssueStatus::ResearchInProgress));
        assert!(!IssueStatus::TriageNeeded.can_transition_to(&IssueStatus::TriageReview));
        assert!(IssueStatus::DevNeeded.can_transition_to(&IssueStatus::DevInProgress));
        assert!(!IssueStatus::DevNeeded.can_transition_to(&IssueStatus::PlanInProgress));
    }

    #[test]
    fn inprogress_transitions_only_to_own_review() {
        assert!(IssueStatus::TriageInProgress.can_transition_to(&IssueStatus::TriageReview));
        assert!(!IssueStatus::TriageInProgress.can_transition_to(&IssueStatus::ResearchReview));
        assert!(!IssueStatus::TriageInProgress.can_transition_to(&IssueStatus::TriageNeeded));
        assert!(IssueStatus::PlanInProgress.can_transition_to(&IssueStatus::PlanReview));
        assert!(!IssueStatus::PlanInProgress.can_transition_to(&IssueStatus::DevReview));
    }

    #[test]
    fn triage_review_routes_to_any_needed() {
        for needed in [
            IssueStatus::TriageNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::SpecNeeded,
            IssueStatus::PlanNeeded,
            IssueStatus::DevNeeded,
        ] {
            assert!(IssueStatus::TriageReview.can_transition_to(&needed));
        }
        assert!(!IssueStatus::TriageReview.can_transition_to(&IssueStatus::TriageInProgress));
    }

    #[test]
    fn review_does_not_route_to_earlier_categories() {
        assert!(!IssueStatus::SpecReview.can_transition_to(&IssueStatus::TriageNeeded));
        assert!(!IssueStatus::ResearchReview.can_transition_to(&IssueStatus::TriageNeeded));
        assert!(!IssueStatus::PlanReview.can_transition_to(&IssueStatus::SpecNeeded));
        assert!(!IssueStatus::PlanReview.can_transition_to(&IssueStatus::ResearchNeeded));
        assert!(!IssueStatus::PlanReview.can_transition_to(&IssueStatus::TriageNeeded));
        assert!(IssueStatus::DevReview.can_transition_to(&IssueStatus::DevNeeded));
        assert!(IssueStatus::DevReview.can_transition_to(&IssueStatus::Done));
        assert!(!IssueStatus::DevReview.can_transition_to(&IssueStatus::PlanNeeded));
    }

    #[test]
    fn review_can_transition_to_done() {
        for review in [
            IssueStatus::TriageReview,
            IssueStatus::ResearchReview,
            IssueStatus::SpecReview,
            IssueStatus::PlanReview,
            IssueStatus::DevReview,
        ] {
            assert!(review.can_transition_to(&IssueStatus::Done), "{review:?} → Done should be allowed");
        }
    }

    #[test]
    fn needed_can_transition_to_done() {
        for needed in [
            IssueStatus::TriageNeeded,
            IssueStatus::ResearchNeeded,
            IssueStatus::SpecNeeded,
            IssueStatus::PlanNeeded,
            IssueStatus::DevNeeded,
        ] {
            assert!(needed.can_transition_to(&IssueStatus::Done), "{needed:?} → Done should be allowed");
        }
    }

    #[test]
    fn spec_review_can_reach_dev_needed() {
        assert!(IssueStatus::SpecReview.can_transition_to(&IssueStatus::DevNeeded));
    }

    #[test]
    fn is_in_progress_returns_true_for_all_inprogress_states() {
        assert!(IssueStatus::TriageInProgress.is_in_progress());
        assert!(IssueStatus::ResearchInProgress.is_in_progress());
        assert!(IssueStatus::SpecInProgress.is_in_progress());
        assert!(IssueStatus::PlanInProgress.is_in_progress());
        assert!(IssueStatus::DevInProgress.is_in_progress());
    }

    #[test]
    fn is_in_progress_returns_false_for_non_inprogress_states() {
        assert!(!IssueStatus::TriageNeeded.is_in_progress());
        assert!(!IssueStatus::TriageReview.is_in_progress());
        assert!(!IssueStatus::ResearchNeeded.is_in_progress());
        assert!(!IssueStatus::DevReview.is_in_progress());
        assert!(!IssueStatus::Done.is_in_progress());
        assert!(!IssueStatus::Backlog.is_in_progress());
        assert!(!IssueStatus::Canceled.is_in_progress());
    }
}
