use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};

use super::schema;
use crate::entities::{
    Command, CommandId, CommandOrigin, LaunchTemplate, LaunchTemplateId, NewCommand,
    NewLaunchTemplate, NotificationRecord, ProjectId, SettingEntry, SettingScope,
};
use crate::error::{OrchestratorError, Result};

pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
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

impl SqliteBackend {
    pub(crate) fn schema_version(&self) -> Result<u32> {
        let conn = self.lock()?;
        read_schema_version(&conn)
    }

    // ── command library ───────────────────────────────────────────────────

    pub(crate) fn list_commands(&self) -> Result<Vec<Command>> {
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

    pub(crate) fn get_command(&self, id: &str) -> Result<Option<Command>> {
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

    pub(crate) fn create_command(&self, draft: NewCommand) -> Result<Command> {
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

    pub(crate) fn delete_command(&self, id: &str) -> Result<()> {
        self.lock()?
            .execute(
                "UPDATE command SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
            )
            .map_err(persist)?;
        Ok(())
    }

    pub(crate) fn seed_commands(&self) -> Result<()> {
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

    // ── launch template ───────────────────────────────────────────────────

    pub(crate) fn create_launch_template(
        &self,
        draft: NewLaunchTemplate,
    ) -> Result<LaunchTemplate> {
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

    pub(crate) fn get_launch_template(
        &self,
        id: &LaunchTemplateId,
    ) -> Result<Option<LaunchTemplate>> {
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

    pub(crate) fn set_launch_template_spec(
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

    // ── settings ──────────────────────────────────────────────────────────

    pub(crate) fn get_setting(&self, scope: &SettingScope, key: &str) -> Result<Option<String>> {
        let (scope_col, project_col) = scope.columns();
        self.lock()?
            .query_row(
                "SELECT value_json FROM setting
                 WHERE scope = ?1 AND project_id = ?2 AND key = ?3",
                params![scope_col, project_col, key],
                |r| r.get(0),
            )
            .optional()
            .map_err(persist)
    }

    pub(crate) fn set_setting(
        &self,
        scope: &SettingScope,
        key: &str,
        value_json: &str,
    ) -> Result<()> {
        let (scope_col, project_col) = scope.columns();
        self.lock()?
            .execute(
                "INSERT INTO setting (scope, project_id, key, value_json)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope, project_id, key)
                 DO UPDATE SET value_json = excluded.value_json",
                params![scope_col, project_col, key, value_json],
            )
            .map_err(persist)?;
        Ok(())
    }

    pub(crate) fn list_settings(&self, scope: &SettingScope) -> Result<Vec<SettingEntry>> {
        let (scope_col, project_col) = scope.columns();
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT key, value_json FROM setting
                 WHERE scope = ?1 AND project_id = ?2
                 ORDER BY key",
            )
            .map_err(persist)?;
        let rows = stmt
            .query_map(params![scope_col, project_col], |row| {
                Ok(SettingEntry {
                    key: row.get(0)?,
                    value_json: row.get(1)?,
                })
            })
            .map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    pub(crate) fn resolve_setting(
        &self,
        project_id: &ProjectId,
        key: &str,
    ) -> Result<Option<String>> {
        if let Some(v) = self.get_setting(&SettingScope::Project(project_id.clone()), key)? {
            return Ok(Some(v));
        }
        self.get_setting(&SettingScope::Global, key)
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    pub(crate) fn insert_notification(&self, rec: &NotificationRecord) -> Result<()> {
        self.lock()?
            .execute(
                "INSERT INTO notification
                     (id, category, severity, title, message, detail, ts, session_id, surface_id, actions_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    rec.id,
                    rec.category,
                    rec.severity,
                    rec.title,
                    rec.message,
                    rec.detail,
                    rec.ts,
                    rec.session_id,
                    rec.surface_id,
                    rec.actions_json,
                ],
            )
            .map_err(persist)?;
        Ok(())
    }

    pub(crate) fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, category, severity, title, message, detail, ts, session_id, surface_id, actions_json
                 FROM notification
                 ORDER BY rowid DESC
                 LIMIT ?1",
            )
            .map_err(persist)?;
        let rows = stmt
            .query_map(params![limit], row_to_notification)
            .map_err(persist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(persist)
    }

    pub(crate) fn prune_notifications(&self, keep: u32) -> Result<()> {
        self.lock()?
            .execute(
                "DELETE FROM notification
                 WHERE rowid NOT IN (
                     SELECT rowid FROM notification ORDER BY rowid DESC LIMIT ?1
                 )",
                params![keep],
            )
            .map_err(persist)?;
        Ok(())
    }
}

fn row_to_notification(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRecord> {
    Ok(NotificationRecord {
        id: row.get(0)?,
        category: row.get(1)?,
        severity: row.get(2)?,
        title: row.get(3)?,
        message: row.get(4)?,
        detail: row.get(5)?,
        ts: row.get(6)?,
        session_id: row.get(7)?,
        surface_id: row.get(8)?,
        actions_json: row.get(9)?,
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
    use crate::entities::{CommandOrigin, NewLaunchTemplate, NotificationRecord, SettingScope};

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

    fn table_exists(store: &SqliteBackend, name: &str) -> bool {
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

    // ── migration ─────────────────────────────────────────────────────────

    #[test]
    fn fresh_store_initializes_to_current_version_and_records_it() {
        let (_dir, path) = temp_db("fresh");
        let store = SqliteBackend::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), schema::current_version());
    }

    #[test]
    fn migration_runner_applies_pending_migrations_in_order() {
        let (_dir, path) = temp_db("forward");
        let migrations = synthetic_migrations();

        let store = SqliteBackend::open_with(&path, &migrations[..1]).unwrap();
        assert_eq!(store.schema_version().unwrap(), 1);
        assert!(table_exists(&store, "marker_one"));
        assert!(!table_exists(&store, "marker_two"));
        drop(store);

        let store = SqliteBackend::open_with(&path, &migrations).unwrap();
        assert_eq!(store.schema_version().unwrap(), 2);
        assert!(table_exists(&store, "marker_two"));
    }

    #[test]
    fn store_newer_than_binary_is_refused() {
        let (_dir, path) = temp_db("newer");
        let migrations = synthetic_migrations();

        SqliteBackend::open_with(&path, &migrations).unwrap();

        let result = SqliteBackend::open_with(&path, &migrations[..1]);
        assert!(matches!(
            result,
            Err(OrchestratorError::StoreVersionTooNew {
                found: 2,
                supported: 1
            })
        ));
    }

    // ── command library ───────────────────────────────────────────────────

    #[test]
    fn prebuilt_entries_present_after_first_open() {
        let (_dir, path) = temp_db("cmd-seed");
        let store = SqliteBackend::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmds = store.list_commands().unwrap();
        assert!(cmds.iter().any(|c| c.name == "login-shell"));
    }

    #[test]
    fn seed_is_idempotent_on_repeated_open() {
        let (_dir, path) = temp_db("cmd-seed-idem");
        let store = SqliteBackend::open(&path).unwrap();
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
        SqliteBackend::open(&path).unwrap();

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let p = path.clone();
                std::thread::spawn(move || {
                    let store = SqliteBackend::open(&p).unwrap();
                    store.seed_commands().unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let store = SqliteBackend::open(&path).unwrap();
        let cmds = store.list_commands().unwrap();
        assert_eq!(
            cmds.iter().filter(|c| c.name == "login-shell").count(),
            1,
            "concurrent open must leave exactly one copy of each prebuilt command"
        );
    }

    #[test]
    fn list_returns_all_non_deleted_commands() {
        let (_dir, path) = temp_db("cmd-list");
        let store = SqliteBackend::open(&path).unwrap();
        store.seed_commands().unwrap();

        let custom = store
            .create_command(NewCommand {
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
        let store = SqliteBackend::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmd = store
            .create_command(NewCommand {
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
        let store = SqliteBackend::open(&path).unwrap();
        let result = store.get_command("no-such-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn custom_command_is_added() {
        let (_dir, path) = temp_db("cmd-add");
        let store = SqliteBackend::open(&path).unwrap();

        let cmd = store
            .create_command(NewCommand {
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
        let store = SqliteBackend::open(&path).unwrap();
        store.seed_commands().unwrap();

        let cmd = store.list_commands().unwrap().into_iter().next().unwrap();
        store.delete_command(cmd.id.as_str()).unwrap();

        let after = store.list_commands().unwrap();
        assert!(!after.iter().any(|c| c.id == cmd.id));
        assert!(store.get_command(cmd.id.as_str()).unwrap().is_none());
    }

    // ── launch template ───────────────────────────────────────────────────

    #[test]
    fn set_launch_template_spec_on_absent_template_is_not_found() {
        let (_dir, path) = temp_db("tmpl-set-absent");
        let store = SqliteBackend::open(&path).unwrap();

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
    fn create_then_get_launch_template_round_trips() {
        let (_dir, path) = temp_db("tmpl-rt");
        let store = SqliteBackend::open(&path).unwrap();

        let tmpl = store
            .create_launch_template(NewLaunchTemplate {
                project_id: ProjectId::unfiled(),
                spec_version: 1,
                spec_json: r#"{"version":1,"items":[]}"#.to_string(),
            })
            .unwrap();

        let fetched = store.get_launch_template(&tmpl.id).unwrap().unwrap();
        assert_eq!(fetched.spec_version, 1);
        assert_eq!(fetched.spec_json, r#"{"version":1,"items":[]}"#);
    }

    // ── notifications (ADR-0031) ──────────────────────────────────────────

    fn notif(id: &str, ts: i64) -> NotificationRecord {
        NotificationRecord {
            id: id.to_string(),
            category: "surface-stopped".to_string(),
            severity: "info".to_string(),
            title: None,
            message: format!("msg {id}"),
            detail: None,
            ts,
            session_id: Some("sess-1".to_string()),
            surface_id: Some("surf-1".to_string()),
            actions_json: None,
        }
    }

    #[test]
    fn notifications_list_newest_first() {
        let (_dir, path) = temp_db("notif-list");
        let store = SqliteBackend::open(&path).unwrap();
        store.insert_notification(&notif("a", 1)).unwrap();
        store.insert_notification(&notif("b", 2)).unwrap();
        store.insert_notification(&notif("c", 3)).unwrap();

        let ids: Vec<String> = store
            .list_notifications(10)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["c", "b", "a"]);
    }

    #[test]
    fn notifications_prune_keeps_newest() {
        let (_dir, path) = temp_db("notif-prune");
        let store = SqliteBackend::open(&path).unwrap();
        for i in 0..5 {
            store
                .insert_notification(&notif(&format!("n{i}"), i))
                .unwrap();
        }
        store.prune_notifications(2).unwrap();
        let ids: Vec<String> = store
            .list_notifications(10)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["n4", "n3"]);
    }

    #[test]
    fn notifications_survive_a_restart() {
        let (_dir, path) = temp_db("notif-persist");
        {
            let store = SqliteBackend::open(&path).unwrap();
            store.insert_notification(&notif("x", 1)).unwrap();
        }
        // Reopen against the same file — the history persists (ADR-0031).
        let store = SqliteBackend::open(&path).unwrap();
        let listed = store.list_notifications(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "x");
    }

    // ── settings ──────────────────────────────────────────────────────────

    #[test]
    fn settings_survive_a_restart() {
        let (_dir, path) = temp_db("settings-persist");
        {
            let store = SqliteBackend::open(&path).unwrap();
            store
                .set_setting(&SettingScope::Global, "theme", r#""light""#)
                .unwrap();
        }
        // Reopen against the same file — the value persists.
        let store = SqliteBackend::open(&path).unwrap();
        assert_eq!(
            store
                .get_setting(&SettingScope::Global, "theme")
                .unwrap()
                .as_deref(),
            Some(r#""light""#)
        );
    }

    #[test]
    fn overwriting_a_setting_upserts_without_duplicate_rows() {
        let (_dir, path) = temp_db("settings-upsert");
        let store = SqliteBackend::open(&path).unwrap();
        store.set_setting(&SettingScope::Global, "k", "1").unwrap();
        store.set_setting(&SettingScope::Global, "k", "2").unwrap();

        assert_eq!(
            store
                .get_setting(&SettingScope::Global, "k")
                .unwrap()
                .as_deref(),
            Some("2")
        );
        let count: i64 = store
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM setting WHERE scope='global' AND project_id='' AND key='k'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "upsert must not create a duplicate row");
    }

    #[test]
    fn project_and_global_settings_are_independent() {
        let (_dir, path) = temp_db("settings-scope");
        let store = SqliteBackend::open(&path).unwrap();
        let pid = ProjectId::unfiled();
        store
            .set_setting(&SettingScope::Global, "env", r#""g""#)
            .unwrap();
        store
            .set_setting(&SettingScope::Project(pid.clone()), "env", r#""p""#)
            .unwrap();

        assert_eq!(
            store.resolve_setting(&pid, "env").unwrap().as_deref(),
            Some(r#""p""#)
        );
        assert_eq!(
            store
                .get_setting(&SettingScope::Global, "env")
                .unwrap()
                .as_deref(),
            Some(r#""g""#)
        );
    }
}
