//! Per-entity async sqlx repository for the `project` table.
//!
//! Methods take a generic `SqliteExecutor` so the same call serves both a
//! direct pool call and a shared transaction (see design D2).

use sqlx::sqlite::SqliteExecutor;

use crate::entities::project::{Project, ProjectId};
use crate::entities::workspace::WorkspaceId;
use crate::shared::{Error, Result};

// -- Repository (unit struct of executor-passing functions) --------------------

pub struct ProjectRepo;

impl ProjectRepo {
    /// Persist a fully-formed project entity. Caller builds the entity and
    /// holds it; repo binds the fields and returns `()`.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, project: &Project) -> Result<()> {
        let sk = project.source_kind.as_str();
        let so = project.sort_order as i64;
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, source_kind, root_path, sort_order)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(project.id.as_str())
        .bind(project.workspace_id.as_str())
        .bind(&project.name)
        .bind(sk)
        .bind(project.root_path.as_deref())
        .bind(so)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Fetch a single project by id. Returns `None` when absent.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &ProjectId) -> Result<Option<Project>> {
        Ok(sqlx::query_as::<_, Project>(
            "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned,
                    CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
             FROM project
             WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?)
    }

    /// Persist mutations to an existing project row (workspace_id, name, source_kind,
    /// root_path, sort_order, pinned, archived_at).
    ///
    /// Archiving sets `archived_at` to the current UTC timestamp (idempotent:
    /// COALESCE preserves an existing timestamp on re-archive). Restoring clears it.
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, project: &Project) -> Result<()> {
        let sk = project.source_kind.as_str();
        let so = project.sort_order as i64;
        let pinned = project.pinned as i64;
        let status = project.status.as_str();
        let affected = sqlx::query(
            "UPDATE project
             SET workspace_id = ?,
                 name         = ?,
                 source_kind  = ?,
                 root_path    = ?,
                 sort_order   = ?,
                 pinned       = ?,
                 archived_at  = CASE
                     WHEN ? = 'archived'
                         THEN COALESCE(archived_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                     ELSE NULL
                 END
             WHERE id = ?",
        )
        .bind(project.workspace_id.as_str())
        .bind(&project.name)
        .bind(sk)
        .bind(project.root_path.as_deref())
        .bind(so)
        .bind(pinned)
        .bind(status)
        .bind(project.id.as_str())
        .execute(exec)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(Error::ProjectNotFound(project.id.as_str().to_owned()));
        }
        Ok(())
    }

    /// Hard-delete a project row.
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &ProjectId) -> Result<()> {
        sqlx::query("DELETE FROM project WHERE id = ?")
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Reassign all projects in `from_workspace` to `to_workspace`.
    ///
    /// Used by `DiscardWorkspace` to move projects to Default before deleting
    /// the workspace; the multi-repo call is atomic when the caller passes a
    /// shared transaction executor.
    pub async fn reassign_workspace<'e>(
        exec: impl SqliteExecutor<'e>,
        from_workspace: &WorkspaceId,
        to_workspace: &WorkspaceId,
    ) -> Result<()> {
        sqlx::query("UPDATE project SET workspace_id = ? WHERE workspace_id = ?")
            .bind(to_workspace.as_str())
            .bind(from_workspace.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Count live surfaces across all sessions belonging to `project_id`.
    ///
    /// Used by `ArchiveProject` to enforce the archive-requires-idle rule without
    /// loading every session and surface row into memory.
    pub async fn count_live_surfaces<'e>(
        exec: impl SqliteExecutor<'e>,
        project_id: &ProjectId,
    ) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM surface s
             JOIN session se ON se.id = s.session_id
             WHERE se.project_id = ?
               AND s.status = 'live'",
        )
        .bind(project_id.as_str())
        .fetch_one(exec)
        .await?;
        Ok(count)
    }
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::project::{ProjectStatus, SourceKind};
    use crate::infra::migrate;

    const DEFAULT_WS: &str = "00000000-0000-0000-0000-000000000001";

    async fn pool() -> sqlx::SqlitePool {
        migrate::open_memory().await.expect("in-memory pool")
    }

    fn ws() -> WorkspaceId {
        WorkspaceId::default_id()
    }

    fn other_ws_id() -> &'static str {
        "ws-other-0000-0000-0000-000000000002"
    }

    async fn seed_workspace(pool: &sqlx::SqlitePool, ws_id: &str) {
        sqlx::query("INSERT INTO workspace (id, name) VALUES (?, ?)")
            .bind(ws_id)
            .bind("Other")
            .execute(pool)
            .await
            .unwrap();
    }

    fn make_project(id: &str, ws: WorkspaceId, name: &str) -> Project {
        Project::new(ProjectId::new(id), ws, name, SourceKind::Blank, None, 0)
    }

    fn make_project_ordered(id: &str, ws: WorkspaceId, name: &str, sort_order: u32) -> Project {
        Project::new(
            ProjectId::new(id),
            ws,
            name,
            SourceKind::Blank,
            None,
            sort_order,
        )
    }

    // -- Scenario: name normalization happens before persist --------------------

    #[tokio::test]
    async fn create_trims_name_and_stored_equals_returned() {
        let pool = pool().await;
        let project = make_project("p-trim", ws(), "  padded name  ");
        ProjectRepo::create(&pool, &project)
            .await
            .expect("create must succeed");

        let fetched = ProjectRepo::get(&pool, &project.id)
            .await
            .expect("get must succeed")
            .expect("project must be present");

        assert_eq!(
            fetched.name, "padded name",
            "stored value must equal returned value"
        );
    }

    // -- Scenario: A repository persists and reads a typed entity -------------

    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = pool().await;
        let project = make_project("proj-rt-01", ws(), "My Project");
        ProjectRepo::create(&pool, &project)
            .await
            .expect("create must succeed");

        let fetched = ProjectRepo::get(&pool, &project.id)
            .await
            .expect("get must succeed")
            .expect("project must be present");

        assert_eq!(fetched.id, project.id);
        assert_eq!(fetched.name, "My Project");
        assert_eq!(fetched.workspace_id.as_str(), DEFAULT_WS);
        assert_eq!(fetched.source_kind, SourceKind::Blank);
        assert!(fetched.root_path.is_none());
        assert_eq!(fetched.sort_order, 0);
        assert!(!fetched.pinned);
        assert_eq!(fetched.status, ProjectStatus::Active);
    }

    #[tokio::test]
    async fn get_absent_project_returns_none() {
        let pool = pool().await;
        let result = ProjectRepo::get(&pool, &ProjectId::new("no-such-id"))
            .await
            .expect("get must not error");
        assert!(result.is_none());
    }

    // -- Scenario: A rename is a plain update ----------------------------------

    #[tokio::test]
    async fn update_persists_name_change() {
        let pool = pool().await;
        let mut project = make_project("p-update", ws(), "Old Name");
        ProjectRepo::create(&pool, &project).await.unwrap();

        project.rename("New Name");
        ProjectRepo::update(&pool, &project).await.unwrap();

        let fetched = ProjectRepo::get(&pool, &project.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "New Name");
    }

    #[tokio::test]
    async fn update_on_missing_project_returns_error() {
        let pool = pool().await;
        let phantom = Project {
            id: ProjectId::new("no-such-project"),
            workspace_id: ws(),
            name: "Ghost".to_owned(),
            source_kind: SourceKind::Blank,
            root_path: None,
            sort_order: 0,
            pinned: false,
            status: ProjectStatus::Active,
        };
        let err = ProjectRepo::update(&pool, &phantom)
            .await
            .expect_err("must error on missing project");
        assert!(
            matches!(err, Error::ProjectNotFound(_)),
            "unexpected error: {err}"
        );
    }

    // -- Delete ----------------------------------------------------------------

    #[tokio::test]
    async fn delete_removes_project() {
        let pool = pool().await;
        let project = make_project("p-delete", ws(), "Doomed");
        ProjectRepo::create(&pool, &project).await.unwrap();

        ProjectRepo::delete(&pool, &project.id).await.unwrap();

        let fetched = ProjectRepo::get(&pool, &project.id).await.unwrap();
        assert!(fetched.is_none(), "deleted project must not be found");
    }

    // -- Scenario: multi-repo call on one tx is atomic -------------------------

    #[tokio::test]
    async fn reassign_and_delete_workspace_are_atomic_on_shared_tx() {
        let pool = pool().await;
        seed_workspace(&pool, other_ws_id()).await;

        // Create projects in the "other" workspace.
        ProjectRepo::create(
            &pool,
            &make_project("p-tx-1", WorkspaceId::new(other_ws_id()), "TX Project 1"),
        )
        .await
        .unwrap();
        ProjectRepo::create(
            &pool,
            &make_project_ordered("p-tx-2", WorkspaceId::new(other_ws_id()), "TX Project 2", 1),
        )
        .await
        .unwrap();

        // Use a transaction: reassign + delete workspace must both succeed or
        // both roll back (atomicity guarantee).
        let mut tx = pool.begin().await.unwrap();
        ProjectRepo::reassign_workspace(&mut *tx, &WorkspaceId::new(other_ws_id()), &ws())
            .await
            .unwrap();
        sqlx::query("DELETE FROM workspace WHERE id = ?")
            .bind(other_ws_id())
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // After the transaction both projects belong to the Default workspace.
        let p1 = ProjectRepo::get(&pool, &ProjectId::new("p-tx-1"))
            .await
            .unwrap()
            .unwrap();
        let p2 = ProjectRepo::get(&pool, &ProjectId::new("p-tx-2"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            p1.workspace_id.as_str(),
            DEFAULT_WS,
            "p1 must be reassigned"
        );
        assert_eq!(
            p2.workspace_id.as_str(),
            DEFAULT_WS,
            "p2 must be reassigned"
        );

        // The other workspace is gone.
        let ws_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workspace WHERE id = ?")
            .bind(other_ws_id())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(ws_count, 0, "workspace must be deleted");
    }

    #[tokio::test]
    async fn rollback_on_mid_tx_error_leaves_state_unchanged() {
        let pool = pool().await;
        seed_workspace(&pool, other_ws_id()).await;

        ProjectRepo::create(
            &pool,
            &make_project(
                "p-rollback",
                WorkspaceId::new(other_ws_id()),
                "Rollback Project",
            ),
        )
        .await
        .unwrap();

        // Begin a transaction, reassign, then deliberately violate FK to trigger a
        // rollback scenario -- we simulate an error by manually rolling back.
        let mut tx = pool.begin().await.unwrap();
        ProjectRepo::reassign_workspace(&mut *tx, &WorkspaceId::new(other_ws_id()), &ws())
            .await
            .unwrap();
        // Roll back instead of committing.
        tx.rollback().await.unwrap();

        // Project must still be in the other workspace.
        let project = ProjectRepo::get(&pool, &ProjectId::new("p-rollback"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            project.workspace_id.as_str(),
            other_ws_id(),
            "rollback must leave workspace_id unchanged"
        );
    }
}
