use sqlx::{FromRow, SqliteExecutor};

use crate::entities::{LaunchTemplate, LaunchTemplateId, NewLaunchTemplate, ProjectId};
use crate::shared::{Error, Listing, Page, Result};

// ── Row type ─────────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct LaunchTemplateRow {
    id: String,
    project_id: String,
    spec_version: i64,
    spec_json: String,
}

impl From<LaunchTemplateRow> for LaunchTemplate {
    fn from(r: LaunchTemplateRow) -> Self {
        LaunchTemplate {
            id: LaunchTemplateId::from_string(r.id),
            project_id: ProjectId::new(r.project_id),
            spec_version: r.spec_version as u32,
            spec_json: r.spec_json,
        }
    }
}

// ── Repo ─────────────────────────────────────────────────────────────────────

/// Per-entity async repository for [`LaunchTemplate`].
///
/// Methods take `impl SqliteExecutor` so a caller can pass either `&SqlitePool`
/// (single-statement, already atomic) or `&mut Transaction` (multi-repo cascade).
pub struct LaunchTemplateRepo;

impl LaunchTemplateRepo {
    /// Persist a new launch template. Returns the created entity.
    pub async fn create<'e>(
        exec: impl SqliteExecutor<'e>,
        tmpl: &NewLaunchTemplate,
    ) -> Result<LaunchTemplate> {
        let id = LaunchTemplateId::mint();
        let id_str = id.as_str().to_owned();
        let project_id = tmpl.project_id.as_str();
        let spec_version = tmpl.spec_version as i64;

        sqlx::query(
            "INSERT INTO launch_template (id, project_id, spec_version, spec_json)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&id_str)
        .bind(project_id)
        .bind(spec_version)
        .bind(&tmpl.spec_json)
        .execute(exec)
        .await?;

        Ok(LaunchTemplate {
            id,
            project_id: tmpl.project_id.clone(),
            spec_version: tmpl.spec_version,
            spec_json: tmpl.spec_json.clone(),
        })
    }

    /// Fetch one launch template by id. Returns `None` if absent.
    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
        let row: Option<LaunchTemplateRow> = sqlx::query_as(
            "SELECT id, project_id, spec_version, spec_json
             FROM launch_template
             WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?;

        Ok(row.map(Into::into))
    }

    /// List launch templates for a project, ordered by `sort_order`.
    ///
    /// Pinning is not a column on `launch_template`; ordering is by `sort_order` only.
    /// Supports `Page::All`, `Page::Offset`, and `Page::Cursor` (cursor = last-seen id).
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        project_id: &ProjectId,
        page: &Page,
    ) -> Result<Listing<LaunchTemplate>> {
        let pid = project_id.as_str();

        match page {
            Page::All => {
                let rows: Vec<LaunchTemplateRow> = sqlx::query_as(
                    "SELECT id, project_id, spec_version, spec_json
                     FROM launch_template
                     WHERE project_id = ?
                     ORDER BY sort_order ASC, id ASC",
                )
                .bind(pid)
                .fetch_all(exec)
                .await?;

                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    None,
                ))
            }

            Page::Offset { limit, offset } => {
                let limit = *limit as i64;
                let offset = *offset as i64;
                // Fetch limit+1 to detect whether a next page exists.
                let mut rows: Vec<LaunchTemplateRow> = sqlx::query_as(
                    "SELECT id, project_id, spec_version, spec_json
                     FROM launch_template
                     WHERE project_id = ?
                     ORDER BY sort_order ASC, id ASC
                     LIMIT ? OFFSET ?",
                )
                .bind(pid)
                .bind(limit + 1)
                .bind(offset)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() as i64 > limit;
                if has_more {
                    rows.truncate(limit as usize);
                }
                let next = if has_more {
                    Some(format!("{}", offset + limit))
                } else {
                    None
                };

                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    next,
                ))
            }

            Page::Cursor { after, limit } => {
                let limit = *limit as i64;

                let mut rows: Vec<LaunchTemplateRow> = if let Some(cursor) = after {
                    // cursor is the id of the last-seen row; use (sort_order, id) for stable ordering
                    sqlx::query_as(
                        "SELECT t.id, t.project_id, t.spec_version, t.spec_json
                         FROM launch_template t
                         WHERE t.project_id = ?
                           AND (t.sort_order, t.id) > (
                               SELECT sort_order, id FROM launch_template WHERE id = ?
                           )
                         ORDER BY t.sort_order ASC, t.id ASC
                         LIMIT ?",
                    )
                    .bind(pid)
                    .bind(cursor)
                    .bind(limit + 1)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(
                        "SELECT id, project_id, spec_version, spec_json
                         FROM launch_template
                         WHERE project_id = ?
                         ORDER BY sort_order ASC, id ASC
                         LIMIT ?",
                    )
                    .bind(pid)
                    .bind(limit + 1)
                    .fetch_all(exec)
                    .await?
                };

                let has_more = rows.len() as i64 > limit;
                if has_more {
                    rows.truncate(limit as usize);
                }
                let next = if has_more {
                    rows.last().map(|r| r.id.clone())
                } else {
                    None
                };

                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    next,
                ))
            }
        }
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

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

    fn new_tmpl(project_id: &str) -> NewLaunchTemplate {
        NewLaunchTemplate {
            project_id: ProjectId::new(project_id),
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
        }
    }

    // Seeded Unfiled project id from the migration.
    const UNFILED: &str = "00000000-0000-0000-0000-000000000000";

    // ── Scenario: A repository persists and reads a typed entity ─────────────

    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = memory_pool().await;
        let created = LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
            .await
            .unwrap();

        let fetched = LaunchTemplateRepo::get(&pool, &created.id)
            .await
            .unwrap()
            .expect("should be present");

        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.project_id, created.project_id);
        assert_eq!(fetched.spec_version, 1);
        assert_eq!(fetched.spec_json, r#"{"items":[]}"#);
    }

    // ── Scenario: Children are found by parent id ─────────────────────────────

    #[tokio::test]
    async fn list_filters_by_project_id() {
        let pool = memory_pool().await;

        // Insert the Unfiled project's template and one more (using the Default workspace project).
        LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
            .await
            .unwrap();

        // Create a second project to hold a different template.
        let other_project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, source_kind) VALUES (?, ?, ?, ?)",
        )
        .bind(&other_project_id)
        .bind("00000000-0000-0000-0000-000000000001") // Default workspace
        .bind("OtherProject")
        .bind("blank")
        .execute(&pool)
        .await
        .unwrap();

        LaunchTemplateRepo::create(&pool, &new_tmpl(&other_project_id))
            .await
            .unwrap();

        let listing = LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::All)
            .await
            .unwrap();

        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].project_id, ProjectId::new(UNFILED));
    }

    // ── Scenario: Unbounded listing is explicit ───────────────────────────────

    #[tokio::test]
    async fn list_all_returns_every_row_for_project() {
        let pool = memory_pool().await;
        for _ in 0..3 {
            LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
                .await
                .unwrap();
        }

        let listing = LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::All)
            .await
            .unwrap();

        assert_eq!(listing.items.len(), 3);
        assert!(listing.next.is_none());
    }

    // ── Scenario: A bounded page returns a continuation cursor ────────────────

    #[tokio::test]
    async fn cursor_pagination_has_next_when_more_remain() {
        let pool = memory_pool().await;
        for _ in 0..5 {
            LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
                .await
                .unwrap();
        }

        let page1 =
            LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::cursor_from_start(3))
                .await
                .unwrap();

        assert_eq!(page1.items.len(), 3);
        assert!(page1.next.is_some(), "expected a continuation cursor");
    }

    #[tokio::test]
    async fn cursor_pagination_last_page_has_no_next() {
        let pool = memory_pool().await;
        for _ in 0..3 {
            LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
                .await
                .unwrap();
        }

        // Page size == total: no continuation expected.
        let listing =
            LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::cursor_from_start(3))
                .await
                .unwrap();

        assert!(listing.next.is_none());
    }

    #[tokio::test]
    async fn cursor_pagination_second_page_completes_the_listing() {
        let pool = memory_pool().await;
        for _ in 0..5 {
            LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
                .await
                .unwrap();
        }

        let page1 =
            LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::cursor_from_start(3))
                .await
                .unwrap();

        let cursor = page1.next.expect("page 1 must have a cursor");

        let page2 = LaunchTemplateRepo::list(
            &pool,
            &ProjectId::new(UNFILED),
            &Page::cursor_after(&cursor, 3),
        )
        .await
        .unwrap();

        assert_eq!(page2.items.len(), 2);
        assert!(page2.next.is_none());

        // No overlap between pages.
        let ids1: std::collections::HashSet<_> = page1
            .items
            .iter()
            .map(|t| t.id.as_str().to_owned())
            .collect();
        let ids2: std::collections::HashSet<_> = page2
            .items
            .iter()
            .map(|t| t.id.as_str().to_owned())
            .collect();
        assert!(ids1.is_disjoint(&ids2));
    }

    // ── Scenario: A rename is a plain update ─────────────────────────────────

    #[tokio::test]
    async fn update_replaces_spec() {
        let pool = memory_pool().await;
        let created = LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
            .await
            .unwrap();

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
            id: LaunchTemplateId::mint(),
            project_id: ProjectId::new(UNFILED),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        };
        let err = LaunchTemplateRepo::update(&pool, &fake).await.unwrap_err();
        assert_eq!(err.code(), "launch_template.not_found");
    }

    // ── Scenario: Delete ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_removes_the_row() {
        let pool = memory_pool().await;
        let created = LaunchTemplateRepo::create(&pool, &new_tmpl(UNFILED))
            .await
            .unwrap();

        LaunchTemplateRepo::delete(&pool, &created.id)
            .await
            .unwrap();

        let fetched = LaunchTemplateRepo::get(&pool, &created.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_not_found_error() {
        let pool = memory_pool().await;
        let id = LaunchTemplateId::mint();
        let err = LaunchTemplateRepo::delete(&pool, &id).await.unwrap_err();
        assert_eq!(err.code(), "launch_template.not_found");
    }

    // ── Scenario: multi-repo call on one tx is atomic ─────────────────────────

    #[tokio::test]
    async fn two_creates_on_one_transaction_are_atomic() {
        let pool = memory_pool().await;

        let mut tx = pool.begin().await.unwrap();

        let t1 = LaunchTemplateRepo::create(&mut *tx, &new_tmpl(UNFILED))
            .await
            .unwrap();
        let t2 = LaunchTemplateRepo::create(&mut *tx, &new_tmpl(UNFILED))
            .await
            .unwrap();

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

        let listing = LaunchTemplateRepo::list(&pool, &ProjectId::new(UNFILED), &Page::All)
            .await
            .unwrap();

        assert!(listing.items.is_empty());
    }
}
