use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{OrchestratorError, Result};
use crate::launch::spec::{CommandRef, LaunchItem, LaunchSpec, WorktreeStep};
use crate::launch::worktree;
use crate::persistence::{NewSurface, SessionId, Store, SurfaceId, SurfaceKind, Worktree};
use crate::surface::runtime::ResolvedCommand;

/// Outcome for a single launch item.
#[derive(Debug)]
pub struct LaunchItemResult {
    pub index: usize,
    pub error: Option<String>,
}

/// Brings a launch item's surface to life once the executor has resolved it. The production
/// implementation dispatches to the surface runtime; tests record the calls.
pub trait SurfaceLauncher {
    fn launch(
        &self,
        surface_id: &SurfaceId,
        kind: SurfaceKind,
        command: ResolvedCommand,
        cwd: Option<String>,
    ) -> impl std::future::Future<Output = Result<()>>;
}

/// Execute all items in `spec` in order against the given session, best-effort: a failed item is
/// recorded and execution continues with the rest.
pub async fn run<L: SurfaceLauncher>(
    spec: &LaunchSpec,
    session_id: &SessionId,
    store: &Arc<dyn Store>,
    launcher: &L,
) -> Vec<LaunchItemResult> {
    let mut results = Vec::with_capacity(spec.items.len());
    for (index, item) in spec.items.iter().enumerate() {
        let error = match try_run_item(item, session_id, store, launcher).await {
            Ok(()) => None,
            Err(e) => Some(e.to_string()),
        };
        results.push(LaunchItemResult { index, error });
    }
    results
}

async fn try_run_item<L: SurfaceLauncher>(
    item: &LaunchItem,
    session_id: &SessionId,
    store: &Arc<dyn Store>,
    launcher: &L,
) -> Result<()> {
    let command = resolve_command(&item.command, store)?;
    let kind = surface_kind_for(&item.target)?;
    let (cwd, worktree_id) = match &item.worktree {
        Some(step) => {
            let worktree = run_worktree_step(step, session_id, store)?;
            (Some(worktree.path), Some(worktree.id))
        }
        None => (None, None),
    };
    let surface = store.create_surface(NewSurface {
        id: None,
        session_id: session_id.clone(),
        kind,
        cwd: cwd.clone(),
        placement: item.placement.clone(),
        worktree_id,
    })?;
    launcher.launch(&surface.id, kind, command, cwd).await
}

/// Run an item's worktree step against the session's project repository root, returning the created
/// worktree. Resolving the repo root requires the session, its project, and the project's root path.
fn run_worktree_step(
    step: &WorktreeStep,
    session_id: &SessionId,
    store: &Arc<dyn Store>,
) -> Result<Worktree> {
    let session = store
        .get_session(session_id)?
        .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.as_str().to_string()))?;
    let project = store.get_project(&session.project_id)?.ok_or_else(|| {
        OrchestratorError::ProjectNotFound(session.project_id.as_str().to_string())
    })?;
    let repo_root = project.root_path.ok_or_else(|| {
        OrchestratorError::WorktreeStepFailed("project has no repository root".to_string())
    })?;
    worktree::execute(&session.project_id, &step.branch, &step.path, &repo_root, store)
}

/// Resolve a launch item's command: a library reference resolves to the stored command; an inline
/// command is used as given.
fn resolve_command(command: &CommandRef, store: &Arc<dyn Store>) -> Result<ResolvedCommand> {
    match command {
        CommandRef::LibraryRef { library_ref } => {
            let stored = store
                .get_command(library_ref)?
                .ok_or_else(|| OrchestratorError::CommandNotFound(library_ref.clone()))?;
            Ok(ResolvedCommand {
                exe: stored.cli,
                args: stored.args,
                env: stored.env.into_iter().collect(),
            })
        }
        CommandRef::Inline { executable, args } => Ok(ResolvedCommand {
            exe: executable.clone(),
            args: args.clone(),
            env: BTreeMap::new(),
        }),
    }
}

fn surface_kind_for(target: &str) -> Result<SurfaceKind> {
    match target {
        "terminal" => Ok(SurfaceKind::Terminal),
        "agent" => Ok(SurfaceKind::Agent),
        "diff" => Ok(SurfaceKind::Diff),
        other => Err(OrchestratorError::UnsupportedSurfaceKind(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch::spec::{CommandRef, LaunchItem, LaunchSpec, WorktreeStep};
    use crate::persistence::memory::InMemoryStore;
    use crate::persistence::{NewProject, NewSession, SourceKind};

    #[derive(Default)]
    struct RecordingLauncher {
        calls: std::sync::Mutex<Vec<(SurfaceKind, ResolvedCommand, Option<String>)>>,
    }

    impl SurfaceLauncher for RecordingLauncher {
        async fn launch(
            &self,
            _surface_id: &SurfaceId,
            kind: SurfaceKind,
            command: ResolvedCommand,
            cwd: Option<String>,
        ) -> Result<()> {
            self.calls.lock().unwrap().push((kind, command, cwd));
            Ok(())
        }
    }

    fn make_store() -> Arc<dyn Store> {
        Arc::new(InMemoryStore::new())
    }

    fn make_session(store: &Arc<dyn Store>) -> SessionId {
        store.create_session(NewSession::default()).unwrap().id
    }

    fn item_with_inline(placement: Option<&str>) -> LaunchItem {
        LaunchItem {
            target: "terminal".to_string(),
            placement: placement.map(str::to_string),
            command: CommandRef::Inline {
                executable: "/bin/sh".to_string(),
                args: vec![],
            },
            worktree: None,
        }
    }

    #[tokio::test]
    async fn items_run_in_list_order() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(None), item_with_inline(None)],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 0);
        assert_eq!(results[1].index, 1);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_none());
        assert_eq!(launcher.calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn failed_item_does_not_block_subsequent_items() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![
                LaunchItem {
                    target: "terminal".to_string(),
                    placement: None,
                    command: CommandRef::LibraryRef {
                        library_ref: "no-such-command".to_string(),
                    },
                    worktree: None,
                },
                item_with_inline(None),
            ],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert_eq!(results.len(), 2);
        assert!(
            results[0].error.is_some(),
            "unknown library_ref should fail"
        );
        assert!(results[1].error.is_none(), "second item should succeed");
        assert_eq!(
            launcher.calls.lock().unwrap().len(),
            1,
            "only the surviving item is launched"
        );
    }

    #[tokio::test]
    async fn resolves_command_and_dispatches_by_target() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(None)],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert!(results[0].error.is_none());
        let calls = launcher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, SurfaceKind::Terminal);
        assert_eq!(calls[0].1.exe, "/bin/sh");
    }

    #[tokio::test]
    async fn unsupported_target_fails_the_item() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let mut item = item_with_inline(None);
        item.target = "browser".to_string();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert!(
            results[0].error.is_some(),
            "unknown target must fail the item"
        );
        assert!(launcher.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn placement_hint_stored_when_present() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(Some("sidebar"))],
        };

        run(&spec, &session_id, &store, &launcher).await;

        let surfaces = store.list_resumable_surfaces().unwrap();
        assert!(surfaces
            .iter()
            .any(|s| s.placement.as_deref() == Some("sidebar")));
    }

    /// Init a git repo with one commit and a spare branch for `git worktree add` to check out.
    fn init_git_repo_with_branch(branch: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(p)
                .output()
                .unwrap();
        };
        git(&["init", "-b", "main"]);
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test"]);
        std::fs::write(p.join("init.txt"), "init").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "init"]);
        git(&["branch", branch]);
        dir
    }

    fn make_project_session(store: &Arc<dyn Store>, root_path: &str) -> SessionId {
        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::GitRepo,
                root_path: Some(root_path.to_string()),
                name: Some("proj".to_string()),
            })
            .unwrap();
        store
            .create_session(NewSession {
                project_id: Some(project.id),
                ..Default::default()
            })
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn worktree_step_sets_cwd_and_records_worktree_id() {
        let repo = init_git_repo_with_branch("wt-feature");
        let store = make_store();
        let session_id = make_project_session(&store, repo.path().to_str().unwrap());

        let wt_parent = tempfile::tempdir().expect("wt parent");
        let wt_path = wt_parent.path().join("wt").to_str().unwrap().to_string();

        let mut item = item_with_inline(None);
        item.worktree = Some(WorktreeStep {
            branch: "wt-feature".to_string(),
            path: wt_path.clone(),
        });
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert!(
            results[0].error.is_none(),
            "worktree step should succeed: {:?}",
            results[0].error
        );
        let surface = store
            .list_resumable_surfaces()
            .unwrap()
            .into_iter()
            .find(|s| s.session_id == session_id)
            .expect("surface created");
        assert_eq!(surface.cwd.as_deref(), Some(wt_path.as_str()));
        assert!(surface.worktree_id.is_some(), "worktree recorded on surface");
        let calls = launcher.calls.lock().unwrap();
        assert_eq!(
            calls[0].2.as_deref(),
            Some(wt_path.as_str()),
            "launch uses the worktree path as cwd"
        );
    }

    #[tokio::test]
    async fn worktree_step_failure_fails_the_item() {
        let not_a_repo = tempfile::tempdir().expect("tempdir");
        let store = make_store();
        let session_id = make_project_session(&store, not_a_repo.path().to_str().unwrap());

        let mut item = item_with_inline(None);
        item.worktree = Some(WorktreeStep {
            branch: "no-such-branch".to_string(),
            path: "/tmp/tillerd-exec-wt-nonexistent".to_string(),
        });
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item],
        };

        let results = run(&spec, &session_id, &store, &launcher).await;

        assert!(
            results[0].error.is_some(),
            "worktree step against a non-repo must fail the item"
        );
        assert!(
            store
                .list_resumable_surfaces()
                .unwrap()
                .iter()
                .all(|s| s.session_id != session_id),
            "no surface created when the worktree step fails"
        );
        assert!(launcher.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn null_placement_falls_back_to_default() {
        let store = make_store();
        let session_id = make_session(&store);
        let launcher = RecordingLauncher::default();
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(None)],
        };

        run(&spec, &session_id, &store, &launcher).await;

        let surfaces = store.list_resumable_surfaces().unwrap();
        assert!(surfaces.iter().any(|s| s.placement.is_none()));
    }
}
