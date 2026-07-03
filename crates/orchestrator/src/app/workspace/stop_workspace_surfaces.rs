use serde::Deserialize;

use crate::context::Ctx;
use crate::shared::message::Command;
use crate::shared::Result;

/// Stop every live surface under the workspace so it becomes idle (precondition
/// for archive). Stops via the runtime port; no DB transaction is held across
/// the runtime effect (D9).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopWorkspaceSurfaces {
    pub id: String,
}

impl Command<Ctx> for StopWorkspaceSurfaces {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::entities::surface::{SurfaceId, SurfaceStatus};

        // Collect live surface ids across all sessions in this workspace.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT sf.id FROM surface sf
             JOIN session s ON s.id = sf.session_id
             JOIN project p ON p.id = s.project_id
             WHERE p.workspace_id = ? AND sf.status = 'live'",
        )
        .bind(&self.id)
        .fetch_all(cx.db())
        .await?;

        for (raw_id,) in rows {
            let surface_id = SurfaceId::from_string(raw_id);
            // 1) runtime stop (outside any tx, per D9)
            cx.runtime().stop(&surface_id).await?;
            // 2) record outcome: transition to idle (emits the surface-status push)
            crate::app::surface::update_status_and_emit(cx, &surface_id, SurfaceStatus::Idle)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::app::workspace::test_util::*;
    use crate::context::Ctx;
    use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
    use crate::infra::migrate;
    use crate::shared::kv::SqliteKv;

    // Scenario: Stopping a scope makes it idle.
    #[tokio::test]
    async fn stop_workspace_surfaces_stops_all_live_surfaces() {
        use crate::entities::session::SessionId;
        use crate::entities::surface::{SurfaceKind, SurfaceStatus};
        use crate::infra::session::SessionRepo;
        use crate::infra::SurfaceRepo;

        let rt = Arc::new(FakeRuntime::new());
        let pool = migrate::open_memory().await.unwrap();
        let kv = SqliteKv::in_memory().await.unwrap();
        let cx = Ctx::new(
            pool,
            kv,
            PathBuf::from("/tmp/test"),
            Runtime::Fake(rt.clone()),
        );

        insert_workspace(&cx, "ws-stop-1", "Stopme").await;
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("proj-stop-1")
            .bind("ws-stop-1")
            .bind("P")
            .execute(cx.db())
            .await
            .unwrap();

        let sess = crate::entities::session::Session {
            id: SessionId::from_string("sess-stop-1"),
            project_id: crate::entities::project::ProjectId::new("proj-stop-1"),
            title: "S".to_owned(),
            title_source: crate::entities::session::TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order: 0,
            pinned: false,
            status: crate::entities::session::SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &sess).await.unwrap();

        // Create two live surfaces.
        for _ in 0..2 {
            let surface = SurfaceRepo::create(
                cx.db(),
                None,
                &SessionId::from_string("sess-stop-1"),
                SurfaceKind::Terminal,
                None,
                None,
                SurfaceStatus::Pending,
            )
            .await
            .unwrap();
            SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Live)
                .await
                .unwrap();
            rt.seed_running(surface.id.clone());
        }

        StopWorkspaceSurfaces {
            id: "ws-stop-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        // All surfaces must now be idle.
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, status FROM surface WHERE session_id = 'sess-stop-1'")
                .fetch_all(cx.db())
                .await
                .unwrap();
        for (_, status) in &rows {
            assert_eq!(status, "idle", "surface must be idle after stop");
        }
        assert_eq!(rows.len(), 2, "both surfaces must be present");
    }
}
