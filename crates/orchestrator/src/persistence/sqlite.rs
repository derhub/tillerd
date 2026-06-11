use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};

use super::schema;
use super::{
    NewSession, NewSurface, Project, ProjectId, Session, SessionId, SourceKind, Store, Surface,
    SurfaceId,
};
use crate::error::{OrchestratorError, Result};

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn default_path() -> PathBuf {
        process_launch::tillerd_dir().join("tillerd.db")
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
        run_migrations(&conn, migrations)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
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

impl Store for SqliteStore {
    fn schema_version(&self) -> Result<u32> {
        let conn = self.lock()?;
        read_schema_version(&conn)
    }

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

    fn create_session(&self, draft: NewSession) -> Result<Session> {
        let id = SessionId::mint();
        let project_id = draft.project.unwrap_or_else(ProjectId::unfiled);
        self.lock()?
            .execute(
                "INSERT INTO session (id, project_id, title) VALUES (?1, ?2, ?3)",
                params![id.as_str(), project_id.as_str(), draft.title],
            )
            .map_err(persist)?;
        Ok(Session {
            id,
            project_id,
            title: draft.title,
        })
    }

    fn create_surface(&self, draft: NewSurface) -> Result<Surface> {
        let id = SurfaceId::mint();
        self.lock()?
            .execute(
                "INSERT INTO surface (id, session_id, kind, cwd) VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.as_str(),
                    draft.session_id.as_str(),
                    draft.kind.as_str(),
                    draft.cwd
                ],
            )
            .map_err(persist)?;
        Ok(Surface {
            id,
            session_id: draft.session_id,
            kind: draft.kind,
            cwd: draft.cwd,
        })
    }
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

    #[test]
    fn session_without_a_project_resolves_to_unfiled() {
        let (_dir, path) = temp_db("unfiled-session");
        let store = SqliteStore::open(&path).unwrap();

        let session = store.create_session(NewSession::default()).unwrap();

        assert_eq!(session.project_id, ProjectId::unfiled());
    }

    #[test]
    fn surface_correlation_id_is_the_shared_surface_id_not_the_session_id() {
        let (_dir, path) = temp_db("two-level-id");
        let store = SqliteStore::open(&path).unwrap();

        let session = store.create_session(NewSession::default()).unwrap();
        let surface = store
            .create_surface(NewSurface {
                session_id: session.id.clone(),
                kind: SurfaceKind::Terminal,
                cwd: None,
            })
            .unwrap();

        assert_eq!(surface.correlation_id(), &surface.id);
        assert_ne!(surface.id.as_str(), session.id.as_str());
    }
}
