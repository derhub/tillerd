# 0023. Workspace data model: one SQLite store and a two-level id model

- Status: superseded by ADR-0033
- Date: 2026-06-11

## Context

The orchestrator (ADR-0022) needs durable product state and a clear identity scheme.
Today both are fragmented:

- State lives in four places: `desktop-store.json` (prefs + a `sessionId -> cwd`
  registry), `server.db` (`sessions(id, cwd, created_at)`, web path, dormant), the
  gate's in-memory session registry, and daemon-local files (`daemon.json`,
  `stopped-sessions.txt`, snapshots).
- There is one conflated id: the orchestration code mints a `session_id` + token,
  registers it with the gate, and injects it into the daemon — so the desktop session,
  the daemon PTY, and the gate principal all share one id. That holds only because a
  desktop "session" is exactly one PTY today.

ADR-0020 split `session` (container) from `surface` (leaf); ADR-0021 added projects,
templates, the command library, worktrees, and the archive lifecycle. The single
conflated id can no longer stand.

## Decision

### One product store

`tillerd.db` (rusqlite), owned by the orchestrator, is the single durable product
store. Service-local runtime and discovery state stays out — `daemon.json`, snapshots,
`stopped-sessions.txt`, and the gate's in-memory registry are ephemeral or discovery
and are excluded (the persistence-model boundary).

Persistence is read and write in Rust only; the renderer never touches the database —
it goes through the orchestrator API (ADR-0022).

### Two-level id model

- **`session_id`** (container) — product-layer; lives only in `tillerd.db`; backends
  never see it.
- **`surface_id`** (leaf) — equals the daemon PTY id, the gate session id, and the
  `correlation_id` for an agent run. This is today's `TILLERD_SESSION_ID`, now scoped
  to a surface. One id per surface, reused across backends rather than minting separate
  per-backend ids.
- **`correlation_id` = `surface_id`** — the only identifier shared across contexts
  (ADR-0020); it threads logs across the daemon and gate hops.

So the product session id never leaves the orchestrator, and no backend imports another
context's session model — the surface id is the shared kernel.

### Schema

```
project(id PK, name, source_kind{blank|local_dir|git_repo|git_worktree},
        root_path?, deleted_at?, created_at, updated_at)
worktree(id PK, project_id FK, path, branch, deleted_at?, created_at)
launch_template(id PK, project_id FK, spec_version, spec_json, updated_at)
session(id PK, project_id FK NOT NULL, title, title_source{inferred|custom},
        spec_version, spec_json, layout_json, deleted_at?, created_at, updated_at)
surface(id PK, session_id FK, kind{terminal|agent|diff}, title, cwd,
        worktree_id FK?, placement, last_status, deleted_at?, created_at)
command(id PK, name, origin{prebuilt|custom}, cli, args_json, env_json, created_at)
secret_ref(id PK, scope{global|project}, project_id?, env_key, keychain_ref, created_at)
setting(scope, project_id?, key, value_json, PK(scope, project_id, key))
meta(key PK, value)   -- DB schema version (distinct from launch-spec version)
```

- `surface.id` is the `surface_id` above (= daemon / gate / correlation id).
- `command` is the global command library; `setting` carries global and per-project
  settings; `secret_ref` holds only an OS-keychain handle, never a plaintext secret.

### Launch spec as a versioned blob

`launch_template.spec_json` and `session.spec_json` store the launch spec as one JSON
document carrying its `spec_version`. It is migrated as a whole on load (lazy
vN -> vN+1, ADR-0021); launch items are not normalized into rows.

### Soft delete and archive

`deleted_at` is the soft-delete timestamp. The "archive" view is rows where
`deleted_at IS NOT NULL`; a hard delete removes the row outright. Soft-deleting a
session soft-deletes its surfaces; the worktree row is soft-deleted but its directory
is kept on disk (recoverable).

### Seeds and migration

- An **"Unfiled" project** is seeded with a fixed id so `session.project_id` is always
  `NOT NULL`; ungrouped sessions belong to it.
- **No pre-v1 data migration.** The existing `desktop-store.json` registry is throwaway;
  the schema starts fresh.

### ID / correlation flow — create a session with an agent surface

```
1. orchestrator mints session_id          -> session row (project_id = P)
2. read spec (template or override)        -> launch items
3. per item: mint surface_id (= correlation_id)
4. agent surface:
     register (surface_id, token) with the gate     [before spawn]
     inject TILLERD_SESSION_ID = surface_id, token, TILLERD_DIR
     daemon spawns the PTY keyed by surface_id
     hooks -> gate (auth by surface_id + token) -> fan-out -> host
              subscribes per surface_id -> routes to that surface
   terminal surface: daemon PTY keyed by surface_id; no gate record
5. surface row (id = surface_id, session_id, kind, cwd, ...)
```

## Consequences

- One durable product store replaces four fragmented ones; service runtime state stays
  service-local, keeping service lifecycle independent of sessions (ADR-0020).
- The product session id is fully hidden from the backends; the surface id is the only
  shared identifier, so no subsystem couples to another's session model.
- Restart recovery is a query: active (`deleted_at IS NULL`) surfaces reconnect to the
  daemon by `surface_id` (which the daemon already replays).
- Storing the spec as a versioned blob keeps lazy migration simple and matches ADR-0021.
- Secrets never sit in the database in plaintext; only keychain handles do.
- The decision constrains the 0.x implementation but ships no code itself. Rollback is
  reverting this file.
