## Context

The panel tree (`apps/ui/app/components/shell`) renders a persisted tree of leaves, each
either empty (`{type:"empty"}` → `EmptyPanel` picker) or bound to a terminal
(`{type:"terminal", placement}` → `DesktopTerminalPane`). `PanelContent` owns the tree
mutations (`usePanelTree`: split/close/setContent/setActiveTab) and the close-confirm
dialog. `panelTree.ts` holds the pure tree ops (`splitNode`, `closeNode`, `setContentNode`,
`findLeaf`, `collectLeaves`). Terminal I/O, status, and the failure overlay live inside
`DesktopTerminalPane`, which binds a data channel via `surfaceResolveOrSpawn` +
`surfaceChannel` and already classifies exit with `isCleanExit`.

Backend is sufficient as-is: `surfaceClose` hard-removes a surface (terminates its PTY),
`surfaceDetach` preserves it, and `(session, placement)` resolves at most one live surface.
No daemon/orchestrator change is in scope.

Current defects: `useShellCommands` blocks close at `leaves.length <= 1`; `PanelContent.runClose`
always `closeNode`s the leaf (never resets to empty); a clean exit only sets `status="exited"`
and freezes the pane. Focus is tracked weakly (`activeLeafRef` on pointer-down) with no ring,
no keyboard nav, and no zoom. Pane shortcuts can't fire while `.xterm` holds focus.

## Goals / Non-Goals

**Goals:**

- Content-dependent close: bound leaf → terminate + reset to empty in place; empty leaf →
  remove; tree never drops below one leaf.
- Clean exit holds scrollback with a Restart / New surface bar; unclean exit keeps the
  failure overlay with a Dismiss-to-empty path.
- Confirm only when the pane's process is live.
- Active-pane focus ring, `Cmd+Alt+Arrow` directional nav, `Cmd+Alt+Z` zoom, and pane
  keybindings that fire through xterm's key handler.

**Non-Goals:**

- The diff panel or any non-terminal surface kind.
- Any daemon/orchestrator/wire change.
- Persisting focus or zoom across reloads (both are transient view state).
- Broadcast-input, pane resize keybindings, or tab/sidebar display-mode work.

## Decisions

### D1 — Close routing is content-dependent in `PanelContent`, not in the tree op

`handleClose(leaf)` branches on `leaf.content.type`:
- `terminal` → confirm-if-running gate → on confirm: `surfaceClose(placement)` then
  `setContent(leaf.id, {type:"empty"})` (reuse the fade delay for the terminal teardown).
- `empty` → `close(leaf.id)` (`closeNode`), which already collapses splits and, guarded so
  the last leaf is never removed, leaves a single empty leaf.

Add a pure helper `resetLeafToEmpty(tree, id)` = `setContentNode(tree, id, {type:"empty"})`
for clarity/testability. Drop the `leaves.length <= 1` guard in `useShellCommands`
`surfaceClose`; the last-leaf guarantee moves into the empty-branch of `handleClose`
(only *empty* leaves can be removed, and never the sole one).

_Alternative rejected:_ one uniform `closeNode` with a post-step re-adding an empty leaf —
more state churn, loses the pane's geometry, and fights the persisted tree.

### D2 — Exit bar and restart live in `DesktopTerminalPane`; reset/remove are lifted to `PanelContent`

The pane owns scrollback + status, so it renders the inline exit bar on clean exit
(new `TerminalExitBar` component: `exited(code) · Restart · New surface`). The pane can
resolve Restart itself (rebind its own channel to a fresh surface); "New surface" and
close must mutate the tree, so the pane calls up via new props
`onRequestReset(placement)` and reports status up via `onStatusChange(placement, status)`.

`PanelContent` keeps `statusByPlacement: Map<placement, status>` fed by `onStatusChange`.
This backs both the confirm-if-running gate (D4) and any tree-level status need. Restart is
handled in-pane: hard-remove the exited surface at the placement, then re-run
`surfaceResolveOrSpawn`, bumping the existing `resumeKey` so `bindChannel` re-runs without
recreating the xterm view. If the runtime cannot re-spawn onto a placement holding an exited
row, fall back to `onRequestReset` + immediate spawn (fresh placement) — same visible result.
A task verifies which path `surfaceResolveOrSpawn` supports before finalizing.

### D3 — Unclean exit reuses the existing `TerminalFailureOverlay`

Wire the overlay's Dismiss to `onRequestReset` (reset leaf to empty) instead of only
clearing local `failureReason`. Resume stays as-is (bump `resumeKey`).

### D4 — Confirm gate consults live process status

`shouldConfirmClose(leaf, skipConfirm, isRunning)` gains an `isRunning` arg;
`PanelContent` derives it from `statusByPlacement` (running = connected/attached, not
`exited`/`error`). Empty leaves and exited terminals never confirm.

### D5 — Focus is state in `PanelContent`; directional nav is geometry-based

Promote focus to `focusedLeafId` state (seeded by the existing pointer-down capture).
`Panel.Frame` renders a focus ring when `id === focusedLeafId`, visually distinct from the
drop-target ring. Directional nav reads the live `getBoundingClientRect` of each
`[data-panel-id]` element and picks the nearest leaf whose center lies in the requested
direction (overlap-projection + distance tiebreak). No geometry is stored — the DOM is the
source of truth, which stays correct across resizes and swaps. On reset/remove of the
focused leaf, focus falls back to the first remaining leaf.

_Alternative rejected:_ deriving adjacency from the tree structure — brittle across nested
splits and doesn't match what the user sees.

### D6 — Zoom is transient state, rendered by short-circuiting the tree walk

`zoomedLeafId` state in `PanelContent`. When set, `PanelTree` renders only that leaf
full-area (skip group/split rendering) rather than restructuring the tree, so the persisted
geometry is untouched and unzoom is a state flip. Closing/removing the zoomed leaf clears
zoom.

### D7 — Pane keybindings route through `attachCustomKeyEventHandler`

Global shortcuts are suppressed while `.xterm` holds focus (`defs.ts:226`). Expose a
`matchPaneShortcut(event): actionId | null` from the keybindings layer (same matching the
global registry uses, filtered to the pane action set: split-h/v, close, new-surface,
nav-left/right/up/down, zoom). In `useTerminalPaneExtras`/`DesktopTerminalPane`, register
`term.attachCustomKeyEventHandler(e => { const id = matchPaneShortcut(e); if (id) {
dispatch(id); return false; } return true; })` so a matched shortcut dispatches the command
and is not written to the PTY. Unmatched keys pass through to the shell unchanged.

New action ids: `surfaceNew` (new-surface, distinct from `surfaceSpawn` which targets an
existing empty leaf — new-surface splits when none is empty), `paneFocusLeft/Right/Up/Down`,
`paneZoomToggle`. Default keys: split-right `Cmd/Ctrl+D`, split-down `Cmd/Ctrl+Shift+D`,
close `Cmd/Ctrl+W`, new-surface `Cmd/Ctrl+T`, nav `Cmd/Ctrl+Alt+Arrow`, zoom
`Cmd/Ctrl+Alt+Z`. Verify none collide with reserved xterm/find bindings (find is already
moved to `Ctrl+Shift+F`).

## Risks / Trade-offs

- [Restart placement reuse depends on runtime behavior on an exited surface row] → D2
  fallback (hard-remove + fresh spawn) keeps the feature UI-only regardless; a task pins
  down the exact path before implementing.
- [`Cmd+W` / `Cmd+T` / `Cmd+D` may collide with reserved chords or the host webview] →
  audit against `keybindings.ts` presets and adjust defaults; all are user-rebindable.
- [Geometry-based nav on unusual nested splits could pick a surprising neighbor] → center-in-
  direction + overlap projection is the standard heuristic; covered by unit tests over
  representative rect layouts.
- [Lifting per-placement status to `PanelContent` adds a channel of state] → keep it a
  single derived `Map` fed by one `onStatusChange` callback; no polling (reuse existing
  status events).

## Migration Plan

Pure additive UI behavior; no data migration. Persisted layout trees are unchanged
(reset-to-empty writes an existing valid `{type:"empty"}` leaf). Rollback is reverting the
UI change. Ship behind no flag — it strictly improves the current broken lifecycle.

## Open Questions

- Exact reserved-chord conflicts for the default keybindings on macOS/Windows/Linux hosts —
  resolved by the keybindings audit task, not blocking design.
