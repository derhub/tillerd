## Why

Workspace CRUD (create, rename, delete, reorder) is fundamental for daily usability. Projects and sessions are the primary organizational unit in the desktop app; users need fast, discoverable ways to manage them without modal dialogs. Inline rename (double-click to edit), context menus, and drag-to-reorder are standard patterns users expect. Completing these operations ships the second-half of 0.0.4's CRUD layer (backend rename/delete landed; this lands the UI). Unblocks 0.0.14 final polish and ships a minimally-usable workspace that doesn't feel skeletal.

## What Changes

- **Inline rename**: Double-click a project or session row in the sidebar to edit its name in-place. Enter confirms, Escape cancels. Calls the existing backend `rename_project` / `rename_session` API.
- **Delete with confirmation**: Right-click context menu → "Delete"; shadcn AlertDialog confirmation. Hard-delete cascades to surfaces (PTYs terminated). Distinct from archive (soft-delete, PTY preserved via `archived_at`).
- **Session reorder**: Drag sessions within a project to reorder; `sort_order` column persists order across restarts. Calls new backend `reorder_session` API.
- **Project reorder**: Drag projects in sidebar; `sort_order` on `projects` table. New backend API.
- **Context menus**: Right-click on project and session rows surfaces full action list (rename, archive, delete, open in new window for projects). Replaces the lightweight 0.0.11 "open in new window only" menu.
- **E2E**: rename, delete, reorder flows tested on macOS and Linux CI via tauri-webdriver.

## Capabilities

### New Capabilities

- `project-session-inline-rename`: Double-click to edit project or session name in-place. Keyboard shortcuts: Enter to confirm, Escape to cancel.
- `project-session-delete`: Hard-delete projects and sessions with shadcn confirmation dialog. Cascades to surfaces (terminates PTYs). Delete is distinct from archive.
- `workspace-reorder`: Drag-to-reorder projects and sessions within the sidebar. Persisted via new `sort_order` column (migration) and new orchestrator API `reorder_project` / `reorder_session`.
- `sidebar-context-actions`: Right-click context menu on project and session rows surfaces full action set (rename, archive, delete, open in new window). Keyboard-accessible via Tab/Enter.

### Modified Capabilities

- None. No existing spec-level behavior changes; all additions are new.

## Impact

**UI**: `apps/ui/app/components/SessionSidebar.tsx` expanded (inline rename state, context menu handlers, drag handlers). New component `InlineRenameInput` for edit-in-place. `ProjectContextMenu` expanded to include delete/archive/rename/open-new-window.

**Backend**: 
- Orchestrator API: new methods `reorder_project(id, sort_order)` / `reorder_session(id, sort_order)`. Existing `rename_project`, `rename_session`, `archiveSession` remain; new `delete_project` / `delete_session` added.
- Persistence: `projects` and `sessions` table schema migration adds `sort_order` (nullable, default NULL → append to end on create).

**E2E**: New desktop-e2e specs in `tests/desktop-e2e/` covering rename, delete, reorder flows.

**Dependencies**: No new npm deps. Drag library (if not already imported) — check existing `@dnd-kit` or `react-dnd` usage; if neither, add lightweight drag polyfill or native HTML5 drag API.
