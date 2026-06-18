## Context

Today `project` is the top of the persisted tree (ADR-0023): `project -> session ->
surface`, one `tillerd.db`, one main window showing all projects. Window geometry is
persisted for that single main window (`window-state`). The orchestrator owns all
persistence in Rust; the renderer reaches it through the SDK + a host adapter (ADR-0022).

This change inserts a `workspace` tier above `project`. It is additive only — the data
model is frozen at 0.0.6, and prior additive migrations (`sort_order`, notification store)
set the precedent.

## Goals / Non-Goals

**Goals:**

- A `workspace` entity grouping projects, with a single un-deletable Default workspace.
- Strict containment: every project belongs to exactly one workspace.
- A single main window with a switcher; sidebar scoped to the active workspace; a workspace
  detachable into its own window (same detach as project/session/panel).
- Zero-loss migration of existing projects into Default.
- Glossary + ADR + roadmap brought into line.

**Non-Goals:**

- Per-workspace data/credential/path isolation (one `tillerd.db`, no isolation boundary).
- Moving the launch template or any settings scope up to the workspace tier.
- A workspace-level settings tier.
- Many-to-many project membership (a project is in exactly one workspace).
- Changing the session/surface id model (ADR-0023 two-level id is untouched).

## Decisions

### Schema (additive migration `migration_vN`)

```
workspace(id PK, name, sort_order, created_at, updated_at)
project: + workspace_id FK -> workspace(id)   -- new column
```

- Default workspace is a seeded row with a fixed well-known id (mirrors the Unfiled
  project's fixed id).
- Migration steps: create `workspace`, seed Default, add `project.workspace_id`, backfill
  every existing project to Default, then enforce non-null at the application layer
  (column stays nullable at rest only transiently during migration).

### Persistence + orchestrator API (`crates/orchestrator`)

New `PersistenceStore` methods: `create_workspace`, `rename_workspace`,
`list_workspaces`, `reorder_workspace`, `delete_workspace` (reassigns its projects to
Default), `move_project(project_id, workspace_id)`. `list_projects` gains an optional
workspace scope and returns `workspace_id` per row. `Orchestrator` exposes thin wrappers,
following the existing project/session method shape.

### SDK + host (`packages/sdk`, desktop adapter)

SDK gains `createWorkspace/renameWorkspace/listWorkspaces/reorderWorkspace/
deleteWorkspace/moveProject` mirroring the existing client method shape. Detaching a
workspace into its own window reuses the existing multi-window plumbing (`window_open`/
`window_focus` host commands, driven from `apps/ui/app/lib/windows.ts`) — no new host
command and no SDK port. A `workspace` `WindowIntent` joins the existing `detached`/
`project` intents, with a `workspace-<id>` label and `?w=workspace&...` query.

### UI (`apps/ui`)

`SessionSidebar` lists projects scoped to the active workspace (uses the new scoped
`listProjects`). A workspace switcher (new component) lists workspaces from
`listWorkspaces`; selecting one re-scopes the sidebar in place. A workspace can also be
detached into its own window via the existing open-in-new-window affordance (mirrors
project/session/panel detach). The "all projects" assumption in the sidebar becomes
"active workspace's projects".

### Window model

Single main window by default; the switcher swaps the active workspace in place (sidebar
re-scopes). A workspace can additionally be detached into its own window — the same runtime
detach as panel/session/project: `window_open` opens (or focuses an already-open) labelled
webview of the same backend, intent carried in the URL (`?w=workspace&...`), re-attach on
close via a `workspace:reattach` event. Detached windows open at the shared default
geometry; no per-workspace geometry is persisted.

### Docs

`CONTEXT.md`: add the Workspace term; revise Project (drop "named workspace root" + the
"_Avoid_: workspace" line). ADR extends ADR-0023. `ROADMAP.md`: relabel 0.0.12 to
"Project & session management" (done), add 0.0.14 "Workspaces", and bump the existing
"UX/UI (ships the working app)" milestone from 0.0.14 to 0.0.15 (0.0.13 "Command center"
is already shipped, so Workspaces takes the next free slot and the unshipped UX/UI
milestone moves down — UX/UI builds on the window/sidebar model Workspaces defines).

## Risks / Trade-offs

- **Frozen-seam touch (additive):** adds a `project` column + a new table. Additive, not
  breaking; no published contract changes shape.
- **Single window + detach vs window-per-workspace:** chosen single-window-with-detach to
  mirror the existing project/session/panel detach model instead of auto-spawning a window
  per workspace. Reuses the existing `window_open`/`window_focus` machinery, so no new host
  command and no per-workspace geometry persistence.
- **Multi-window assertions under e2e:** tauri-webdriver sees only the active webview, not
  native windows (testing memory). Window open/focus is asserted via a DOM affordance and
  the host command contract, not by driving a second native window. Sidebar scoping and
  switcher selection are DOM-driveable.
- **Migration correctness:** backfill-to-Default is covered by a Rust migration test
  seeding a pre-workspace store and asserting every project lands in Default with sessions
  intact.
