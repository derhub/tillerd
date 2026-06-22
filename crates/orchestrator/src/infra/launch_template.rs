use sqlx::SqliteExecutor;

use crate::entities::{LaunchTemplate, LaunchTemplateId};
use crate::shared::{Error, Result};

// -- Repo ---------------------------------------------------------------------

/// Per-entity async repository for [`LaunchTemplate`].
///
/// Methods take `impl SqliteExecutor` so a caller can pass either `&SqlitePool`
/// (single-statement, already atomic) or `&mut Transaction` (multi-repo cascade).
pub struct LaunchTemplateRepo;

impl LaunchTemplateRepo {
    /// Persist a launch template (id minted by the caller). Returns `()`; the
    /// caller already holds the entity it built.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, tmpl: &LaunchTemplate) -> Result<()> {
        sqlx::query(
            "INSERT INTO launch_template (id, project_id, spec_version, spec_json)
             VALUES (?, ?, ?, ?)",
        )
        .bind(tmpl.id.as_str())
        .bind(tmpl.project_id.as_str())
        .bind(tmpl.spec_version as i64)
        .bind(&tmpl.spec_json)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Fetch one launch template by id. Returns `None` if absent.
    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
        Ok(sqlx::query_as::<_, LaunchTemplate>(
            "SELECT id, project_id, spec_version, spec_json
             FROM launch_template
             WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?)
    }

    /// Replace the spec (version + json) for an existing launch template.
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, tmpl: &LaunchTemplate) -> Result<()> {
        let rows = sqlx::query(
            "UPDATE launch_template
             SET spec_version = ?, spec_json = ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?",
        )
        .bind(tmpl.spec_version as i64)
        .bind(&tmpl.spec_json)
        .bind(tmpl.id.as_str())
        .execute(exec)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(Error::LaunchTemplateNotFound(tmpl.id.as_str().to_owned()));
        }
        Ok(())
    }

    /// Hard-delete a launch template by id.
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &LaunchTemplateId) -> Result<()> {
        let rows = sqlx::query("DELETE FROM launch_template WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(Error::LaunchTemplateNotFound(id.as_str().to_owned()));
        }
        Ok(())
    }
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::entities::ProjectId;

    // -- helpers ---------------------------------------------------------------

    async fn apply_migrations(pool: &SqlitePool) {
        sqlx::migrate!("src/infra/migrations")
            .run(pool)
            .await
            .expect("migrations");
    }

    async fn memory_pool() -> SqlitePool {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .expect("pool");
        apply_migrations(&pool).await;
        pool
    }

    fn new_tmpl(project_id: &str) -> LaunchTemplate {
        LaunchTemplate {
            id: LaunchTemplateId::from_string(uuid::Uuid::new_v4().to_string()),
            project_id: ProjectId::new(project_id),
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
        }
    }

    // Seeded Unfiled project id from the migration.
    const UNFILED: &str = "00000000-0000-0000-0000-000000000000";

    async fn count_for_project(pool: &SqlitePool, project_id: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM launch_template WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // -- Scenario: A repository persists and reads a typed entity -------------

    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = memory_pool().await;
        let tmpl = new_tmpl(UNFILED);
        LaunchTemplateRepo::create(&pool, &tmpl).await.unwrap();

        let fetched = LaunchTemplateRepo::get(&pool, &tmpl.id)
            .await
            .unwrap()
            .expect("should be present");

        assert_eq!(fetched.id, tmpl.id);
        assert_eq!(fetched.project_id, tmpl.project_id);
        assert_eq!(fetched.spec_version, 1);
        assert_eq!(fetched.spec_json, r#"{"items":[]}"#);
    }

    // -- Scenario: A rename is a plain update ---------------------------------

    #[tokio::test]
    async fn update_replaces_spec() {
        let pool = memory_pool().await;
        let created = new_tmpl(UNFILED);
        LaunchTemplateRepo::create(&pool, &created).await.unwrap();

        let updated = LaunchTemplate {
            spec_version: 2,
            spec_json: r#"{"items":["a"]}"#.to_owned(),
            ..created.clone()
        };
        LaunchTemplateRepo::update(&pool, &updated).await.unwrap();

        let fetched = LaunchTemplateRepo::get(&pool, &created.id)
            .await
            .unwrap()
            .expect("present");

        assert_eq!(fetched.spec_version, 2);
        assert_eq!(fetched.spec_json, r#"{"items":["a"]}"#);
    }

    #[tokio::test]
    async fn update_nonexistent_returns_not_found_error() {
        let pool = memory_pool().await;
        let fake = LaunchTemplate {
            id: LaunchTemplateId::from_string(uuid::Uuid::new_v4().to_string()),
            project_id: ProjectId::new(UNFILED),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        };
        let err = LaunchTemplateRepo::update(&pool, &fake).await.unwrap_err();
        assert_eq!(err.code(), "launch_template.not_found");
    }

    // -- Scenario: Delete ------------------------------------------------------

    #[tokio::test]
    async fn delete_removes_the_row() {
        let pool = memory_pool().await;
        let created = new_tmpl(UNFILED);
        LaunchTemplateRepo::create(&pool, &created).await.unwrap();

        LaunchTemplateRepo::delete(&pool, &created.id)
            .await
            .unwrap();

        let fetched = LaunchTemplateRepo::get(&pool, &created.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_not_found_error() {
        let pool = memory_pool().await;
        let id = LaunchTemplateId::from_string(uuid::Uuid::new_v4().to_string());
        let err = LaunchTemplateRepo::delete(&pool, &id).await.unwrap_err();
        assert_eq!(err.code(), "launch_template.not_found");
    }

    // -- Scenario: multi-repo call on one tx is atomic -------------------------

    #[tokio::test]
    async fn two_creates_on_one_transaction_are_atomic() {
        let pool = memory_pool().await;

        let mut tx = pool.begin().await.unwrap();

        let t1 = new_tmpl(UNFILED);
        let t2 = new_tmpl(UNFILED);
        LaunchTemplateRepo::create(&mut *tx, &t1).await.unwrap();
        LaunchTemplateRepo::create(&mut *tx, &t2).await.unwrap();

        tx.commit().await.unwrap();

        // Both rows visible after commit.
        let got1 = LaunchTemplateRepo::get(&pool, &t1.id)
            .await
            .unwrap()
            .expect("t1 present");
        let got2 = LaunchTemplateRepo::get(&pool, &t2.id)
            .await
            .unwrap()
            .expect("t2 present");

        assert_eq!(got1.id, t1.id);
        assert_eq!(got2.id, t2.id);
    }

    #[tokio::test]
    async fn rolled_back_transaction_leaves_no_rows() {
        let pool = memory_pool().await;

        let mut tx = pool.begin().await.unwrap();

        LaunchTemplateRepo::create(&mut *tx, &new_tmpl(UNFILED))
            .await
            .unwrap();

        tx.rollback().await.unwrap();

        assert_eq!(count_for_project(&pool, UNFILED).await, 0);
    }
}
