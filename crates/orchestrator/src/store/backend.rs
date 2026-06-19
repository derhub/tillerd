//! Closed `Backend` enum over the concrete `infra` backends.
//!
//! Each variant wraps one backend behind `Arc` so a `Backend` clones cheaply and a
//! `spawn_blocking` closure can own it. The enum exposes one async forwarding method per
//! persisted operation; each `match`es the variant and calls the backend's existing
//! (sync, behavior-preserving) method. `Fs`/`Sqlite` run the blocking call off the runtime
//! via `spawn_blocking`; `Memory` runs inline. Domain operations are served by `Fs`/`Memory`,
//! operational operations by `Sqlite`/`Memory`; the impossible variant pair returns a
//! `Persistence` error (never reached given composition-root wiring).

use std::sync::Arc;

use tokio::task::spawn_blocking;

use crate::entities::{
    Command, LaunchTemplate, LaunchTemplateId, NewCommand, NewLaunchTemplate, NewProject,
    NewSession, NewSurface, NewWorkspace, NotificationRecord, Project, ProjectId, Session,
    SessionId, SettingEntry, SettingScope, Surface, SurfaceId, Workspace, WorkspaceId,
};
use crate::error::{OrchestratorError, Result};
use crate::infra::fs::FsBackend;
use crate::infra::memory::MemoryBackend;
use crate::infra::sqlite::SqliteBackend;

/// Storage backend selected at the composition root. Cheap to clone (shared `Arc`).
#[derive(Clone)]
pub enum Backend {
    Fs(Arc<FsBackend>),
    Sqlite(Arc<SqliteBackend>),
    Memory(Arc<MemoryBackend>),
}

fn wrong_backend(op: &str) -> OrchestratorError {
    OrchestratorError::Persistence(format!("backend does not serve `{op}`"))
}

/// Run a blocking storage closure off the async runtime.
async fn blocking<T>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T>
where
    T: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(result) => result,
        Err(e) => Err(OrchestratorError::Persistence(format!(
            "background storage task failed: {e}"
        ))),
    }
}

impl Backend {
    // ── workspace (domain) ────────────────────────────────────────────────

    pub async fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_workspace(draft)).await
            }
            Backend::Memory(b) => b.create_workspace(draft),
            Backend::Sqlite(_) => Err(wrong_backend("create_workspace")),
        }
    }

    pub async fn rename_workspace(&self, id: WorkspaceId, name: String) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.rename_workspace(&id, &name)).await
            }
            Backend::Memory(b) => b.rename_workspace(&id, &name),
            Backend::Sqlite(_) => Err(wrong_backend("rename_workspace")),
        }
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_workspaces()).await
            }
            Backend::Memory(b) => b.list_workspaces(),
            Backend::Sqlite(_) => Err(wrong_backend("list_workspaces")),
        }
    }

    pub async fn reorder_workspace(&self, id: WorkspaceId, sort_order: u32) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.reorder_workspace(&id, sort_order)).await
            }
            Backend::Memory(b) => b.reorder_workspace(&id, sort_order),
            Backend::Sqlite(_) => Err(wrong_backend("reorder_workspace")),
        }
    }

    pub async fn delete_workspace(&self, id: WorkspaceId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.delete_workspace(&id)).await
            }
            Backend::Memory(b) => b.delete_workspace(&id),
            Backend::Sqlite(_) => Err(wrong_backend("delete_workspace")),
        }
    }

    // ── project (domain) ──────────────────────────────────────────────────

    pub async fn get_project(&self, id: ProjectId) -> Result<Option<Project>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_project(&id)).await
            }
            Backend::Memory(b) => b.get_project(&id),
            Backend::Sqlite(_) => Err(wrong_backend("get_project")),
        }
    }

    pub async fn create_project(&self, draft: NewProject) -> Result<Project> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_project(draft)).await
            }
            Backend::Memory(b) => b.create_project(draft),
            Backend::Sqlite(_) => Err(wrong_backend("create_project")),
        }
    }

    pub async fn rename_project(&self, id: ProjectId, name: String) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.rename_project(&id, &name)).await
            }
            Backend::Memory(b) => b.rename_project(&id, &name),
            Backend::Sqlite(_) => Err(wrong_backend("rename_project")),
        }
    }

    pub async fn list_projects(&self, workspace_id: Option<WorkspaceId>) -> Result<Vec<Project>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_projects(workspace_id.as_ref())).await
            }
            Backend::Memory(b) => b.list_projects(workspace_id.as_ref()),
            Backend::Sqlite(_) => Err(wrong_backend("list_projects")),
        }
    }

    pub async fn move_project(
        &self,
        project_id: ProjectId,
        workspace_id: WorkspaceId,
    ) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.move_project(&project_id, &workspace_id)).await
            }
            Backend::Memory(b) => b.move_project(&project_id, &workspace_id),
            Backend::Sqlite(_) => Err(wrong_backend("move_project")),
        }
    }

    pub async fn archive_project(&self, id: ProjectId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.archive_project(&id)).await
            }
            Backend::Memory(b) => b.archive_project(&id),
            Backend::Sqlite(_) => Err(wrong_backend("archive_project")),
        }
    }

    pub async fn hard_delete_project(&self, id: ProjectId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.hard_delete_project(&id)).await
            }
            Backend::Memory(b) => b.hard_delete_project(&id),
            Backend::Sqlite(_) => Err(wrong_backend("hard_delete_project")),
        }
    }

    pub async fn reorder_project(&self, id: ProjectId, sort_order: u32) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.reorder_project(&id, sort_order)).await
            }
            Backend::Memory(b) => b.reorder_project(&id, sort_order),
            Backend::Sqlite(_) => Err(wrong_backend("reorder_project")),
        }
    }

    // ── session (domain) ──────────────────────────────────────────────────

    pub async fn create_session(
        &self,
        draft: NewSession,
        spec: Option<(u32, String)>,
    ) -> Result<Session> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_session(draft, spec)).await
            }
            Backend::Memory(b) => b.create_session_inner(draft, spec),
            Backend::Sqlite(_) => Err(wrong_backend("create_session")),
        }
    }

    pub async fn rename_session(&self, id: SessionId, title: String) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.rename_session(&id, &title)).await
            }
            Backend::Memory(b) => b.rename_session(&id, &title),
            Backend::Sqlite(_) => Err(wrong_backend("rename_session")),
        }
    }

    pub async fn list_sessions(&self, project_id: Option<ProjectId>) -> Result<Vec<Session>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_sessions(project_id.as_ref())).await
            }
            Backend::Memory(b) => b.list_sessions(project_id.as_ref()),
            Backend::Sqlite(_) => Err(wrong_backend("list_sessions")),
        }
    }

    pub async fn get_session(&self, id: SessionId) -> Result<Option<Session>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_session(&id)).await
            }
            Backend::Memory(b) => b.get_session(&id),
            Backend::Sqlite(_) => Err(wrong_backend("get_session")),
        }
    }

    pub async fn archive_session(&self, id: SessionId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.archive_session(&id)).await
            }
            Backend::Memory(b) => b.archive_session(&id),
            Backend::Sqlite(_) => Err(wrong_backend("archive_session")),
        }
    }

    pub async fn hard_delete_session(&self, id: SessionId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.hard_delete_session(&id)).await
            }
            Backend::Memory(b) => b.hard_delete_session(&id),
            Backend::Sqlite(_) => Err(wrong_backend("hard_delete_session")),
        }
    }

    pub async fn reorder_session(&self, id: SessionId, sort_order: u32) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.reorder_session(&id, sort_order)).await
            }
            Backend::Memory(b) => b.reorder_session(&id, sort_order),
            Backend::Sqlite(_) => Err(wrong_backend("reorder_session")),
        }
    }

    // ── surface (domain) ──────────────────────────────────────────────────

    pub async fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_surface(draft)).await
            }
            Backend::Memory(b) => b.create_surface(draft),
            Backend::Sqlite(_) => Err(wrong_backend("create_surface")),
        }
    }

    pub async fn get_surface(&self, id: SurfaceId) -> Result<Option<Surface>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_surface(&id)).await
            }
            Backend::Memory(b) => b.get_surface(&id),
            Backend::Sqlite(_) => Err(wrong_backend("get_surface")),
        }
    }

    pub async fn find_session_surface_by_placement(
        &self,
        session_id: SessionId,
        placement: String,
    ) -> Result<Option<Surface>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.find_session_surface_by_placement(&session_id, &placement)).await
            }
            Backend::Memory(b) => b.find_session_surface_by_placement(&session_id, &placement),
            Backend::Sqlite(_) => Err(wrong_backend("find_session_surface_by_placement")),
        }
    }

    pub async fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_resumable_surfaces()).await
            }
            Backend::Memory(b) => b.list_resumable_surfaces(),
            Backend::Sqlite(_) => Err(wrong_backend("list_resumable_surfaces")),
        }
    }

    pub async fn update_surface_status(&self, id: SurfaceId, status: String) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.update_surface_status(&id, &status)).await
            }
            Backend::Memory(b) => b.update_surface_status(&id, &status),
            Backend::Sqlite(_) => Err(wrong_backend("update_surface_status")),
        }
    }

    pub async fn soft_delete_surface(&self, id: SurfaceId) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.soft_delete_surface(&id)).await
            }
            Backend::Memory(b) => b.soft_delete_surface(&id),
            Backend::Sqlite(_) => Err(wrong_backend("soft_delete_surface")),
        }
    }

    pub async fn add_surface_to_session(
        &self,
        session_id: SessionId,
        surface_id: SurfaceId,
    ) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.add_surface_to_session(&session_id, &surface_id)).await
            }
            Backend::Memory(b) => b.add_surface_to_session(&session_id, &surface_id),
            Backend::Sqlite(_) => Err(wrong_backend("add_surface_to_session")),
        }
    }

    pub async fn remove_surface_from_session(
        &self,
        session_id: SessionId,
        surface_id: SurfaceId,
    ) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.remove_surface_from_session(&session_id, &surface_id)).await
            }
            Backend::Memory(b) => b.remove_surface_from_session(&session_id, &surface_id),
            Backend::Sqlite(_) => Err(wrong_backend("remove_surface_from_session")),
        }
    }

    // ── session layout/spec (domain) ──────────────────────────────────────

    pub async fn set_session_spec(
        &self,
        id: SessionId,
        spec_version: u32,
        spec_json: String,
    ) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.set_session_spec(&id, spec_version, &spec_json)).await
            }
            Backend::Memory(b) => b.set_session_spec(&id, spec_version, &spec_json),
            Backend::Sqlite(_) => Err(wrong_backend("set_session_spec")),
        }
    }

    pub async fn set_session_layout(&self, id: SessionId, layout_json: String) -> Result<()> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.set_session_layout(&id, &layout_json)).await
            }
            Backend::Memory(b) => b.set_session_layout(&id, &layout_json),
            Backend::Sqlite(_) => Err(wrong_backend("set_session_layout")),
        }
    }

    pub async fn get_session_layout(&self, id: SessionId) -> Result<Option<String>> {
        match self {
            Backend::Fs(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_session_layout(&id)).await
            }
            Backend::Memory(b) => b.get_session_layout(&id),
            Backend::Sqlite(_) => Err(wrong_backend("get_session_layout")),
        }
    }

    // ── meta (operational) ────────────────────────────────────────────────

    pub async fn schema_version(&self) -> Result<u32> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.schema_version()).await
            }
            Backend::Memory(b) => b.schema_version(),
            Backend::Fs(_) => Err(wrong_backend("schema_version")),
        }
    }

    // ── command (operational) ─────────────────────────────────────────────

    pub async fn list_commands(&self) -> Result<Vec<Command>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_commands()).await
            }
            Backend::Memory(b) => b.list_commands(),
            Backend::Fs(_) => Err(wrong_backend("list_commands")),
        }
    }

    pub async fn get_command(&self, id: String) -> Result<Option<Command>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_command(&id)).await
            }
            Backend::Memory(b) => b.get_command(&id),
            Backend::Fs(_) => Err(wrong_backend("get_command")),
        }
    }

    pub async fn create_command(&self, draft: NewCommand) -> Result<Command> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_command(draft)).await
            }
            Backend::Memory(b) => b.create_command(draft),
            Backend::Fs(_) => Err(wrong_backend("create_command")),
        }
    }

    pub async fn delete_command(&self, id: String) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.delete_command(&id)).await
            }
            Backend::Memory(b) => b.delete_command(&id),
            Backend::Fs(_) => Err(wrong_backend("delete_command")),
        }
    }

    pub async fn seed_commands(&self) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.seed_commands()).await
            }
            Backend::Memory(b) => b.seed_commands(),
            Backend::Fs(_) => Err(wrong_backend("seed_commands")),
        }
    }

    // ── launch template (operational) ─────────────────────────────────────

    pub async fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.create_launch_template(draft)).await
            }
            Backend::Memory(b) => b.create_launch_template(draft),
            Backend::Fs(_) => Err(wrong_backend("create_launch_template")),
        }
    }

    pub async fn get_launch_template(
        &self,
        id: LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_launch_template(&id)).await
            }
            Backend::Memory(b) => b.get_launch_template(&id),
            Backend::Fs(_) => Err(wrong_backend("get_launch_template")),
        }
    }

    pub async fn set_launch_template_spec(
        &self,
        id: LaunchTemplateId,
        spec_version: u32,
        spec_json: String,
    ) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.set_launch_template_spec(&id, spec_version, &spec_json)).await
            }
            Backend::Memory(b) => b.set_launch_template_spec(&id, spec_version, &spec_json),
            Backend::Fs(_) => Err(wrong_backend("set_launch_template_spec")),
        }
    }

    // ── settings (operational) ────────────────────────────────────────────

    pub async fn get_setting(&self, scope: SettingScope, key: String) -> Result<Option<String>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.get_setting(&scope, &key)).await
            }
            Backend::Memory(b) => b.get_setting(&scope, &key),
            Backend::Fs(_) => Err(wrong_backend("get_setting")),
        }
    }

    pub async fn set_setting(
        &self,
        scope: SettingScope,
        key: String,
        value_json: String,
    ) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.set_setting(&scope, &key, &value_json)).await
            }
            Backend::Memory(b) => b.set_setting(&scope, &key, &value_json),
            Backend::Fs(_) => Err(wrong_backend("set_setting")),
        }
    }

    pub async fn list_settings(&self, scope: SettingScope) -> Result<Vec<SettingEntry>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_settings(&scope)).await
            }
            Backend::Memory(b) => b.list_settings(&scope),
            Backend::Fs(_) => Err(wrong_backend("list_settings")),
        }
    }

    pub async fn resolve_setting(
        &self,
        project_id: ProjectId,
        key: String,
    ) -> Result<Option<String>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.resolve_setting(&project_id, &key)).await
            }
            Backend::Memory(b) => b.resolve_setting(&project_id, &key),
            Backend::Fs(_) => Err(wrong_backend("resolve_setting")),
        }
    }

    // ── notifications (operational) ───────────────────────────────────────

    pub async fn insert_notification(&self, rec: NotificationRecord) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.insert_notification(&rec)).await
            }
            Backend::Memory(b) => b.insert_notification(&rec),
            Backend::Fs(_) => Err(wrong_backend("insert_notification")),
        }
    }

    pub async fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.list_notifications(limit)).await
            }
            Backend::Memory(b) => b.list_notifications(limit),
            Backend::Fs(_) => Err(wrong_backend("list_notifications")),
        }
    }

    pub async fn prune_notifications(&self, keep: u32) -> Result<()> {
        match self {
            Backend::Sqlite(b) => {
                let b = Arc::clone(b);
                blocking(move || b.prune_notifications(keep)).await
            }
            Backend::Memory(b) => b.prune_notifications(keep),
            Backend::Fs(_) => Err(wrong_backend("prune_notifications")),
        }
    }
}
