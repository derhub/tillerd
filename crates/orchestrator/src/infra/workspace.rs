use sqlx::{FromRow, SqliteExecutor, SqlitePool};

use crate::entities::workspace::{NewWorkspace, Workspace, WorkspaceId, WorkspaceStatus};
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

// ── Row type ──────────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct WorkspaceRow {
    id: String,
    name: String,
    sort_order: i64,
    pinned: i64,
    archived_at: Option<String>,
}

impl From<WorkspaceRow> for Workspace {
    fn from(r: WorkspaceRow) -> Self {
        Workspace {
            id: WorkspaceId::new(r.id),
            name: r.name,
            sort_order: r.sort_order as u32,
            pinned: r.pinned != 0,
            status: if r.archived_at.is_some() {
                WorkspaceStatus::Archived
            } else {
                WorkspaceStatus::Active
            },
        }
    }
}

// ── Repo ─────────────────────────────────────────────────────────────────────

/// Per-entity async repository for `Workspace`. Methods take a sqlx executor
/// so the same method serves a direct pool call and a shared transaction.
pub struct WorkspaceRepo;

impl WorkspaceRepo {
    pub async fn create<'e>(
        exec: impl SqliteExecutor<'e>,
        params: &NewWorkspace,
        id: &WorkspaceId,
    ) -> Result<()> {
        sqlx::query("INSERT INTO workspace (id, name, sort_order, pinned) VALUES (?, ?, ?, ?)")
            .bind(id.as_str())
            .bind(&params.name)
            .bind(0i64)
            .bind(0i64)
            .execute(exec)
            .await?;
        Ok(())
    }

    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &WorkspaceId,
    ) -> Result<Option<Workspace>> {
        let row: Option<WorkspaceRow> = sqlx::query_as(
            "SELECT id, name, sort_order, pinned, archived_at FROM workspace WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?;
        Ok(row.map(Into::into))
    }

    /// List workspaces ordered pinned-first, then by `sort_order`.
    ///
    /// Includes both active and archived. Pass `Page::All` for an unbounded
    /// listing; `Page::Offset` or `Page::Cursor` for a bounded page.
    pub async fn list(pool: &SqlitePool, page: &Page) -> Result<Listing<Workspace>> {
        match page {
            Page::All => {
                let rows: Vec<WorkspaceRow> = sqlx::query_as(
                    "SELECT id, name, sort_order, pinned, archived_at
                     FROM workspace
                     ORDER BY pinned DESC, sort_order",
                )
                .fetch_all(pool)
                .await?;
                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    None,
                ))
            }
            Page::Offset { limit, offset } => {
                // Fetch limit+1 to detect whether a next page exists.
                let rows: Vec<WorkspaceRow> = sqlx::query_as(
                    "SELECT id, name, sort_order, pinned, archived_at
                     FROM workspace
                     ORDER BY pinned DESC, sort_order
                     LIMIT ? OFFSET ?",
                )
                .bind(*limit as i64 + 1)
                .bind(*offset as i64)
                .fetch_all(pool)
                .await?;
                let has_more = rows.len() > *limit as usize;
                let items: Vec<Workspace> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    Some((*offset + *limit).to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
            Page::Cursor { after, limit } => {
                let (items, next) = cursor_page(pool, after.as_deref(), *limit).await?;
                Ok(Listing::new(items, next))
            }
        }
    }

    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, ws: &Workspace) -> Result<()> {
        // For archive: COALESCE preserves an existing timestamp (idempotent re-archive).
        // For active: NULL clears it (restore).
        sqlx::query(
            "UPDATE workspace
             SET name        = ?,
                 sort_order  = ?,
                 pinned      = ?,
                 archived_at = CASE
                     WHEN ? = 'archived'
                         THEN COALESCE(archived_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                     ELSE NULL
                 END
             WHERE id = ?",
        )
        .bind(&ws.name)
        .bind(ws.sort_order as i64)
        .bind(ws.pinned as i64)
        .bind(ws.status.as_str())
        .bind(ws.id.as_str())
        .execute(exec)
        .await?;
        Ok(())
    }

    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &WorkspaceId) -> Result<()> {
        sqlx::query("DELETE FROM workspace WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }
}

// ── Cursor pagination helper ──────────────────────────────────────────────────

// Cursor format: opaque string that encodes the row's stable position in the
// `ORDER BY pinned DESC, sort_order` sequence.  We implement this as OFFSET:
// the cursor value is the next-page offset so we can derive it without a
// window function.  Format: decimal offset integer.

async fn cursor_page(
    pool: &SqlitePool,
    after: Option<&str>,
    limit: u32,
) -> Result<(Vec<Workspace>, Option<String>)> {
    // Cursor is the next-page offset encoded as a decimal string.
    let offset: i64 = match after {
        None => 0,
        Some(s) => s.parse::<i64>().unwrap_or(0),
    };

    // Fetch limit+1 to detect whether a next page exists without a second query.
    let rows: Vec<WorkspaceRow> = sqlx::query_as(
        "SELECT id, name, sort_order, pinned, archived_at
         FROM workspace
         ORDER BY pinned DESC, sort_order
         LIMIT ? OFFSET ?",
    )
    .bind(limit as i64 + 1)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items: Vec<Workspace> = rows
        .into_iter()
        .take(limit as usize)
        .map(Into::into)
        .collect();
    let next = if has_more {
        Some((offset + limit as i64).to_string())
    } else {
        None
    };
    Ok((items, next))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrate;

    async fn pool() -> SqlitePool {
        migrate::open_memory().await.expect("in-memory pool")
    }

    fn new_ws(name: &str) -> NewWorkspace {
        NewWorkspace {
            name: name.to_owned(),
        }
    }

    fn ws_id(s: &str) -> WorkspaceId {
        WorkspaceId::new(s)
    }

    // Scenario: A repository persists and reads a typed entity (round-trip).
    #[tokio::test]
    async fn workspace_round_trips_through_create_and_get() {
        let pool = pool().await;
        let id = ws_id("ws-rt-1");
        WorkspaceRepo::create(&pool, &new_ws("Round-trip"), &id)
            .await
            .unwrap();
        let ws = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(ws.id, id);
        assert_eq!(ws.name, "Round-trip");
        assert_eq!(ws.sort_order, 0);
        assert!(!ws.pinned);
        assert_eq!(ws.status, WorkspaceStatus::Active);
    }

    #[tokio::test]
    async fn get_returns_none_for_absent_workspace() {
        let pool = pool().await;
        let got = WorkspaceRepo::get(&pool, &ws_id("nonexistent"))
            .await
            .unwrap();
        assert!(got.is_none());
    }

    // Scenario: A rename is a plain update.
    #[tokio::test]
    async fn update_persists_new_name() {
        let pool = pool().await;
        let id = ws_id("ws-upd-1");
        WorkspaceRepo::create(&pool, &new_ws("Before"), &id)
            .await
            .unwrap();
        let mut ws = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        ws.rename("After");
        WorkspaceRepo::update(&pool, &ws).await.unwrap();
        let reloaded = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(reloaded.name, "After");
    }

    // Scenario: archive sets status and persists.
    #[tokio::test]
    async fn update_archives_workspace() {
        let pool = pool().await;
        let id = ws_id("ws-arch-1");
        WorkspaceRepo::create(&pool, &new_ws("Archivable"), &id)
            .await
            .unwrap();
        let mut ws = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        ws.status = WorkspaceStatus::Archived;
        WorkspaceRepo::update(&pool, &ws).await.unwrap();
        let reloaded = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(reloaded.status, WorkspaceStatus::Archived);
    }

    #[tokio::test]
    async fn update_restores_workspace_from_archived() {
        let pool = pool().await;
        let id = ws_id("ws-restore-1");
        WorkspaceRepo::create(&pool, &new_ws("Restore"), &id)
            .await
            .unwrap();
        let mut ws = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        ws.status = WorkspaceStatus::Archived;
        WorkspaceRepo::update(&pool, &ws).await.unwrap();

        let mut archived = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        archived.status = WorkspaceStatus::Active;
        WorkspaceRepo::update(&pool, &archived).await.unwrap();

        let reloaded = WorkspaceRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(reloaded.status, WorkspaceStatus::Active);
    }

    // Scenario: delete removes the workspace.
    #[tokio::test]
    async fn delete_removes_workspace() {
        let pool = pool().await;
        let id = ws_id("ws-del-1");
        WorkspaceRepo::create(&pool, &new_ws("ToDelete"), &id)
            .await
            .unwrap();
        WorkspaceRepo::delete(&pool, &id).await.unwrap();
        let got = WorkspaceRepo::get(&pool, &id).await.unwrap();
        assert!(got.is_none());
    }

    // Scenario: Children are found by parent id (list returns all workspaces).
    #[tokio::test]
    async fn list_all_returns_created_workspace() {
        let pool = pool().await;
        let id = ws_id("ws-list-1");
        WorkspaceRepo::create(&pool, &new_ws("Listed"), &id)
            .await
            .unwrap();
        let listing = WorkspaceRepo::list(&pool, &Page::All).await.unwrap();
        assert!(listing.items.iter().any(|w| w.id.as_str() == "ws-list-1"));
    }

    // Scenario: A pinned item sorts ahead of unpinned.
    #[tokio::test]
    async fn list_returns_pinned_workspaces_first() {
        let pool = pool().await;
        // Insert two additional workspaces (Default is seeded at sort_order 0).
        let unpinned_id = ws_id("ws-pin-unpinned");
        let pinned_id = ws_id("ws-pin-pinned");
        WorkspaceRepo::create(&pool, &new_ws("Unpinned"), &unpinned_id)
            .await
            .unwrap();
        WorkspaceRepo::create(&pool, &new_ws("Pinned"), &pinned_id)
            .await
            .unwrap();

        // Pin the second one.
        let mut pinned = WorkspaceRepo::get(&pool, &pinned_id)
            .await
            .unwrap()
            .unwrap();
        pinned.pinned = true;
        // Set a higher sort_order so it would naturally sort last.
        pinned.sort_order = 999;
        WorkspaceRepo::update(&pool, &pinned).await.unwrap();

        let listing = WorkspaceRepo::list(&pool, &Page::All).await.unwrap();
        let first = listing.items.first().expect("at least one item");
        assert!(
            first.pinned,
            "first listed workspace must be pinned; got: {:?}",
            first
        );
    }

    // Scenario: A bounded page returns a continuation cursor.
    #[tokio::test]
    async fn cursor_page_returns_next_when_more_rows_remain() {
        let pool = pool().await;
        // Seed enough workspaces so page-size 1 produces a next cursor.
        // Default workspace is already seeded; add one more.
        WorkspaceRepo::create(&pool, &new_ws("Extra"), &ws_id("ws-cur-1"))
            .await
            .unwrap();

        let listing = WorkspaceRepo::list(&pool, &Page::cursor_from_start(1))
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert!(
            listing.next.is_some(),
            "next cursor must be set when more rows remain"
        );
    }

    #[tokio::test]
    async fn cursor_page_at_end_has_no_next() {
        let pool = pool().await;
        // Only the seeded Default workspace. One result, no next.
        let listing = WorkspaceRepo::list(&pool, &Page::cursor_from_start(10))
            .await
            .unwrap();
        assert!(!listing.items.is_empty());
        assert!(
            listing.next.is_none(),
            "no next when all rows fit in the page"
        );
    }

    #[tokio::test]
    async fn cursor_continues_from_returned_cursor() {
        let pool = pool().await;
        // Default workspace seeded. Add two more so we have 3 total.
        WorkspaceRepo::create(&pool, &new_ws("W2"), &ws_id("ws-seq-2"))
            .await
            .unwrap();
        WorkspaceRepo::create(&pool, &new_ws("W3"), &ws_id("ws-seq-3"))
            .await
            .unwrap();

        let page1 = WorkspaceRepo::list(&pool, &Page::cursor_from_start(1))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 1);
        let cursor = page1.next.expect("must have next after page 1 of 3");

        let page2 = WorkspaceRepo::list(&pool, &Page::cursor_after(cursor, 1))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        // page2 item must differ from page1 item.
        assert_ne!(
            page2.items[0].id, page1.items[0].id,
            "page 2 must advance past page 1"
        );
    }

    // Scenario: offset pagination.
    #[tokio::test]
    async fn offset_page_returns_correct_slice() {
        let pool = pool().await;
        WorkspaceRepo::create(&pool, &new_ws("W2"), &ws_id("ws-off-2"))
            .await
            .unwrap();
        WorkspaceRepo::create(&pool, &new_ws("W3"), &ws_id("ws-off-3"))
            .await
            .unwrap();

        let page = WorkspaceRepo::list(&pool, &Page::offset(2, 0))
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
        // Next cursor is present because there are 3 rows and we took 2.
        assert!(page.next.is_some());

        let page2 = WorkspaceRepo::list(&pool, &Page::offset(2, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert!(page2.next.is_none());
    }

    // Scenario: multi-repo call on one tx is atomic.
    #[tokio::test]
    async fn two_creates_on_a_shared_transaction_are_atomic() {
        let pool = pool().await;
        let id_a = ws_id("ws-tx-a");
        let id_b = ws_id("ws-tx-b");

        // Both creates go through a single transaction.
        let mut tx = pool.begin().await.unwrap();
        WorkspaceRepo::create(&mut *tx, &new_ws("TxA"), &id_a)
            .await
            .unwrap();
        WorkspaceRepo::create(&mut *tx, &new_ws("TxB"), &id_b)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let a = WorkspaceRepo::get(&pool, &id_a).await.unwrap();
        let b = WorkspaceRepo::get(&pool, &id_b).await.unwrap();
        assert!(a.is_some(), "ws-tx-a must be visible after commit");
        assert!(b.is_some(), "ws-tx-b must be visible after commit");
    }

    #[tokio::test]
    async fn rolled_back_transaction_persists_nothing() {
        let pool = pool().await;
        let id = ws_id("ws-rollback-1");

        let mut tx = pool.begin().await.unwrap();
        WorkspaceRepo::create(&mut *tx, &new_ws("Ephemeral"), &id)
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        let got = WorkspaceRepo::get(&pool, &id).await.unwrap();
        assert!(got.is_none(), "rolled-back workspace must not be visible");
    }
}
