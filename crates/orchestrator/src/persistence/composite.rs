//! Composite store: the full `Store` facade backed by two planes.
//!
//! Domain entities (workspace/project/session/surface) are delegated to the file-tree
//! [`FsBackend`]; operational state (schema_version/command/setting/notification/
//! launch_template) is delegated to the SQLite [`SqliteBackend`]. The only cross-plane
//! method is [`create_session`](Store::create_session): a `template_id` is resolved to
//! a `(spec_version, spec_json)` pair via the operational store before the domain store
//! materializes the session.

use std::path::PathBuf;

use super::{
    Command, LaunchTemplate, LaunchTemplateId, NewCommand, NewLaunchTemplate, NewProject,
    NewSession, NewSurface, NewWorkspace, NotificationRecord, OperationalStore, Project, ProjectId,
    Session, SessionId, SettingEntry, SettingScope, Store, Surface, SurfaceId, Workspace,
    WorkspaceId,
};
use crate::error::{OrchestratorError, Result};
use crate::infra::fs::{DomainStore, FsBackend};
use crate::infra::sqlite::SqliteBackend;

/// The full `Store` facade composed of a file-tree domain plane and a SQLite
/// operational plane.
pub struct CompositeStore {
    domain: FsBackend,
    op: SqliteBackend,
}

impl CompositeStore {
    /// Open both planes: the domain tree rooted at `data_root` and the operational
    /// SQLite database at `store_path`.
    pub fn open(data_root: PathBuf, store_path: PathBuf) -> Result<Self> {
        let domain = FsBackend::open(data_root)?;
        let op = SqliteBackend::open(&store_path)?;
        Ok(Self { domain, op })
    }
}

impl Store for CompositeStore {
    fn schema_version(&self) -> Result<u32> {
        self.op.schema_version()
    }

    // ── project ───────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        self.domain.get_project(id)
    }

    fn create_project(&self, draft: NewProject) -> Result<Project> {
        self.domain.create_project(draft)
    }

    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
        self.domain.rename_project(id, name)
    }

    fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>> {
        self.domain.list_projects(workspace_id)
    }

    fn move_project(&self, project_id: &ProjectId, workspace_id: &WorkspaceId) -> Result<()> {
        self.domain.move_project(project_id, workspace_id)
    }

    fn archive_project(&self, id: &ProjectId) -> Result<()> {
        self.domain.archive_project(id)
    }

    fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
        self.domain.hard_delete_project(id)
    }

    fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()> {
        self.domain.reorder_project(id, sort_order)
    }

    // ── workspace ─────────────────────────────────────────────────────────

    fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace> {
        self.domain.create_workspace(draft)
    }

    fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()> {
        self.domain.rename_workspace(id, name)
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        self.domain.list_workspaces()
    }

    fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()> {
        self.domain.reorder_workspace(id, sort_order)
    }

    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        self.domain.delete_workspace(id)
    }

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession) -> Result<Session> {
        // Cross-plane: resolve the operational launch template into a concrete spec
        // pair, then hand the resolved spec to the domain store.
        let spec = match draft.template_id {
            Some(ref tid) => {
                let tmpl = self.op.get_launch_template(tid)?.ok_or_else(|| {
                    OrchestratorError::LaunchTemplateNotFound(tid.as_str().to_string())
                })?;
                let instantiated = crate::launch::spec::instantiate_for_session(&tmpl.spec_json)?;
                Some((tmpl.spec_version, instantiated))
            }
            None => None,
        };
        self.domain.create_session(draft, spec)
    }

    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
        self.domain.rename_session(id, title)
    }

    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
        self.domain.list_sessions(project_id)
    }

    fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        self.domain.get_session(id)
    }

    fn archive_session(&self, id: &SessionId) -> Result<()> {
        self.domain.archive_session(id)
    }

    fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
        self.domain.hard_delete_session(id)
    }

    fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()> {
        self.domain.reorder_session(id, sort_order)
    }

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        self.domain.create_surface(draft)
    }

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        self.domain.get_surface(id)
    }

    fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>> {
        self.domain
            .find_session_surface_by_placement(session_id, placement)
    }

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        self.domain.list_resumable_surfaces()
    }

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        self.domain.update_surface_status(id, status)
    }

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        self.domain.soft_delete_surface(id)
    }

    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()> {
        self.domain.add_surface_to_session(session_id, surface_id)
    }

    fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        self.domain
            .remove_surface_from_session(session_id, surface_id)
    }

    // ── layout ────────────────────────────────────────────────────────────

    fn set_session_spec(&self, id: &SessionId, spec_version: u32, spec_json: &str) -> Result<()> {
        self.domain.set_session_spec(id, spec_version, spec_json)
    }

    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        self.domain.set_session_layout(id, layout_json)
    }

    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        self.domain.get_session_layout(id)
    }

    // ── command library ───────────────────────────────────────────────────

    fn list_commands(&self) -> Result<Vec<Command>> {
        self.op.list_commands()
    }

    fn get_command(&self, id: &str) -> Result<Option<Command>> {
        self.op.get_command(id)
    }

    fn create_command(&self, draft: NewCommand) -> Result<Command> {
        self.op.create_command(draft)
    }

    fn delete_command(&self, id: &str) -> Result<()> {
        self.op.delete_command(id)
    }

    fn seed_commands(&self) -> Result<()> {
        self.op.seed_commands()
    }

    // ── launch template ───────────────────────────────────────────────────

    fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate> {
        self.op.create_launch_template(draft)
    }

    fn get_launch_template(&self, id: &LaunchTemplateId) -> Result<Option<LaunchTemplate>> {
        self.op.get_launch_template(id)
    }

    fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        self.op
            .set_launch_template_spec(id, spec_version, spec_json)
    }

    // ── settings ──────────────────────────────────────────────────────────

    fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>> {
        self.op.get_setting(scope, key)
    }

    fn set_setting(&self, scope: &SettingScope, key: &str, value_json: &str) -> Result<()> {
        self.op.set_setting(scope, key, value_json)
    }

    fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>> {
        self.op.list_settings(scope)
    }

    fn resolve_setting(&self, project_id: &ProjectId, key: &str) -> Result<Option<String>> {
        self.op.resolve_setting(project_id, key)
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    fn insert_notification(&self, rec: &NotificationRecord) -> Result<()> {
        self.op.insert_notification(rec)
    }

    fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        self.op.list_notifications(limit)
    }

    fn prune_notifications(&self, keep: u32) -> Result<()> {
        self.op.prune_notifications(keep)
    }
}
