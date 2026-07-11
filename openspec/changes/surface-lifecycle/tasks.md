## 1. Tree ops + close lifecycle (foundation)

- [x] 1.1 Add `resetLeafToEmpty(tree, id)` to `lib/panelTree.ts` (wraps `setContentNode` with `{type:"empty"}`); unit-test it resets a bound leaf and leaves siblings untouched.
- [x] 1.2 Add a last-leaf guard for removal: `closeLeafSafe` resets the sole leaf to empty instead of returning `null`. Unit-test: closing the sole leaf yields a single empty leaf, never an empty tree.
- [x] 1.3 Extend `shouldConfirmClose(leaf, skipConfirm, isRunning)` with the `isRunning` arg; unit-test: bound+running+not-skipped → true; bound+exited → false; empty → false.
- [x] 1.4 In `PanelContent.handleClose`/`runClose`, branch on `leaf.content.type`: terminal → confirm-if-running gate then `surfaceClose(placement)` + `resetToEmpty`; empty → `close(leaf.id)` (remove/collapse) with the last-leaf guard. Keep the close fade cadence for removal.
- [x] 1.5 Remove the `leaves.length <= 1` block in `useShellCommands` `surfaceClose`; route it through the same `handleClose`.
- [x] 1.6 `PanelContent`: add `statusByPlacement` fed by a new `onStatusChange(placement, status)` from the pane; derive `isRunning` for the confirm gate from it.
- [x] 1.7 `Panel.CloseButton`: hide only when the leaf is the sole empty leaf (`canClose`); show for every terminal leaf (including the only one).

## 2. Terminal exit lifecycle (DesktopTerminalPane)

- [x] 2.1 Verify against `surfaceResolveOrSpawn`: a cleanly-exited surface persists as `idle`; the resolver resumes it in place (fresh PTY, same surface id + placement). Restart is UI-only, no surfaceClose. (design D2)
- [x] 2.2 New `TerminalExitBar` component: `Process exited/stopped · [Restart] · [New surface]`, terminal-token styled, keyboard-accessible.
- [x] 2.3 `DesktopTerminalPane`: on clean exit keep scrollback, render `TerminalExitBar`; add `onStatusChange` and `onRequestReset` props; report status transitions up.
- [x] 2.4 Restart handler: in-pane resume — clear scrollback, bump `resumeKey` so `bindChannel` re-resolves the same `(session, placement)` without recreating the xterm view. New surface → `onRequestReset`.
- [x] 2.5 Wire `TerminalFailureOverlay` Dismiss → `onRequestReset` (reset to empty); Resume unchanged.
- [x] 2.6 Thread `onStatusChange`/`onRequestReset` through `PanelTree.renderContent` → `PanelContent`.

## 3. Active-pane focus + directional navigation

- [x] 3.1 Promote focus to `focusedLeafId` state in `PanelContent` (seed from the pointer-down capture); on reset/remove of the focused leaf, fall back to the first remaining leaf.
- [x] 3.2 `Panel.Frame`: render a focus ring when the leaf is focused, visually distinct from the drop-target ring.
- [x] 3.3 Geometry nav helper `nearestLeafInDirection(leafId, dir, rects)`: nearest center-in-direction with overlap projection; unit-tested over representative rect layouts.
- [x] 3.4 Bind `paneFocusLeft/Right/Up/Down` handlers in `useShellCommands` to move `focusedLeafId`; no-op when no neighbor.

## 4. Zoom / maximize pane

- [x] 4.1 `zoomedLeafId` transient state in `PanelContent`; clear it when the zoomed leaf is reset/removed.
- [x] 4.2 `PanelTree`: when zoom is set, render only that leaf full-area (short-circuit the group walk); persisted tree untouched.
- [x] 4.3 Bind `paneZoomToggle` in `useShellCommands`; unit-tested it acts on the active leaf.

## 5. Keybindings routed through xterm

- [x] 5.1 Add action ids `surfaceNew`, `paneFocusLeft/Right/Up/Down`, `paneZoomToggle` + definitions/default keys in `defs.ts`.
- [x] 5.2 `surfaceNew` handler: spawn into the focused empty leaf, else split the focused leaf and spawn.
- [x] 5.3 Expose `usePaneShortcutDispatch()` matcher scoped to `PANE_ACTION_IDS`; unit-tested via the command routing.
- [x] 5.4 In `useTerminalPaneExtras`, fold the pane-shortcut match into the existing `attachCustomKeyEventHandler`: matched → dispatch + return false (not written to PTY); unmatched → pass through.
- [x] 5.5 Audit default keys against `keybindings.ts` presets and reserved chords; `surfaceNew` → `Cmd+Shift+T` (Cmd+T taken by "New terminal"), splits keep `Cmd+\`/`Cmd+Shift+\`.

## 6. Verify

- [x] 6.1 Added/extended unit tests (panelTree ops, paneNavigation, TerminalExitBar, Panel canClose, useShellCommands).
- [x] 6.2 `bun run check-types`, `bun run lint`, `bun test` (apps/ui 468) green; root `bun run test` 16/16 tasks green.
