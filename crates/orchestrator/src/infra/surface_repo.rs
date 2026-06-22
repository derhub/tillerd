//! Per-entity async sqlx repository for [`Surface`].
//!
//! All methods take `impl SqliteExecutor<'_>` so the same function works whether
//! the caller passes `cx.db()` (a pool ref) or `&mut *tx` (a mutable transaction
//! borrow).  The repo owns the `Row -> Entity` mapping; no slug-tree, no directory
//! moves.

use sqlx::SqliteExecutor;

use crate::entities::session::SessionId;
use crate::entities::surface::{NewSurface, Surface, SurfaceId, SurfaceKind, SurfaceStatus};
use crate::shared::errors::{Error, Result};
use crate::shared::pagination::{Listing, Page};

// ── Row ───────────────────────────────────────────────────────────────────────

struct SurfaceRow {
    id: String,
    session_id: String,
    kind: String,
    cwd: Option<String>,
    placement: Option<String>,
    status: String,
    created_at: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for SurfaceRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(SurfaceRow {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            kind: row.try_get("kind")?,
            cwd: row.try_get("cwd")?,
            placement: row.try_get("placement")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl TryFrom<SurfaceRow> for Surface {
    type Error = Error;

    fn try_from(r: SurfaceRow) -> Result<Self> {
        let kind = match r.kind.as_str() {
            "terminal" => SurfaceKind::Terminal,
            "diff" => SurfaceKind::Diff,
            other => {
                return Err(Error::Validation {
                    field: "kind",
                    reason: format!("unknown surface kind: {other}"),
                })
            }
        };
        let status = match r.status.as_str() {
            "pending" => SurfaceStatus::Pending,
            "live" => SurfaceStatus::Live,
            "idle" => SurfaceStatus::Idle,
            "failed" => SurfaceStatus::Failed,
            other => {
                return Err(Error::Validation {
                    field: "status",
                    reason: format!("unknown surface status: {other}"),
                })
            }
        };
        Ok(Surface {
            id: SurfaceId::from_string(r.id),
            session_id: SessionId::from_string(r.session_id),
            kind,
            cwd: r.cwd,
            placement: r.placement,
            status,
        })
    }
}

fn map_rows(rows: Vec<SurfaceRow>) -> Result<Vec<Surface>> {
    rows.into_iter().map(Surface::try_from).collect()
}

// ── Repository ────────────────────────────────────────────────────────────────

pub struct SurfaceRepo;

impl SurfaceRepo {
    /// Insert a new surface row at status `pending`. Returns the created `Surface`.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, new: &NewSurface) -> Result<Surface> {
        let id = new.id.clone().unwrap_or_else(SurfaceId::mint);

        sqlx::query(
            "INSERT INTO surface (id, session_id, kind, cwd, placement, status)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(new.session_id.as_str())
        .bind(new.kind.as_str())
        .bind(new.cwd.as_deref())
        .bind(new.placement.as_deref())
        .bind(SurfaceStatus::Pending.as_str())
        .execute(exec)
        .await?;

        Ok(Surface {
            id,
            session_id: new.session_id.clone(),
            kind: new.kind,
            cwd: new.cwd.clone(),
            placement: new.placement.clone(),
            status: SurfaceStatus::Pending,
        })
    }

    /// Fetch one surface by id. Returns `None` when not found.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &SurfaceId) -> Result<Option<Surface>> {
        let row: Option<SurfaceRow> = sqlx::query_as(
            "SELECT id, session_id, kind, cwd, placement, status, created_at
             FROM surface WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?;
        row.map(Surface::try_from).transpose()
    }

    /// List surfaces in a session ordered live-first then by `created_at`, with
    /// optional pagination.
    ///
    /// Surfaces have no `pinned` column; the pinned-first contract is fulfilled by
    /// ordering `live` status first (live surfaces are the "active" / promoted ones).
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        session_id: &SessionId,
        page: Page,
    ) -> Result<Listing<Surface>> {
        match page {
            Page::All => {
                let rows: Vec<SurfaceRow> = sqlx::query_as(
                    "SELECT id, session_id, kind, cwd, placement, status, created_at
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC",
                )
                .bind(session_id.as_str())
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(map_rows(rows)?, None))
            }

            Page::Offset { limit, offset } => {
                let fetch = (limit as i64) + 1;
                let rows: Vec<SurfaceRow> = sqlx::query_as(
                    "SELECT id, session_id, kind, cwd, placement, status, created_at
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                     LIMIT ? OFFSET ?",
                )
                .bind(session_id.as_str())
                .bind(fetch)
                .bind(offset as i64)
                .fetch_all(exec)
                .await?;

                let has_next = rows.len() as u32 > limit;
                let next = has_next.then(|| (offset + limit).to_string());
                let items = map_rows(rows.into_iter().take(limit as usize).collect())?;
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                // Cursor is the `created_at` of the last row on the previous page.
                // Because live surfaces sort first, cursor-based paging within a
                // mixed-status set may skip some non-live rows; callers that need
                // strict stability should use offset pagination or Page::All.
                let fetch = (limit as i64) + 1;
                let rows: Vec<SurfaceRow> = if let Some(cursor) = after {
                    sqlx::query_as(
                        "SELECT id, session_id, kind, cwd, placement, status, created_at
                         FROM surface
                         WHERE session_id = ? AND created_at > ?
                         ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                         LIMIT ?",
                    )
                    .bind(session_id.as_str())
                    .bind(cursor)
                    .bind(fetch)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(
                        "SELECT id, session_id, kind, cwd, placement, status, created_at
                         FROM surface
                         WHERE session_id = ?
                         ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                         LIMIT ?",
                    )
                    .bind(session_id.as_str())
                    .bind(fetch)
                    .fetch_all(exec)
                    .await?
                };

                let has_next = rows.len() as u32 > limit;
                let next_cursor = has_next
                    .then(|| rows.get(limit as usize - 1).map(|r| r.created_at.clone()))
                    .flatten();
                let items = map_rows(rows.into_iter().take(limit as usize).collect())?;
                Ok(Listing::new(items, next_cursor))
            }
        }
    }

    /// Update mutable fields of a surface (cwd and placement).
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, surface: &Surface) -> Result<()> {
        sqlx::query("UPDATE surface SET cwd = ?, placement = ? WHERE id = ?")
            .bind(surface.cwd.as_deref())
            .bind(surface.placement.as_deref())
            .bind(surface.id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Transition a surface to a new status (D9: persist intent -> record outcome).
    pub async fn update_status<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SurfaceId,
        status: SurfaceStatus,
    ) -> Result<()> {
        sqlx::query("UPDATE surface SET status = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Delete a surface row by id (used by `CloseSurface`).
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &SurfaceId) -> Result<()> {
        sqlx::query("DELETE FROM surface WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Find a surface by session + placement slot. Used by `FindSurfaceByPlacement`.
    pub async fn find_by_placement<'e>(
        exec: impl SqliteExecutor<'e>,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        let row: Option<SurfaceRow> = sqlx::query_as(
            "SELECT id, session_id, kind, cwd, placement, status, created_at
             FROM surface WHERE session_id = ? AND placement = ?",
        )
        .bind(session_id.as_str())
        .bind(placement)
        .fetch_optional(exec)
        .await?;
        row.map(Surface::try_from).transpose()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrate;

    // Insert a session row referencing the seeded Unfiled project so that the
    // `surface.session_id REFERENCES session(id)` FK constraint is satisfied.
    async fn seed_session(pool: &sqlx::SqlitePool, id: &str) -> SessionId {
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind(id)
            .bind("00000000-0000-0000-0000-000000000000") // seeded Unfiled project
            .bind("test")
            .execute(pool)
            .await
            .expect("seed session");
        SessionId::from_string(id)
    }

    fn new_terminal(session: &SessionId) -> NewSurface {
        NewSurface {
            id: None,
            session_id: session.clone(),
            kind: SurfaceKind::Terminal,
            cwd: Some("/work".to_owned()),
            placement: None,
        }
    }

    // Scenario: A repository persists and reads a typed entity (round-trip)
    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-rt").await;
        let new = NewSurface {
            id: None,
            session_id: sess.clone(),
            kind: SurfaceKind::Terminal,
            cwd: Some("/work".to_owned()),
            placement: Some("main".to_owned()),
        };

        let created = SurfaceRepo::create(&pool, &new).await.unwrap();
        assert_eq!(created.kind, SurfaceKind::Terminal);
        assert_eq!(created.cwd.as_deref(), Some("/work"));
        assert_eq!(created.placement.as_deref(), Some("main"));
        assert_eq!(created.status, SurfaceStatus::Pending);
        assert_eq!(created.session_id, sess);

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.session_id, sess);
        assert_eq!(fetched.kind, SurfaceKind::Terminal);
        assert_eq!(fetched.cwd.as_deref(), Some("/work"));
        assert_eq!(fetched.placement.as_deref(), Some("main"));
        assert_eq!(fetched.status, SurfaceStatus::Pending);
    }

    #[tokio::test]
    async fn get_returns_none_for_absent_id() {
        let pool = migrate::open_memory().await.unwrap();
        let missing = SurfaceId::from_string("no-such-id");
        let result = SurfaceRepo::get(&pool, &missing).await.unwrap();
        assert!(result.is_none());
    }

    // Scenario: Children are found by parent id
    #[tokio::test]
    async fn list_filters_by_session() {
        let pool = migrate::open_memory().await.unwrap();
        let sess_a = seed_session(&pool, "s-a").await;
        let sess_b = seed_session(&pool, "s-b").await;

        SurfaceRepo::create(&pool, &new_terminal(&sess_a))
            .await
            .unwrap();
        SurfaceRepo::create(&pool, &new_terminal(&sess_a))
            .await
            .unwrap();
        SurfaceRepo::create(&pool, &new_terminal(&sess_b))
            .await
            .unwrap();

        let result = SurfaceRepo::list(&pool, &sess_a, Page::All).await.unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.items.iter().all(|s| s.session_id == sess_a));
    }

    // Scenario: A bounded page returns a continuation cursor
    #[tokio::test]
    async fn list_offset_pagination_returns_next_cursor() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-pg").await;
        for _ in 0..5 {
            SurfaceRepo::create(&pool, &new_terminal(&sess))
                .await
                .unwrap();
        }

        let page1 = SurfaceRepo::list(&pool, &sess, Page::offset(3, 0))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 3);
        assert!(page1.has_next());

        let page2 = SurfaceRepo::list(&pool, &sess, Page::offset(3, 3))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert!(!page2.has_next());
    }

    // Scenario: Unbounded listing is explicit
    #[tokio::test]
    async fn list_all_returns_every_row() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-all").await;
        for _ in 0..4 {
            SurfaceRepo::create(&pool, &new_terminal(&sess))
                .await
                .unwrap();
        }
        let result = SurfaceRepo::list(&pool, &sess, Page::All).await.unwrap();
        assert_eq!(result.items.len(), 4);
        assert!(!result.has_next());
    }

    #[tokio::test]
    async fn update_persists_cwd_and_placement() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-upd").await;
        let created = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();

        let updated = Surface {
            cwd: Some("/updated".to_owned()),
            placement: Some("slot-1".to_owned()),
            ..created.clone()
        };
        SurfaceRepo::update(&pool, &updated).await.unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.cwd.as_deref(), Some("/updated"));
        assert_eq!(fetched.placement.as_deref(), Some("slot-1"));
    }

    #[tokio::test]
    async fn delete_removes_the_row() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-del").await;
        let created = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();

        SurfaceRepo::delete(&pool, &created.id).await.unwrap();

        let result = SurfaceRepo::get(&pool, &created.id).await.unwrap();
        assert!(result.is_none());
    }

    // Status transitions
    #[tokio::test]
    async fn update_status_transitions_pending_to_live() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-st1").await;
        let created = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();
        assert_eq!(created.status, SurfaceStatus::Pending);

        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Live)
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SurfaceStatus::Live);
    }

    #[tokio::test]
    async fn update_status_transitions_live_to_idle() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-st2").await;
        let created = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();

        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Live)
            .await
            .unwrap();
        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Idle)
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SurfaceStatus::Idle);
    }

    #[tokio::test]
    async fn update_status_transitions_to_failed() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-st3").await;
        let created = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();

        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Failed)
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SurfaceStatus::Failed);
    }

    // Scenario: A pinned item sorts ahead — surfaces use live-first ordering
    #[tokio::test]
    async fn list_returns_live_surfaces_first() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-ord").await;

        let first = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();
        let second = SurfaceRepo::create(&pool, &new_terminal(&sess))
            .await
            .unwrap();

        // Make the second one live; the first stays idle.
        SurfaceRepo::update_status(&pool, &first.id, SurfaceStatus::Idle)
            .await
            .unwrap();
        SurfaceRepo::update_status(&pool, &second.id, SurfaceStatus::Live)
            .await
            .unwrap();

        let result = SurfaceRepo::list(&pool, &sess, Page::All).await.unwrap();
        assert_eq!(
            result.items[0].id, second.id,
            "live surface must sort first"
        );
        assert_eq!(result.items[1].id, first.id);
    }

    // Scenario: multi-repo call on one tx is atomic
    #[tokio::test]
    async fn two_creates_on_one_transaction_both_commit() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-tx").await;
        let na = new_terminal(&sess);
        let nb = new_terminal(&sess);

        let (a, b) = {
            let mut tx = pool.begin().await.unwrap();
            let a = SurfaceRepo::create(&mut *tx, &na).await.unwrap();
            let b = SurfaceRepo::create(&mut *tx, &nb).await.unwrap();
            tx.commit().await.unwrap();
            (a, b)
        };

        let all = SurfaceRepo::list(&pool, &sess, Page::All).await.unwrap();
        let ids: Vec<&SurfaceId> = all.items.iter().map(|s| &s.id).collect();
        assert!(ids.contains(&&a.id));
        assert!(ids.contains(&&b.id));
    }

    #[tokio::test]
    async fn transaction_rollback_leaves_no_rows() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-rb").await;
        let na = new_terminal(&sess);

        let id = {
            let mut tx = pool.begin().await.unwrap();
            let s = SurfaceRepo::create(&mut *tx, &na).await.unwrap();
            tx.rollback().await.unwrap();
            s.id
        };

        let result = SurfaceRepo::get(&pool, &id).await.unwrap();
        assert!(result.is_none(), "rolled-back row must not persist");
    }

    // Scenario: Spawn mints a placement that find resolves
    #[tokio::test]
    async fn find_by_placement_returns_surface_for_known_slot() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-fp1").await;
        let new = NewSurface {
            id: None,
            session_id: sess.clone(),
            kind: SurfaceKind::Terminal,
            cwd: None,
            placement: Some("panel-1".to_owned()),
        };
        let created = SurfaceRepo::create(&pool, &new).await.unwrap();

        let found = SurfaceRepo::find_by_placement(&pool, &sess, "panel-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, created.id);
    }

    #[tokio::test]
    async fn find_by_placement_returns_none_for_absent_slot() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-fp2").await;
        let result = SurfaceRepo::find_by_placement(&pool, &sess, "no-slot")
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
