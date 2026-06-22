//! Per-entity async sqlx repository for the `project` table.
//!
//! Methods take a generic `SqliteExecutor` so the same call serves both a
//! direct pool call and a shared transaction (see design D2).

use sqlx::sqlite::SqliteExecutor;
use sqlx::FromRow;

use crate::entities::project::{Project, ProjectId, ProjectStatus, SourceKind};
use crate::entities::workspace::WorkspaceId;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Error, Result};

// ── Row type (owned by this module) ──────────────────────────────────────────

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    workspace_id: String,
    name: String,
    source_kind: String,
    root_path: Option<String>,
    sort_order: i64,
    pinned: i64,
    archived_at: Option<String>,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        let source_kind = match r.source_kind.as_str() {
            "local_dir" => SourceKind::LocalDir,
            "git_repo" => SourceKind::GitRepo,
            _ => SourceKind::Blank,
        };
        let status = if r.archived_at.is_some() {
            ProjectStatus::Archived
        } else {
            ProjectStatus::Active
        };
        Project {
            id: ProjectId::new(r.id),
            workspace_id: WorkspaceId::new(r.workspace_id),
            name: r.name,
            source_kind,
            root_path: r.root_path,
            sort_order: r.sort_order as u32,
            pinned: r.pinned != 0,
            status,
        }
    }
}

// ── Repository (unit struct of executor-passing functions) ────────────────────

pub struct ProjectRepo;

impl ProjectRepo {
    /// Insert a new project row and return the entity.
    ///
    /// The caller provides a fresh UUID `id` so the repo stays pure (no hidden
    /// side-effects, easy to test with a deterministic id).
    pub async fn create<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &str,
        workspace_id: &WorkspaceId,
        name: &str,
        source_kind: SourceKind,
        root_path: Option<&str>,
        sort_order: u32,
    ) -> Result<Project> {
        let sk = source_kind.as_str();
        let so = sort_order as i64;
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, source_kind, root_path, sort_order)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(workspace_id.as_str())
        .bind(name)
        .bind(sk)
        .bind(root_path)
        .bind(so)
        .execute(exec)
        .await?;

        Ok(Project {
            id: ProjectId::new(id),
            workspace_id: workspace_id.clone(),
            name: name.trim().to_owned(),
            source_kind,
            root_path: root_path.map(ToOwned::to_owned),
            sort_order,
            pinned: false,
            status: ProjectStatus::Active,
        })
    }

    /// Fetch a single project by id. Returns `None` when absent.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &ProjectId) -> Result<Option<Project>> {
        let row: Option<ProjectRow> = sqlx::query_as(
            "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
             FROM project
             WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?;
        Ok(row.map(Into::into))
    }

    /// List projects under a workspace, pinned-first then by `sort_order`.
    ///
    /// Supports `Page::All`, `Page::Offset`, and `Page::Cursor`. Cursor is the
    /// `sort_order` value (as a decimal string) of the last item on the previous
    /// page; items are returned with `sort_order >= cursor_value`, unpinned-first
    /// boundary handled by combining `(pinned DESC, sort_order)` ordering with a
    /// row-number cursor embedded in the cursor string.
    ///
    /// The cursor format is `"{pinned}:{sort_order}:{id}"` — stable across
    /// inserts as long as the sort columns don't change.
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        workspace_id: &WorkspaceId,
        page: &Page,
    ) -> Result<Listing<Project>> {
        match page {
            Page::All => {
                let rows: Vec<ProjectRow> = sqlx::query_as(
                    "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
                     FROM project
                     WHERE workspace_id = ?
                     ORDER BY pinned DESC, sort_order, id",
                )
                .bind(workspace_id.as_str())
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    None,
                ))
            }

            Page::Offset { limit, offset } => {
                // Fetch one extra to detect whether a next page exists.
                let fetch = (*limit as i64) + 1;
                let rows: Vec<ProjectRow> = sqlx::query_as(
                    "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
                     FROM project
                     WHERE workspace_id = ?
                     ORDER BY pinned DESC, sort_order, id
                     LIMIT ? OFFSET ?",
                )
                .bind(workspace_id.as_str())
                .bind(fetch)
                .bind(*offset as i64)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<Project> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    let new_offset = offset + limit;
                    Some(format!("offset:{new_offset}"))
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                // Cursor format: "{pinned}:{sort_order}:{id}"
                // Rows strictly after the cursor position in (pinned DESC, sort_order, id) order.
                let fetch = (*limit as i64) + 1;
                let rows: Vec<ProjectRow> = if let Some(cursor) = after {
                    let (c_pinned, c_sort, c_id) = parse_cursor(cursor)?;
                    sqlx::query_as(
                        "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
                         FROM project
                         WHERE workspace_id = ?
                           AND (
                             pinned < ?
                             OR (pinned = ? AND sort_order > ?)
                             OR (pinned = ? AND sort_order = ? AND id > ?)
                           )
                         ORDER BY pinned DESC, sort_order, id
                         LIMIT ?",
                    )
                    .bind(workspace_id.as_str())
                    .bind(c_pinned)
                    .bind(c_pinned)
                    .bind(c_sort)
                    .bind(c_pinned)
                    .bind(c_sort)
                    .bind(&c_id)
                    .bind(fetch)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(
                        "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
                         FROM project
                         WHERE workspace_id = ?
                         ORDER BY pinned DESC, sort_order, id
                         LIMIT ?",
                    )
                    .bind(workspace_id.as_str())
                    .bind(fetch)
                    .fetch_all(exec)
                    .await?
                };

                let has_more = rows.len() > *limit as usize;
                let items: Vec<Project> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();

                let next = if has_more {
                    items
                        .last()
                        .map(|p| make_cursor(p.pinned, p.sort_order, p.id.as_str()))
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
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

    /// Fuzzy-search projects by name within a workspace (sqlite-side LIKE filter,
    /// match-ordered by position of the term in the name).
    ///
    /// Returns up to `limit` projects whose name contains `query` (case-insensitive).
    /// The result is ordered: exact match first, prefix match, then substring match,
    /// all secondary-sorted by `sort_order`.
    pub async fn search<'e>(
        exec: impl SqliteExecutor<'e>,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Project>> {
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT id, workspace_id, name, source_kind, root_path, sort_order, pinned, archived_at
             FROM project
             WHERE workspace_id = ?
               AND name LIKE ? ESCAPE '\\'
             ORDER BY
               CASE WHEN lower(name) = lower(?) THEN 0
                    WHEN lower(name) LIKE lower(?) || '%' THEN 1
                    ELSE 2
               END,
               sort_order,
               id
             LIMIT ?",
        )
        .bind(workspace_id.as_str())
        .bind(&pattern)
        .bind(query)
        .bind(format!("{}%", query))
        .bind(limit as i64)
        .fetch_all(exec)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
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

// ── Cursor helpers ────────────────────────────────────────────────────────────

fn make_cursor(pinned: bool, sort_order: u32, id: &str) -> String {
    format!("{}:{}:{}", pinned as i64, sort_order, id)
}

fn parse_cursor(cursor: &str) -> Result<(i64, i64, String)> {
    let mut parts = cursor.splitn(3, ':');
    let pinned: i64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid_cursor(cursor))?;
    let sort_order: i64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid_cursor(cursor))?;
    let id = parts
        .next()
        .ok_or_else(|| invalid_cursor(cursor))?
        .to_owned();
    Ok((pinned, sort_order, id))
}

fn invalid_cursor(cursor: &str) -> Error {
    Error::Validation {
        field: "cursor",
        reason: format!("invalid cursor: {cursor}"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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

    // ── Scenario: A repository persists and reads a typed entity ─────────────

    #[tokio::test]
    async fn round_trip_create_and_get() {
        let pool = pool().await;
        let created = ProjectRepo::create(
            &pool,
            "proj-rt-01",
            &ws(),
            "My Project",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .expect("create must succeed");

        let fetched = ProjectRepo::get(&pool, &created.id)
            .await
            .expect("get must succeed")
            .expect("project must be present");

        assert_eq!(fetched.id, created.id);
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

    // ── Scenario: Children are found by parent id ─────────────────────────────

    #[tokio::test]
    async fn list_filters_by_workspace() {
        let pool = pool().await;
        seed_workspace(&pool, other_ws_id()).await;

        ProjectRepo::create(
            &pool,
            "p-ws1",
            &ws(),
            "Proj WS1",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();
        ProjectRepo::create(
            &pool,
            "p-ws2",
            &WorkspaceId::new(other_ws_id()),
            "Proj WS2",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();

        let listing = ProjectRepo::list(&pool, &ws(), &Page::All)
            .await
            .expect("list must succeed");

        // The Unfiled seed project is also under Default workspace; verify our
        // new project is there and the ws2 project is not.
        let ids: Vec<&str> = listing.items.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"p-ws1"), "own project must appear");
        assert!(
            !ids.contains(&"p-ws2"),
            "other-workspace project must not appear"
        );
    }

    // ── Scenario: A pinned item sorts ahead of unpinned ───────────────────────

    #[tokio::test]
    async fn list_returns_pinned_before_unpinned() {
        let pool = pool().await;

        // sort_order: pinned=20, unpinned=10 — pinned still comes first.
        ProjectRepo::create(
            &pool,
            "p-unpinned",
            &ws(),
            "Unpinned",
            SourceKind::Blank,
            None,
            10,
        )
        .await
        .unwrap();
        let mut pinned = ProjectRepo::create(
            &pool,
            "p-pinned",
            &ws(),
            "Pinned",
            SourceKind::Blank,
            None,
            20,
        )
        .await
        .unwrap();
        pinned.pinned = true;
        ProjectRepo::update(&pool, &pinned).await.unwrap();

        let listing = ProjectRepo::list(&pool, &ws(), &Page::All).await.unwrap();

        // Filter out the seeded Unfiled project.
        let names: Vec<&str> = listing
            .items
            .iter()
            .filter(|p| p.id.as_str() != "00000000-0000-0000-0000-000000000000")
            .map(|p| p.name.as_str())
            .collect();

        assert_eq!(names, vec!["Pinned", "Unpinned"]);
    }

    // ── Scenario: A bounded page returns a continuation cursor ────────────────

    #[tokio::test]
    async fn offset_pagination_returns_cursor_when_more_remain() {
        let pool = pool().await;

        for i in 0u32..4 {
            ProjectRepo::create(
                &pool,
                &format!("p-page-{i:02}"),
                &ws(),
                &format!("Page {i}"),
                SourceKind::Blank,
                None,
                i,
            )
            .await
            .unwrap();
        }

        let page1 = ProjectRepo::list(
            &pool,
            &ws(),
            &Page::Offset {
                limit: 2,
                offset: 0,
            },
        )
        .await
        .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next.is_some(), "must carry a continuation cursor");
    }

    #[tokio::test]
    async fn offset_last_page_has_no_next_cursor() {
        let pool = pool().await;

        for i in 0u32..2 {
            ProjectRepo::create(
                &pool,
                &format!("p-last-{i:02}"),
                &ws(),
                &format!("Last {i}"),
                SourceKind::Blank,
                None,
                i,
            )
            .await
            .unwrap();
        }

        // 3 total rows (2 new + 1 Unfiled seed) with limit=10 -> fits on one page.
        let page = ProjectRepo::list(
            &pool,
            &ws(),
            &Page::Offset {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
        assert!(page.next.is_none(), "no next cursor on last page");
    }

    #[tokio::test]
    async fn cursor_pagination_pages_through_all_items() {
        let pool = pool().await;

        for i in 0u32..3 {
            ProjectRepo::create(
                &pool,
                &format!("p-cursor-{i:02}"),
                &ws(),
                &format!("Cursor {i}"),
                SourceKind::Blank,
                None,
                i + 100, // avoid collision with seed Unfiled sort_order=0
            )
            .await
            .unwrap();
        }

        let p1 = ProjectRepo::list(
            &pool,
            &ws(),
            &Page::Cursor {
                after: None,
                limit: 2,
            },
        )
        .await
        .unwrap();
        assert_eq!(p1.items.len(), 2);
        let cursor = p1.next.clone().expect("must have next cursor");

        let p2 = ProjectRepo::list(
            &pool,
            &ws(),
            &Page::Cursor {
                after: Some(cursor),
                limit: 2,
            },
        )
        .await
        .unwrap();
        // 4 total rows (3 new + 1 Unfiled seed); first page took 2, second page has 2.
        assert_eq!(p2.items.len(), 2);

        // No overlap.
        let p1_ids: Vec<&str> = p1.items.iter().map(|p| p.id.as_str()).collect();
        let p2_ids: Vec<&str> = p2.items.iter().map(|p| p.id.as_str()).collect();
        for id in &p2_ids {
            assert!(!p1_ids.contains(id), "pages must not overlap");
        }
    }

    // ── Scenario: A rename is a plain update ──────────────────────────────────

    #[tokio::test]
    async fn update_persists_name_change() {
        let pool = pool().await;
        let mut project = ProjectRepo::create(
            &pool,
            "p-update",
            &ws(),
            "Old Name",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();

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

    // ── Delete ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_removes_project_from_list() {
        let pool = pool().await;
        let project = ProjectRepo::create(
            &pool,
            "p-delete",
            &ws(),
            "Doomed",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();

        ProjectRepo::delete(&pool, &project.id).await.unwrap();

        let fetched = ProjectRepo::get(&pool, &project.id).await.unwrap();
        assert!(fetched.is_none(), "deleted project must not be found");

        let listing = ProjectRepo::list(&pool, &ws(), &Page::All).await.unwrap();
        assert!(
            !listing.items.iter().any(|p| p.id == project.id),
            "deleted project must not appear in list"
        );
    }

    // ── Scenario: multi-repo call on one tx is atomic ─────────────────────────

    #[tokio::test]
    async fn reassign_and_delete_workspace_are_atomic_on_shared_tx() {
        let pool = pool().await;
        seed_workspace(&pool, other_ws_id()).await;

        // Create projects in the "other" workspace.
        ProjectRepo::create(
            &pool,
            "p-tx-1",
            &WorkspaceId::new(other_ws_id()),
            "TX Project 1",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();
        ProjectRepo::create(
            &pool,
            "p-tx-2",
            &WorkspaceId::new(other_ws_id()),
            "TX Project 2",
            SourceKind::Blank,
            None,
            1,
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
        let listing = ProjectRepo::list(&pool, &ws(), &Page::All).await.unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"p-tx-1"), "project 1 must be reassigned");
        assert!(ids.contains(&"p-tx-2"), "project 2 must be reassigned");

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
            "p-rollback",
            &WorkspaceId::new(other_ws_id()),
            "Rollback Project",
            SourceKind::Blank,
            None,
            0,
        )
        .await
        .unwrap();

        // Begin a transaction, reassign, then deliberately violate FK to trigger a
        // rollback scenario — we simulate an error by manually rolling back.
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
