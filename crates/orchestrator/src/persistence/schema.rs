/// Single operational migration (v1 — clean cutover, D7).
///
/// Operational tables only: `meta`, `command`, `setting`, `notification`,
/// `launch_template`. Domain entities (workspace/project/session/surface) now live
/// in the file-tree store, not SQLite. The `schema_version` value in `meta` is
/// written by the migration runner, not seeded here.
fn migration_v1() -> String {
    "CREATE TABLE meta (
         key   TEXT PRIMARY KEY,
         value TEXT NOT NULL
     );

     CREATE TABLE command (
         id         TEXT PRIMARY KEY,
         name       TEXT NOT NULL,
         origin     TEXT NOT NULL CHECK (origin IN ('prebuilt','custom')),
         cli        TEXT NOT NULL,
         args_json  TEXT,
         env_json   TEXT,
         deleted_at TEXT,
         created_at TEXT NOT NULL DEFAULT (datetime('now'))
     );

     CREATE TABLE setting (
         scope      TEXT NOT NULL,
         project_id TEXT,
         key        TEXT NOT NULL,
         value_json TEXT NOT NULL,
         PRIMARY KEY (scope, project_id, key)
     );

     CREATE TABLE notification (
         id           TEXT PRIMARY KEY,
         category     TEXT NOT NULL,
         severity     TEXT NOT NULL,
         title        TEXT,
         message      TEXT NOT NULL,
         detail       TEXT,
         ts           INTEGER NOT NULL,
         session_id   TEXT,
         surface_id   TEXT,
         actions_json TEXT
     );

     CREATE TABLE launch_template (
         id           TEXT PRIMARY KEY,
         project_id   TEXT NOT NULL,
         spec_version INTEGER NOT NULL,
         spec_json    TEXT NOT NULL,
         updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
     );"
    .to_string()
}

pub fn migrations() -> Vec<String> {
    vec![migration_v1()]
}

pub fn current_version() -> u32 {
    1
}
