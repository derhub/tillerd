## Why

Sessions currently start as bare containers with no declarative description of what surfaces
to create or how — every surface is launched ad-hoc. The launch system gives each project a
versioned, declarative template that drives surface creation, making workflows repeatable and
composable without touching surface code.

## What Changes

- **New**: a versioned launch spec — an ordered list of launch items stored as a JSON blob on
  the project template row; lazy per-load migration upgrades older blobs in-memory and writes
  the result back (ADR-0021, ADR-0023).
- **New**: command library — a global, durable table of named commands; prebuilt entries
  (login shell, agent CLI preset) seeded at first run; user-added entries created via API.
- **New**: launch items — each item carries: `target` (surface kind), `placement` (named
  region), `command` (library ref or inline `cli`/`args`/`env`), `pre`/`post` scripts,
  `autoSpawn` scripts, and an optional `worktree` step (create → cd → run).
- **New**: template → instance flow — instantiating a project template writes a
  `spec_json` snapshot onto the session row; the session may diverge from the template
  thereafter.
- **New**: worktree ownership — a `worktree` table row is created by the worktree launch
  step and associated with its project; surfaces reference the worktree.
- **New**: Tauri IPC bridge for workspace commands — `workspace_host.rs` command handlers
  wiring the UI SDK to the Rust store (project/session/layout CRUD).
- **Modified**: `surface_host.rs` — surface creation accepts a placement hint from the
  launch item and stores it on the surface row.

## Already Built (do not re-implement)

The following is complete and must not be touched unless a task explicitly modifies it:

- **DB schema v1 + v2 migration**: all 9 tables (`project`, `worktree`, `launch_template`,
  `session`, `surface`, `command`, `secret_ref`, `setting`, `meta`); `title_source` CHECK
  expanded to `agent-title|branch|both|custom` in migration v2; Unfiled project seeded.
  File: `crates/orchestrator/src/persistence/schema.rs`.
- **Store trait — 24 methods**: full project/session/surface/layout CRUD including
  cascading soft-delete and `add/remove_surface_from_session`.
  File: `crates/orchestrator/src/persistence/mod.rs`.
- **`SqliteStore` — all 24 methods implemented**: project CRUD (with Unfiled guard and
  atomic cascade), session CRUD, surface CRUD, layout get/set. Full test coverage.
  File: `crates/orchestrator/src/persistence/sqlite.rs`.
- **`InMemoryStore` — all 24 methods implemented**: test double matching `SqliteStore`
  behavior. File: `crates/orchestrator/src/persistence/memory.rs`.
- **Persistence types**: `ProjectId` (with `UNFILED` const), `SessionId`, `SurfaceId`,
  `Project`, `Session` (with `title_source: TitleSource`, `created_at`), `Surface`
  (with `correlation_id()`), `NewProject`, `NewSession`, `NewSurface`, `SourceKind`,
  `SurfaceKind`, `TitleSource`.
  File: `crates/orchestrator/src/persistence/mod.rs`.
- **`SurfaceApi` with `session_id` parameter**: `create_terminal_surface(session_id, ...)`,
  `create_agent_surface(session_id, ...)`, `remove()`. No implicit session creation.
  File: `crates/orchestrator/src/surface/api.rs`.
- **SDK workspace client**: command constants (`PROJECT_CREATE`, `PROJECT_RENAME`,
  `PROJECT_LIST`, `PROJECT_ARCHIVE`, `SESSION_CREATE`, `SESSION_RENAME`, `SESSION_LIST`,
  `SESSION_ARCHIVE`, `SESSION_LAYOUT_SET`, `SESSION_LAYOUT_GET`), typed request/response
  interfaces, `WorkspaceClient` interface, `OrchestratorClient` extending it.
  Files: `packages/sdk/src/orchestrator/workspace.ts`,
  `packages/sdk/src/orchestrator/client.ts`.
- **Panel tree engine**: `PanelNode` type union, `DEFAULT_LAYOUT` (sidebar + terminal +
  diff), full mutation API (`splitNode`, `closeNode`, `setContentNode`,
  `setDisplayModeNode`, `setActiveTabNode`), serialization/deserialization.
  File: `apps/ui/app/lib/panelTree.ts`.
- **`usePanelTree` hook**: reads layout from DB via `client.getSessionLayout` on mount
  (discards legacy localStorage key), persists on every mutation via
  `client.setSessionLayout`. File: `apps/ui/app/lib/usePanelTree.ts`.
- **`SessionSidebar` component**: project-grouped sessions (named projects first,
  Unfiled last, hidden when empty), create-project dialog, per-project create-session
  button, per-session archive button.
  File: `apps/ui/app/components/SessionSidebar.tsx`.

## Net-New Work for 0.0.5

### 1. Tauri IPC bridge — workspace commands

`workspace_host.rs` does not exist. The SDK client is complete; the store is complete;
nothing connects them at the Tauri layer.

- Create `apps/desktop/src-tauri/src/workspace_host.rs` with `#[tauri::command]` handlers
  for: `project_create`, `project_rename`, `project_list`, `project_archive`,
  `session_create`, `session_rename`, `session_list`, `session_archive`,
  `session_layout_get`, `session_layout_set`.
- Register all 10 handlers in `apps/desktop/src-tauri/src/lib.rs` `generate_handler![]`.
- Map `OrchestratorError` variants to serializable error responses (same pattern as
  `surface_host.rs`).

### 2. Launch spec — versioned JSON blob + lazy migration engine

The `launch_template` table and `spec_json`/`spec_version` columns exist in the schema
but no code reads or writes them.

- Define `LaunchSpec` (version field + ordered `Vec<LaunchItem>`) in
  `crates/orchestrator/src/launch/spec.rs`.
- Define `LaunchItem` fields: `target: SurfaceKind`, `placement: Option<String>`,
  `command: CommandRef`, `pre`/`post`/`autoSpawn: Vec<String>`,
  `worktree: Option<WorktreeStep>`.
- Implement the lazy migration engine: `migrate(blob: &str, from: u32) -> Result<LaunchSpec>`;
  each step is a pure function from vN JSON to vN+1 JSON; write-back on load.
- No migration steps are needed for v1 yet — the engine only needs to handle the current
  version and be extendable.

### 3. Command library — CRUD + seed

The `command` table exists in the schema; no code reads or writes it.

- Add store methods: `seed_commands`, `list_commands`, `create_command`, `get_command`,
  `delete_command` to the `Store` trait and both implementations.
- Seed two prebuilt entries at first run (inside `SqliteStore::open` when schema version
  is freshly initialized): login shell (`$SHELL -l`) and agent CLI preset
  (command name + args from `contracts-rs`).
- Add SDK command constants and Tauri handlers for command library CRUD.

### 4. Launch item executor

No launch-item execution exists anywhere.

- Implement `LaunchExecutor::run(spec: &LaunchSpec, session_id: SessionId)` in
  `crates/orchestrator/src/launch/executor.rs`.
- For each `LaunchItem` in order: resolve command ref against the library, apply the
  worktree step if present, call `SurfaceApi::create_terminal_surface` or
  `create_agent_surface` with the resolved `placement` string, run `pre` scripts before
  and `post` scripts after the command starts.
- Best-effort failure model: a failed item writes an error status to its surface row;
  remaining items proceed.

### 5. Worktree step — create → cd → run

The `worktree` table exists; no code creates worktree rows or runs `git worktree add`.

- Add store methods: `create_worktree`, `list_worktrees`, `archive_worktree` to `Store`
  trait and both implementations.
- Implement `WorktreeStep::execute(project_id, branch, path) -> Result<WorktreeRow>` in
  `crates/orchestrator/src/launch/worktree.rs`: runs `git worktree add <path> <branch>`,
  creates the DB row, returns the path for the launch item's `cwd`.
- Surface creation passes the `worktree_id` when a worktree step produced a row.

### 6. Named-region placement resolver

`surface.placement` is stored but never read. The panel tree engine has no concept of
named regions.

- Define a minimal named-region vocabulary in `apps/ui/app/lib/panelTree.ts`:
  `"center"` (maps to the primary content leaf) and `"side"` (maps to the sidebar leaf).
- Add `resolveRegion(tree: PanelNode, region: string): string | null` returning the
  panel `id` for the named region, or `null` if not found.
- `AppShell` uses `resolveRegion` when placing a new surface from a launch item.
- The `placement` field on `LaunchItem` is `Option<String>`; unknown values fall back to
  the default content panel.

### 7. Template → instance flow

No code snapshots a `launch_template` spec onto a session at creation time.

- Extend `NewSession` with `template_id: Option<LaunchTemplateId>`.
- When `template_id` is set, `SqliteStore::create_session` reads the template's
  `spec_json`/`spec_version` and writes them onto the new session row atomically.
- After creation, the session's `spec_json` is independent of the template (divergence
  allowed).

## Capabilities

### New Capabilities

- `launch-spec`: versioned JSON blob schema (fields, constraints, nullable defaults);
  lazy migration engine that upgrades stored specs from vN to vN+1 on load.
- `command-library`: CRUD for global named commands; prebuilt seed entries; library-ref
  resolution at launch time.
- `launch-item`: per-item execution contract — target/placement/command resolution,
  pre/post script execution, autoSpawn lifecycle, worktree step (create → cd → run);
  best-effort failure model (failed item → error-state surface, others proceed).
- `template-instance`: template snapshot written to session on instantiation; session
  divergence after creation; project owns template, session owns its copy.
- `project-worktree`: worktree row lifecycle — created by the worktree step, owned by
  the project, referenced by a surface; archive-over-delete applies.
- `workspace-ipc`: Tauri command handlers bridging the SDK workspace client to the Rust
  store for project/session/layout CRUD.

### Modified Capabilities

- `workspace-persistence`: store gains command library methods and worktree CRUD;
  `NewSession` gains `template_id`; `SqliteStore::create_session` gains template
  snapshot logic.
- `surface-runtime`: surface creation accepts a placement hint (named region) from the
  launch item rather than being ad-hoc.

## Impact

- **Rust**: `crates/orchestrator` — new `launch/` module (spec, executor, worktree);
  store trait extensions for command library and worktree CRUD.
- **Tauri**: new `workspace_host.rs`; updated `lib.rs` handler registration.
- **DB**: no new tables beyond what schema v1 already defines; `launch_template` and
  `command` tables are populated for the first time; no schema migration needed.
- **SDK / contracts**: command library types added to SDK if exposed to UI.
- **UI**: named-region resolver in `panelTree.ts`; `AppShell` placement wiring.
- **No new crates**: all work is in-crate per house rules.
