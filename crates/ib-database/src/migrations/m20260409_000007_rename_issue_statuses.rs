use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Status string renames: (old_value, new_value)
/// Only the 7 renamed variants are listed; 11 others are unchanged.
const RENAMES: &[(&str, &str)] = &[
    ("Triage", "TriageNeeded"),
    ("ResearchInReview", "ResearchReview"),
    ("ReadyForPlan", "PlanNeeded"),
    ("PlanInReview", "PlanReview"),
    ("ReadyForDev", "DevNeeded"),
    ("InDev", "DevInProgress"),
    ("CodeReview", "DevReview"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (old, new) in RENAMES {
            db.execute_unprepared(&format!("UPDATE issues SET status = '{new}' WHERE status = '{old}'"))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (old, new) in RENAMES.iter().rev() {
            db.execute_unprepared(&format!("UPDATE issues SET status = '{old}' WHERE status = '{new}'"))
                .await?;
        }
        Ok(())
    }
}
