pub mod composite;
pub mod memory;
pub mod schema;
pub mod sqlite;
pub mod tree;

pub use composite::CompositeStore;
pub use schema::current_version as current_schema_version;
pub use sqlite::SqliteStore;

pub use crate::entities::*;

use crate::error::Result;

pub trait Store: Send + Sync {
    fn schema_version(&self) -> Result<u32>;

    // ── project ──────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>>;

    /// Create a project; infers name from source when `draft.name` is `None`.
    fn create_project(&self, draft: NewProject) -> Result<Project>;

    /// Rename a project. Returns `ProjectNotFound` for unknown id,
    /// `ProjectIsUnfiled` for the built-in Unfiled project.
    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()>;

    /// Return non-archived projects, optionally scoped to a single workspace. Ordered by
    /// `sort_order` ascending, falling back to `created_at` descending.
    fn list_projects(&self, workspace_id: Option<&WorkspaceId>) -> Result<Vec<Project>>;

    /// Move a project to a different workspace. Returns `ProjectNotFound` for an unknown
    /// project, `WorkspaceNotFound` for an unknown target workspace.
    fn move_project(&self, project_id: &ProjectId, workspace_id: &WorkspaceId) -> Result<()>;

    /// Soft-delete a project and cascade to its sessions and surfaces atomically.
    /// Returns `ProjectIsUnfiled` for the built-in project, `ProjectNotFound` if missing.
    fn archive_project(&self, id: &ProjectId) -> Result<()>;

    /// Permanently remove an already-archived project and all descendant rows.
    /// Returns `ProjectNotArchived` if the project is not archived.
    fn hard_delete_project(&self, id: &ProjectId) -> Result<()>;

    /// Reorder a project to a new sort position.
    /// Returns `ProjectNotFound` if the project does not exist.
    fn reorder_project(&self, id: &ProjectId, sort_order: u32) -> Result<()>;

    // ── workspace ─────────────────────────────────────────────────────────

    /// Create a workspace, ordered last. An empty name is allowed and stored as-is.
    fn create_workspace(&self, draft: NewWorkspace) -> Result<Workspace>;

    /// Rename a workspace. Returns `WorkspaceNotFound` for an unknown id.
    fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> Result<()>;

    /// Return all workspaces ordered by `sort_order` ascending, falling back to creation
    /// time. The Default workspace is always present.
    fn list_workspaces(&self) -> Result<Vec<Workspace>>;

    /// Reorder a workspace to a new sort position.
    /// Returns `WorkspaceNotFound` if the workspace does not exist.
    fn reorder_workspace(&self, id: &WorkspaceId, sort_order: u32) -> Result<()>;

    /// Delete a non-Default workspace, reassigning its projects to the Default workspace.
    /// Returns `WorkspaceIsDefault` for the Default workspace, `WorkspaceNotFound` if missing.
    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()>;

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession) -> Result<Session>;

    /// Rename a session and set `title_source` to `Custom`.
    /// Returns `SessionNotFound` for unknown id.
    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()>;

    /// Return non-archived sessions. Pass `Some(project_id)` to filter by project.
    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>>;

    /// Get a single session by id (non-archived).
    fn get_session(&self, id: &SessionId) -> Result<Option<Session>>;

    /// Soft-delete a session and cascade to its surfaces atomically.
    fn archive_session(&self, id: &SessionId) -> Result<()>;

    /// Permanently remove an already-archived session and its surface rows.
    fn hard_delete_session(&self, id: &SessionId) -> Result<()>;

    /// Reorder a session to a new sort position.
    /// Returns `SessionNotFound` if the session does not exist.
    fn reorder_session(&self, id: &SessionId, sort_order: u32) -> Result<()>;

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface>;

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>>;

    /// The session's live surface at `placement`, if any (the one to re-attach to on revisit).
    fn find_session_surface_by_placement(
        &self,
        session_id: &SessionId,
        placement: &str,
    ) -> Result<Option<Surface>>;

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>>;

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()>;

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()>;

    /// Associate a surface with a session.
    /// Returns `SurfaceConflict` if the surface already belongs to a different session.
    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()>;

    /// Soft-delete a surface without terminating its PTY.
    fn remove_surface_from_session(
        &self,
        session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()>;

    // ── layout ────────────────────────────────────────────────────────────

    /// Replace a session's launch spec blob and version (used when spawn/close diverge the spec).
    /// Returns `SessionNotFound` if the session does not exist.
    fn set_session_spec(&self, id: &SessionId, spec_version: u32, spec_json: &str) -> Result<()>;

    /// Persist the layout JSON blob for a session.
    /// Returns `SessionNotFound` if the session does not exist.
    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()>;

    /// Return the stored layout JSON blob, or `None` if not yet set.
    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>>;

    // ── command library ───────────────────────────────────────────────────

    /// Return all non-deleted commands.
    fn list_commands(&self) -> Result<Vec<Command>>;

    /// Return a single command by id, or `None` if not found.
    fn get_command(&self, id: &str) -> Result<Option<Command>>;

    /// Persist a new command and return it.
    fn create_command(&self, draft: NewCommand) -> Result<Command>;

    /// Soft-delete a command by id.
    fn delete_command(&self, id: &str) -> Result<()>;

    /// Insert prebuilt seed entries if they are absent (idempotent).
    fn seed_commands(&self) -> Result<()>;

    // ── launch template ───────────────────────────────────────────────────

    /// Persist a new launch template.
    fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate>;

    /// Return a template by id.
    fn get_launch_template(&self, id: &LaunchTemplateId) -> Result<Option<LaunchTemplate>>;

    /// Update the spec blob and version for an existing template.
    fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()>;

    // ── settings ──────────────────────────────────────────────────────────

    /// Read a setting's JSON value for an exact scope, or `None` if unset.
    fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>>;

    /// Insert or replace a setting's JSON value for a scope.
    fn set_setting(&self, scope: &SettingScope, key: &str, value_json: &str) -> Result<()>;

    /// All settings stored under a scope.
    fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>>;

    /// Resolve a key for a project: the project-scoped value if present, else the
    /// global value, else `None`.
    fn resolve_setting(&self, project_id: &ProjectId, key: &str) -> Result<Option<String>> {
        if let Some(v) = self.get_setting(&SettingScope::Project(project_id.clone()), key)? {
            return Ok(Some(v));
        }
        self.get_setting(&SettingScope::Global, key)
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    /// Append a notification to the durable history.
    fn insert_notification(&self, rec: &NotificationRecord) -> Result<()>;

    /// The most recent `limit` notifications, newest first.
    fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>>;

    /// Retain only the most recent `keep` notifications, discarding older ones.
    fn prune_notifications(&self, keep: u32) -> Result<()>;
}

/// Operational persistence contract: meta, command, setting, notification, launch_template.
///
/// Implemented by `SqliteStore`. `CompositeStore` delegates these methods to it.
pub trait OperationalStore: Send + Sync {
    fn schema_version(&self) -> Result<u32>;
    fn list_commands(&self) -> Result<Vec<Command>>;
    fn get_command(&self, id: &str) -> Result<Option<Command>>;
    fn create_command(&self, draft: NewCommand) -> Result<Command>;
    fn delete_command(&self, id: &str) -> Result<()>;
    fn seed_commands(&self) -> Result<()>;
    fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate>;
    fn get_launch_template(&self, id: &LaunchTemplateId) -> Result<Option<LaunchTemplate>>;
    fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()>;
    fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>>;
    fn set_setting(&self, scope: &SettingScope, key: &str, value_json: &str) -> Result<()>;
    fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>>;
    fn resolve_setting(&self, project_id: &ProjectId, key: &str) -> Result<Option<String>>;
    fn insert_notification(&self, rec: &NotificationRecord) -> Result<()>;
    fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>>;
    fn prune_notifications(&self, keep: u32) -> Result<()>;
}
