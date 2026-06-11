//! The product schema as ordered, append-only migrations (ADR-0023).
//!
//! [`MIGRATIONS`] holds one SQL script per schema version, applied in order. The
//! current version is the count of migrations; the lazy runner in
//! [`super::sqlite`] applies any the store is missing on open. Migrations are
//! never edited or reordered once shipped — a new version is a new entry.

use super::ProjectId;

/// Schema version 1: the full ADR-0023 product schema plus the seeded Unfiled
/// project. The Unfiled id is interpolated from [`ProjectId::UNFILED`] so the
/// constant stays the single source of truth.
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

/// All schema migrations in order. Index `n` is the migration that brings the
/// store from version `n` to version `n + 1`.
pub fn migrations() -> Vec<String> {
    vec![migration_v1()]
}

/// The schema version this binary expects: the number of migrations.
pub fn current_version() -> u32 {
    migrations().len() as u32
}
