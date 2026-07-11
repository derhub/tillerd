//! Per-entity async sqlx repository for [`Surface`].
//!
//! All methods take `impl SqliteExecutor<'_>` so the same function works whether
//! the caller passes `cx.db()` (a pool ref) or `&mut *tx` (a mutable transaction
//! borrow).  Reads decode straight into the [`Surface`] entity via `query_as`
//! (`kind`/`status` are `sqlx::Type` newtypes); the read-model `*View` projections
//! live in `app/surface/`.
//!
//! This repo keeps only the command-path methods plus `list`, which is consumed by
//! the `session`/`project`/`workspace` commands (cross-domain). The read-only
//! `get_surface_by_id`/`find_by_placement`/`list_resumable` SELECTs moved into the
//! `app/surface/` query handlers as `SurfaceView` projections.

use sqlx::SqliteExecutor;

use crate::entities::session::SessionId;
use crate::entities::surface::{Surface, SurfaceId, SurfaceKind, SurfaceStatus};
use crate::shared::errors::Result;
use crate::shared::pagination::{Listing, Page};

/// A [`Surface`] plus the `created_at` cursor-key column needed to mint the
/// continuation cursor without re-querying. `Surface` itself carries no
/// `created_at`, so cursor paging flattens it alongside.
#[derive(sqlx::FromRow)]
struct CursorRow {
    #[sqlx(flatten)]
    surface: Surface,
    created_at: String,
}

pub struct SurfaceRepo;

impl SurfaceRepo {
    /// Insert a new surface row at the given `initial_status`. Returns the created `Surface`.
    /// Pass `id = None` to mint a fresh UUID.
    pub async fn create<'e>(
        exec: impl SqliteExecutor<'e>,
        id: Option<&str>,
        session_id: &SessionId,
        kind: SurfaceKind,
        cwd: Option<&str>,
        placement: Option<&str>,
        initial_status: SurfaceStatus,
    ) -> Result<Surface> {
        let surface_id = id
            .map(SurfaceId::from_string)
            .unwrap_or_else(SurfaceId::mint);

        sqlx::query(
            "INSERT INTO surface (id, session_id, kind, cwd, placement, status)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(surface_id.as_str())
        .bind(session_id.as_str())
        .bind(kind.as_str())
        .bind(cwd)
        .bind(placement)
        .bind(initial_status.as_str())
        .execute(exec)
        .await?;

        Ok(Surface {
            id: surface_id,
            session_id: session_id.clone(),
            kind,
            cwd: cwd.map(str::to_owned),
            placement: placement.map(str::to_owned),
            status: initial_status,
        })
    }

    /// Fetch one surface by id. Returns `None` when not found. Command path
    /// (read-modify-write): `require_surface` reads the entity before a transition.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &SurfaceId) -> Result<Option<Surface>> {
        Ok(sqlx::query_as::<_, Surface>(
            "SELECT id, session_id, kind, cwd, status, placement
             FROM surface WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?)
    }

    /// List surfaces in a session ordered by `id` (stable insertion order), with
    /// optional pagination. Consumed by the `session`/`project`/`workspace`
    /// commands (cross-domain), so it returns the entity, not a read `*View`.
    ///
    /// Semantic ordering (live-first) is the caller's responsibility; this raw
    /// method returns rows in a stable, predictable order only.
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        session_id: &SessionId,
        page: Page,
    ) -> Result<Listing<Surface>> {
        match page {
            Page::All => {
                let items = sqlx::query_as::<_, Surface>(
                    "SELECT id, session_id, kind, cwd, status, placement
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY id ASC",
                )
                .bind(session_id.as_str())
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                let fetch = (limit as i64) + 1;
                let rows = sqlx::query_as::<_, Surface>(
                    "SELECT id, session_id, kind, cwd, status, placement
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY id ASC
                     LIMIT ? OFFSET ?",
                )
                .bind(session_id.as_str())
                .bind(fetch)
                .bind(offset as i64)
                .fetch_all(exec)
                .await?;

                let has_next = rows.len() as u32 > limit;
                let next = has_next.then(|| (offset + limit).to_string());
                let items: Vec<Surface> = rows.into_iter().take(limit as usize).collect();
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                let fetch = (limit as i64) + 1;
                let rows: Vec<CursorRow> = if let Some(cursor) = after {
                    sqlx::query_as::<_, CursorRow>(
                        "SELECT id, session_id, kind, cwd, status, placement, created_at
                         FROM surface
                         WHERE session_id = ? AND created_at > ?
                         ORDER BY id ASC
                         LIMIT ?",
                    )
                    .bind(session_id.as_str())
                    .bind(cursor)
                    .bind(fetch)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as::<_, CursorRow>(
                        "SELECT id, session_id, kind, cwd, status, placement, created_at
                         FROM surface
                         WHERE session_id = ?
                         ORDER BY id ASC
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
                let items: Vec<Surface> = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(|r| r.surface)
                    .collect();
                Ok(Listing::new(items, next_cursor))
            }
        }
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

    /// Transition only from an expected prior status; returns whether a row
    /// changed. Lets a spawn confirmation (`pending -> live`) lose gracefully to
    /// an exit-frame write that already recorded the PTY's death.
    pub async fn update_status_from<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SurfaceId,
        from: SurfaceStatus,
        to: SurfaceStatus,
    ) -> Result<bool> {
        let result = sqlx::query("UPDATE surface SET status = ? WHERE id = ? AND status = ?")
            .bind(to.as_str())
            .bind(id.as_str())
            .bind(from.as_str())
            .execute(exec)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Stamp the millis at which the row's PTY was last confirmed running
    /// (elapsed-since-spawn display, ui-panel-compound spec). Only ever moves
    /// forward on a real spawn/respawn -- never cleared on stop/exit, so the
    /// last known spawn moment survives an idle/failed transition.
    pub async fn set_spawned_at<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SurfaceId,
        spawned_at_ms: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE surface SET spawned_at = ? WHERE id = ?")
            .bind(spawned_at_ms)
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Fetch the surface bound to a session + placement slot, if any. Executor-generic
    /// so `SwapPlacement` can resolve both sides on the pool, then rebind them inside
    /// one transaction via `update_placement`.
    pub async fn find_by_placement<'e>(
        exec: impl SqliteExecutor<'e>,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        Ok(sqlx::query_as::<_, Surface>(
            "SELECT id, session_id, kind, cwd, status, placement
             FROM surface WHERE session_id = ? AND placement = ?",
        )
        .bind(session_id.as_str())
        .bind(placement)
        .fetch_optional(exec)
        .await?)
    }

    /// Rebind a surface's placement slot (used by `SwapPlacement`).
    pub async fn update_placement<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &SurfaceId,
        placement: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE surface SET placement = ? WHERE id = ?")
            .bind(placement)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::surface::SurfaceKind;
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

    async fn new_terminal<'e>(exec: impl SqliteExecutor<'e>, session: &SessionId) -> Surface {
        SurfaceRepo::create(
            exec,
            None,
            session,
            SurfaceKind::Terminal,
            Some("/work"),
            None,
            SurfaceStatus::Pending,
        )
        .await
        .expect("new_terminal")
    }

    // Scenario: A repository persists and reads a typed entity (round-trip)
    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-rt").await;

        let created = SurfaceRepo::create(
            &pool,
            None,
            &sess,
            SurfaceKind::Terminal,
            Some("/work"),
            Some("main"),
            SurfaceStatus::Pending,
        )
        .await
        .unwrap();
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

        new_terminal(&pool, &sess_a).await;
        new_terminal(&pool, &sess_a).await;
        new_terminal(&pool, &sess_b).await;

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
            new_terminal(&pool, &sess).await;
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
            new_terminal(&pool, &sess).await;
        }
        let result = SurfaceRepo::list(&pool, &sess, Page::All).await.unwrap();
        assert_eq!(result.items.len(), 4);
        assert!(!result.has_next());
    }

    #[tokio::test]
    async fn delete_removes_the_row() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-del").await;
        let created = new_terminal(&pool, &sess).await;

        SurfaceRepo::delete(&pool, &created.id).await.unwrap();

        let result = SurfaceRepo::get(&pool, &created.id).await.unwrap();
        assert!(result.is_none());
    }

    // Status transitions
    #[tokio::test]
    async fn update_status_transitions_pending_to_live() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-st1").await;
        let created = new_terminal(&pool, &sess).await;
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
        let created = new_terminal(&pool, &sess).await;

        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Live)
            .await
            .unwrap();
        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Idle)
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SurfaceStatus::Idle);
    }

    // Scenario: a fresh row has no spawn timestamp until stamped.
    #[tokio::test]
    async fn spawned_at_is_null_before_it_is_stamped() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-sp1").await;
        let created = new_terminal(&pool, &sess).await;

        let (spawned_at,): (Option<i64>,) =
            sqlx::query_as("SELECT spawned_at FROM surface WHERE id = ?")
                .bind(created.id.as_str())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(spawned_at, None);
    }

    // Scenario: stamping persists the exact millis passed in.
    #[tokio::test]
    async fn set_spawned_at_persists_the_stamp() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-sp2").await;
        let created = new_terminal(&pool, &sess).await;

        SurfaceRepo::set_spawned_at(&pool, &created.id, 1_700_000_000_000)
            .await
            .unwrap();

        let (spawned_at,): (Option<i64>,) =
            sqlx::query_as("SELECT spawned_at FROM surface WHERE id = ?")
                .bind(created.id.as_str())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(spawned_at, Some(1_700_000_000_000));
    }

    #[tokio::test]
    async fn update_status_transitions_to_failed() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-st3").await;
        let created = new_terminal(&pool, &sess).await;

        SurfaceRepo::update_status(&pool, &created.id, SurfaceStatus::Failed)
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SurfaceStatus::Failed);
    }

    // Scenario: A surface is looked up by its natural session + placement key.
    #[tokio::test]
    async fn find_by_placement_resolves_the_bound_surface() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-fbp").await;
        let created = SurfaceRepo::create(
            &pool,
            None,
            &sess,
            SurfaceKind::Terminal,
            None,
            Some("main"),
            SurfaceStatus::Pending,
        )
        .await
        .unwrap();

        let found = SurfaceRepo::find_by_placement(&pool, &sess, "main")
            .await
            .unwrap();
        assert_eq!(found.map(|s| s.id), Some(created.id));

        let missing = SurfaceRepo::find_by_placement(&pool, &sess, "no-such-slot")
            .await
            .unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn update_placement_rebinds_the_slot() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-upl").await;
        let created = new_terminal(&pool, &sess).await;

        SurfaceRepo::update_placement(&pool, &created.id, Some("moved"))
            .await
            .unwrap();

        let fetched = SurfaceRepo::get(&pool, &created.id).await.unwrap().unwrap();
        assert_eq!(fetched.placement.as_deref(), Some("moved"));
    }

    // Scenario: Raw list returns a stable, predictable order regardless of status.
    // Live-first ordering is the app layer's responsibility (via SURFACE_ORDER constant).
    #[tokio::test]
    async fn list_returns_rows_in_stable_id_order() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-ord").await;

        // Insert with explicit ids so insertion and id order are the same.
        SurfaceRepo::create(
            &pool,
            Some("aaa"),
            &sess,
            SurfaceKind::Terminal,
            None,
            None,
            SurfaceStatus::Idle,
        )
        .await
        .unwrap();
        SurfaceRepo::create(
            &pool,
            Some("bbb"),
            &sess,
            SurfaceKind::Terminal,
            None,
            None,
            SurfaceStatus::Live,
        )
        .await
        .unwrap();

        let result = SurfaceRepo::list(&pool, &sess, Page::All).await.unwrap();
        // Raw repo returns id-ascending; the live "bbb" does NOT float to front.
        assert_eq!(result.items[0].id.as_str(), "aaa");
        assert_eq!(result.items[1].id.as_str(), "bbb");
    }

    // Scenario: multi-repo call on one tx is atomic
    #[tokio::test]
    async fn two_creates_on_one_transaction_both_commit() {
        let pool = migrate::open_memory().await.unwrap();
        let sess = seed_session(&pool, "s-tx").await;

        let (a, b) = {
            let mut tx = pool.begin().await.unwrap();
            let a = SurfaceRepo::create(
                &mut *tx,
                None,
                &sess,
                SurfaceKind::Terminal,
                Some("/work"),
                None,
                SurfaceStatus::Pending,
            )
            .await
            .unwrap();
            let b = SurfaceRepo::create(
                &mut *tx,
                None,
                &sess,
                SurfaceKind::Terminal,
                Some("/work"),
                None,
                SurfaceStatus::Pending,
            )
            .await
            .unwrap();
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

        let id = {
            let mut tx = pool.begin().await.unwrap();
            let s = SurfaceRepo::create(
                &mut *tx,
                None,
                &sess,
                SurfaceKind::Terminal,
                Some("/work"),
                None,
                SurfaceStatus::Pending,
            )
            .await
            .unwrap();
            tx.rollback().await.unwrap();
            s.id
        };

        let result = SurfaceRepo::get(&pool, &id).await.unwrap();
        assert!(result.is_none(), "rolled-back row must not persist");
    }
}
