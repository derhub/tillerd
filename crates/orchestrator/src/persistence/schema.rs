use super::{ProjectId, WorkspaceId};

fn migration_v1() -> String {
    format!(
        "CREATE TABLE meta (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );

         CREATE TABLE project (
             id          TEXT PRIMARY KEY,
             name        TEXT NOT NULL,
             source_kind TEXT NOT NULL
                 CHECK (source_kind IN ('blank','local_dir','git_repo','git_worktree')),
             root_path   TEXT,
             deleted_at  TEXT,
             created_at  TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE worktree (
             id         TEXT PRIMARY KEY,
             project_id TEXT NOT NULL REFERENCES project(id),
             path       TEXT NOT NULL,
             branch     TEXT,
             deleted_at TEXT,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE launch_template (
             id           TEXT PRIMARY KEY,
             project_id   TEXT NOT NULL REFERENCES project(id),
             spec_version INTEGER NOT NULL,
             spec_json    TEXT NOT NULL,
             updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE session (
             id           TEXT PRIMARY KEY,
             project_id   TEXT NOT NULL REFERENCES project(id),
             title        TEXT,
             title_source TEXT NOT NULL DEFAULT 'inferred'
                 CHECK (title_source IN ('inferred','custom')),
             spec_version INTEGER,
             spec_json    TEXT,
             layout_json  TEXT,
             deleted_at   TEXT,
             created_at   TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE surface (
             id          TEXT PRIMARY KEY,
             session_id  TEXT NOT NULL REFERENCES session(id),
             kind        TEXT NOT NULL CHECK (kind IN ('terminal','agent','diff')),
             title       TEXT,
             cwd         TEXT,
             worktree_id TEXT REFERENCES worktree(id),
             placement   TEXT,
             last_status TEXT,
             deleted_at  TEXT,
             created_at  TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE command (
             id        TEXT PRIMARY KEY,
             name      TEXT NOT NULL,
             origin    TEXT NOT NULL CHECK (origin IN ('prebuilt','custom')),
             cli       TEXT NOT NULL,
             args_json TEXT,
             env_json  TEXT,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE secret_ref (
             id           TEXT PRIMARY KEY,
             scope        TEXT NOT NULL CHECK (scope IN ('global','project')),
             project_id   TEXT REFERENCES project(id),
             env_key      TEXT NOT NULL,
             keychain_ref TEXT NOT NULL,
             created_at   TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE TABLE setting (
             scope      TEXT NOT NULL,
             project_id TEXT,
             key        TEXT NOT NULL,
             value_json TEXT NOT NULL,
             PRIMARY KEY (scope, project_id, key)
         );

         INSERT INTO project (id, name, source_kind, root_path)
             VALUES ('{unfiled}', 'Unfiled', 'blank', NULL);",
        unfiled = ProjectId::UNFILED,
    )
}

/// Widen `session.title_source` CHECK to the four-strategy enum and change the default
/// from `'inferred'` to `'agent-title'`. SQLite does not support ALTER COLUMN, so we
/// recreate the table, copy data (mapping 'inferred' -> 'agent-title'), and drop the old one.
fn migration_v2() -> String {
    "CREATE TABLE session_new (
         id           TEXT PRIMARY KEY,
         project_id   TEXT NOT NULL REFERENCES project(id),
         title        TEXT NOT NULL DEFAULT '',
         title_source TEXT NOT NULL DEFAULT 'agent-title'
             CHECK (title_source IN ('agent-title','branch','both','custom')),
         spec_version INTEGER,
         spec_json    TEXT,
         layout_json  TEXT,
         deleted_at   TEXT,
         created_at   TEXT NOT NULL DEFAULT (datetime('now')),
         updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
     );

     INSERT INTO session_new
         SELECT id, project_id,
                COALESCE(title, ''),
                CASE title_source
                    WHEN 'inferred' THEN 'agent-title'
                    WHEN 'custom'   THEN 'custom'
                    ELSE                 'agent-title'
                END,
                spec_version, spec_json, layout_json,
                deleted_at, created_at, updated_at
         FROM session;

     DROP TABLE session;
     ALTER TABLE session_new RENAME TO session;"
        .to_string()
}

/// Add `deleted_at` to the `command` table to enable soft-delete.
fn migration_v3() -> String {
    "ALTER TABLE command ADD COLUMN deleted_at TEXT;".to_string()
}

// Partial index: soft-deleted and null-placement (pre-migration) rows are excluded.
fn migration_v4() -> String {
    "CREATE UNIQUE INDEX surface_session_placement
         ON surface(session_id, placement)
         WHERE deleted_at IS NULL AND placement IS NOT NULL;"
        .to_string()
}

// Durable user-facing notification history (ADR-0031). Additive table under the 0.0.6 data-model
// freeze: nothing existing changes. `ts` is event time (epoch ms); `rowid` orders insertion for
// list/prune. Retention prunes to the most recent N (see `prune_notifications`).
fn migration_v5() -> String {
    "CREATE TABLE notification (
         id           TEXT PRIMARY KEY,
         category     TEXT NOT NULL,
         severity     TEXT NOT NULL,
         title        TEXT,
         message      TEXT NOT NULL,
         detail       TEXT,
         ts           INTEGER NOT NULL,
         session_id   TEXT,
         surface_id   TEXT,
         actions_json TEXT,
         created_at   TEXT NOT NULL DEFAULT (datetime('now'))
     );"
    .to_string()
}

fn migration_v6() -> String {
    "ALTER TABLE project ADD COLUMN sort_order INTEGER;
     ALTER TABLE session ADD COLUMN sort_order INTEGER;"
        .to_string()
}

// Insert the `workspace` tier above `project` (ADR-0032). Additive under the 0.0.6 data-model
// freeze: a new table plus a nullable `project.workspace_id`, backfilled to the seeded Default
// workspace so no project is left unassigned. The column stays nullable at rest only transiently
// during this migration; the application layer treats it as non-null thereafter.
fn migration_v7() -> String {
    format!(
        "CREATE TABLE workspace (
             id         TEXT PRIMARY KEY,
             name       TEXT NOT NULL,
             sort_order INTEGER,
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );

         INSERT INTO workspace (id, name) VALUES ('{default}', 'Default');

         ALTER TABLE project ADD COLUMN workspace_id TEXT REFERENCES workspace(id);

         UPDATE project SET workspace_id = '{default}' WHERE workspace_id IS NULL;",
        default = WorkspaceId::DEFAULT,
    )
}

pub fn migrations() -> Vec<String> {
    vec![
        migration_v1(),
        migration_v2(),
        migration_v3(),
        migration_v4(),
        migration_v5(),
        migration_v6(),
        migration_v7(),
    ]
}

pub fn current_version() -> u32 {
    migrations().len() as u32
}
