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
        use crate::app::surface::{confirm_spawn_and_emit, update_status_and_emit};
        use crate::entities::launch_spec;
        use crate::entities::surface::SurfaceStatus;
        use crate::entities::SurfaceKind;
        use crate::infra::daemon_pty_api::{Geometry, SpawnRequest};

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

        if spec.items.is_empty() {
            return Ok(());
        }
        let workspace_id = crate::app::surface::workspace_id_for_session(cx, &id).await?;

        for item in &spec.items {
            // Resolve before persisting anything: an unresolvable reference fails just
            // this item (typed not-found), no surface created, the run continues.
            let command = match crate::app::command::resolve(cx, &item.command).await {
                Ok(command) => command,
                Err(_) => continue,
            };

            // D9: persist intent (pending).
            let surface = SurfaceRepo::create(
                cx.db(),
                None,
                &id,
                SurfaceKind::Terminal,
                None,
                item.placement.as_deref(),
                SurfaceStatus::Pending,
            )
            .await?;

            let request = SpawnRequest {
                surface: surface.id.clone(),
                command: Some(command),
                token: String::new(),
                geometry: Geometry {
                    cols: 220,
                    rows: 50,
                },
                cwd: String::new(),
            };

            // D9: run effect lock-free, record outcome (emits surface-status push).
            match cx.runtime().spawn(request).await {
                Ok(()) => {
                    confirm_spawn_and_emit(cx, &surface.id, &workspace_id).await?;
                }
                Err(e) => {
                    update_status_and_emit(cx, &surface.id, &workspace_id, SurfaceStatus::Failed)
                        .await?;
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
    use crate::app::session::test_util::{create_one, ctx, ctx_with_runtime};
    use crate::entities::command::{Command, CommandId, CommandOrigin};
    use crate::infra::surface_repo::SurfaceRepo;
    use crate::infra::CommandRepo;
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

    // Scenario: Library reference resolves
    #[tokio::test]
    async fn launch_session_resolves_a_library_command_reference() {
        let (bus, pool, runtime) = ctx_with_runtime().await;
        let id = create_one(&bus).await;

        let mut env = std::collections::HashMap::new();
        env.insert("FOO".to_owned(), "bar".to_owned());
        let cmd = Command {
            id: CommandId::mint(),
            name: "htop".to_owned(),
            origin: CommandOrigin::Custom,
            cli: "/usr/bin/htop".to_owned(),
            args: vec!["--sort-key".to_owned(), "cpu".to_owned()],
            env,
            pinned: false,
        };
        CommandRepo::create(&pool, &cmd).await.unwrap();

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: format!(
                r#"{{"version":1,"items":[{{"target":"main","placement":"p1","command":{{"library_ref":"{}"}}}}]}}"#,
                cmd.id.as_str()
            ),
        })
        .await
        .unwrap();

        bus.execute(LaunchSession { id: id.clone() }).await.unwrap();

        let surfaces = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert_eq!(surfaces.items.len(), 1);
        let surface_id = surfaces.items[0].id.clone();
        let spawned = runtime
            .spawn_command(&surface_id)
            .expect("a command payload was recorded");
        assert_eq!(spawned.exe, "/usr/bin/htop");
        assert_eq!(
            spawned.args,
            vec!["--sort-key".to_owned(), "cpu".to_owned()]
        );
        assert_eq!(spawned.env.get("FOO"), Some(&"bar".to_owned()));
    }

    // Scenario: Inline command is used as given
    #[tokio::test]
    async fn launch_session_uses_an_inline_command_as_given() {
        let (bus, pool, runtime) = ctx_with_runtime().await;
        let id = create_one(&bus).await;

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[
                {"target":"main","placement":"p1","command":{"executable":"/bin/sh","args":["-c","echo hi"]}}
            ]}"#
            .to_owned(),
        })
        .await
        .unwrap();

        bus.execute(LaunchSession { id: id.clone() }).await.unwrap();

        let surfaces = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert_eq!(surfaces.items.len(), 1);
        let surface_id = surfaces.items[0].id.clone();
        let spawned = runtime
            .spawn_command(&surface_id)
            .expect("a command payload was recorded");
        assert_eq!(spawned.exe, "/bin/sh");
        assert_eq!(spawned.args, vec!["-c".to_owned(), "echo hi".to_owned()]);
        assert!(spawned.env.is_empty());
    }

    // Scenario: Unknown reference fails the item, launch continues
    #[tokio::test]
    async fn launch_session_skips_an_unresolvable_item_and_continues() {
        let (bus, pool, _runtime) = ctx_with_runtime().await;
        let id = create_one(&bus).await;

        bus.execute(ApplyLaunchSpec {
            id: id.clone(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[
                {"target":"main","placement":"p1","command":{"executable":"/bin/sh","args":[]}},
                {"target":"side","placement":"p2","command":{"library_ref":"does-not-exist"}},
                {"target":"aux","placement":"p3","command":{"executable":"/bin/sh","args":[]}}
            ]}"#
            .to_owned(),
        })
        .await
        .unwrap();

        bus.execute(LaunchSession { id: id.clone() })
            .await
            .expect("the run completes best-effort despite the unresolvable item");

        let surfaces = SurfaceRepo::list(&pool, &SessionId::from_string(&id), Page::All)
            .await
            .unwrap();
        assert_eq!(
            surfaces.items.len(),
            2,
            "the unresolvable item creates no surface"
        );
        // Repo listing orders by id (a UUID), not insertion order -- compare as a set.
        let mut placements: Vec<_> = surfaces.items.iter().map(|s| s.placement.clone()).collect();
        placements.sort();
        assert_eq!(
            placements,
            vec![Some("p1".to_owned()), Some("p3".to_owned())]
        );
    }
}
