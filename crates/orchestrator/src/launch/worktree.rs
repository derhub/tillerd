use std::sync::Arc;

use crate::error::{OrchestratorError, Result};
use crate::persistence::{NewWorktree, ProjectId, Store, Worktree};

/// Execute the worktree step: run `git worktree add <path> <branch>`, write a row to the store,
/// and return the created `Worktree` (the caller uses `worktree.path` as the surface's `cwd`).
pub fn execute(
    project_id: &ProjectId,
    branch: &str,
    path: &str,
    store: &Arc<dyn Store>,
) -> Result<Worktree> {
    let status = std::process::Command::new("git")
        .args(["worktree", "add", path, branch])
        .output()
        .map_err(|e| OrchestratorError::WorktreeStepFailed(e.to_string()))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr).into_owned();
        return Err(OrchestratorError::WorktreeStepFailed(stderr));
    }

    store.create_worktree(NewWorktree {
        project_id: project_id.clone(),
        path: path.to_string(),
        branch: Some(branch.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::memory::InMemoryStore;

    /// Create a temp dir with a committed file so `git worktree add` has a branch to check out.
    fn init_git_repo_with_commit() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(p)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(p)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("init.txt"), "init").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(p)
            .output()
            .unwrap();
        dir
    }

    #[test]
    fn worktree_step_creates_directory_writes_row_and_returns_path() {
        let repo = init_git_repo_with_commit();
        // Create a new branch so no other worktree holds it
        std::process::Command::new("git")
            .args(["branch", "wt-test"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        let wt_parent = tempfile::tempdir().expect("wt parent");
        let wt_path = wt_parent.path().join("my-worktree");
        let wt_path_str = wt_path.to_str().unwrap().to_string();

        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let project_id = ProjectId::unfiled();

        // execute runs `git worktree add` from the cwd; set cwd to the repo
        let saved = std::env::current_dir().ok();
        let _ = std::env::set_current_dir(repo.path());
        let worktree = execute(&project_id, "wt-test", &wt_path_str, &store).unwrap();
        if let Some(d) = saved {
            let _ = std::env::set_current_dir(d);
        }

        assert_eq!(worktree.path, wt_path_str);
        assert_eq!(worktree.branch.as_deref(), Some("wt-test"));
        assert_eq!(worktree.project_id, project_id);

        let rows = store.list_worktrees(&project_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, wt_path_str);
    }

    #[test]
    fn worktree_step_failure_returns_typed_error_without_writing_row() {
        let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
        let project_id = ProjectId::unfiled();

        // Use a non-existent path that will fail git (not in a git repo, no branch)
        let err = execute(
            &project_id,
            "no-such-branch",
            "/tmp/tillerd-wt-test-nonexistent-xyzzy",
            &store,
        )
        .unwrap_err();

        assert!(
            matches!(err, OrchestratorError::WorktreeStepFailed(_)),
            "expected WorktreeStepFailed, got: {err}"
        );
        let worktrees = store.list_worktrees(&project_id).unwrap();
        assert!(
            worktrees.is_empty(),
            "no worktree row should be written on failure"
        );
    }
}
