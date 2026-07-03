use serde::Deserialize;

use crate::app::workspace::WorkspaceActivityView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::Result;

/// Per-workspace activity rollup (ADR-0044): running / failed surface counts for
/// every workspace in one aggregate, derived from the persisted surface status.
/// Workspaces with no surfaces report zero counts.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaceActivity {}

impl Query<Ctx> for ListWorkspaceActivity {
    type Out = Vec<WorkspaceActivityView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let items = sqlx::query_as::<_, WorkspaceActivityView>(
            "SELECT w.id AS workspace_id,
                    COUNT(CASE WHEN sf.status = 'live' THEN 1 END) AS running,
                    COUNT(CASE WHEN sf.status = 'failed' THEN 1 END) AS failed
             FROM workspace w
             LEFT JOIN project p ON p.workspace_id = w.id
             LEFT JOIN session s ON s.project_id = p.id
             LEFT JOIN surface sf ON sf.session_id = s.id
             GROUP BY w.id
             ORDER BY w.id",
        )
        .fetch_all(cx.db())
        .await?;
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::test_util::*;
    use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
    use crate::entities::surface::{SurfaceKind, SurfaceStatus};
    use crate::infra::session::SessionRepo;
    use crate::infra::SurfaceRepo;

    async fn insert_session(cx: &crate::context::Ctx, workspace: &str, project: &str, id: &str) {
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind(project)
            .bind(workspace)
            .bind("P")
            .execute(cx.db())
            .await
            .unwrap();
        let session = Session {
            id: SessionId::from_string(id),
            project_id: crate::entities::project::ProjectId::new(project),
            title: "S".to_owned(),
            title_source: TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order: 0,
            pinned: false,
            status: SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &session).await.unwrap();
    }

    async fn insert_surface_with_status(
        cx: &crate::context::Ctx,
        session: &str,
        status: SurfaceStatus,
    ) {
        let surface = SurfaceRepo::create(
            cx.db(),
            None,
            &SessionId::from_string(session),
            SurfaceKind::Terminal,
            None,
            None,
            SurfaceStatus::Pending,
        )
        .await
        .unwrap();
        SurfaceRepo::update_status(cx.db(), &surface.id, status)
            .await
            .unwrap();
    }

    fn counts_for<'a>(
        rows: &'a [WorkspaceActivityView],
        workspace: &str,
    ) -> &'a WorkspaceActivityView {
        rows.iter()
            .find(|r| r.workspace_id == workspace)
            .expect("workspace present in rollup")
    }

    // Scenario: Rollup reflects runtime surface state (running/failed/idle mix).
    #[tokio::test]
    async fn rollup_counts_running_and_failed_per_workspace() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-act-a", "A").await;
        insert_workspace(&cx, "ws-act-b", "B").await;
        insert_session(&cx, "ws-act-a", "p-act-a", "s-act-a").await;
        insert_session(&cx, "ws-act-b", "p-act-b", "s-act-b").await;

        insert_surface_with_status(&cx, "s-act-a", SurfaceStatus::Live).await;
        insert_surface_with_status(&cx, "s-act-a", SurfaceStatus::Live).await;
        insert_surface_with_status(&cx, "s-act-a", SurfaceStatus::Failed).await;
        insert_surface_with_status(&cx, "s-act-a", SurfaceStatus::Idle).await;
        insert_surface_with_status(&cx, "s-act-b", SurfaceStatus::Failed).await;

        let rows = ListWorkspaceActivity {}.handle(&cx).await.unwrap();

        let a = counts_for(&rows, "ws-act-a");
        assert_eq!((a.running, a.failed), (2, 1), "idle must not count");
        let b = counts_for(&rows, "ws-act-b");
        assert_eq!((b.running, b.failed), (0, 1));
    }

    // Scenario: A workspace with no surfaces reports zero counts (not absence).
    #[tokio::test]
    async fn empty_workspace_reports_zero_counts() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-act-empty", "Empty").await;

        let rows = ListWorkspaceActivity {}.handle(&cx).await.unwrap();

        let empty = counts_for(&rows, "ws-act-empty");
        assert_eq!((empty.running, empty.failed), (0, 0));
    }

    // Scenario: One round trip returns every workspace, including the seeded Default.
    #[tokio::test]
    async fn rollup_includes_every_workspace_in_one_result() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-act-one", "One").await;

        let rows = ListWorkspaceActivity {}.handle(&cx).await.unwrap();

        let ids: Vec<&str> = rows.iter().map(|r| r.workspace_id.as_str()).collect();
        assert!(ids.contains(&"ws-act-one"));
        assert!(
            ids.contains(&crate::entities::WorkspaceId::DEFAULT),
            "seeded Default workspace must appear"
        );
    }
}
