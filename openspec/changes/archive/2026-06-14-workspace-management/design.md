## Context

The sidebar currently displays a read-mostly view of projects and sessions grouped hierarchically. Projects are fetched via `listProjects()` and sessions via `listSessions()`. The UI is built on `SessionSidebar.tsx` which groups sessions by project and renders project headers and session rows.

Backend support for rename/delete is partially complete:
- **Existing**: `rename_project()`, `rename_session()`, `archiveSession()` methods in the orchestrator API and persistence layer
- **Missing**: `delete_project()`, `delete_session()`, `reorder_project()`, `reorder_session()` methods; `sort_order` column in the database schema

Current limitations:
- Sessions are displayed in creation order (no sort_order persistence)
- Projects have no reorder capability
- Context menu in 0.0.11 has only "Open in new window"
- Inline rename/delete are not implemented (awaiting this milestone per roadmap comment)

## Goals / Non-Goals

**Goals:**

- Implement inline rename (double-click, Enter/Escape) for projects and sessions
- Implement hard-delete with confirmation for projects and sessions
- Add drag-to-reorder for projects and sessions with persistence across restarts
- Expand context menu to include rename, archive, delete, and open-in-new-window (projects only)
- Make reorder fully keyboard-accessible (Tab through menu, Enter to invoke)
- Write E2E tests covering all four capabilities on macOS and Linux CI
- Preserve backward compatibility — existing projects/sessions without sort_order default to creation order (append-last on new items)

**Non-Goals:**

- Multi-select or batch operations
- Undo/redo for delete or rename
- Drag sessions across projects (sessions reorder only within their project)
- Drag projects past Unfiled (Unfiled group always remains last)
- Custom sorting strategies (sort_order is the sole ordering mechanism; no "sort by name" option)
- Server/web adapter for workspace management (desktop only in 0.x; server revived in Rust at 0.1.4)

## Decisions

### Decision 1: Add `sort_order` column to projects and sessions tables

**Chosen approach:** Nullable integer column, lazy-migrated. Default NULL → append-last behavior (new items get max(sort_order) + 1).

**Rationale:** 
- Avoids renumbering gaps if items are deleted
- Nullable simplifies migration (existing rows stay NULL; old read code still works)
- Append-last is the most predictable default for new items

**Alternatives considered:**
- Integer starting at 0 with gap-filling — adds complexity to migration and delete logic
- Use created_at timestamp for implicit ordering — doesn't support user reordering

**Implementation:**
- Migration: `ALTER TABLE projects ADD COLUMN sort_order INTEGER` (same for sessions)
- Orchestrator: `listProjects()` and `listSessions()` now sort by `COALESCE(sort_order, created_at DESC)`
- Backend API: new methods `reorder_project(id, newIndex)` and `reorder_session(id, newIndex)` that update sort_order atomically

### Decision 2: Inline rename with Enter/Escape cancel pattern

**Chosen approach:** Double-click to activate edit mode, Enter to confirm, Escape to cancel. Empty name for sessions defaults to ID prefix on next render.

**Rationale:**
- Double-click is the standard pattern (matches most UI frameworks)
- Keyboard-only closure (Escape) is expected by keyboard users
- Empty session names are allowed (fallback to ID) per roadmap spec, reducing validation friction

**Alternatives considered:**
- Single-click → edit mode — too error-prone (accidental clicks trigger edit)
- Triple-click to select all — non-standard; double-click + automatic select is simpler

**Implementation:**
- Add `InlineRenameInput` component with `contentEditable` or `<input>` and focus management
- `SessionSidebar` tracks `editingId` state (project or session being edited)
- On blur or Escape, cancel; on Enter, call backend API and refresh

### Decision 3: Drag-to-reorder using HTML5 drag API (no external library unless needed)

**Chosen approach:** Use native HTML5 `draggable` attribute and `dragstart`, `dragover`, `drop` events. If performance issues emerge, evaluate `@dnd-kit` as a lightweight alternative.

**Rationale:**
- Zero new dependencies initially
- Desktop-only scope (0.x terminal-only) simplifies cross-browser concerns
- Native API is sufficient for linear list reordering

**Alternatives considered:**
- `@dnd-kit` — battle-tested, but adds a dependency; defer until MVP proves drag is needed
- `react-dnd` — heavier, more suited to complex nested drag trees
- Custom pointer events — more control but reinvents the wheel

**Implementation:**
- Mark projects and sessions with `draggable="true"`
- Track drag state (source ID, hover target) in component state
- On `drop`, call `reorder_project(id, newIndex)` or `reorder_session(id, newIndex)`
- Visual feedback: highlight drop target, show insertion indicator

### Decision 4: Expanded context menu with full action set

**Chosen approach:** Extend `ProjectContextMenu` to include Rename, Archive, Delete, and Open-in-new-window. Same component pattern for sessions (Rename, Archive, Delete; no open-in-new-window).

**Rationale:**
- Consolidates actions in one discoverable UI (right-click)
- Matches user expectations from 0.0.12 roadmap ("full project action list lands in 0.0.12")
- Reuses the lightweight context-menu component (no modal overhead)

**Alternatives considered:**
- Buttons on hover — clutters the sidebar, harder to discover
- Move all actions to a settings/properties pane — too many clicks

**Implementation:**
- Extend `ProjectContextMenu` to render all four actions for projects
- Create parallel `SessionContextMenu` for sessions (three actions)
- Wire up handlers to `onRename` (activate inline edit), `onArchive`, `onDelete`, `onOpenInNewWindow`

### Decision 5: Delete as hard-delete, archive as soft-delete (distinct operations)

**Chosen approach:** Delete = immediate removal, archive = soft-delete (marked with `deleted_at`, PTY preserved). Both appear in context menus; archive is the "safer" choice.

**Rationale:**
- Aligns with 0.0.4 design (backend already supports archive)
- Hard-delete with confirmation is the destructive operation (clear intent required)
- Soft-delete (archive) is discoverable without modal friction (users can undo later if needed)
- Roadmap explicitly calls out "archive" and "delete" as separate

**Alternatives considered:**
- Archive-only with a separate "Permanently Delete" in an Archive view — adds complexity
- Delete-only — loses the safety of soft-delete

**Implementation:**
- New backend API: `delete_project(id)` and `delete_session(id)` (hard-delete, cascades to surfaces)
- Reuse existing: `archiveSession(id)` already works; add `archiveProject(id)` if needed
- Both operations refresh the sidebar on success

## Risks / Trade-offs

| Risk | Mitigation |
|------|-----------|
| **Drag-drop performance on large lists** | Virtualize project/session lists if they grow; lazy-load sessions; HTML5 drag is efficient for lists under ~1000 items. Desktop 0.x scope is small. Evaluate if e2e shows slowdown. |
| **Sort order races during concurrent updates** | Reorder is synchronous per user session; concurrent reorders in separate windows are race-prone but rare. Database should enforce uniqueness check or use timestamp for conflict resolution if needed. Document as "last-write-wins" for now. |
| **Migration complexity if sort_order is NULL** | Read code must handle NULL → fallback to creation order. Keep this explicit in the sort clause: `COALESCE(sort_order, ROWID)` or similar. Test with a mixed dataset (old + new rows). |
| **Undo/redo expectations** | No undo for delete or rename. Users expect immediate feedback and confirmation dialogs instead. Set expectations in dialogs ("Delete <name>? This cannot be undone."). |
| **E2E drag flake** | HTML5 drag is not reliably automated in WebDriver. Use [tauri-webdriver helpers](../../tests/desktop-e2e/helpers.ts) for drag simulation (execute DOM events manually if needed). Test carefully on both macOS and Linux. |

## Migration Plan

1. **Database migration**: `ALTER TABLE projects ADD COLUMN sort_order INTEGER DEFAULT NULL`; `ALTER TABLE sessions ADD COLUMN sort_order INTEGER DEFAULT NULL`
2. **Orchestrator API**: Add `reorder_project(id, newIndex)` and `reorder_session(id, newIndex)` methods; update `listProjects()` and `listSessions()` to sort by `COALESCE(sort_order, created_at DESC)`
3. **Backend API**: Expose new reorder endpoints via SDK client
4. **UI**: Replace `SessionSidebar.tsx` with drag support, inline-edit, expanded context menu
5. **E2E**: Write and run specs to green on macOS and Linux CI
6. **Testing**: Verify mixed dataset (old NULL sort_order + new records) sorts correctly

**Rollback:** No rollback needed; sort_order is additive. If reorder is broken, app falls back to creation order (NULL → created_at DESC).

## Open Questions

1. **Drag library finality**: Should we commit to `@dnd-kit` now or keep HTML5 drag and evaluate after MVP?
   - **Proposal**: Start with HTML5, add `@dnd-kit` only if performance or UX feedback demands it.

2. **Unfiled project constraint**: Can Unfiled be dragged? Should it stay at the bottom always?
   - **Proposal**: Unfiled is always last (hard constraint). Drag handlers reject attempts to move it above named projects.

3. **Session drag across projects**: Currently a non-goal. If users request it, should we support it?
   - **Proposal**: Reject drag across projects initially. Revisit in 0.1.x if feedback warrants.

4. **Archive vs Delete UX**: Should archive be hidden behind a "More" menu or remain visible in the full context menu?
   - **Proposal**: Archive is visible in the context menu. It's a first-class operation (same tier as Delete). Desktop users are power users.
