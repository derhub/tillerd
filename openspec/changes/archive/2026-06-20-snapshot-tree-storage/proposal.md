## Why

ADR-0023 made one SQLite file (`tillerd.db`) authoritative for the whole product tree,
conflating two kinds of data with opposite needs: domain (what the user authored —
workspaces, projects, sessions, panel layout, surface bindings) wants to be readable,
diffable, versionable, and portable; operational data wants to be fast, machine-local, and
disposable. ADR-0033 splits these into two planes. This slice lands the **domain plane** — a
readable JSON snapshot tree replacing the SQLite domain tables — and removes the worktree
entity first to clear persistence before the rewrite. It is the foundation the remaining
0.0.15 slices (operational `state.db` + state-model, settings/profiles/secrets, reconcile)
build on.

This supersedes ADR-0023's single-store data model — a frozen seam at 0.0.6 — authorized by
ADR-0033/0034 (merged in #38). Pre-v1, dev-only data is discarded (clean cutover, no
migration). **BREAKING.**

## What Changes

- **BREAKING — drop the worktree entity and provisioning, first.** Remove the `worktree`
  table, `Worktree`/`WorktreeId`/`NewWorktree` types, the three `Store` worktree methods, the
  `git worktree add` launch step (`launch/worktree.rs`, `WorktreeStep`, `run_worktree_step`),
  the `git_worktree` source kind (Rust enum + SQL CHECK + SDK union), `worktree_id` on the
  surface, and the two worktree error variants. A surface becomes `{ id, kind, placement, cwd }`;
  a working directory is just `cwd` (relative to project root).
- **BREAKING — domain plane = JSON snapshot tree.** Persist `workspace → project → session` as
  a directory tree: one slug directory per entity with the stable `id` inside its JSON file,
  containment encoding hierarchy (no `workspace_id`/`project_id` columns), explicit `sortOrder`,
  `layout.json` holding the panel tree + surface bindings. Writes are atomic (write-temp-rename).
  Rename re-slugs via atomic subtree move (collisions disambiguated `foo` → `foo-2`). Archive
  moves the subtree to `.archive/`. Replaces the SQLite `workspace`/`project`/`session`/`surface`
  domain tables and `layout_json`/`deleted_at` columns.
- **In-memory id→path index, scan-built at boot.** Domain refs resolve by stable `id` through an
  index rebuilt by scanning the tree at startup. Persisting the index to `state.db` is deferred
  to slice 2 (flagged below).
- **Operational tables stay in SQLite.** `meta`, `command`, `setting`, `notification`,
  `launch_template` remain in the existing store (the precursor to `state.db`, formalized in
  slice 2). Only the domain tables move out now.
- **Store-trait split.** The monolithic `Store` trait splits into a domain store (file-tree
  backed) and an operational store (SQLite backed). Operational tables drop their FK clauses to
  domain ids (`setting.project_id`, `notification.session_id/surface_id`,
  `launch_template.project_id`) — ids become opaque text keys validated against the in-memory
  index, not foreign keys.
- **Invariants that move DB → store code.** Placement uniqueness — one live surface per
  `(session, placement)`, previously the `surface_session_placement` partial UNIQUE index — is
  enforced in store code before writing `layout.json`, still raising `SurfaceConflict`. The
  domain store provides in-process write serialization (replacing SQLite's connection mutex) so
  concurrent writes from multiple windows stay consistent (single embedded orchestrator process;
  an in-process lock suffices).
- **Data root path.** The product-store path resolves to a relocatable **directory** (the data
  root), not a single `.db` file.

Out of scope (later 0.0.15 slices): operational `state.db` (final typed schema, persisted
id→path index, baselines), the state-model contract / lifecycle FSM / guards / sync status,
settings profiles + cascade, Stronghold secrets, reconcile (startup 2-way + Re-sync 3-way
`merge3`, malformed-file fallbacks), and the TanStack client. No file watchers in any slice.

## Capabilities

### New Capabilities

- `snapshot-tree-store`: the domain plane — `workspace → project → session` persisted as a
  JSON file tree (slug dirs + stable `id`, containment hierarchy, `sortOrder`, atomic
  write-temp-rename, re-slug-on-rename subtree move, `.archive/` subtree move), with an
  in-memory id→path index rebuilt by scanning at boot. Owns the domain CRUD/list/reorder/archive
  behavior previously backed by SQLite rows.

### Modified Capabilities

- `workspace-persistence`: narrowed to the **operational SQLite store** (meta/command/setting/
  notification/launch_template) — domain entities and the worktree entity removed from its store
  trait; schema-version/migration language scoped to operational tables only.
- `workspace-management`: workspace persisted as a slug dir + `workspace.json`; ordering via a
  `sortOrder` field; non-deletable Default unchanged in intent.
- `project-management`: drop the `git-worktree` source kind; project persisted as `project.json`;
  archive = subtree move (not `deleted_at`); remove the "worktree directory kept" scenario.
- `session-container`: session persisted as `session.json`; archive = subtree move; cascade is
  the directory move.
- `layout-persistence`: panel tree + surface bindings persisted as `layout.json` (not the
  `layout_json` column).
- `surface-runtime`: surface = `{ id, kind, placement, cwd }` — drop the worktree reference at
  creation; archive via the file tree.
- `launch-execution`: drop the worktree step from the executor; `cwd` set directly.
- `launch-item`: remove worktree from the best-effort failure model.
- `launch-spec`: drop the `worktree` launch-item field.
- `runtime-paths`: the product-store path builder returns the data-root **directory**, not a
  single store file.
- `project-worktree`: **REMOVED** — the entire capability is retired with the worktree entity.

## Impact

- **Rust orchestrator** (`crates/orchestrator`): `persistence/{mod.rs,schema.rs,sqlite.rs,memory.rs}`
  (new file-tree domain store + Store-trait split: domain vs operational); `launch/`
  (`worktree.rs` removed, `spec.rs`, `executor.rs`, `mod.rs`); `surface/api.rs`; `error.rs`;
  whatever resolves the store path (`runtime-paths`/`tillerd_paths`).
- **SDK** (`packages/sdk`): `SourceKind` union drops `git_worktree`.
- **No UI change required** beyond types — domain CRUD API shape over IPC is preserved where
  possible (file-backed behind the same orchestrator methods); deviations surface in design.
- **Testing**: a tempdir-backed domain-store test double replaces `InMemoryStore` for domain
  coverage, keeping unit tests isolated and order-independent.
- **OpenSpec**: 1 new spec, 10 modified deltas, 1 retired (`project-worktree`).
- **Flagged for slice 2**: the in-memory id→path index becomes a persisted `state.db` table;
  per-entity baseline snapshots (for reconcile) are not introduced here.
- **Clean cutover**: existing `tillerd.db` domain rows are discarded (pre-v1, dev-only).
