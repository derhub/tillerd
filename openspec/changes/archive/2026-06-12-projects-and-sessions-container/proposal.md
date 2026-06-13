## Why

The workspace has a fully-defined schema (ADR-0023) and a two-level id model (ADR-0020/ADR-0023), but the orchestrator exposes only 8 store operations (all surface-oriented) and no project or session management. The `SurfaceApi` creates an unnamed, unowned session for every surface call — there is no way to name, group, list, or resume sessions from the UI. Version 0.0.4 closes that gap: complete CRUD for the project -> session -> surface hierarchy, layout persistence per session, and an archive lifecycle, so the UI has a real workspace to drive.

## What Changes

- Add orchestrator store operations for project management: create (blank / local-dir / git-repo / git-worktree sources), name inference, rename, list, archive (soft-delete cascading to child sessions and surfaces), hard-delete of archived projects.
- Add orchestrator store operations for session management: create (named, under a project), title inference (agent-title / branch / both, or custom), rename, list (optionally filtered by project), open/switch, add surface, remove surface, archive (soft-delete cascading to surfaces), hard-delete of archived sessions, resume after restart.
- Add layout persistence: store and restore the panel-tree layout (`layout_json`) per session so the arrangement survives a restart. Currently the panel tree is persisted only to browser `localStorage` as a single global blob; this change migrates that to a per-session DB column.
- Add archive lifecycle: soft-delete (`deleted_at`) marks items archived and cascades to children; hard-delete acts only on already-archived items; worktree directories are never removed by the orchestrator.
- Enforce the built-in "Unfiled" project at the operations layer (already seeded in migration v1; this change prevents deletion and ensures all orphan sessions resolve to it).
- Invert the surface-creation flow in `SurfaceApi`: callers supply a `session_id` rather than the API creating an implicit unnamed session per call.
- Expose the new workspace operations through the SDK client so the UI can call them.

## Already Built

The following exist today and are not re-built by this change:

- **Database schema** — all 9 tables including `project`, `session` (with `layout_json`, `title_source`), `surface` (with `deleted_at`), seeded Unfiled row (`ProjectId::UNFILED = "00000000-0000-0000-0000-000000000000"`), foreign-key constraints, soft-delete columns on all lifecycle tables. (`crates/orchestrator/src/persistence/schema.rs`)
- **Persistence types** — `ProjectId`, `SessionId`, `SurfaceId`, `SourceKind`, `SurfaceKind`, `Project`, `Session`, `NewSession`, `Surface`, `NewSurface`, `Surface::correlation_id()`. (`crates/orchestrator/src/persistence/mod.rs`)
- **Store trait skeleton** — 8 implemented methods: `schema_version`, `get_project`, `create_session`, `create_surface`, `get_surface`, `list_resumable_surfaces`, `update_surface_status`, `soft_delete_surface`. (`crates/orchestrator/src/persistence/sqlite.rs` and `memory.rs`)
- **Two-level id model** — `SurfaceId` equals the daemon PTY id and is the shared correlation id; `SessionId` is the product-layer container id, never visible to backends. Tests verify the separation.
- **Surface runtime and API** — `SurfaceApi`, `create_terminal_surface`, `create_agent_surface`, `input`, `resize`, `resume_all`, `detach`, `remove`. (`crates/orchestrator/src/surface/`)
- **Panel tree engine** — `PanelNode` type system, serialization/deserialization, all mutation functions, `usePanelTree` hook (currently backed by a single `localStorage` key). (`apps/ui/app/lib/panelTree.ts`, `usePanelTree.ts`)
- **Sidebar skeleton** — `SessionSidebar` component renders a flat session list by truncated id and cwd basename; has a "New session" button. (`apps/ui/app/components/SessionSidebar.tsx`)

## Capabilities

### New Capabilities

- `project-management`: Create a project from one of four sources (blank, local-dir, git-repo, git-worktree) with name inference and custom override; rename; list; open; archive (soft-delete cascading to child sessions and surfaces); hard-delete archived projects. Unfiled project always present and non-deletable.
- `session-container`: Create a session under a project; infer title from agent-title, branch, or both (user-selectable strategy); set or override a custom title; list sessions (optionally filtered by project); open/switch to a session; add a surface to a session; remove a surface from a session; archive a session (soft-delete cascading to surfaces); hard-delete archived sessions; resume session state after orchestrator restart.
- `layout-persistence`: Store the panel-tree layout as a versioned JSON blob per session (`layout_json`); restore layout on session open; update layout on any panel mutation. Replaces the current global `localStorage` layout.

### Modified Capabilities

- `surface-runtime`: Surface creation now requires a caller-supplied `session_id`; the `SurfaceApi` no longer mints an implicit session per call. A surface record is created when added to a session and soft-deleted (not PTY-terminated) when removed via session container operations. PTY semantics are unchanged.
- `ui-session-sidebar`: The sidebar currently shows a flat list of active sessions by truncated id and working directory. This change requires the sidebar to show project-grouped sessions with inferred titles, support the new project/session create and archive actions, and reflect the Unfiled project for ungrouped sessions.

## Impact

- `crates/orchestrator`: new store methods for project/session CRUD, layout get/set, cascading soft-delete, and hard-delete; `SurfaceApi` refactored to accept a caller-supplied `session_id`; `ProjectId::UNFILED` invariant enforced in operations.
- `crates/contracts-rs`: new request/response types for project and session CRUD exposed through the SDK boundary.
- `apps/ui`: sidebar gains project grouping and inferred titles; `usePanelTree` is refactored to persist per-session layout to the DB instead of a global `localStorage` key; session list and project list components added.
- No new crates. No changes to daemon, gate, or memorya.
