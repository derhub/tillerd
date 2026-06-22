use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::{WorkspaceId, WorkspaceStatus};
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Archive a workspace. Rejected if Default or if any session under it is live.
/// Cascades: archives all sessions across all projects in the workspace.
/// Uses a transaction because it spans session-level updates.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveWorkspace {
    pub id: String,
}

impl Command<Ctx> for ArchiveWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.guard_not_default()?;
        ws.guard_active()?;

        // Check that every session under this workspace is idle.
        // A single query counts live surfaces across all projects/sessions in scope.
        let live_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM surface
             WHERE status = 'live'
               AND session_id IN (
                   SELECT s.id FROM session s
                   JOIN project p ON p.id = s.project_id
                   WHERE p.workspace_id = ?
               )",
        )
        .bind(&self.id)
        .fetch_one(cx.db())
        .await?;

        if live_count > 0 {
            // Find the first non-idle session to surface in the error.
            let session_id: String = sqlx::query_scalar(
                "SELECT s.id FROM session s
                 JOIN project p ON p.id = s.project_id
                 WHERE p.workspace_id = ?
                   AND EXISTS (
                       SELECT 1 FROM surface
                       WHERE session_id = s.id AND status = 'live'
                   )
                 LIMIT 1",
            )
            .bind(&self.id)
            .fetch_one(cx.db())
            .await?;
            return Err(Error::SessionNotIdle(format!(
                "session {session_id} has {live_count} live surface(s)"
            )));
        }

        ws.status = WorkspaceStatus::Archived;
        // Archive all sessions in the workspace in the same transaction.
        cx.transaction(async |tx| {
            // Archive all sessions under this workspace's projects.
            sqlx::query(
                "UPDATE session SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE archived_at IS NULL
                   AND project_id IN (
                       SELECT id FROM project WHERE workspace_id = ?
                   )",
            )
            .bind(&self.id)
            .execute(&mut **tx)
            .await?;
            WorkspaceRepo::update(&mut **tx, &ws).await
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;
    use crate::infra::WorkspaceRepo;

    // Scenario: Archive is rejected unless all in-scope sessions are idle.
    #[tokio::test]
    async fn archive_workspace_is_rejected_when_a_session_has_live_surfaces() {
        use crate::entities::session::SessionId;
        use crate::entities::surface::{SurfaceKind, SurfaceStatus};
        use crate::infra::session::SessionRepo;
        use crate::infra::SurfaceRepo;

        let cx = ctx().await;
        insert_workspace(&cx, "ws-arch-busy", "Busy").await;

        // Project under this workspace.
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("proj-arch-busy")
            .bind("ws-arch-busy")
            .bind("Proj")
            .execute(cx.db())
            .await
            .unwrap();

        // Session under that project.
        let sess = crate::entities::session::Session {
            id: SessionId::from_string("sess-arch-busy"),
            project_id: crate::entities::project::ProjectId::new("proj-arch-busy"),
            title: "Live session".to_owned(),
            title_source: crate::entities::session::TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order: 0,
            pinned: false,
            status: crate::entities::session::SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &sess).await.unwrap();

        // Create a live surface in that session.
        let surface = SurfaceRepo::create(
            cx.db(),
            None,
            &SessionId::from_string("sess-arch-busy"),
            SurfaceKind::Terminal,
            None,
            None,
            crate::entities::surface::SurfaceStatus::Pending,
        )
        .await
        .unwrap();
        SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Live)
            .await
            .unwrap();

        let err = ArchiveWorkspace {
            id: "ws-arch-busy".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "session.not_idle");
    }

    #[tokio::test]
    async fn archive_workspace_succeeds_when_all_sessions_are_idle() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-arch-idle", "Idle").await;

        ArchiveWorkspace {
            id: "ws-arch-idle".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-arch-idle"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.status, WorkspaceStatus::Archived);
    }

    // Scenario: Archiving the Default workspace is rejected.
    #[tokio::test]
    async fn archive_default_workspace_is_rejected() {
        let cx = ctx().await;
        let err = ArchiveWorkspace {
            id: WorkspaceId::DEFAULT.to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "workspace.is_default");
    }

    // Scenario: Archive is rejected on already-archived workspace.
    #[tokio::test]
    async fn archive_already_archived_workspace_is_rejected() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-arch-twice", "Double-archive").await;
        ArchiveWorkspace {
            id: "ws-arch-twice".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        let err = ArchiveWorkspace {
            id: "ws-arch-twice".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "workspace.already_archived");
    }
}
