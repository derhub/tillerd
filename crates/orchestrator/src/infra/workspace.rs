use sqlx::SqliteExecutor;

use crate::entities::workspace::{Workspace, WorkspaceId};
use crate::shared::Result;

/// Per-entity async repository for `Workspace`. Methods take a sqlx executor
/// so the same method serves a direct pool call and a shared transaction.
pub struct WorkspaceRepo;

impl WorkspaceRepo {
    /// Insert a new workspace row from a fully-built entity.
    ///
    /// The caller assigns the id and builds the entity, so the repo stays pure.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, ws: &Workspace) -> Result<()> {
        sqlx::query("INSERT INTO workspace (id, name, sort_order, pinned) VALUES (?, ?, ?, ?)")
            .bind(ws.id.as_str())
            .bind(&ws.name)
            .bind(ws.sort_order as i64)
            .bind(ws.pinned as i64)
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Fetch a single workspace by id. Returns `None` when absent.
    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &WorkspaceId,
    ) -> Result<Option<Workspace>> {
        Ok(sqlx::query_as::<_, Workspace>(
            "SELECT id, name, sort_order, pinned,
                    CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
             FROM workspace
             WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::workspace::WorkspaceStatus;
    use crate::infra::migrate;

    async fn pool() -> sqlx::SqlitePool {
        migrate::open_memory().await.expect("in-memory pool")
    }

    fn ws_id(s: &str) -> WorkspaceId {
        WorkspaceId::new(s)
    }

    fn workspace(id: &str, name: &str) -> Workspace {
        Workspace {
            id: WorkspaceId::new(id),
            name: name.to_owned(),
            sort_order: 0,
            pinned: false,
            status: WorkspaceStatus::Active,
        }
    }

    // Scenario: A repository persists and reads a typed entity (round-trip).
    #[tokio::test]
    async fn workspace_round_trips_through_create_and_get() {
        let pool = pool().await;
        let id = ws_id("ws-rt-1");
        WorkspaceRepo::create(&pool, &workspace("ws-rt-1", "Round-trip"))
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
        WorkspaceRepo::create(&pool, &workspace("ws-upd-1", "Before"))
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
        WorkspaceRepo::create(&pool, &workspace("ws-arch-1", "Archivable"))
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
        WorkspaceRepo::create(&pool, &workspace("ws-restore-1", "Restore"))
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
        WorkspaceRepo::create(&pool, &workspace("ws-del-1", "ToDelete"))
            .await
            .unwrap();
        WorkspaceRepo::delete(&pool, &id).await.unwrap();
        let got = WorkspaceRepo::get(&pool, &id).await.unwrap();
        assert!(got.is_none());
    }

    // Scenario: multi-repo call on one tx is atomic.
    #[tokio::test]
    async fn two_creates_on_a_shared_transaction_are_atomic() {
        let pool = pool().await;
        let id_a = ws_id("ws-tx-a");
        let id_b = ws_id("ws-tx-b");

        // Both creates go through a single transaction.
        let mut tx = pool.begin().await.unwrap();
        WorkspaceRepo::create(&mut *tx, &workspace("ws-tx-a", "TxA"))
            .await
            .unwrap();
        WorkspaceRepo::create(&mut *tx, &workspace("ws-tx-b", "TxB"))
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
        WorkspaceRepo::create(&mut *tx, &workspace("ws-rollback-1", "Ephemeral"))
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        let got = WorkspaceRepo::get(&pool, &id).await.unwrap();
        assert!(got.is_none(), "rolled-back workspace must not be visible");
    }
}
