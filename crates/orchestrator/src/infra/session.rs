//! Per-entity async sqlx repository for [`Session`].
//!
//! All methods take `impl SqliteExecutor` so the same function works whether
//! the caller passes `cx.db()` (a pool ref) or `&mut *tx` (a mutable
//! transaction borrow). The repo owns the `Row -> Entity` mapping; no
//! slug-tree, no directory moves.

use sqlx::{AssertSqlSafe, SqliteExecutor};

use crate::entities::project::ProjectId;
use crate::entities::session::{Session, SessionId, SessionStatus};
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

// -- repository ----------------------------------------------------------------

/// Entity projection: maps straight onto `Session` (status derived from `archived_at`).
const SELECT: &str = "SELECT id, project_id, title, title_source, spec_version, spec_json,
                             sort_order, pinned, created_at,
                             CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
                      FROM session";

/// Stateless repository for the `session` table.
pub struct SessionRepo;

impl SessionRepo {
    /// Insert a new session row. The caller sets `session.id`.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, session: &Session) -> Result<()> {
        sqlx::query(
            "INSERT INTO session
                 (id, project_id, title, title_source, spec_version, spec_json,
                  sort_order, pinned, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(session.project_id.as_str())
        .bind(&session.title)
        .bind(session.title_source.as_str())
        .bind(session.spec_version.map(|v| v as i64))
        .bind(&session.spec_json)
        .bind(session.sort_order as i64)
        .bind(session.pinned as i64)
        .bind(&session.created_at)
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Fetch one session by id. Returns `None` if absent.
    ///
    /// Used by command read-modify-write paths; maps straight onto the `Session`
    /// entity with `status` derived from `archived_at` in the SELECT.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &SessionId) -> Result<Option<Session>> {
        Ok(
            sqlx::query_as::<_, Session>(AssertSqlSafe(format!("{SELECT} WHERE id = ?")))
                .bind(id.as_str())
                .fetch_optional(exec)
                .await?,
        )
    }

    /// List sessions for a project as typed entities. A command-path helper for the
    /// project aggregate (archive/duplicate/stop cascade over their child sessions);
    /// read endpoints project straight to `SessionView` in the app layer instead.
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        project_id: &ProjectId,
        page: Page,
    ) -> Result<Listing<Session>> {
        let order = "ORDER BY pinned DESC, sort_order ASC, id ASC";
        let where_parent = "WHERE project_id = ?";

        match page {
            Page::All => {
                let items: Vec<Session> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {where_parent} {order}")))
                        .bind(project_id.as_str())
                        .fetch_all(exec)
                        .await?;
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let items: Vec<Session> = sqlx::query_as(AssertSqlSafe(format!(
                    "{SELECT} {where_parent} {order} LIMIT ? OFFSET ?"
                )))
                .bind(project_id.as_str())
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                let fetch_n = limit as i64 + 1;
                let rows: Vec<Session> = if let Some(cursor) = after {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "WITH anchor AS (
                             SELECT pinned, sort_order FROM session WHERE id = ?
                         )
                         {SELECT} {where_parent}
                           AND (pinned < (SELECT pinned FROM anchor)
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order > (SELECT sort_order FROM anchor))
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order = (SELECT sort_order FROM anchor)
                                    AND id > ?))
                         {order}
                         LIMIT ?"
                    )))
                    .bind(&cursor)
                    .bind(project_id.as_str())
                    .bind(&cursor)
                    .bind(fetch_n)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} {where_parent} {order} LIMIT ?"
                    )))
                    .bind(project_id.as_str())
                    .bind(fetch_n)
                    .fetch_all(exec)
                    .await?
                };
                let has_more = rows.len() > limit as usize;
                let items: Vec<Session> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    items.last().map(|s| s.id.as_str().to_owned())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }

    /// Persist mutable fields (title, title_source, spec, sort_order, pinned, archived_at).
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, session: &Session) -> Result<()> {
        let archived_at: Option<&str> = match session.status {
            SessionStatus::Archived => Some("archived"),
            SessionStatus::Active => None,
        };
        sqlx::query(
            "UPDATE session
             SET project_id = ?, title = ?, title_source = ?, spec_version = ?, spec_json = ?,
                 sort_order = ?, pinned = ?, archived_at = ?
             WHERE id = ?",
        )
        .bind(session.project_id.as_str())
        .bind(&session.title)
        .bind(session.title_source.as_str())
        .bind(session.spec_version.map(|v| v as i64))
        .bind(&session.spec_json)
        .bind(session.sort_order as i64)
        .bind(session.pinned as i64)
        .bind(archived_at)
        .bind(session.id.as_str())
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Set `archived_at` to mark the session archived.
    pub async fn set_archived<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SessionId,
        archived_at: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE session SET archived_at = ? WHERE id = ?")
            .bind(archived_at)
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Clear `archived_at` to restore an archived session to active.
    pub async fn set_active<'e>(exec: impl SqliteExecutor<'e>, id: &SessionId) -> Result<()> {
        sqlx::query("UPDATE session SET archived_at = NULL WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Hard-delete a session row.
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &SessionId) -> Result<()> {
        sqlx::query("DELETE FROM session WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Read the panel-tree geometry blob for a session.
    pub async fn get_panel_tree<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SessionId,
    ) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT panel_tree_json FROM session WHERE id = ?")
                .bind(id.as_str())
                .fetch_optional(exec)
                .await?;
        Ok(row.and_then(|(v,)| v))
    }

    /// Write the panel-tree geometry blob for a session.
    pub async fn set_panel_tree<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SessionId,
        panel_tree_json: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE session SET panel_tree_json = ? WHERE id = ?")
            .bind(panel_tree_json)
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }
}

// -- tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::session::TitleSource;
    use crate::infra::migrate;

    fn unfiled() -> ProjectId {
        ProjectId::new("00000000-0000-0000-0000-000000000000")
    }

    fn make_session(id: &str, project_id: &ProjectId, sort_order: u32) -> Session {
        Session {
            id: SessionId::from_string(id),
            project_id: project_id.clone(),
            title: format!("Session {id}"),
            title_source: TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order,
            pinned: false,
            status: SessionStatus::Active,
        }
    }

    // Scenario: A repository persists and reads a typed entity
    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s = make_session("s-rt-1", &pid, 0);

        SessionRepo::create(&pool, &s).await.unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap().unwrap();
        assert_eq!(got.id.as_str(), "s-rt-1");
        assert_eq!(got.title, "Session s-rt-1");
        assert_eq!(got.project_id.as_str(), pid.as_str());
        assert_eq!(got.title_source, TitleSource::AgentTitle);
        assert_eq!(got.status, SessionStatus::Active);
    }

    #[tokio::test]
    async fn get_returns_none_for_absent_id() {
        let pool = migrate::open_memory().await.unwrap();
        let got = SessionRepo::get(&pool, &SessionId::from_string("no-such"))
            .await
            .unwrap();
        assert!(got.is_none());
    }

    // Scenario: update persists mutable fields
    #[tokio::test]
    async fn update_persists_title_and_title_source() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let mut s = make_session("s-upd-title", &pid, 0);
        SessionRepo::create(&pool, &s).await.unwrap();

        s.rename("My custom title");
        SessionRepo::update(&pool, &s).await.unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap().unwrap();
        assert_eq!(got.title, "My custom title");
        assert_eq!(got.title_source, TitleSource::Custom);
    }

    #[tokio::test]
    async fn update_persists_sort_order_and_pinned() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let mut s = make_session("s-upd-order", &pid, 0);
        SessionRepo::create(&pool, &s).await.unwrap();

        s.sort_order = 42;
        s.pinned = true;
        SessionRepo::update(&pool, &s).await.unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap().unwrap();
        assert_eq!(got.sort_order, 42);
        assert!(got.pinned);
    }

    // Scenario: delete removes the row
    #[tokio::test]
    async fn delete_removes_row() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s = make_session("s-del-1", &pid, 0);
        SessionRepo::create(&pool, &s).await.unwrap();
        SessionRepo::delete(&pool, &s.id).await.unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap();
        assert!(got.is_none());
    }

    // Scenario: archive and restore
    #[tokio::test]
    async fn archive_marks_session_archived() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s = make_session("s-arch-1", &pid, 0);
        SessionRepo::create(&pool, &s).await.unwrap();

        SessionRepo::set_archived(&pool, &s.id, "2026-06-01T00:00:00.000Z")
            .await
            .unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap().unwrap();
        assert_eq!(got.status, SessionStatus::Archived);
    }

    #[tokio::test]
    async fn restore_marks_session_active() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s = make_session("s-arch-2", &pid, 0);
        SessionRepo::create(&pool, &s).await.unwrap();
        SessionRepo::set_archived(&pool, &s.id, "2026-06-01T00:00:00.000Z")
            .await
            .unwrap();

        SessionRepo::set_active(&pool, &s.id).await.unwrap();

        let got = SessionRepo::get(&pool, &s.id).await.unwrap().unwrap();
        assert_eq!(got.status, SessionStatus::Active);
    }

    // Scenario: multi-repo call on one tx is atomic
    #[tokio::test]
    async fn two_creates_on_one_transaction_both_commit() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s1 = make_session("s-tx-1", &pid, 0);
        let s2 = make_session("s-tx-2", &pid, 1);

        {
            let mut tx = pool.begin().await.unwrap();
            SessionRepo::create(&mut *tx, &s1).await.unwrap();
            SessionRepo::create(&mut *tx, &s2).await.unwrap();
            tx.commit().await.unwrap();
        }

        assert!(SessionRepo::get(&pool, &s1.id).await.unwrap().is_some());
        assert!(SessionRepo::get(&pool, &s2.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn transaction_rollback_leaves_no_rows() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        let s = make_session("s-rb-1", &pid, 0);

        {
            let mut tx = pool.begin().await.unwrap();
            SessionRepo::create(&mut *tx, &s).await.unwrap();
            tx.rollback().await.unwrap();
        }

        let got = SessionRepo::get(&pool, &s.id).await.unwrap();
        assert!(got.is_none(), "rolled-back row must not persist");
    }
}
