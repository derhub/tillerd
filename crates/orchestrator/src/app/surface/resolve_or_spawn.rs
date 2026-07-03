use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::{SurfaceId, SurfaceKind, SurfaceStatus};
use crate::infra::daemon_pty_api::{Geometry, SpawnRequest};
use crate::infra::SurfaceRepo;
use crate::shared::errors::Result;
use crate::shared::message::Query;

use super::common::{default_cwd, DEFAULT_GEOMETRY};
use super::SurfaceView;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ResolveOrSpawnSurface {
    pub session: String,
    pub placement: String,
    pub cwd: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

impl Query<Ctx> for ResolveOrSpawnSurface {
    type Out = SurfaceView;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let existing = sqlx::query_as::<_, SurfaceView>(
            "SELECT id, session_id, kind, cwd, status, placement
             FROM surface WHERE session_id = ? AND placement = ?",
        )
        .bind(&self.session)
        .bind(&self.placement)
        .fetch_optional(cx.db())
        .await?;

        if let Some(view) = existing {
            if view.status == "live" {
                return Ok(view);
            }

            let surface_id = SurfaceId::from_string(&view.id);
            let geometry = match (self.cols, self.rows) {
                (Some(cols), Some(rows)) => Geometry { cols, rows },
                _ => DEFAULT_GEOMETRY,
            };
            let request = SpawnRequest {
                surface: surface_id.clone(),
                command: None,
                token: uuid::Uuid::new_v4().to_string(),
                geometry,
                cwd: view.cwd.clone().unwrap_or_else(default_cwd),
            };

            match cx.runtime().spawn(request).await {
                Ok(()) => {
                    // Resume of an existing idle/failed record: the prior status is
                    // not unique, so this write stays unconditional (a same-instant
                    // exit frame can be overwritten -- reconcile converges on boot).
                    super::status_events::update_status_and_emit(
                        cx,
                        &surface_id,
                        SurfaceStatus::Live,
                    )
                    .await?;
                    let mut updated = view;
                    updated.status = "live".to_owned();
                    Ok(updated)
                }
                Err(e) => {
                    super::status_events::update_status_and_emit(
                        cx,
                        &surface_id,
                        SurfaceStatus::Failed,
                    )
                    .await?;
                    Err(e)
                }
            }
        } else {
            let session_id = SessionId::from_string(&self.session);
            let kind = SurfaceKind::Terminal;
            let geometry = match (self.cols, self.rows) {
                (Some(cols), Some(rows)) => Geometry { cols, rows },
                _ => DEFAULT_GEOMETRY,
            };

            let surface = SurfaceRepo::create(
                cx.db(),
                None,
                &session_id,
                kind,
                self.cwd.as_deref(),
                Some(&self.placement),
                SurfaceStatus::Pending,
            )
            .await?;

            let request = SpawnRequest {
                surface: surface.id.clone(),
                command: None,
                token: uuid::Uuid::new_v4().to_string(),
                geometry,
                cwd: self.cwd.clone().unwrap_or_else(default_cwd),
            };

            match cx.runtime().spawn(request).await {
                Ok(()) => {
                    super::status_events::confirm_spawn_and_emit(cx, &surface.id).await?;
                    Ok(SurfaceView {
                        id: surface.id.as_str().to_owned(),
                        session_id: self.session.clone(),
                        kind: kind.as_str().to_owned(),
                        cwd: self.cwd.clone(),
                        status: "live".to_owned(),
                        placement: Some(self.placement.clone()),
                    })
                }
                Err(e) => {
                    super::status_events::update_status_and_emit(
                        cx,
                        &surface.id,
                        SurfaceStatus::Failed,
                    )
                    .await?;
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::surface::test_util::{harness, seed_session};
    use crate::entities::SurfaceId;
    use crate::infra::daemon_pty_api::RuntimeCall;

    #[tokio::test]
    async fn resolve_or_spawn_spawns_new_surface_if_absent() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-resolve").await;

        let view = h
            .bus
            .query(ResolveOrSpawnSurface {
                session: session.clone(),
                placement: "main".to_owned(),
                cwd: Some("/work".to_owned()),
                cols: Some(100),
                rows: Some(30),
            })
            .await
            .unwrap();

        assert_eq!(view.status, "live");
        assert_eq!(view.placement, Some("main".to_owned()));
        assert_eq!(view.cwd, Some("/work".to_owned()));

        let id = SurfaceId::from_string(&view.id);
        assert!(h.runtime.is_running(&id));
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Spawn(id)]);
    }

    #[tokio::test]
    async fn resolve_or_spawn_resolves_existing_live_surface_without_spawning() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-resolve-existing").await;

        let first = h
            .bus
            .query(ResolveOrSpawnSurface {
                session: session.clone(),
                placement: "main".to_owned(),
                cwd: Some("/work".to_owned()),
                cols: Some(100),
                rows: Some(30),
            })
            .await
            .unwrap();

        h.runtime.clear_calls();

        let second = h
            .bus
            .query(ResolveOrSpawnSurface {
                session: session.clone(),
                placement: "main".to_owned(),
                cwd: Some("/work".to_owned()),
                cols: Some(100),
                rows: Some(30),
            })
            .await
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.status, "live");
        assert!(h.runtime.calls().is_empty());
    }

    #[tokio::test]
    async fn resolve_or_spawn_attempts_spawning_existing_failed_surface() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-resolve-failed").await;

        h.runtime.fail_next_spawn();
        let first_res = h
            .bus
            .query(ResolveOrSpawnSurface {
                session: session.clone(),
                placement: "main".to_owned(),
                cwd: Some("/work".to_owned()),
                cols: Some(100),
                rows: Some(30),
            })
            .await;
        assert!(first_res.is_err());

        let existing = sqlx::query_as::<_, SurfaceView>(
            "SELECT id, session_id, kind, cwd, status, placement
             FROM surface WHERE session_id = ? AND placement = ?",
        )
        .bind(&session)
        .bind("main")
        .fetch_one(&h.pool)
        .await
        .unwrap();
        assert_eq!(existing.status, "failed");

        h.runtime.clear_calls();

        let second = h
            .bus
            .query(ResolveOrSpawnSurface {
                session: session.clone(),
                placement: "main".to_owned(),
                cwd: Some("/work".to_owned()),
                cols: Some(100),
                rows: Some(30),
            })
            .await
            .unwrap();

        assert_eq!(existing.id, second.id);
        assert_eq!(second.status, "live");

        let id = SurfaceId::from_string(&second.id);
        assert!(h.runtime.is_running(&id));
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Spawn(id)]);
    }
}
