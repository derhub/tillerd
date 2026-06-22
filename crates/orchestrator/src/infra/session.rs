//! Per-entity async sqlx repository for [`Session`].
//!
//! All methods take `impl SqliteExecutor` so the same function works whether
//! the caller passes `cx.db()` (a pool ref) or `&mut *tx` (a mutable
//! transaction borrow). The repo owns the `Row -> Entity` mapping; no
//! slug-tree, no directory moves.

use sqlx::{AssertSqlSafe, SqliteExecutor};

use crate::entities::project::ProjectId;
use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

// ── Row ───────────────────────────────────────────────────────────────────────

struct SessionRow {
    id: String,
    project_id: String,
    title: String,
    title_source: String,
    spec_version: Option<i64>,
    spec_json: Option<String>,
    sort_order: i64,
    pinned: i64,
    archived_at: Option<String>,
    created_at: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for SessionRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(SessionRow {
            id: row.try_get("id")?,
            project_id: row.try_get("project_id")?,
            title: row.try_get("title")?,
            title_source: row.try_get("title_source")?,
            spec_version: row.try_get("spec_version")?,
            spec_json: row.try_get("spec_json")?,
            sort_order: row.try_get("sort_order")?,
            pinned: row.try_get("pinned")?,
            archived_at: row.try_get("archived_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl From<SessionRow> for Session {
    fn from(r: SessionRow) -> Self {
        let title_source = match r.title_source.as_str() {
            "branch" => TitleSource::Branch,
            "both" => TitleSource::Both,
            "custom" => TitleSource::Custom,
            _ => TitleSource::AgentTitle,
        };
        let status = if r.archived_at.is_some() {
            SessionStatus::Archived
        } else {
            SessionStatus::Active
        };
        Session {
            id: SessionId::from_string(r.id),
            project_id: ProjectId::new(r.project_id),
            title: r.title,
            title_source,
            created_at: r.created_at,
            spec_version: r.spec_version.map(|v| v as u32),
            spec_json: r.spec_json,
            sort_order: r.sort_order as u32,
            pinned: r.pinned != 0,
            status,
        }
    }
}

// ── repository ────────────────────────────────────────────────────────────────

const SELECT: &str = "SELECT id, project_id, title, title_source, spec_version, spec_json,
                             sort_order, pinned, archived_at, created_at
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
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &SessionId) -> Result<Option<Session>> {
        let row: Option<SessionRow> =
            sqlx::query_as(AssertSqlSafe(format!("{SELECT} WHERE id = ?")))
                .bind(id.as_str())
                .fetch_optional(exec)
                .await?;
        Ok(row.map(Session::from))
    }

    /// List sessions for a project, pinned-first then by sort_order.
    ///
    /// `Page::Cursor` uses keyset pagination; the cursor is the `id` of the
    /// last seen row (looked up via a subquery for stable ordering).
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        project_id: &ProjectId,
        page: Page,
    ) -> Result<Listing<Session>> {
        let order = "ORDER BY pinned DESC, sort_order ASC, id ASC";
        let where_parent = "WHERE project_id = ?";

        match page {
            Page::All => {
                let rows: Vec<SessionRow> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {where_parent} {order}")))
                        .bind(project_id.as_str())
                        .fetch_all(exec)
                        .await?;
                let items = rows.into_iter().map(Session::from).collect();
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let rows: Vec<SessionRow> = sqlx::query_as(AssertSqlSafe(format!(
                    "{SELECT} {where_parent} {order} LIMIT ? OFFSET ?"
                )))
                .bind(project_id.as_str())
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(exec)
                .await?;
                let items = rows.into_iter().map(Session::from).collect();
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                // Fetch limit+1 to detect whether a next page exists without a
                // COUNT query. If we get more than limit rows back, a next page
                // exists; we truncate to limit before returning.
                let fetch_n = limit as i64 + 1;
                let rows: Vec<SessionRow> = if let Some(cursor) = after {
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
                let items: Vec<Session> = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(Session::from)
                    .collect();
                let next = if has_more {
                    items.last().map(|s| s.id.as_str().to_owned())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }

    /// List ALL sessions across every project, pinned-first then by sort_order.
    /// The sidebar groups sessions by project, so it loads them in one call.
    pub async fn list_all<'e>(
        exec: impl SqliteExecutor<'e>,
        page: Page,
    ) -> Result<Listing<Session>> {
        let order = "ORDER BY pinned DESC, sort_order ASC, id ASC";
        match page {
            Page::All => {
                let rows: Vec<SessionRow> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order}")))
                        .fetch_all(exec)
                        .await?;
                let items = rows.into_iter().map(Session::from).collect();
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let rows: Vec<SessionRow> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order} LIMIT ? OFFSET ?")))
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(exec)
                        .await?;
                let items = rows.into_iter().map(Session::from).collect();
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                let fetch_n = limit as i64 + 1;
                let rows: Vec<SessionRow> = if let Some(cursor) = after {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "WITH anchor AS (
                             SELECT pinned, sort_order FROM session WHERE id = ?
                         )
                         {SELECT}
                         WHERE (pinned < (SELECT pinned FROM anchor)
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order > (SELECT sort_order FROM anchor))
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order = (SELECT sort_order FROM anchor)
                                    AND id > ?))
                         {order}
                         LIMIT ?"
                    )))
                    .bind(&cursor)
                    .bind(&cursor)
                    .bind(fetch_n)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order} LIMIT ?")))
                        .bind(fetch_n)
                        .fetch_all(exec)
                        .await?
                };
                let has_more = rows.len() > limit as usize;
                let items: Vec<Session> = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(Session::from)
                    .collect();
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

    /// Search sessions by title using a sqlite LIKE filter.
    /// Returns all sessions whose title contains `query` (case-insensitive).
    pub async fn search<'e>(exec: impl SqliteExecutor<'e>, query: &str) -> Result<Vec<Session>> {
        let pattern = format!("%{}%", query);
        let rows: Vec<SessionRow> = sqlx::query_as(AssertSqlSafe(format!(
            "{SELECT} WHERE title LIKE ? ORDER BY pinned DESC, sort_order ASC, id ASC"
        )))
        .bind(pattern)
        .fetch_all(exec)
        .await?;
        Ok(rows.into_iter().map(Session::from).collect())
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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

    // Scenario: Children are found by parent id
    #[tokio::test]
    async fn list_filters_by_project_id() {
        let pool = migrate::open_memory().await.unwrap();
        let unfiled_pid = unfiled();

        // Insert a second project under the seeded Default workspace.
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("proj-other")
            .bind("00000000-0000-0000-0000-000000000001")
            .bind("Other")
            .execute(&pool)
            .await
            .unwrap();

        let other_pid = ProjectId::new("proj-other");

        let s1 = make_session("s-f-1", &unfiled_pid, 0);
        let s2 = make_session("s-f-2", &unfiled_pid, 1);
        let s_other = make_session("s-f-other", &other_pid, 0);

        SessionRepo::create(&pool, &s1).await.unwrap();
        SessionRepo::create(&pool, &s2).await.unwrap();
        SessionRepo::create(&pool, &s_other).await.unwrap();

        let listing = SessionRepo::list(&pool, &unfiled_pid, Page::All)
            .await
            .unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|s| s.id.as_str()).collect();

        assert!(ids.contains(&"s-f-1"));
        assert!(ids.contains(&"s-f-2"));
        assert!(!ids.contains(&"s-f-other"));
    }

    // Scenario: A pinned item sorts ahead of unpinned
    #[tokio::test]
    async fn list_returns_pinned_first() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();

        let unpinned = make_session("s-unpinned", &pid, 0);
        let pinned = Session {
            pinned: true,
            sort_order: 99, // high sort_order; pinned flag dominates
            ..make_session("s-pinned", &pid, 99)
        };

        SessionRepo::create(&pool, &unpinned).await.unwrap();
        SessionRepo::create(&pool, &pinned).await.unwrap();

        let listing = SessionRepo::list(&pool, &pid, Page::All).await.unwrap();
        assert_eq!(listing.items[0].id.as_str(), "s-pinned");
        assert_eq!(listing.items[1].id.as_str(), "s-unpinned");
    }

    // Scenario: Offset pagination
    #[tokio::test]
    async fn list_offset_respects_limit_and_offset() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        for i in 0u32..5 {
            SessionRepo::create(&pool, &make_session(&format!("s-off-{i}"), &pid, i))
                .await
                .unwrap();
        }

        let page1 = SessionRepo::list(&pool, &pid, Page::offset(2, 0))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].sort_order, 0);
        assert_eq!(page1.items[1].sort_order, 1);

        let page2 = SessionRepo::list(&pool, &pid, Page::offset(2, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.items[0].sort_order, 2);
        assert_eq!(page2.items[1].sort_order, 3);
    }

    // Scenario: A bounded cursor page returns a continuation cursor
    #[tokio::test]
    async fn list_cursor_returns_next_when_more_remain() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        for i in 0u32..4 {
            SessionRepo::create(&pool, &make_session(&format!("s-cur-{i}"), &pid, i))
                .await
                .unwrap();
        }

        let page1 = SessionRepo::list(&pool, &pid, Page::cursor_from_start(2))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next.is_some(), "should have a continuation cursor");

        let cursor = page1.next.unwrap();
        let page2 = SessionRepo::list(&pool, &pid, Page::cursor_after(cursor, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(page2.next.is_none(), "last page has no cursor");
    }

    #[tokio::test]
    async fn list_cursor_last_page_has_no_next() {
        let pool = migrate::open_memory().await.unwrap();
        let pid = unfiled();
        for i in 0u32..3 {
            SessionRepo::create(&pool, &make_session(&format!("s-last-{i}"), &pid, i))
                .await
                .unwrap();
        }

        let listing = SessionRepo::list(&pool, &pid, Page::cursor_from_start(10))
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 3);
        assert!(listing.next.is_none());
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

        let listing = SessionRepo::list(&pool, &pid, Page::All).await.unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"s-tx-1"));
        assert!(ids.contains(&"s-tx-2"));
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
