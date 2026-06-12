# Tasks — launch-system

## 1. Launch spec types + migration engine
- [x] 1.1 Write failing tests: parse valid v1 blob with items, empty item list, reject missing-version blob, reject unknown-future-version blob, current-version passes through without write-back, older blob is migrated and written back (launch-spec: versioned schema + lazy migration engine).
- [x] 1.2 Define `LaunchSpec`, `LaunchItem`, `CommandRef`, `WorktreeStep`, `PlacementHint` types in `crates/orchestrator/src/launch/spec.rs`; implement `parse_spec(blob: &str) -> Result<LaunchSpec>` and `migrate(blob: &str, from: u32, to: u32) -> Result<(LaunchSpec, Option<String>)>` — returns a write-back blob only when migration actually ran.

## 2. Command library — store trait extensions
- [x] 2.1 Write failing tests against `InMemoryStore`: prebuilt entries present after `open`, seed is idempotent, CRUD round-trips (list all, get by id, create custom, delete), get unknown id returns not-found, list excludes deleted, library-ref resolution succeeds on known and returns error on unknown (command-library spec).
- [x] 2.2 Add `CommandId`, `Command`, `CommandOrigin`, `NewCommand` types to `crates/orchestrator/src/persistence/mod.rs`; add `create_command`, `get_command`, `list_commands`, `delete_command`, `seed_commands` methods to the `Store` trait; implement in `SqliteStore` and `InMemoryStore`; seed on store open.

## 3. Worktree store trait extensions
- [x] 3.1 Write failing tests against `InMemoryStore`: worktree row created and survives restart (serialized via sqlite round-trip for sqlite tests), archived worktree absent from list, surface records worktree reference (project-worktree spec: worktree row lifecycle + ownership).
- [x] 3.2 Add `WorktreeId`, `Worktree`, `NewWorktree` types to `crates/orchestrator/src/persistence/mod.rs`; add `create_worktree`, `list_worktrees`, `archive_worktree` methods to the `Store` trait; implement in `SqliteStore` and `InMemoryStore`.

## 4. Surface schema extensions — placement + worktree_id
- [x] 4.1 Write failing tests: surface created with placement string and worktree reference stores both fields; surface created without stores null; round-trip via `get_surface` (workspace-persistence + surface-runtime: placement + worktree reference).
- [x] 4.2 Extend `NewSurface` with `placement: Option<String>` and `worktree_id: Option<WorktreeId>`; extend `Surface` with the same fields; update `create_surface` SQL to insert both; update `row_to_surface` to read them; update `InMemoryStore::create_surface` accordingly. Update all existing call-sites that construct `NewSurface`.

## 5. Session template reference — persistence
- [x] 5.1 Write failing tests: `NewSession` without template produces null spec fields; `NewSession` with valid template copies spec blob + version atomically; template update after session creation does not affect the session; session spec update does not affect the template (template-instance + workspace-persistence specs).
- [x] 5.2 Add `template_id: Option<LaunchTemplateId>` to `NewSession`; add `spec_json: Option<String>` and `spec_version: Option<u32>` to `Session`; add `LaunchTemplateId`, `LaunchTemplate`, `NewLaunchTemplate` types; add `create_launch_template`, `get_launch_template`, `set_launch_template_spec`, `get_session_spec` methods to `Store`; implement in both stores; ensure session creation copies template spec atomically in `SqliteStore`.

## 6. Worktree step executor
- [x] 6.1 Write failing tests (mock git): worktree step creates directory via `git worktree add`, writes worktree row to store, returns path as cwd; step failure on git error returns typed error without writing row (project-worktree: worktree step create/cd/run).
- [x] 6.2 Implement `WorktreeStep::execute(project_id, branch, path, store) -> Result<Worktree>` in `crates/orchestrator/src/launch/worktree.rs`; runs `git worktree add <path> <branch>` via `std::process::Command`; on success writes row; on failure returns `OrchestratorError::WorktreeStepFailed`.

## 7. Launch item executor
- [x] 7.1 Write failing tests: items run in order; pre-script failure skips surface creation for that item but continues to next; post-scripts run after surface starts; auto-spawn injected on initial attach; failed item error recorded on surface row; placement hint stored (launch-item spec).
- [x] 7.2 Implement `LaunchExecutor::run(spec, session_id, store, surface_api) -> Vec<LaunchItemResult>` in `crates/orchestrator/src/launch/executor.rs`; best-effort model: collect errors per item, continue on failure; pre/post scripts via `process-launch`; placement + worktree_id forwarded to `NewSurface`.

## 8. Workspace IPC — Tauri command handlers
- [x] 8.1 Write failing tests (in-process handler invocation with a fake store): project-create delegates to store; session-list returns non-archived sessions with optional project filter; session-layout set/get round-trip; not-found error serialized; unfiled guard serialized; list-commands returns all entries; create-command persists entry (workspace-ipc spec).
- [x] 8.2 Add Tauri command handlers for `project_create`, `session_list`, `session_layout_set`, `session_layout_get`, `command_list`, `command_create` in `apps/desktop/src-tauri/src/commands/workspace.rs` (or the existing workspace commands file); map `OrchestratorError` to serializable responses.
