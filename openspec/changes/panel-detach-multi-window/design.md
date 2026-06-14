## Context

**Current State:**
- `window_host.rs` provides `window_open(label, query)` and `window_focus(label)` Tauri commands (ready, 0.0.11-scoped).
- Panel system: panels bind to surfaces by `(session, placement)` (ADR-0030). One panel tree per session; all panels in a session share the same ParentSurface (orchestrator event sink subscription).
- Event sink: already supports multiple concurrent subscribers per window (no backend change needed).
- UI: Panel component has detach button slots; PanelGroup manages tree geometry and surface binding via `panelTree.ts`.

**Constraints:**
- Placement/surface binding model is frozen (ADR-0030); cannot change.
- Window state must be persisted across restarts (Tauri window-state plugin or localStorage).
- Child windows are standard Tauri WebviewWindows, not iframes; each has its own renderer instance.

## Goals / Non-Goals

**Goals:**
- Allow users to detach a panel into a standalone child window.
- Allow users to open a project view in a new window.
- Child windows remain open when parent closes; parent close does NOT cascade.
- Parent can re-attach a detached child's panel back into the panel tree.
- Window geometry is persisted.

**Non-Goals:**
- Nested window hierarchies (grandchild windows).
- Window-to-window drag-and-drop of panels.
- Maximized/tiled window layouts (0.0.14 UX pass).
- Server/web adapter for multi-window (desktop-only, 0.x terminal-only).

## Decisions

### 1. Window Identity via Query Parameter

**Decision:** Child windows are identified by a query parameter in the URL (`?w=detached&session=<sessionId>&placement=<placementId>` or `?w=project&session=<sessionId>`). The renderer reads this on mount to determine window type and context.

**Rationale:** Query params are lightweight, readable, and survive window restart. No new IPC or backend storage needed. React Router already parses query params. Window label (Tauri API) stays in sync but is internal; the URL is the source of truth for the renderer.

**Alternatives Considered:**
- Store window → (session, placement) map in a backend service: adds complexity and potential stale data after restarts.
- Use Tauri label directly: labels are opaque strings; makes URL construction harder.

### 2. Panel Placeholder vs. Hidden Panel

**Decision:** When a panel is detached, the parent shows a greyed-out placeholder with a "Focus →" button (not an empty space, not a closed panel).

**Rationale:** The placeholder maintains the parent's panel tree geometry and makes it clear which panel was detached and how to bring it back. Hiding the panel would leave an empty panel slot; closing it would require re-attachment logic to recreate the slot.

**Alternatives Considered:**
- Auto-hide the placeholder when the user navigates to a different session: confusing; user might not remember which panel is detached.
- Remove the placeholder entirely: loses visual record of detached state.

### 3. State Management: Detached Placements Map

**Decision:** The renderer maintains a `detachedPlacements` set (session + placement) tracking which surfaces are currently detached. When a panel is detached, add its placement to this set. When re-attached, remove it. Persist this map to localStorage or derive it from open windows.

**Rationale:** Keeps the UI layer aware of detached state without needing a backend service. Allows the placeholder "Focus →" button to know which window to focus. If deriving from open windows, query Tauri API for all open window labels and parse their query params; this is the source of truth after restarts.

**Alternatives Considered:**
- Centralized window registry in orchestrator: adds backend state, complicates shutdown/restart scenarios.
- Hardcoded persistence to a config file: avoids session-memory, but requires careful cleanup on crash.

### 4. Re-attach Flow: Panel Returns to Original Slot

**Decision:** Re-attach removes the placement from `detachedPlacements`, closes the child window, and shows the panel's content in the parent placeholder. The panel tree slot remains; placeholder becomes active content.

**Rationale:** Simplest for the user; panel returns to its original position. Avoids re-creating the slot or asking the user where to place it.

**Alternatives Considered:**
- Floating panel that user must drag into place: adds interaction complexity; re-attach becomes a 2-step operation.
- Auto-hide placeholder when other panels fill it: confusing if user re-attaches later and expects to see the placeholder.

### 5. Child Window Lifecycle: Independent of Parent

**Decision:** Child windows are independent Tauri windows. Parent close does NOT close children. Parent restart does NOT invalidate child windows; they can still re-attach using the same (session, placement) pair.

**Rationale:** Matches user expectations for desktop windows; aligns with the "picture-in-picture" mental model. Children often outlive their parent (e.g., parent closed, child reopened later).

**Alternatives Considered:**
- Parent owns children; close parent = close all children: violates user expectations and prevents viewing panels after parent closes.

### 6. Query Param Format for Child Windows

**Decision:**
- Panel detach: `?w=detached&session=<sessionId>&placement=<placementId>`
- Project in new window: `?w=project&session=<sessionId>`

Both include a `parentLabel` or `parentId` optional parameter to allow the child to identify its parent for re-attach focus.

**Rationale:** Explicit, readable, and extensible. The `w=` prefix signals window type. Optional parent reference simplifies re-attach (child calls `window_focus(parentLabel)`).

**Alternatives Considered:**
- Single query param `windowId`: opaque; requires a backend registry to resolve.
- Fragment route (e.g., `/#/detached?session=...`): fragments don't survive window restart in some cases.

### 7. Window-State Plugin vs. localStorage

**Decision:** Use Tauri's window-state plugin if available in the workspace; otherwise, fall back to localStorage for window geometry. Detached/project state (which placements are open) stored in renderer state, derived from query params.

**Rationale:** window-state plugin handles maximized/minimized state automatically. localStorage is simpler to audit but less robust. Detached state is transient (rebuilt on restart by querying open windows); no need to persist to disk.

**Alternatives Considered:**
- Custom SQLite persistence: overkill for geometry; Tauri plugin is standard.
- Always use localStorage: misses window state (max/min), less reliable.

## Risks / Trade-offs

**[Risk] Window state divergence after crash**: If the parent crashes with a detached child open, the child's query param still references the (session, placement), but the parent doesn't know the child exists. Re-attach will fail if the parent tries to remove a non-existent placeholder.

- **Mitigation**: Re-attach logic checks if the placement exists in the parent; if not, just close the child gracefully. Child can show a "parent not available" message.

**[Risk] Multiple children detached from the same placement**: If user detaches the same panel twice (via a bug or unlikely UI sequence), two windows claim the same (session, placement).

- **Mitigation**: Only one child can be "live" for a placement at a time. Enforce in the detach command: check `detachedPlacements` before allowing a new detach. If already detached, bring the existing window to focus instead.

**[Risk] Placement ID collision or stale reference**: If a placement is deleted (session closed, surface removed) but a child window still references it, the child will fail to load content.

- **Mitigation**: Child window checks if placement exists when loading. If not found, show an error message ("surface no longer exists") instead of crashing.

**[Risk] Event sink subscription per window**: If parent and child both subscribe to the same session's event sink, each receives all events. Large event volume could cause redundant updates.

- **Mitigation**: Event deduplication in the renderer; both windows read the same state snapshot and only re-render on deltas. Orchestrator's event sink is efficient; volume is not a bottleneck.

## Migration Plan

No breaking changes. Feature is additive:
1. Add detach button to Panel.Header (opt-in display based on `canDetach` prop).
2. Add "Open in new window" context menu to sidebar (new action).
3. Implement `window_open` command dispatch on detach and project-in-new-window actions.
4. Update query param parsing in root shell to detect child window type.
5. Conditional re-attach button rendering based on query params.

All code is feature-gated behind 0.0.11; no risk to existing 0.0.10 behavior.

## Open Questions

1. **Parent label persistence**: Should the child remember its parent's window label to enable focus? Or always focus the "main" window by label? Need to decide label convention for the main window (e.g., always "main" or dynamically assigned).
2. **Session-scoped window state**: Should window geometry be tied to a session? E.g., different projects get different window sizes? Or global setting? Deferred to 0.0.14 UX pass if prioritized.
3. **Project window vs. session**: When opening a project in a new window, which session appears first? The most recent, or an empty picker? Clarify in E2E scenarios.
