use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::infra::surface_repo::SurfaceRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Instantiate a session's launch spec onto the runtime (D9 side-effect shape).
/// A session with no spec launches nothing.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSession {
    pub id: String,
}

impl Command<Ctx> for LaunchSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::entities::launch_spec;
        use crate::entities::surface::SurfaceStatus;
        use crate::entities::SurfaceKind;
        use crate::infra::runtime::{Geometry, SpawnRequest};

        let id = SessionId::from_string(&self.id);
        let s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;

        let (spec_version, spec_json) = match (s.spec_version, s.spec_json) {
            (Some(v), Some(j)) => (v, j),
            _ => return Ok(()),
        };

        let spec = launch_spec::migrate(&spec_json, spec_version)
            .map_err(|e| Error::Validation {
                field: "spec",
                reason: e.to_string(),
            })?
            .0;

        for item in &spec.items {
            // D9: persist intent (pending).
            let surface = SurfaceRepo::create(
                cx.db(),
                None,
                &id,
                SurfaceKind::Terminal,
                None,
                item.placement.as_deref(),
            )
            .await?;

            let request = SpawnRequest {
                surface: surface.id.clone(),
                kind: surface.kind,
                command: None,
                token: String::new(),
                geometry: Geometry {
                    cols: 220,
                    rows: 50,
                },
                cwd: String::new(),
            };

            // D9: run effect lock-free, record outcome.
            match cx.runtime().spawn(request).await {
                Ok(()) => {
                    SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Live).await?;
                }
                Err(e) => {
                    SurfaceRepo::update_status(cx.db(), &surface.id, SurfaceStatus::Failed).await?;
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::apply_launch_spec::ApplyLaunchSpec;
    use crate::app::session::test_util::{create_one, ctx};
    use crate::infra::surface_repo::SurfaceRepo;
    use crate::shared::pagination::Page;

    // Scenario: Launching a session instantiates its spec onto the runtime
    #[tokio::test]
    async fn launch_session_with_no_spec_launches_nothing() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(LaunchSession { id: id.clone() }).await.unwrap();

        let surfaces = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert!(surfaces.items.is_empty());
    }

    #[tokio::test]
    async fn launch_session_spawns_one_surface_per_spec_item() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[
                {"target":"main","placement":"p1","command":{"executable":"/bin/sh","args":[]}},
                {"target":"side","placement":"p2","command":{"executable":"/bin/sh","args":[]}}
            ]}"#
            .to_owned(),
        })
        .await
        .unwrap();

        bus.execute(LaunchSession { id: id.clone() }).await.unwrap();

        let surfaces = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert_eq!(surfaces.items.len(), 2);
        assert!(surfaces.items.iter().all(|sf| sf.status.is_live()));
    }
}
