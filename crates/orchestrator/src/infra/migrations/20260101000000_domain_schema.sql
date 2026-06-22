-- Domain schema v1: clean break from the slug-tree representation.
-- All domain entities live here; user-config (settings/profile/theme/keybindings) lives in files.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ── workspace ──────────────────────────────────────────────────────────────────
CREATE TABLE workspace (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    pinned      INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived_at TEXT,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Seed the built-in Default workspace (id mirrors WorkspaceId::DEFAULT).
INSERT INTO workspace (id, name, sort_order) VALUES
    ('00000000-0000-0000-0000-000000000001', 'Default', 0);

-- ── project ────────────────────────────────────────────────────────────────────
CREATE TABLE project (
    id           TEXT    PRIMARY KEY,
    workspace_id TEXT    NOT NULL REFERENCES workspace(id),
    name         TEXT    NOT NULL,
    source_kind  TEXT    NOT NULL DEFAULT 'blank' CHECK (source_kind IN ('blank', 'local_dir', 'git_repo')),
    root_path    TEXT,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    pinned       INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived_at  TEXT,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX project_workspace ON project (workspace_id, pinned DESC, sort_order);

-- Seed the built-in Unfiled project (id mirrors ProjectId::UNFILED).
INSERT INTO project (id, workspace_id, name, source_kind) VALUES
    ('00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000001', 'Unfiled', 'blank');

-- ── session ────────────────────────────────────────────────────────────────────
CREATE TABLE session (
    id              TEXT    PRIMARY KEY,
    project_id      TEXT    NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    title           TEXT    NOT NULL DEFAULT '',
    title_source    TEXT    NOT NULL DEFAULT 'agent-title'
                            CHECK (title_source IN ('agent-title', 'branch', 'both', 'custom')),
    spec_version    INTEGER,
    spec_json       TEXT,
    panel_tree_json TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    pinned          INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived_at     TEXT,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX session_project ON session (project_id, pinned DESC, sort_order);

-- ── surface ────────────────────────────────────────────────────────────────────
-- status: pending (intent persisted, spawn not yet attempted),
--         live    (running in the daemon),
--         idle    (PTY stopped; row kept for resume),
--         failed  (spawn attempt failed).
CREATE TABLE surface (
    id         TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL DEFAULT 'terminal' CHECK (kind IN ('terminal', 'diff')),
    cwd        TEXT,
    placement  TEXT,
    status     TEXT NOT NULL DEFAULT 'pending'
               CHECK (status IN ('pending', 'live', 'idle', 'failed')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX surface_session ON surface (session_id);
-- placement must be unique within a session when set.
CREATE UNIQUE INDEX surface_placement ON surface (session_id, placement)
    WHERE placement IS NOT NULL;

-- ── command library ────────────────────────────────────────────────────────────
-- origin: prebuilt (immutable, seeded at boot) | custom (user-created).
CREATE TABLE command (
    id         TEXT    PRIMARY KEY,
    name       TEXT    NOT NULL,
    origin     TEXT    NOT NULL DEFAULT 'custom' CHECK (origin IN ('prebuilt', 'custom')),
    cli        TEXT    NOT NULL,
    args_json  TEXT    NOT NULL DEFAULT '[]',
    env_json   TEXT    NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0,
    pinned     INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    deleted_at TEXT,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX command_active ON command (pinned DESC, sort_order)
    WHERE deleted_at IS NULL;

-- Seed the built-in prebuilt commands.
INSERT INTO command (id, name, origin, cli, args_json) VALUES
    ('00000000-0000-0000-0000-000000000101', 'login-shell', 'prebuilt', '/bin/bash', '["-l"]');

-- ── launch_template ────────────────────────────────────────────────────────────
-- A project-bound saved launch spec (the recipe: surfaces + placements).
CREATE TABLE launch_template (
    id           TEXT    PRIMARY KEY,
    project_id   TEXT    NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    name         TEXT    NOT NULL DEFAULT '',
    spec_version INTEGER NOT NULL DEFAULT 1,
    spec_json    TEXT    NOT NULL DEFAULT '{}',
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX launch_template_project ON launch_template (project_id, sort_order);

-- ── notification ───────────────────────────────────────────────────────────────
CREATE TABLE notification (
    id           TEXT    PRIMARY KEY,
    category     TEXT    NOT NULL,
    severity     TEXT    NOT NULL,
    title        TEXT,
    message      TEXT    NOT NULL,
    detail       TEXT,
    ts           INTEGER NOT NULL,
    session_id   TEXT,
    surface_id   TEXT,
    actions_json TEXT,
    read         INTEGER NOT NULL DEFAULT 0 CHECK (read IN (0, 1)),
    snooze_until INTEGER
);

CREATE INDEX notification_unread ON notification (ts DESC) WHERE read = 0;

-- ── kv store (for shared::kv::SqliteKv) ───────────────────────────────────────
CREATE TABLE kv (
    key        TEXT    PRIMARY KEY,
    value      BLOB    NOT NULL,
    expires_at INTEGER
);
