pub mod model;
pub mod repository;
pub mod service;

pub use model::{IssueRelationship, IssueRelationshipId, IssueRelationships, NewIssueRelationship, RelatedIssueSummary, RelationshipKind};
pub use repository::IssueRelationshipRepository;
#[cfg(any(test, feature = "test-support"))]
pub use repository::MockIssueRelationshipRepository;
pub use service::IssueRelationshipService;
