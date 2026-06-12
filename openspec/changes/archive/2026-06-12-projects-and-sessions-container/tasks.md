## 1. Store trait: project CRUD methods

- [x] 1.1 Add typed errors: `ProjectNotFound`, `ProjectIsUnfiled`, `ProjectNotArchived`, `SessionNotFound`, `SessionNotArchived`, `SurfaceConflict` to `OrchestratorError`
- [x] 1.2 Add `NewProject` / `TitleSource` types and extend `Session` with `title_source` + `created_at` fields in `persistence/mod.rs`
- [x] 1.3 Add `create_project`, `rename_project`, `list_projects`, `archive_project`, `hard_delete_project` to `Store` trait
- [x] 1.4 Implement all project methods on `SqliteStore` (with Unfiled guard and atomic cascade)
- [x] 1.5 Implement all project methods on `InMemoryStore` (test double)

## 2. Store trait: session CRUD methods

- [x] 2.1 Add `create_session` (extended: accepts `project_id`, `title_source`, `title`), `rename_session`, `list_sessions`, `archive_session`, `hard_delete_session` to `Store` trait
- [x] 2.2 Implement all session methods on `SqliteStore` (with cascade on archive)
- [x] 2.3 Implement all session methods on `InMemoryStore`

## 3. Store trait: add/remove surface from session + layout persistence

- [x] 3.1 Add `add_surface_to_session`, `remove_surface_from_session`, `set_session_layout`, `get_session_layout` to `Store` trait
- [x] 3.2 Implement on `SqliteStore`
- [x] 3.3 Implement on `InMemoryStore`

## 4. Schema migration: title_source enum expansion

- [x] 4.1 Add migration v2 that widens `title_source` CHECK to include `agent-title`, `branch`, `both` (SQLite: recreate table or use `WITHOUT ROWID` alter pattern)

## 5. Surface-creation flow inversion

- [x] 5.1 Add `session_id` parameter to `SurfaceApi::create_terminal_surface` and `create_agent_surface`; remove implicit session creation

## 6. SDK: workspace client types and methods

- [x] 6.1 Add `PROJECT_CREATE`, `PROJECT_RENAME`, `PROJECT_LIST`, `PROJECT_ARCHIVE` command constants and request/response TS types to `packages/sdk/src/orchestrator/`
- [x] 6.2 Add `SESSION_CREATE`, `SESSION_RENAME`, `SESSION_LIST`, `SESSION_ARCHIVE`, `SESSION_LAYOUT_SET`, `SESSION_LAYOUT_GET` command constants and types
- [x] 6.3 Extend `OrchestratorClient` interface and `createOrchestratorClient` implementation with project + session methods

## 7. UI: session sidebar update

- [x] 7.1 Update `SessionSidebar` to show project-grouped sessions with inferred titles; hide empty Unfiled group; show Unfiled last
- [x] 7.2 Add new-project control and new-session-per-project control to sidebar
- [x] 7.3 Add archive control per session row

## 8. UI: layout persistence migration

- [x] 8.1 Refactor `usePanelTree` to persist layout to DB via SDK instead of localStorage; discard legacy localStorage key on init
