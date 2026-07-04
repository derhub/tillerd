use serde::Deserialize;

use crate::context::{Ctx, SqliteTx};
use crate::entities::launch_spec::{self, CommandRef, LaunchItem, LaunchSpec};
use crate::entities::session::SessionId;
use crate::entities::{SurfaceKind, SurfaceStatus};
use crate::infra::daemon_pty_api::{Geometry, SpawnRequest};
use crate::infra::session::SessionRepo;
use crate::infra::SurfaceRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

use super::common::{default_cwd, DEFAULT_GEOMETRY};

/// Wire shape for a spawn-time command reference: camelCase (`libraryRef`) like its
/// sibling transport params, unlike the opaque snake_case `command` persisted inside
/// a launch spec item's raw JSON blob (`CommandRef`). Transport-only -- `SpawnSurface`
/// itself carries the decomposed primitive fields (message-dto: DTO fields are plain
/// built-in types; a sum type is reassembled at the edge, not held on the DTO).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(untagged)]
pub enum SpawnCommandRef {
    // `rename_all` on the enum only renames variant names (irrelevant here, since
    // this is untagged); each struct variant needs its own to camelCase its fields.
    #[serde(rename_all = "camelCase")]
    LibraryRef { library_ref: String },
    Inline {
        executable: String,
        args: Vec<String>,
    },
}

impl SpawnCommandRef {
    /// Decompose into `SpawnSurface`'s flat primitive fields.
    pub fn into_dto_fields(self) -> (Option<String>, Option<String>, Vec<String>) {
        match self {
            SpawnCommandRef::LibraryRef { library_ref } => (Some(library_ref), None, Vec::new()),
            SpawnCommandRef::Inline { executable, args } => (None, Some(executable), args),
        }
    }
}

/// Add a surface to a session: persist a `pending` row (committed), spawn its PTY
/// lock-free via the runtime port, then record the outcome. The sqlite write lock
/// is never held across the spawn.
///
/// `kind` is the wire string (`terminal`/`diff`); `cols`/`rows` carry the spawn
/// geometry (both present -> explicit geometry, else the default). `command_*`
/// (mutually exclusive: a library ref, or an inline executable + args), when given,
/// is resolved and the surface diverges the session's launch spec so it survives a
/// reconcile; a commandless spawn (login shell) leaves the spec untouched -- [D].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSurface {
    pub session: String,
    pub kind: String,
    pub cwd: Option<String>,
    pub placement: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    #[serde(default)]
    pub command_library_ref: Option<String>,
    #[serde(default)]
    pub command_executable: Option<String>,
    #[serde(default)]
    pub command_args: Vec<String>,
}

impl Command<Ctx> for SpawnSurface {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let kind = parse_kind(&self.kind)?;

        // Capability check before any persist: unsupported kinds leave no trace.
        require_terminal(kind)?;

        let geometry = match (self.cols, self.rows) {
            (Some(cols), Some(rows)) => Geometry { cols, rows },
            _ => DEFAULT_GEOMETRY,
        };

        // Convert the DTO's flat primitives into the domain sum type at the edge,
        // then resolve before any persist: an unresolvable reference leaves no
        // trace, same as the unsupported-kind guard above.
        let command_ref = command_ref_from_dto(
            self.command_library_ref.as_deref(),
            self.command_executable.as_deref(),
            &self.command_args,
        );
        let resolved = match &command_ref {
            Some(cmd_ref) => Some(crate::app::command::resolve(cx, cmd_ref).await?),
            None => None,
        };

        let session_id = SessionId::from_string(&self.session);
        let workspace_id = super::status_events::workspace_id_for_session(cx, &session_id).await?;

        // 1) persist intent, and (only when a command was given) diverge the launch
        // spec in the same write -- both commit or neither does.
        let surface = cx
            .transaction(async |tx| {
                let surface = SurfaceRepo::create(
                    &mut **tx,
                    None,
                    &session_id,
                    kind,
                    self.cwd.as_deref(),
                    self.placement.as_deref(),
                    SurfaceStatus::Pending,
                )
                .await?;

                if let Some(cmd_ref) = command_ref {
                    append_launch_item(tx, &session_id, self.placement.as_deref(), cmd_ref).await?;
                }

                Ok(surface)
            })
            .await?;

        // 2) run the effect lock-free -- no transaction held
        let request = SpawnRequest {
            surface: surface.id.clone(),
            command: resolved,
            token: uuid::Uuid::new_v4().to_string(),
            geometry,
            cwd: self.cwd.clone().unwrap_or_else(default_cwd),
        };
        match cx.runtime().spawn(request).await {
            // 3) record the outcome (emits the surface-status push)
            Ok(()) => {
                super::status_events::confirm_spawn_and_emit(cx, &surface.id, &workspace_id).await
            }
            Err(e) => {
                super::status_events::update_status_and_emit(
                    cx,
                    &surface.id,
                    &workspace_id,
                    SurfaceStatus::Failed,
                )
                .await?;
                Err(e)
            }
        }
    }
}

/// Reassemble the domain `CommandRef` from the DTO's flat primitive fields. A
/// library ref wins if both are given; `None` when neither is given (login shell).
fn command_ref_from_dto(
    library_ref: Option<&str>,
    executable: Option<&str>,
    args: &[String],
) -> Option<CommandRef> {
    match (library_ref, executable) {
        (Some(library_ref), _) => Some(CommandRef::LibraryRef {
            library_ref: library_ref.to_owned(),
        }),
        (None, Some(executable)) => Some(CommandRef::Inline {
            executable: executable.to_owned(),
            args: args.to_vec(),
        }),
        (None, None) => None,
    }
}

/// Append the newly spawned surface as a launch item so a later reconcile can
/// bring it back.
async fn append_launch_item(
    tx: &mut SqliteTx<'_>,
    session_id: &SessionId,
    placement: Option<&str>,
    command: CommandRef,
) -> Result<()> {
    let mut session = SessionRepo::get(&mut **tx, session_id)
        .await?
        .ok_or_else(|| Error::SessionNotFound(session_id.as_str().to_owned()))?;

    let mut spec = match (session.spec_version, &session.spec_json) {
        (Some(v), Some(j)) => launch_spec::migrate(j, v)?.0,
        _ => LaunchSpec {
            version: launch_spec::CURRENT_SPEC_VERSION,
            items: Vec::new(),
        },
    };
    spec.items.push(LaunchItem {
        target: "terminal".to_owned(),
        placement: placement.map(str::to_owned),
        command,
    });

    session.spec_version = Some(launch_spec::CURRENT_SPEC_VERSION);
    session.spec_json = Some(serde_json::to_string(&spec)?);
    SessionRepo::update(&mut **tx, &session).await
}

fn parse_kind(kind: &str) -> Result<SurfaceKind> {
    match kind {
        "terminal" => Ok(SurfaceKind::Terminal),
        "diff" => Ok(SurfaceKind::Diff),
        other => Err(Error::Validation {
            field: "kind",
            reason: format!("unknown surface kind: {other}"),
        }),
    }
}

/// Reject any kind other than Terminal before any side effect is produced.
/// The only supported kind for PTY spawning is Terminal; this is the app-layer
/// capability gate (the daemon is kind-agnostic and must not see unsupported kinds).
fn require_terminal(kind: SurfaceKind) -> Result<()> {
    if kind == SurfaceKind::Terminal {
        Ok(())
    } else {
        Err(Error::Validation {
            field: "kind",
            reason: format!(
                "surface kind '{}' cannot be spawned as a PTY",
                kind.as_str()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app::session::GetLaunchSpec;
    use crate::app::surface::test_util::{harness, one_surface, seed_session, spawn};
    use crate::entities::command::{Command, CommandId, CommandOrigin};
    use crate::entities::launch_spec::CommandRef;
    use crate::entities::SurfaceId;
    use crate::infra::daemon_pty_api::RuntimeCall;
    use crate::infra::{CommandRepo, SurfaceRepo};
    use crate::shared::pagination::Page;
    use crate::shared::Error;

    use super::SpawnSurface;

    // Scenario: A spawn never holds the write lock across the effect; outcome live
    #[tokio::test]
    async fn spawn_persists_intent_then_records_live_after_the_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-spawn").await;

        h.bus.execute(spawn(&session)).await.unwrap();

        let surface = one_surface(&h, &session).await;
        assert_eq!(surface.status, "live");
        let id = SurfaceId::from_string(&surface.id);
        assert!(h.runtime.is_running(&id));
        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Spawn(id)]);
    }

    // Scenario: a failed effect records `failed` and is reconcilable, not stranded
    #[tokio::test]
    async fn spawn_records_failed_when_the_effect_fails() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-fail").await;
        h.runtime.fail_next_spawn();

        let result = h.bus.execute(spawn(&session)).await;
        assert!(result.is_err());

        let surface = one_surface(&h, &session).await;
        assert_eq!(surface.status, "failed");
        assert!(!h.runtime.is_running(&SurfaceId::from_string(&surface.id)));
    }

    // Scenario: Unsupported kind is rejected before persist -- no row, no runtime call
    #[tokio::test]
    async fn spawn_rejects_unsupported_kind_before_any_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-kind").await;

        let cmd = SpawnSurface {
            session: session.clone(),
            kind: "diff".to_owned(),
            cwd: None,
            placement: None,
            cols: None,
            rows: None,
            command_library_ref: None,
            command_executable: None,
            command_args: Vec::new(),
        };
        let result = h.bus.execute(cmd).await;
        assert!(
            result.is_err(),
            "diff kind must be rejected with a validation error"
        );

        // No row persisted, no runtime call made.
        let rows = SurfaceRepo::list(
            &h.pool,
            &crate::entities::session::SessionId::from_string(&session),
            Page::All,
        )
        .await
        .unwrap();
        assert!(
            rows.items.is_empty(),
            "no row must be persisted for an unsupported kind"
        );
        assert!(
            h.runtime.calls().is_empty(),
            "runtime must not be called for unsupported kind"
        );
    }

    fn custom_command(cli: &str, args: &[&str]) -> Command {
        Command {
            id: CommandId::mint(),
            name: "test-cmd".to_owned(),
            origin: CommandOrigin::Custom,
            cli: cli.to_owned(),
            args: args.iter().map(|a| a.to_string()).collect(),
            env: Default::default(),
            pinned: false,
        }
    }

    // Scenario: a library command reference resolves and diverges the spec
    #[tokio::test]
    async fn spawn_with_a_library_command_resolves_it_and_diverges_the_spec() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-cmd-lib").await;
        let cmd = custom_command("/usr/bin/htop", &["--sort-key", "cpu"]);
        CommandRepo::create(&h.pool, &cmd).await.unwrap();

        h.bus
            .execute(SpawnSurface {
                session: session.clone(),
                kind: "terminal".to_owned(),
                cwd: None,
                placement: Some("p1".to_owned()),
                cols: None,
                rows: None,
                command_library_ref: Some(cmd.id.as_str().to_owned()),
                command_executable: None,
                command_args: Vec::new(),
            })
            .await
            .unwrap();

        let surface = one_surface(&h, &session).await;
        let spawned = h
            .runtime
            .spawn_command(&SurfaceId::from_string(&surface.id))
            .expect("a command payload was recorded");
        assert_eq!(spawned.exe, "/usr/bin/htop");
        assert_eq!(
            spawned.args,
            vec!["--sort-key".to_owned(), "cpu".to_owned()]
        );

        let spec = h
            .bus
            .query(GetLaunchSpec {
                id: session.clone(),
            })
            .await
            .unwrap()
            .expect("spawn diverges the spec when a command is given");
        assert_eq!(spec.0.items.len(), 1);
        assert_eq!(spec.0.items[0].target, "terminal");
        assert_eq!(spec.0.items[0].placement.as_deref(), Some("p1"));
        assert_eq!(
            spec.0.items[0].command,
            CommandRef::LibraryRef {
                library_ref: cmd.id.as_str().to_owned()
            }
        );
    }

    // Scenario: an inline command is used as given and diverges the spec
    #[tokio::test]
    async fn spawn_with_an_inline_command_uses_it_as_given_and_diverges_the_spec() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-cmd-inline").await;

        h.bus
            .execute(SpawnSurface {
                session: session.clone(),
                kind: "terminal".to_owned(),
                cwd: None,
                placement: Some("p1".to_owned()),
                cols: None,
                rows: None,
                command_library_ref: None,
                command_executable: Some("/bin/sh".to_owned()),
                command_args: vec!["-c".to_owned(), "echo hi".to_owned()],
            })
            .await
            .unwrap();

        let surface = one_surface(&h, &session).await;
        let spawned = h
            .runtime
            .spawn_command(&SurfaceId::from_string(&surface.id))
            .expect("a command payload was recorded");
        assert_eq!(spawned.exe, "/bin/sh");
        assert_eq!(spawned.args, vec!["-c".to_owned(), "echo hi".to_owned()]);

        let spec = h
            .bus
            .query(GetLaunchSpec {
                id: session.clone(),
            })
            .await
            .unwrap()
            .expect("spawn diverges the spec when a command is given");
        assert_eq!(
            spec.0.items[0].command,
            CommandRef::Inline {
                executable: "/bin/sh".to_owned(),
                args: vec!["-c".to_owned(), "echo hi".to_owned()],
            }
        );
    }

    // [D]: a commandless (login-shell) spawn leaves the spec untouched.
    #[tokio::test]
    async fn spawn_without_a_command_does_not_diverge_the_spec() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-no-cmd").await;

        h.bus.execute(spawn(&session)).await.unwrap();

        let spec = h.bus.query(GetLaunchSpec { id: session }).await.unwrap();
        assert!(spec.is_none());
    }

    // Scenario: an unknown library reference fails before any persist -- no row, no
    // spec change, no runtime call.
    #[tokio::test]
    async fn spawn_with_an_unresolvable_command_fails_before_any_effect() {
        let h = harness().await;
        let session = seed_session(&h.pool, "s-cmd-missing").await;

        let result = h
            .bus
            .execute(SpawnSurface {
                session: session.clone(),
                kind: "terminal".to_owned(),
                cwd: None,
                placement: Some("p1".to_owned()),
                cols: None,
                rows: None,
                command_library_ref: Some("does-not-exist".to_owned()),
                command_executable: None,
                command_args: Vec::new(),
            })
            .await;
        assert!(matches!(result, Err(Error::CommandNotFound(_))));

        let rows = SurfaceRepo::list(
            &h.pool,
            &crate::entities::session::SessionId::from_string(&session),
            Page::All,
        )
        .await
        .unwrap();
        assert!(rows.items.is_empty(), "no row must be persisted");

        let spec = h.bus.query(GetLaunchSpec { id: session }).await.unwrap();
        assert!(spec.is_none(), "the spec must not diverge");

        assert!(
            h.runtime.calls().is_empty(),
            "runtime must not be called when the command fails to resolve"
        );
    }
}
