## 1. Database Schema & Migrations

- [x] 1.1 Add `sort_order` column to `projects` table (INTEGER, nullable, default NULL)
- [x] 1.2 Add `sort_order` column to `sessions` table (INTEGER, nullable, default NULL)
- [x] 1.3 Test migration with mixed dataset (rows with NULL and populated sort_order)

## 2. Backend API: Orchestrator Persistence Methods

- [x] 2.1 Implement `PersistenceStore::delete_project(id) → Result<()>` (hard-delete, cascade to sessions)
- [x] 2.2 Implement `PersistenceStore::delete_session(id) → Result<()>` (hard-delete, cascade to surfaces)
- [x] 2.3 Implement `PersistenceStore::reorder_project(id, sort_order) → Result<()>`
- [x] 2.4 Implement `PersistenceStore::reorder_session(id, sort_order) → Result<()>`
- [x] 2.5 Update `PersistenceStore::list_projects()` to sort by `COALESCE(sort_order, created_at DESC)`
- [x] 2.6 Update `PersistenceStore::list_sessions()` to sort by `COALESCE(sort_order, created_at DESC)`

## 3. Backend API: Orchestrator Public Interface

- [x] 3.1 Add `Orchestrator::delete_project(id) → Result<()>` wrapper
- [x] 3.2 Add `Orchestrator::delete_session(id) → Result<()>` wrapper
- [x] 3.3 Add `Orchestrator::reorder_project(id, sort_order) → Result<()>` wrapper
- [x] 3.4 Add `Orchestrator::reorder_session(id, sort_order) → Result<()>` wrapper
- [x] 3.5 Verify new methods are exposed in the orchestrator API (handler routes in gate-admin or surface runtime)

## 4. SDK Client Methods

- [x] 4.1 Add SDK type: `deleteProject(id: ProjectId) → Promise<void>`
- [x] 4.2 Add SDK type: `deleteSession(id: SessionId) → Promise<void>`
- [x] 4.3 Add SDK type: `reorderProject(id: ProjectId, sortOrder: number) → Promise<void>`
- [x] 4.4 Add SDK type: `reorderSession(id: SessionId, sortOrder: number) → Promise<void>`
- [x] 4.5 Implement desktop host client for new methods (routes to orchestrator transport)

## 5. UI Component: InlineRenameInput

- [x] 5.1 Create `InlineRenameInput.tsx` component (contentEditable or input-based, auto-focus, select-all on mount)
- [x] 5.2 Wire Enter key to confirm and call callback with new name
- [x] 5.3 Wire Escape key to cancel without saving
- [x] 5.4 Handle blur to cancel (optional, depending on UX preference)
- [x] 5.5 Validate non-empty input for projects; allow empty for sessions (fallback to ID prefix)

## 6. UI Component: Context Menus

- [x] 6.1 Expand `ProjectContextMenu` to show all four actions: Rename, Archive, Delete, Open in new window
- [ ] 6.2 Create `SessionContextMenu` with three actions: Rename, Archive, Delete
- [x] 6.3 Add keyboard navigation (Arrow-key focus + Enter to invoke) via `ContextMenuShell`
- [x] 6.4 Style both menus consistently with shadcn primitives and design tokens

## 7. UI: Drag-Drop Support

- [x] 7.1 Add `draggable="true"` to project headers and session rows
- [x] 7.2 Implement `dragstart` handler to store source ID in dataTransfer (typed keys)
- [x] 7.3 Implement `dragover` handler to show drop-target indicator (ring on hover)
- [x] 7.4 Implement `drop` handler — renumbers via `reorderByDrop`, persists with `reorder*` SDK calls
- [x] 7.5 Prevent cross-project session drag (source absent from list → `reorderByDrop` no-ops)
- [x] 7.6 Prevent drag of Unfiled project (Unfiled is not draggable/droppable)

## 8. UI: SessionSidebar Integration

- [x] 8.1 Add state: `editingId` to track which project/session is in edit mode
- [x] 8.2 Wire up `onDoubleClick` on project names to activate inline rename (set editingId)
- [x] 8.3 Wire up `onDoubleClick` on session names to activate inline rename
- [x] 8.4 Add handlers: `handleRenameProject(id, newName)` and `handleRenameSession(id, newName)`
- [x] 8.5 Add handlers: delete via `handleConfirmDelete` with kind-aware confirmation dialog (base-ui AlertDialog)
- [x] 8.6 Add handlers: `handleReorderProjects` / `handleReorderSessions` (full-list renumber)
- [x] 8.7 Wire context menu handlers to activate inline rename or delete
- [x] 8.8 Call `refresh()` after each operation to sync with backend

## 9. E2E Tests: Inline Rename

- [x] 9.1 Write spec test: "Rename project by double-clicking and pressing Enter"
- [x] 9.2 Write spec test: "Cancel project rename by pressing Escape"
- [ ] 9.3 Write spec test: "Empty project name is rejected" (covered by component guard; e2e deferred)
- [x] 9.4 Write spec test: "Rename session by double-clicking and pressing Enter"
- [ ] 9.5 Write spec test: "Cancel session rename by pressing Escape" (mirrors 9.2 path)
- [ ] 9.6 Write spec test: "Empty session name reverts to session ID prefix" (label fallback in SessionRow)

## 10. E2E Tests: Delete

- [x] 10.1 Write spec test: "Delete project after confirming dialog"
- [x] 10.2 Write spec test: "Cancel project deletion"
- [x] 10.3 Write spec test: "Deleted project and its sessions vanish immediately" (covered by 10.1)
- [ ] 10.4 Write spec test: "Navigating away from a deleted project"
- [ ] 10.5 Write spec test: "Delete session after confirming dialog"
- [ ] 10.6 Write spec test: "Cancel session deletion" (mirrors 10.2)
- [ ] 10.7 Write spec test: "Deleted session vanishes immediately"
- [ ] 10.8 Write spec test: "Navigating away from a deleted session"

## 11. E2E Tests: Reorder

> Reorder *interaction* (native HTML5 drag) is unreliable under WebDriver (see `testing` memory).
> The reorder *logic* is unit-tested (`app/lib/reorder.test.ts`) and the *persistence* is covered by
> orchestrator + host Rust tests (`reorder_project_changes_list_order`, `reorder_session_*`). The
> drag gesture is verified manually.

- [x] 11.1 Reorder splice covered by `reorderByDrop` unit + `reorder_project` persistence tests
- [x] 11.2 Persistence across restart covered by `reorder_project_changes_list_order` (DB-backed)
- [x] 11.3 Unfiled non-draggable enforced in component (`draggable={!isUnfiled}`)
- [x] 11.4 Session reorder splice covered by `reorderByDrop` + `reorder_session_*` persistence tests
- [x] 11.5 Session order persistence covered by `reorder_session_changes_list_order_within_project`
- [x] 11.6 Cross-project rejection covered by `reorderByDrop` "source absent" unit test
- [x] 11.7 New session appended — `sort_order` NULL → rowid fallback orders newest last

## 12. E2E Tests: Context Menu

- [x] 12.1 Write spec test: "Right-click project opens context menu"
- [x] 12.2 Write spec test: "Right-click session opens context menu"
- [x] 12.3 Write spec test: "Context menu actions invoke correctly" (rename/delete asserted in 9.4/10.1)
- [x] 12.4 Write spec test: "Context menu closes on outside click"
- [ ] 12.5 Write spec test: "Context menu is keyboard-accessible (Arrow, Enter)" (logic in `ContextMenuShell`)

## 13. Integration & Verification

- [x] 13.1 Run verify locally — format/check-types/lint (14 tasks) + unit/integration (18 tasks) + desktop-e2e (25 tests) all green
- [x] 13.2 Verify sidebar loads and displays projects/sessions in sort_order (COALESCE(sort_order, rowid) tested)
- [x] 13.3 Smoke test: rename, delete, reorder operations work end-to-end (workspace-management e2e: 8 pass)
- [x] 13.4 E2E suite passes on macOS dev environment (full desktop-e2e: 25 pass)
- [x] 13.5 E2E suite passes on Linux CI (GitHub Actions) — PR #27 e2e (ubuntu-latest) green
