use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let db = manager.get_database_backend();

        // Find the SuperAdmin user's id for backfilling existing rows.
        // Falls back to 0 on a fresh database (e.g. test environments) where no
        // users exist yet — 0 is a safe sentinel meaning "system/unknown".
        let superadmin_id: i64 = conn
            .query_one_raw(Statement::from_string(
                db,
                "SELECT id FROM users WHERE capabilities LIKE '%SuperAdmin%' LIMIT 1".to_owned(),
            ))
            .await?
            .and_then(|row| row.try_get("", "id").ok())
            .unwrap_or(0);

        // Add submitter_id NOT NULL with the superadmin default.
        // The database fills all existing rows with the default value at ADD COLUMN
        // time.
        conn.execute_unprepared(&format!("ALTER TABLE issues ADD COLUMN submitter_id BIGINT NOT NULL DEFAULT {superadmin_id}"))
            .await?;

        // Add assigned_id as nullable (null = unassigned)
        conn.execute_unprepared("ALTER TABLE issues ADD COLUMN assigned_id BIGINT").await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Column removal is intentionally not implemented.
        // Rolling back this migration requires manual intervention.
        Ok(())
    }
}
