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
- [ ] 6.3 Add keyboard navigation (Tab/Shift+Tab to focus items, Enter to invoke)
- [x] 6.4 Style both menus consistently with shadcn primitives and design tokens

## 7. UI: Drag-Drop Support

- [ ] 7.1 Add `draggable="true"` to project headers and session rows
- [ ] 7.2 Implement `dragstart` handler to store source ID in dataTransfer
- [ ] 7.3 Implement `dragover` handler to show drop-target indicator (visual feedback)
- [ ] 7.4 Implement `drop` handler to call `reorder_project()` or `reorder_session()` and update order
- [ ] 7.5 Prevent cross-project session drag (reject drop if target project differs)
- [ ] 7.6 Prevent drag of Unfiled project below named projects

## 8. UI: SessionSidebar Integration

- [x] 8.1 Add state: `editingId` to track which project/session is in edit mode
- [x] 8.2 Wire up `onDoubleClick` on project names to activate inline rename (set editingId)
- [ ] 8.3 Wire up `onDoubleClick` on session names to activate inline rename
- [x] 8.4 Add handlers: `handleRenameProject(id, newName)` and `handleRenameSession(id, newName)`
- [x] 8.5 Add handlers: `handleDeleteProject(id)` and `handleDeleteSession(id)` with confirmation dialogs (shadcn AlertDialog)
- [ ] 8.6 Add handlers: `handleReorderProject(id, newIndex)` and `handleReorderSession(id, newIndex)`
- [x] 8.7 Wire context menu handlers to activate inline rename or delete
- [x] 8.8 Call `refresh()` after each operation to sync with backend

## 9. E2E Tests: Inline Rename

- [x] 9.1 Write spec test: "Rename project by double-clicking and pressing Enter"
- [x] 9.2 Write spec test: "Cancel project rename by pressing Escape"
- [ ] 9.3 Write spec test: "Empty project name is rejected"
- [ ] 9.4 Write spec test: "Rename session by double-clicking and pressing Enter"
- [ ] 9.5 Write spec test: "Cancel session rename by pressing Escape"
- [ ] 9.6 Write spec test: "Empty session name reverts to session ID prefix"

## 10. E2E Tests: Delete

- [x] 10.1 Write spec test: "Delete project after confirming dialog"
- [ ] 10.2 Write spec test: "Cancel project deletion"
- [ ] 10.3 Write spec test: "Deleted project and its sessions vanish immediately"
- [ ] 10.4 Write spec test: "Navigating away from a deleted project"
- [ ] 10.5 Write spec test: "Delete session after confirming dialog"
- [ ] 10.6 Write spec test: "Cancel session deletion"
- [ ] 10.7 Write spec test: "Deleted session vanishes immediately"
- [ ] 10.8 Write spec test: "Navigating away from a deleted session"

## 11. E2E Tests: Reorder

- [ ] 11.1 Write spec test: "Drag project to new position within the sidebar"
- [ ] 11.2 Write spec test: "Project order persists after app restart"
- [ ] 11.3 Write spec test: "Drag project between Unfiled and named projects" (should fail / prevent)
- [ ] 11.4 Write spec test: "Drag session to new position within the project"
- [ ] 11.5 Write spec test: "Session order persists after app restart"
- [ ] 11.6 Write spec test: "Cannot drag session across projects"
- [ ] 11.7 Write spec test: "New session appears at the end of the list"

## 12. E2E Tests: Context Menu

- [x] 12.1 Write spec test: "Right-click project opens context menu"
- [ ] 12.2 Write spec test: "Right-click session opens context menu"
- [ ] 12.3 Write spec test: "Context menu actions invoke correctly (rename, archive, delete, open)"
- [ ] 12.4 Write spec test: "Context menu closes on outside click"
- [ ] 12.5 Write spec test: "Context menu is keyboard-accessible (Tab, Arrow, Enter)"

## 13. Integration & Verification

- [ ] 13.1 Run `bun run verify` locally (format, types, lint, tests, e2e) until green
- [ ] 13.2 Verify sidebar loads and displays projects/sessions in sort_order (mixed NULL/populated)
- [ ] 13.3 Smoke test: rename, delete, reorder operations work end-to-end
- [ ] 13.4 E2E suite passes on macOS dev environment
- [ ] 13.5 E2E suite passes on Linux CI (GitHub Actions)
