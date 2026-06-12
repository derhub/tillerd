use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};

use super::schema;
use super::{
    Command, CommandId, CommandOrigin, LaunchTemplate, LaunchTemplateId, NewCommand,
    NewLaunchTemplate, NewProject, NewSession, NewSurface, NewWorktree, Project, ProjectId,
    Session, SessionId, SourceKind, Store, Surface, SurfaceId, SurfaceKind, TitleSource, Worktree,
    WorktreeId,
};
use crate::error::{OrchestratorError, Result};

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn default_path() -> PathBuf {
        tillerd_paths::store()
    }

    pub fn open_default() -> Result<Self> {
        Self::open(&Self::default_path())
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with(path, &schema::migrations())
    }

    fn open_with(path: &Path, migrations: &[String]) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OrchestratorError::Persistence(e.to_string()))?;
        }
        let conn = Connection::open(path).map_err(persist)?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(persist)?;
        // Wait on a contended write lock instead of erroring immediately, so concurrent opens
        // (each seeding via INSERT OR IGNORE) serialize cleanly rather than hit SQLITE_BUSY.
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(persist)?;
        run_migrations(&conn, migrations)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        // Only seed if the command table exists (guards against synthetic migration sets in tests).
        let has_command: i64 = store
            .lock()?
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='command'",
                [],
                |r| r.get(0),
            )
            .map_err(persist)?;
        if has_command > 0 {
            store.seed_commands()?;
        }
        Ok(store)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| OrchestratorError::Persistence("store mutex poisoned".to_string()))
    }
}

fn persist(e: rusqlite::Error) -> OrchestratorError {
    OrchestratorError::Persistence(e.to_string())
}

fn read_schema_version(conn: &Connection) -> Result<u32> {
    let has_meta: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
            [],
            |row| row.get(0),
        )
        .map_err(persist)?;
    if has_meta == 0 {
        return Ok(0);
    }
    let recorded: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(persist)?;
    Ok(recorded.and_then(|v| v.parse().ok()).unwrap_or(0))
}

fn run_migrations(conn: &Connection, migrations: &[String]) -> Result<u32> {
    let supported = migrations.len() as u32;
    let current = read_schema_version(conn)?;
    if current > supported {
        return Err(OrchestratorError::StoreVersionTooNew {
            found: current,
            supported,
        });
    }
    for version in current..supported {
        let tx = conn.unchecked_transaction().map_err(persist)?;
        tx.execute_batch(&migrations[version as usize])
            .map_err(persist)?;
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
            params![(version + 1).to_string()],
        )
        .map_err(persist)?;
        tx.commit().map_err(persist)?;
    }
    Ok(supported)
}

fn parse_source_kind(s: &str) -> SourceKind {
    match s {
        "local_dir" => SourceKind::LocalDir,
        "git_repo" => SourceKind::GitRepo,
        "git_worktree" => SourceKind::GitWorktree,
        _ => SourceKind::Blank,
    }
}

fn parse_surface_kind(s: &str) -> SurfaceKind {
    match s {
        "diff" => SurfaceKind::Diff,
        _ => SurfaceKind::Terminal,
    }
}

fn parse_title_source(s: &str) -> TitleSource {
    match s {
        "branch" => TitleSource::Branch,
        "both" => TitleSource::Both,
        "custom" => TitleSource::Custom,
        _ => TitleSource::AgentTitle,
    }
}

/// Infer a project name from source kind and root path.
/// Returns `None` when inference is not possible (blank source, or no path).
fn infer_project_name(source: SourceKind, root_path: Option<&str>) -> Option<String> {
    match source {
        SourceKind::Blank => None,
        SourceKind::LocalDir | SourceKind::GitWorktree => root_path.and_then(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        }),
        SourceKind::GitRepo => root_path.and_then(|p| infer_git_repo_name(std::path::Path::new(p))),
    }
}

/// Read `origin` remote URL from `.git/config` and extract the repo name.
fn infer_git_repo_name(root: &std::path::Path) -> Option<String> {
    let config_path = root.join(".git").join("config");
    let content = std::fs::read_to_string(config_path).ok()?;
    // Look for `url = ...` under `[remote "origin"]`
    let mut in_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == r#"[remote "origin"]"# {
            in_origin = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_origin = false;
        }
        if in_origin {
            if let Some(rest) = trimmed.strip_prefix("url") {
                let url = rest.trim_start_matches([' ', '=']).trim();
                // strip trailing .git suffix
                let base = url.trim_end_matches(".git");
                return base.rsplit('/').next().map(|s| s.to_string());
            }
        }
    }
    // Fallback: directory basename
    root.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

fn query_sessions(conn: &Connection, project_id: Option<&str>) -> Result<Vec<Session>> {
    let mut out = Vec::new();
    if let Some(pid) = project_id {
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, created_at, spec_version, spec_json
                 FROM session
                 WHERE deleted_at IS NULL AND project_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(persist)?;
        let rows = stmt
            .query_map(params![pid], row_to_session)
            .map_err(persist)?;
        for r in rows {
            out.push(r.map_err(persist)?);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, title, title_source, created_at, spec_version, spec_json
                 FROM session
                 WHERE deleted_at IS NULL
                 ORDER BY created_at DESC",
            )
            .map_err(persist)?;
        let rows = stmt.query_map([], row_to_session).map_err(persist)?;
        for r in rows {
            out.push(r.map_err(persist)?);
        }
    }
    Ok(out)
}

impl Store for SqliteStore {
    fn schema_version(&self) -> Result<u32> {
        let conn = self.lock()?;
        read_schema_version(&conn)
    }

    // ── project ───────────────────────────────────────────────────────────

    fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        self.lock()?
            .query_row(
                "SELECT id, name, source_kind, root_path
                 FROM project
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
                |row| {
                    let id: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let source_kind: String = row.get(2)?;
                    let root_path: Option<String> = row.get(3)?;
                    Ok(Project {
                        id: ProjectId::new(id),
                        name,
                        source_kind: parse_source_kind(&source_kind),
                        root_path,
                    })
                },
            )
            .optional()
            .map_err(persist)
    }

    fn create_project(&self, draft: NewProject) -> Result<Project> {
        let id = ProjectId::new(uuid::Uuid::new_v4().to_string());
        let name = draft
            .name
            .or_else(|| infer_project_name(draft.source_kind, draft.root_path.as_deref()))
            .unwrap_or_default();
        self.lock()?
            .execute(
                "INSERT INTO project (id, name, source_kind, root_path)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.as_str(),
                    &name,
                    draft.source_kind.as_str(),
                    &draft.root_path
                ],
            )
            .map_err(persist)?;
        Ok(Project {
            id,
            name,
            source_kind: draft.source_kind,
            root_path: draft.root_path,
        })
    }

    fn rename_project(&self, id: &ProjectId, name: &str) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let rows = self
            .lock()?
            .execute(
                "UPDATE project SET name = ?1, updated_at = datetime('now')
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![name, id.as_str()],
            )
            .map_err(persist)?;
        if rows == 0 {
            return Err(OrchestratorError::ProjectNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, source_kind, root_path
                 FROM project
                 WHERE deleted_at IS NULL
                 ORDER BY created_at DESC",
            )
            .map_err(persist)?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let source_kind: String = row.get(2)?;
                let root_path: Option<String> = row.get(3)?;
                Ok(Project {
                    id: ProjectId::new(id),
                    name,
                    source_kind: parse_source_kind(&source_kind),
                    root_path,
                })
            })
            .map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    fn archive_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction().map_err(persist)?;
        // cascade surfaces → sessions → project
        tx.execute(
            "UPDATE surface SET deleted_at = datetime('now')
             WHERE deleted_at IS NULL
               AND session_id IN (
                   SELECT id FROM session WHERE project_id = ?1 AND deleted_at IS NULL
               )",
            params![id.as_str()],
        )
        .map_err(persist)?;
        tx.execute(
            "UPDATE session SET deleted_at = datetime('now')
             WHERE project_id = ?1 AND deleted_at IS NULL",
            params![id.as_str()],
        )
        .map_err(persist)?;
        let rows = tx
            .execute(
                "UPDATE project SET deleted_at = datetime('now')
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
            )
            .map_err(persist)?;
        tx.commit().map_err(persist)?;
        if rows == 0 {
            return Err(OrchestratorError::ProjectNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    fn hard_delete_project(&self, id: &ProjectId) -> Result<()> {
        if id.is_unfiled() {
            return Err(OrchestratorError::ProjectIsUnfiled);
        }
        let conn = self.lock()?;
        // Check that project is archived
        let archived: Option<i64> = conn
            .query_row(
                "SELECT count(*) FROM project WHERE id = ?1 AND deleted_at IS NOT NULL",
                params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(persist)?;
        if archived.unwrap_or(0) == 0 {
            // Either not found or not archived
            let exists: i64 = conn
                .query_row(
                    "SELECT count(*) FROM project WHERE id = ?1",
                    params![id.as_str()],
                    |r| r.get(0),
                )
                .map_err(persist)?;
            return if exists == 0 {
                Err(OrchestratorError::ProjectNotFound(id.as_str().to_string()))
            } else {
                Err(OrchestratorError::ProjectNotArchived)
            };
        }
        let tx = conn.unchecked_transaction().map_err(persist)?;
        tx.execute(
            "DELETE FROM surface WHERE session_id IN (SELECT id FROM session WHERE project_id = ?1)",
            params![id.as_str()],
        )
        .map_err(persist)?;
        tx.execute(
            "DELETE FROM session WHERE project_id = ?1",
            params![id.as_str()],
        )
        .map_err(persist)?;
        tx.execute("DELETE FROM project WHERE id = ?1", params![id.as_str()])
            .map_err(persist)?;
        tx.commit().map_err(persist)?;
        Ok(())
    }

    // ── session ───────────────────────────────────────────────────────────

    fn create_session(&self, draft: NewSession) -> Result<Session> {
        let id = SessionId::mint();
        let project_id = draft.project_id.unwrap_or_else(ProjectId::unfiled);
        let title = draft.title.unwrap_or_default();
        let conn = self.lock()?;

        // Resolve template spec if provided — done inside a transaction for atomicity.
        let (spec_version, spec_json) = if let Some(ref tid) = draft.template_id {
            let row: Option<(u32, String)> = conn
                .query_row(
                    "SELECT spec_version, spec_json FROM launch_template WHERE id = ?1",
                    params![tid.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(persist)?;
            match row {
                Some((v, j)) => (Some(v), Some(j)),
                None => {
                    return Err(OrchestratorError::LaunchTemplateNotFound(
                        tid.as_str().to_string(),
                    ))
                }
            }
        } else {
            (None, None)
        };

        conn.execute(
            "INSERT INTO session (id, project_id, title, title_source, spec_version, spec_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.as_str(),
                project_id.as_str(),
                &title,
                draft.title_source.as_str(),
                spec_version,
                spec_json.as_deref()
            ],
        )
        .map_err(persist)?;
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM session WHERE id = ?1",
                params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(persist)?;
        Ok(Session {
            id,
            project_id,
            title,
            title_source: draft.title_source,
            created_at,
            spec_version,
            spec_json,
        })
    }

    fn rename_session(&self, id: &SessionId, title: &str) -> Result<()> {
        let rows = self
            .lock()?
            .execute(
                "UPDATE session SET title = ?1, title_source = 'custom',
                                    updated_at = datetime('now')
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![title, id.as_str()],
            )
            .map_err(persist)?;
        if rows == 0 {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    fn list_sessions(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>> {
        let pid_str = project_id.map(|p| p.as_str().to_string());
        let conn = self.lock()?;
        query_sessions(&conn, pid_str.as_deref())
    }

    fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        self.lock()?
            .query_row(
                "SELECT id, project_id, title, title_source, created_at, spec_version, spec_json
                 FROM session
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
                row_to_session,
            )
            .optional()
            .map_err(persist)
    }

    fn archive_session(&self, id: &SessionId) -> Result<()> {
        let conn = self.lock()?;
        let tx = conn.unchecked_transaction().map_err(persist)?;
        tx.execute(
            "UPDATE surface SET deleted_at = datetime('now')
             WHERE session_id = ?1 AND deleted_at IS NULL",
            params![id.as_str()],
        )
        .map_err(persist)?;
        let rows = tx
            .execute(
                "UPDATE session SET deleted_at = datetime('now')
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
            )
            .map_err(persist)?;
        tx.commit().map_err(persist)?;
        if rows == 0 {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    fn hard_delete_session(&self, id: &SessionId) -> Result<()> {
        let conn = self.lock()?;
        let archived: i64 = conn
            .query_row(
                "SELECT count(*) FROM session WHERE id = ?1 AND deleted_at IS NOT NULL",
                params![id.as_str()],
                |r| r.get(0),
            )
            .map_err(persist)?;
        if archived == 0 {
            let exists: i64 = conn
                .query_row(
                    "SELECT count(*) FROM session WHERE id = ?1",
                    params![id.as_str()],
                    |r| r.get(0),
                )
                .map_err(persist)?;
            return if exists == 0 {
                Err(OrchestratorError::SessionNotFound(id.as_str().to_string()))
            } else {
                Err(OrchestratorError::SessionNotArchived)
            };
        }
        let tx = conn.unchecked_transaction().map_err(persist)?;
        tx.execute(
            "DELETE FROM surface WHERE session_id = ?1",
            params![id.as_str()],
        )
        .map_err(persist)?;
        tx.execute("DELETE FROM session WHERE id = ?1", params![id.as_str()])
            .map_err(persist)?;
        tx.commit().map_err(persist)?;
        Ok(())
    }

    // ── surface ───────────────────────────────────────────────────────────

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let id = draft.id.clone().unwrap_or_else(SurfaceId::mint);
        let worktree_id_str = draft.worktree_id.as_ref().map(|w| w.as_str().to_string());
        self.lock()?
            .execute(
                "INSERT INTO surface (id, session_id, kind, cwd, placement, worktree_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id.as_str(),
                    draft.session_id.as_str(),
                    draft.kind.as_str(),
                    draft.cwd,
                    draft.placement,
                    worktree_id_str
                ],
            )
            .map_err(persist)?;
        Ok(Surface {
            id,
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
            last_status: None,
            placement: draft.placement,
            worktree_id: draft.worktree_id,
        })
    }

    fn get_surface(&self, id: &SurfaceId) -> Result<Option<Surface>> {
        self.lock()?
            .query_row(
                "SELECT id, session_id, kind, cwd, last_status, placement, worktree_id
                 FROM surface
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
                row_to_surface,
            )
            .optional()
            .map_err(persist)
    }

    fn list_resumable_surfaces(&self) -> Result<Vec<Surface>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.session_id, s.kind, s.cwd, s.last_status, s.placement, s.worktree_id
                 FROM surface s
                 JOIN session ses ON s.session_id = ses.id
                 WHERE s.deleted_at IS NULL AND ses.deleted_at IS NULL",
            )
            .map_err(persist)?;
        let rows = stmt.query_map([], row_to_surface).map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    fn update_surface_status(&self, id: &SurfaceId, status: &str) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE surface SET last_status = ?1 WHERE id = ?2",
                params![status, id.as_str()],
            )
            .map_err(persist)?;
        Ok(())
    }

    fn soft_delete_surface(&self, id: &SurfaceId) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE surface SET deleted_at = datetime('now') WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(persist)?;
        Ok(())
    }

    fn add_surface_to_session(&self, session_id: &SessionId, surface_id: &SurfaceId) -> Result<()> {
        let conn = self.lock()?;
        // Check if surface already has a session
        let current_session: Option<String> = conn
            .query_row(
                "SELECT session_id FROM surface WHERE id = ?1 AND deleted_at IS NULL",
                params![surface_id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(persist)?
            .flatten();
        if let Some(cur) = current_session {
            if cur != session_id.as_str() {
                return Err(OrchestratorError::SurfaceConflict(
                    surface_id.as_str().to_string(),
                ));
            }
            // already associated with same session — idempotent
            return Ok(());
        }
        conn.execute(
            "UPDATE surface SET session_id = ?1 WHERE id = ?2",
            params![session_id.as_str(), surface_id.as_str()],
        )
        .map_err(persist)?;
        Ok(())
    }

    fn remove_surface_from_session(
        &self,
        _session_id: &SessionId,
        surface_id: &SurfaceId,
    ) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE surface SET deleted_at = datetime('now')
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![surface_id.as_str()],
            )
            .map_err(persist)?;
        Ok(())
    }

    // ── layout ────────────────────────────────────────────────────────────

    fn set_session_layout(&self, id: &SessionId, layout_json: &str) -> Result<()> {
        let rows = self
            .lock()?
            .execute(
                "UPDATE session SET layout_json = ?1, updated_at = datetime('now')
                 WHERE id = ?2 AND deleted_at IS NULL",
                params![layout_json, id.as_str()],
            )
            .map_err(persist)?;
        if rows == 0 {
            return Err(OrchestratorError::SessionNotFound(id.as_str().to_string()));
        }
        Ok(())
    }

    fn get_session_layout(&self, id: &SessionId) -> Result<Option<String>> {
        let layout: Option<Option<String>> = self
            .lock()?
            .query_row(
                "SELECT layout_json FROM session WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(persist)?;
        match layout {
            None => Err(OrchestratorError::SessionNotFound(id.as_str().to_string())),
            Some(blob) => Ok(blob),
        }
    }

    // ── command library ───────────────────────────────────────────────────

    fn list_commands(&self) -> Result<Vec<Command>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, origin, cli, args_json, env_json
                 FROM command
                 WHERE deleted_at IS NULL",
            )
            .map_err(persist)?;
        let rows = stmt.query_map([], row_to_command).map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    fn get_command(&self, id: &str) -> Result<Option<Command>> {
        self.lock()?
            .query_row(
                "SELECT id, name, origin, cli, args_json, env_json
                 FROM command
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                row_to_command,
            )
            .optional()
            .map_err(persist)
    }

    fn create_command(&self, draft: NewCommand) -> Result<Command> {
        let id = CommandId::mint();
        let args_json = serde_json::to_string(&draft.args).unwrap_or_default();
        let env_json = serde_json::to_string(&draft.env).unwrap_or_default();
        self.lock()?
            .execute(
                "INSERT INTO command (id, name, origin, cli, args_json, env_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id.as_str(),
                    &draft.name,
                    draft.origin.as_str(),
                    &draft.cli,
                    &args_json,
                    &env_json
                ],
            )
            .map_err(persist)?;
        Ok(Command {
            id,
            name: draft.name,
            origin: draft.origin,
            cli: draft.cli,
            args: draft.args,
            env: draft.env,
        })
    }

    fn delete_command(&self, id: &str) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE command SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
            )
            .map_err(persist)?;
        Ok(())
    }

    fn seed_commands(&self) -> Result<()> {
        // One lock + INSERT OR IGNORE per prebuilt: race-free under concurrent open (no
        // exists-check / lock-release / re-insert window).
        let conn = self.lock()?;
        for cmd in prebuilt_commands() {
            let args_json = serde_json::to_string(&cmd.args).unwrap_or_default();
            let env_json = serde_json::to_string(&cmd.env).unwrap_or_default();
            conn.execute(
                "INSERT OR IGNORE INTO command (id, name, origin, cli, args_json, env_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    cmd.id.as_str(),
                    &cmd.name,
                    cmd.origin.as_str(),
                    &cmd.cli,
                    &args_json,
                    &env_json
                ],
            )
            .map_err(persist)?;
        }
        Ok(())
    }

    // ── worktree ──────────────────────────────────────────────────────────

    fn create_worktree(&self, draft: NewWorktree) -> Result<Worktree> {
        let id = WorktreeId::mint();
        self.lock()?
            .execute(
                "INSERT INTO worktree (id, project_id, path, branch)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.as_str(),
                    draft.project_id.as_str(),
                    &draft.path,
                    draft.branch.as_deref()
                ],
            )
            .map_err(persist)?;
        Ok(Worktree {
            id,
            project_id: draft.project_id,
            path: draft.path,
            branch: draft.branch,
        })
    }

    fn list_worktrees(&self, project_id: &ProjectId) -> Result<Vec<Worktree>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, path, branch
                 FROM worktree
                 WHERE project_id = ?1 AND deleted_at IS NULL",
            )
            .map_err(persist)?;
        let rows = stmt
            .query_map(params![project_id.as_str()], row_to_worktree)
            .map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    fn archive_worktree(&self, id: &WorktreeId) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE worktree SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
                params![id.as_str()],
            )
            .map_err(persist)?;
        Ok(())
    }

    // ── launch template ───────────────────────────────────────────────────

    fn create_launch_template(&self, draft: NewLaunchTemplate) -> Result<LaunchTemplate> {
        let id = LaunchTemplateId::mint();
        self.lock()?
            .execute(
                "INSERT INTO launch_template (id, project_id, spec_version, spec_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.as_str(),
                    draft.project_id.as_str(),
                    draft.spec_version,
                    &draft.spec_json
                ],
            )
            .map_err(persist)?;
        Ok(LaunchTemplate {
            id,
            project_id: draft.project_id,
            spec_version: draft.spec_version,
            spec_json: draft.spec_json,
        })
    }

    fn get_launch_template(&self, id: &LaunchTemplateId) -> Result<Option<LaunchTemplate>> {
        self.lock()?
            .query_row(
                "SELECT id, project_id, spec_version, spec_json
                 FROM launch_template
                 WHERE id = ?1",
                params![id.as_str()],
                row_to_launch_template,
            )
            .optional()
            .map_err(persist)
    }

    fn set_launch_template_spec(
        &self,
        id: &LaunchTemplateId,
        spec_version: u32,
        spec_json: &str,
    ) -> Result<()> {
        let affected = self
            .lock()?
            .execute(
                "UPDATE launch_template SET spec_version = ?1, spec_json = ?2,
                                           updated_at = datetime('now')
                 WHERE id = ?3",
                params![spec_version, spec_json, id.as_str()],
            )
            .map_err(persist)?;
        if affected == 0 {
            return Err(OrchestratorError::LaunchTemplateNotFound(
                id.as_str().to_string(),
            ));
        }
        Ok(())
    }
}

fn row_to_surface(row: &rusqlite::Row<'_>) -> rusqlite::Result<Surface> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let cwd: Option<String> = row.get(3)?;
    let last_status: Option<String> = row.get(4)?;
    let placement: Option<String> = row.get(5)?;
    let worktree_id: Option<String> = row.get(6)?;
    Ok(Surface {
        id: SurfaceId::from_string(id),
        session_id: SessionId::from_string(session_id),
        kind: parse_surface_kind(&kind),
        cwd,
        last_status,
        placement,
        worktree_id: worktree_id.map(WorktreeId::from_string),
    })
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let title: String = row.get(2)?;
    let title_source: String = row.get(3)?;
    let created_at: String = row.get(4)?;
    let spec_version: Option<u32> = row.get(5)?;
    let spec_json: Option<String> = row.get(6)?;
    Ok(Session {
        id: SessionId::from_string(id),
        project_id: ProjectId::new(project_id),
        title,
        title_source: parse_title_source(&title_source),
        created_at,
        spec_version,
        spec_json,
    })
}

fn row_to_command(row: &rusqlite::Row<'_>) -> rusqlite::Result<Command> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let origin_str: String = row.get(2)?;
    let cli: String = row.get(3)?;
    let args_json: Option<String> = row.get(4)?;
    let env_json: Option<String> = row.get(5)?;
    let origin = match origin_str.as_str() {
        "custom" => CommandOrigin::Custom,
        _ => CommandOrigin::Prebuilt,
    };
    let args: Vec<String> = args_json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    let env: std::collections::HashMap<String, String> = env_json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    Ok(Command {
        id: CommandId::from_string(id),
        name,
        origin,
        cli,
        args,
        env,
    })
}

fn row_to_worktree(row: &rusqlite::Row<'_>) -> rusqlite::Result<Worktree> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let path: String = row.get(2)?;
    let branch: Option<String> = row.get(3)?;
    Ok(Worktree {
        id: WorktreeId::from_string(id),
        project_id: ProjectId::new(project_id),
        path,
        branch,
    })
}

fn row_to_launch_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<LaunchTemplate> {
    let id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let spec_version: u32 = row.get(2)?;
    let spec_json: String = row.get(3)?;
    Ok(LaunchTemplate {
        id: LaunchTemplateId::from_string(id),
        project_id: ProjectId::new(project_id),
        spec_version,
        spec_json,
    })
}

/// Hard-coded prebuilt command seeds.
fn prebuilt_commands() -> Vec<Command> {
    vec![Command {
        id: CommandId::from_string("00000000-0000-0000-0000-000000000101"),
        name: "login-shell".to_string(),
        origin: CommandOrigin::Prebuilt,
        cli: "/bin/bash".to_string(),
        args: vec!["-l".to_string()],
        env: Default::default(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SurfaceKind;

    fn temp_db(tag: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{tag}.db"));
        (dir, path)
    }

    fn synthetic_migrations() -> Vec<String> {
        vec![
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE marker_one (x INTEGER);"
                .to_string(),
            "CREATE TABLE marker_two (y INTEGER);".to_string(),
        ]
    }

    fn table_exists(store: &SqliteStore, name: &str) -> bool {
        let conn = store.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![name],
                |r| r.get(0),
            )
            .unwrap();
        count == 1
    }

    fn make_session(store: &SqliteStore) -> Session {
        store.create_session(NewSession::default()).unwrap()
    }

    fn make_surface(store: &SqliteStore) -> Surface {
        let session = make_session(store);
        store
            .create_surface(NewSurface {
                id: None,
                session_id: session.id,
                kind: SurfaceKind::Terminal,
                cwd: Some("/tmp".to_string()),
                placement: None,
                worktree_id: None,
            })
            .unwrap()
    }

    // ── migration ─────────────────────────────────────────────────────────

    #[test]
    fn fresh_store_initializes_to_current_version_and_records_it() {
        let (_dir, path) = temp_db("fresh");
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), schema::current_version());
    }

    #[test]
    fn fresh_store_seeds_the_unfiled_project() {
        let (_dir, path) = temp_db("seed");
        let store = SqliteStore::open(&path).unwrap();
        let unfiled = store.get_project(&ProjectId::unfiled()).unwrap();
        assert_eq!(unfiled.map(|p| p.name), Some("Unfiled".to_string()));
    }

    #[test]
    fn migration_runner_applies_pending_migrations_in_order() {
        let (_dir, path) = temp_db("forward");
        let migrations = synthetic_migrations();

        let store = SqliteStore::open_with(&path, &migrations[..1]).unwrap();
        assert_eq!(store.schema_version().unwrap(), 1);
        assert!(table_exists(&store, "marker_one"));
        assert!(!table_exists(&store, "marker_two"));
        drop(store);

        let store = SqliteStore::open_with(&path, &migrations).unwrap();
        assert_eq!(store.schema_version().unwrap(), 2);
        assert!(table_exists(&store, "marker_two"));
    }

    #[test]
    fn store_newer_than_binary_is_refused() {
        let (_dir, path) = temp_db("newer");
        let migrations = synthetic_migrations();

        SqliteStore::open_with(&path, &migrations).unwrap();

        let result = SqliteStore::open_with(&path, &migrations[..1]);
        assert!(matches!(
            result,
            Err(OrchestratorError::StoreVersionTooNew {
                found: 2,
                supported: 1
            })
        ));
    }

    // ── session defaults ──────────────────────────────────────────────────

    #[test]
    fn session_without_a_project_resolves_to_unfiled() {
        let (_dir, path) = temp_db("unfiled-session");
        let store = SqliteStore::open(&path).unwrap();

        let session = store.create_session(NewSession::default()).unwrap();

        assert_eq!(session.project_id, ProjectId::unfiled());
    }

    // ── surface id model ──────────────────────────────────────────────────

    #[test]
    fn surface_correlation_id_is_the_shared_surface_id_not_the_session_id() {
        let (_dir, path) = temp_db("two-level-id");
        let store = SqliteStore::open(&path).unwrap();

        let session = store.create_session(NewSession::default()).unwrap();
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: session.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
                worktree_id: None,
            })
            .unwrap();

        assert_eq!(surface.correlation_id(), &surface.id);
        assert_ne!(surface.id.as_str(), session.id.as_str());
    }

    #[test]
    fn create_then_get_surface_round_trips_including_last_status_none() {
        let (_dir, path) = temp_db("get-surface");
        let store = SqliteStore::open(&path).unwrap();

        let created = make_surface(&store);
        let fetched = store.get_surface(&created.id).unwrap().unwrap();

        assert_eq!(fetched, created);
        assert!(fetched.last_status.is_none());
    }

    #[test]
    fn list_resumable_surfaces_includes_a_created_surface() {
        let (_dir, path) = temp_db("list-resumable");
        let store = SqliteStore::open(&path).unwrap();

        let created = make_surface(&store);
        let list = store.list_resumable_surfaces().unwrap();

        assert!(list.iter().any(|s| s.id == created.id));
    }

    #[test]
    fn soft_delete_excludes_surface_from_list_and_get() {
        let (_dir, path) = temp_db("soft-delete");
        let store = SqliteStore::open(&path).unwrap();

        let surface = make_surface(&store);
        store.soft_delete_surface(&surface.id).unwrap();

        assert!(store.get_surface(&surface.id).unwrap().is_none());
        let list = store.list_resumable_surfaces().unwrap();
        assert!(!list.iter().any(|s| s.id == surface.id));
    }

    #[test]
    fn update_surface_status_is_reflected_by_get_surface() {
        let (_dir, path) = temp_db("update-status");
        let store = SqliteStore::open(&path).unwrap();

        let surface = make_surface(&store);
        store.update_surface_status(&surface.id, "running").unwrap();

        let fetched = store.get_surface(&surface.id).unwrap().unwrap();
        assert_eq!(fetched.last_status.as_deref(), Some("running"));
    }

    // ── project CRUD ──────────────────────────────────────────────────────

    #[test]
    fn create_project_blank_uses_supplied_name() {
        let (_dir, path) = temp_db("proj-blank");
        let store = SqliteStore::open(&path).unwrap();

        let p = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("My Project".to_string()),
            })
            .unwrap();

        assert_eq!(p.name, "My Project");
        assert_eq!(p.source_kind, SourceKind::Blank);
    }

    #[test]
    fn create_project_local_dir_infers_basename() {
        let (_dir, path) = temp_db("proj-dir");
        let store = SqliteStore::open(&path).unwrap();

        let p = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: Some("/home/user/myapp".to_string()),
                name: None,
            })
            .unwrap();

        assert_eq!(p.name, "myapp");
    }

    #[test]
    fn create_project_custom_name_overrides_inference() {
        let (_dir, path) = temp_db("proj-override");
        let store = SqliteStore::open(&path).unwrap();

        let p = store
            .create_project(NewProject {
                source_kind: SourceKind::LocalDir,
                root_path: Some("/home/user/myapp".to_string()),
                name: Some("custom-name".to_string()),
            })
            .unwrap();

        assert_eq!(p.name, "custom-name");
    }

    #[test]
    fn rename_project_persists_and_is_reflected_in_list() {
        let (_dir, path) = temp_db("proj-rename");
        let store = SqliteStore::open(&path).unwrap();

        let p = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("old".to_string()),
            })
            .unwrap();
        store.rename_project(&p.id, "new").unwrap();

        let list = store.list_projects().unwrap();
        let found = list.iter().find(|x| x.id == p.id).unwrap();
        assert_eq!(found.name, "new");
    }

    #[test]
    fn rename_unknown_project_returns_not_found() {
        let (_dir, path) = temp_db("proj-rename-missing");
        let store = SqliteStore::open(&path).unwrap();

        let err = store
            .rename_project(&ProjectId::new("no-such-id"), "x")
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ProjectNotFound(_)));
    }

    #[test]
    fn rename_unfiled_project_is_rejected() {
        let (_dir, path) = temp_db("proj-rename-unfiled");
        let store = SqliteStore::open(&path).unwrap();

        let err = store
            .rename_project(&ProjectId::unfiled(), "x")
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::ProjectIsUnfiled));
    }

    #[test]
    fn list_projects_returns_only_active_projects_newest_first() {
        let (_dir, path) = temp_db("proj-list");
        let store = SqliteStore::open(&path).unwrap();

        let p1 = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("first".to_string()),
            })
            .unwrap();
        let p2 = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("second".to_string()),
            })
            .unwrap();
        store.archive_project(&p1.id).unwrap();

        let list = store.list_projects().unwrap();
        // only active projects; p1 is archived
        assert!(!list.iter().any(|p| p.id == p1.id));
        assert!(list.iter().any(|p| p.id == p2.id));
    }

    #[test]
    fn archive_project_cascades_to_sessions_and_surfaces() {
        let (_dir, path) = temp_db("proj-archive-cascade");
        let store = SqliteStore::open(&path).unwrap();

        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        let sess = store
            .create_session(NewSession {
                project_id: Some(proj.id.clone()),
                ..Default::default()
            })
            .unwrap();
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
                worktree_id: None,
            })
            .unwrap();

        store.archive_project(&proj.id).unwrap();

        // project gone from active list
        assert!(store.get_project(&proj.id).unwrap().is_none());
        // session gone
        assert!(store.get_session(&sess.id).unwrap().is_none());
        // surface gone
        assert!(store.get_surface(&surface.id).unwrap().is_none());
    }

    #[test]
    fn archive_unfiled_project_is_rejected() {
        let (_dir, path) = temp_db("proj-archive-unfiled");
        let store = SqliteStore::open(&path).unwrap();

        let err = store.archive_project(&ProjectId::unfiled()).unwrap_err();
        assert!(matches!(err, OrchestratorError::ProjectIsUnfiled));
    }

    #[test]
    fn hard_delete_project_removes_all_rows() {
        let (_dir, path) = temp_db("proj-hard-delete");
        let store = SqliteStore::open(&path).unwrap();

        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        store.archive_project(&proj.id).unwrap();
        store.hard_delete_project(&proj.id).unwrap();

        // project row is gone (even from archived query)
        let conn = store.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM project WHERE id = ?1",
                params![proj.id.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn hard_delete_active_project_is_rejected() {
        let (_dir, path) = temp_db("proj-hard-delete-active");
        let store = SqliteStore::open(&path).unwrap();

        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();

        let err = store.hard_delete_project(&proj.id).unwrap_err();
        assert!(matches!(err, OrchestratorError::ProjectNotArchived));
    }

    // ── session CRUD ──────────────────────────────────────────────────────

    #[test]
    fn create_session_with_custom_title_source() {
        let (_dir, path) = temp_db("sess-custom");
        let store = SqliteStore::open(&path).unwrap();

        let sess = store
            .create_session(NewSession {
                title_source: TitleSource::Custom,
                title: Some("My session".to_string()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(sess.title, "My session");
        assert_eq!(sess.title_source, TitleSource::Custom);
    }

    #[test]
    fn create_session_with_branch_strategy_stores_title() {
        let (_dir, path) = temp_db("sess-branch");
        let store = SqliteStore::open(&path).unwrap();

        let sess = store
            .create_session(NewSession {
                title_source: TitleSource::Branch,
                title: Some("feat/x".to_string()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(sess.title, "feat/x");
        assert_eq!(sess.title_source, TitleSource::Branch);
    }

    #[test]
    fn create_session_agent_title_stores_empty_title() {
        let (_dir, path) = temp_db("sess-agent-title");
        let store = SqliteStore::open(&path).unwrap();

        let sess = store
            .create_session(NewSession {
                title_source: TitleSource::AgentTitle,
                title: None,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(sess.title, "");
        assert_eq!(sess.title_source, TitleSource::AgentTitle);
    }

    #[test]
    fn rename_session_updates_title_and_sets_custom_source() {
        let (_dir, path) = temp_db("sess-rename");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        store.rename_session(&sess.id, "renamed").unwrap();

        let fetched = store.get_session(&sess.id).unwrap().unwrap();
        assert_eq!(fetched.title, "renamed");
        assert_eq!(fetched.title_source, TitleSource::Custom);
    }

    #[test]
    fn rename_unknown_session_returns_not_found() {
        let (_dir, path) = temp_db("sess-rename-missing");
        let store = SqliteStore::open(&path).unwrap();

        let err = store
            .rename_session(&SessionId::from_string("no-such"), "x")
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::SessionNotFound(_)));
    }

    #[test]
    fn list_sessions_returns_active_only_and_supports_project_filter() {
        let (_dir, path) = temp_db("sess-list");
        let store = SqliteStore::open(&path).unwrap();

        let proj = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        let s1 = store
            .create_session(NewSession {
                project_id: Some(proj.id.clone()),
                ..Default::default()
            })
            .unwrap();
        let s2 = store
            .create_session(NewSession {
                project_id: Some(proj.id.clone()),
                ..Default::default()
            })
            .unwrap();
        // archive s2
        store.archive_session(&s2.id).unwrap();

        let all = store.list_sessions(None).unwrap();
        assert!(all.iter().any(|s| s.id == s1.id));
        assert!(!all.iter().any(|s| s.id == s2.id));

        let filtered = store.list_sessions(Some(&proj.id)).unwrap();
        assert!(filtered.iter().any(|s| s.id == s1.id));
        assert!(!filtered.iter().any(|s| s.id == s2.id));
    }

    #[test]
    fn archive_session_cascades_surfaces() {
        let (_dir, path) = temp_db("sess-archive");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
                worktree_id: None,
            })
            .unwrap();

        store.archive_session(&sess.id).unwrap();

        assert!(store.get_session(&sess.id).unwrap().is_none());
        assert!(store.get_surface(&surface.id).unwrap().is_none());
        let resumable = store.list_resumable_surfaces().unwrap();
        assert!(!resumable.iter().any(|s| s.id == surface.id));
    }

    #[test]
    fn hard_delete_session_removes_session_and_surface_rows() {
        let (_dir, path) = temp_db("sess-hard-delete");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
                worktree_id: None,
            })
            .unwrap();
        store.archive_session(&sess.id).unwrap();
        store.hard_delete_session(&sess.id).unwrap();

        let conn = store.lock().unwrap();
        let sess_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM session WHERE id = ?1",
                params![sess.id.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let surf_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM surface WHERE id = ?1",
                params![surface.id.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sess_count, 0);
        assert_eq!(surf_count, 0);
    }

    #[test]
    fn hard_delete_active_session_is_rejected() {
        let (_dir, path) = temp_db("sess-hard-delete-active");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        let err = store.hard_delete_session(&sess.id).unwrap_err();
        assert!(matches!(err, OrchestratorError::SessionNotArchived));
    }

    // ── add/remove surface ────────────────────────────────────────────────

    #[test]
    fn add_surface_to_session_conflict_is_rejected() {
        let (_dir, path) = temp_db("surf-conflict");
        let store = SqliteStore::open(&path).unwrap();

        let s1 = make_session(&store);
        let s2 = make_session(&store);
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: s1.id,
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: None,
                worktree_id: None,
            })
            .unwrap();

        let err = store
            .add_surface_to_session(&s2.id, &surface.id)
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::SurfaceConflict(_)));
    }

    #[test]
    fn remove_surface_from_session_soft_deletes_row() {
        let (_dir, path) = temp_db("surf-remove");
        let store = SqliteStore::open(&path).unwrap();

        let surface = make_surface(&store);
        store
            .remove_surface_from_session(&surface.session_id, &surface.id)
            .unwrap();

        assert!(store.get_surface(&surface.id).unwrap().is_none());
    }

    // ── layout ────────────────────────────────────────────────────────────

    #[test]
    fn set_and_get_session_layout_round_trips() {
        let (_dir, path) = temp_db("layout-rt");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        store
            .set_session_layout(&sess.id, r#"{"type":"leaf"}"#)
            .unwrap();
        let blob = store.get_session_layout(&sess.id).unwrap();
        assert_eq!(blob.as_deref(), Some(r#"{"type":"leaf"}"#));
    }

    #[test]
    fn get_session_layout_returns_none_when_not_set() {
        let (_dir, path) = temp_db("layout-null");
        let store = SqliteStore::open(&path).unwrap();

        let sess = make_session(&store);
        let blob = store.get_session_layout(&sess.id).unwrap();
        assert!(blob.is_none());
    }

    #[test]
    fn set_layout_for_unknown_session_returns_not_found() {
        let (_dir, path) = temp_db("layout-missing");
        let store = SqliteStore::open(&path).unwrap();

        let err = store
            .set_session_layout(&SessionId::from_string("no-such"), "{}")
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::SessionNotFound(_)));
    }

    // ── list_resumable_surfaces only returns surfaces from active sessions ─

    #[test]
    fn resumable_surfaces_excludes_archived_session_surfaces() {
        let (_dir, path) = temp_db("resumable-arch");
        let store = SqliteStore::open(&path).unwrap();

        let surface = make_surface(&store);
        store.archive_session(&surface.session_id).unwrap();

        let list = store.list_resumable_surfaces().unwrap();
        assert!(!list.iter().any(|s| s.id == surface.id));
    }

    // ── surface placement + worktree_id ───────────────────────────────────

    #[test]
    fn surface_created_with_placement_and_worktree_id_round_trips() {
        let (_dir, path) = temp_db("surf-placement");
        let store = SqliteStore::open(&path).unwrap();

        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        let wt = store
            .create_worktree(NewWorktree {
                project_id: project.id,
                path: "/tmp/wt1".to_string(),
                branch: Some("feat".to_string()),
            })
            .unwrap();
        let sess = make_session(&store);
        let wt_id = wt.id;
        let surface = store
            .create_surface(NewSurface {
                id: None,
                session_id: sess.id,
                kind: SurfaceKind::Terminal,
                cwd: None,
                placement: Some("sidebar".to_string()),
                worktree_id: Some(wt_id.clone()),
            })
            .unwrap();

        let fetched = store.get_surface(&surface.id).unwrap().unwrap();
        assert_eq!(fetched.placement.as_deref(), Some("sidebar"));
        assert_eq!(
            fetched.worktree_id.as_ref().map(|w| w.as_str()),
            Some(wt_id.as_str())
        );
    }

    #[test]
    fn surface_created_without_placement_stores_null() {
        let (_dir, path) = temp_db("surf-no-placement");
        let store = SqliteStore::open(&path).unwrap();

        let surface = make_surface(&store);
        let fetched = store.get_surface(&surface.id).unwrap().unwrap();
        assert!(fetched.placement.is_none());
        assert!(fetched.worktree_id.is_none());
    }

    // ── command library ───────────────────────────────────────────────────

    #[test]
    fn prebuilt_entries_present_after_first_open() {
        let (_dir, path) = temp_db("cmd-seed");
        let store = SqliteStore::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmds = store.list_commands().unwrap();
        assert!(cmds.iter().any(|c| c.name == "login-shell"));
    }

    #[test]
    fn seed_is_idempotent_on_repeated_open() {
        let (_dir, path) = temp_db("cmd-seed-idem");
        let store = SqliteStore::open(&path).unwrap();
        store.seed_commands().unwrap();
        store.seed_commands().unwrap();

        let cmds = store.list_commands().unwrap();
        let login_count = cmds.iter().filter(|c| c.name == "login-shell").count();
        assert_eq!(login_count, 1);
    }

    #[test]
    fn seed_under_concurrent_open_leaves_one_copy() {
        let (_dir, path) = temp_db("cmd-seed-concurrent");
        // Pre-create schema + seed once so the threads only contend on the idempotent insert.
        SqliteStore::open(&path).unwrap();

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let p = path.clone();
                std::thread::spawn(move || {
                    let store = SqliteStore::open(&p).unwrap();
                    store.seed_commands().unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let store = SqliteStore::open(&path).unwrap();
        let cmds = store.list_commands().unwrap();
        assert_eq!(
            cmds.iter().filter(|c| c.name == "login-shell").count(),
            1,
            "concurrent open must leave exactly one copy of each prebuilt command"
        );
    }

    #[test]
    fn set_launch_template_spec_on_absent_template_is_not_found() {
        let (_dir, path) = temp_db("tmpl-set-absent");
        let store = SqliteStore::open(&path).unwrap();

        let err = store
            .set_launch_template_spec(
                &LaunchTemplateId::from_string("no-such-template"),
                2,
                r#"{"version":2,"items":[]}"#,
            )
            .unwrap_err();

        assert!(matches!(err, OrchestratorError::LaunchTemplateNotFound(_)));
    }

    #[test]
    fn list_returns_all_non_deleted_commands() {
        let (_dir, path) = temp_db("cmd-list");
        let store = SqliteStore::open(&path).unwrap();
        store.seed_commands().unwrap();

        let custom = store
            .create_command(crate::persistence::NewCommand {
                name: "my-tool".to_string(),
                origin: CommandOrigin::Custom,
                cli: "/usr/bin/mytool".to_string(),
                args: vec![],
                env: Default::default(),
            })
            .unwrap();

        let cmds = store.list_commands().unwrap();
        assert!(cmds.iter().any(|c| c.name == "login-shell"));
        assert!(cmds.iter().any(|c| c.id == custom.id));
    }

    #[test]
    fn get_returns_matching_entry() {
        let (_dir, path) = temp_db("cmd-get");
        let store = SqliteStore::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmd = store
            .create_command(crate::persistence::NewCommand {
                name: "x".to_string(),
                origin: CommandOrigin::Custom,
                cli: "/x".to_string(),
                args: vec![],
                env: Default::default(),
            })
            .unwrap();

        let fetched = store.get_command(cmd.id.as_str()).unwrap();
        assert_eq!(fetched.map(|c| c.id), Some(cmd.id));
    }

    #[test]
    fn get_on_unknown_id_returns_none() {
        let (_dir, path) = temp_db("cmd-get-missing");
        let store = SqliteStore::open(&path).unwrap();
        let result = store.get_command("no-such-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn custom_command_is_added() {
        let (_dir, path) = temp_db("cmd-add");
        let store = SqliteStore::open(&path).unwrap();

        let cmd = store
            .create_command(crate::persistence::NewCommand {
                name: "custom-tool".to_string(),
                origin: CommandOrigin::Custom,
                cli: "/bin/tool".to_string(),
                args: vec!["--flag".to_string()],
                env: Default::default(),
            })
            .unwrap();

        assert_eq!(cmd.origin, CommandOrigin::Custom);
        assert_eq!(cmd.name, "custom-tool");
    }

    #[test]
    fn command_is_deleted() {
        let (_dir, path) = temp_db("cmd-delete");
        let store = SqliteStore::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmd = store.list_commands().unwrap().into_iter().next().unwrap();
        store.delete_command(cmd.id.as_str()).unwrap();

        let after = store.list_commands().unwrap();
        assert!(!after.iter().any(|c| c.id == cmd.id));
        assert!(store.get_command(cmd.id.as_str()).unwrap().is_none());
    }

    // ── worktree ──────────────────────────────────────────────────────────

    #[test]
    fn worktree_row_created_and_returned() {
        let (_dir, path) = temp_db("wt-create");
        let store = SqliteStore::open(&path).unwrap();

        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();

        let wt = store
            .create_worktree(NewWorktree {
                project_id: project.id.clone(),
                path: "/tmp/wt".to_string(),
                branch: Some("main".to_string()),
            })
            .unwrap();

        let list = store.list_worktrees(&project.id).unwrap();
        assert!(list.iter().any(|w| w.id == wt.id));
    }

    #[test]
    fn archived_worktree_absent_from_list() {
        let (_dir, path) = temp_db("wt-archive");
        let store = SqliteStore::open(&path).unwrap();

        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();

        let wt = store
            .create_worktree(NewWorktree {
                project_id: project.id.clone(),
                path: "/tmp/wt2".to_string(),
                branch: None,
            })
            .unwrap();

        store.archive_worktree(&wt.id).unwrap();

        let list = store.list_worktrees(&project.id).unwrap();
        assert!(!list.iter().any(|w| w.id == wt.id));
    }

    #[test]
    fn worktree_row_survives_store_reopen() {
        let (_dir, path) = temp_db("wt-persist");
        let project_id;
        let wt_id;
        {
            let store = SqliteStore::open(&path).unwrap();
            let project = store
                .create_project(NewProject {
                    source_kind: SourceKind::Blank,
                    root_path: None,
                    name: Some("p".to_string()),
                })
                .unwrap();
            project_id = project.id.clone();
            let wt = store
                .create_worktree(NewWorktree {
                    project_id: project.id,
                    path: "/tmp/wt3".to_string(),
                    branch: Some("feat".to_string()),
                })
                .unwrap();
            wt_id = wt.id;
        }
        let store2 = SqliteStore::open(&path).unwrap();
        let list = store2.list_worktrees(&project_id).unwrap();
        assert!(list.iter().any(|w| w.id == wt_id));
    }

    // ── session template reference ────────────────────────────────────────

    #[test]
    fn new_session_without_template_produces_null_spec_fields() {
        let (_dir, path) = temp_db("sess-no-tmpl");
        let store = SqliteStore::open(&path).unwrap();

        let sess = store.create_session(NewSession::default()).unwrap();
        assert!(sess.spec_version.is_none());
        assert!(sess.spec_json.is_none());
    }

    #[test]
    fn new_session_with_template_copies_spec_atomically() {
        let (_dir, path) = temp_db("sess-with-tmpl");
        let store = SqliteStore::open(&path).unwrap();

        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        let tmpl = store
            .create_launch_template(crate::persistence::NewLaunchTemplate {
                project_id: project.id,
                spec_version: 1,
                spec_json: r#"{"version":1,"items":[]}"#.to_string(),
            })
            .unwrap();

        let sess = store
            .create_session(NewSession {
                template_id: Some(tmpl.id),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(sess.spec_version, Some(1));
        assert_eq!(
            sess.spec_json.as_deref(),
            Some(r#"{"version":1,"items":[]}"#)
        );
    }

    #[test]
    fn template_update_after_session_creation_does_not_affect_session() {
        let (_dir, path) = temp_db("sess-tmpl-diverge");
        let store = SqliteStore::open(&path).unwrap();

        let project = store
            .create_project(NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("p".to_string()),
            })
            .unwrap();
        let tmpl = store
            .create_launch_template(crate::persistence::NewLaunchTemplate {
                project_id: project.id,
                spec_version: 1,
                spec_json: r#"{"version":1,"items":[]}"#.to_string(),
            })
            .unwrap();

        let sess = store
            .create_session(NewSession {
                template_id: Some(tmpl.id.clone()),
                ..Default::default()
            })
            .unwrap();

        store
            .set_launch_template_spec(
                &tmpl.id,
                1,
                r#"{"version":1,"items":[{"target":"x","command":{"executable":"/x","args":[]}}]}"#,
            )
            .unwrap();

        let reloaded = store.get_session(&sess.id).unwrap().unwrap();
        assert_eq!(
            reloaded.spec_json.as_deref(),
            Some(r#"{"version":1,"items":[]}"#)
        );
    }
}
