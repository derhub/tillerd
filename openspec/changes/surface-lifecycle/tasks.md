## 1. Tree ops + close lifecycle (foundation)

- [ ] 1.1 Add `resetLeafToEmpty(tree, id)` to `lib/panelTree.ts` (wraps `setContentNode` with `{type:"empty"}`); unit-test it resets a bound leaf and leaves siblings untouched.
- [ ] 1.2 Add a last-leaf guard for removal: a helper (or `closeNode` caller contract) that, when the target is the only leaf, resets it to empty instead of returning `null`. Unit-test: closing the sole leaf yields a single empty leaf, never an empty tree.
- [ ] 1.3 Extend `shouldConfirmClose(leaf, skipConfirm, isRunning)` with the `isRunning` arg; unit-test: bound+running+not-skipped → true; bound+exited → false; empty → false.
- [ ] 1.4 In `PanelContent.handleClose`, branch on `leaf.content.type`: terminal → confirm-if-running gate then `surfaceClose(placement)` + `resetLeafToEmpty`; empty → `close(leaf.id)` (remove/collapse) with the last-leaf guard. Keep the close fade cadence.
- [ ] 1.5 Remove the `leaves.length <= 1` block in `useShellCommands` `surfaceClose`; route it through the same `handleClose`.
- [ ] 1.6 `PanelContent`: add `statusByPlacement` map fed by a new `onStatusChange(placement, status)` from the pane; derive `isRunning` for the confirm gate from it.
- [ ] 1.7 `Panel.CloseButton`: hide only when the leaf is the sole empty leaf; show for every terminal leaf (including the only one). Update the `totalPanels<=1` logic accordingly.

## 2. Terminal exit lifecycle (DesktopTerminalPane)

- [ ] 2.1 Verify against `surfaceResolveOrSpawn`/`surfaceClose` whether a placement holding a cleanly-exited surface can be re-spawned; record the chosen Restart path (reuse placement vs hard-remove + fresh spawn).
- [ ] 2.2 New `TerminalExitBar` component: `exited(<code>) · [Restart] · [New surface]`, styled to the terminal surface, keyboard-accessible.
- [ ] 2.3 `DesktopTerminalPane`: on clean exit keep scrollback, render `TerminalExitBar`; add `onStatusChange` and `onRequestReset(placement)` props; report status transitions up.
- [ ] 2.4 Restart handler: per 2.1, respawn a fresh surface into the pane and rebind the channel (bump `resumeKey`) without recreating the xterm view. New surface → `onRequestReset`.
- [ ] 2.5 Wire `TerminalFailureOverlay` Dismiss → `onRequestReset` (reset to empty); Resume unchanged.
- [ ] 2.6 Thread `onStatusChange`/`onRequestReset` through `PanelTree.renderContent` → `PanelContent`.

## 3. Active-pane focus + directional navigation

- [ ] 3.1 Promote focus to `focusedLeafId` state in `PanelContent` (seed from the existing pointer-down capture); on reset/remove of the focused leaf, fall back to the first remaining leaf.
- [ ] 3.2 `Panel.Frame`: render a focus ring when the leaf is focused, visually distinct from the drop-target ring; expose focused state via context.
- [ ] 3.3 Geometry nav helper `nearestLeafInDirection(leafId, dir)`: read `[data-panel-id]` rects, pick nearest center-in-direction with overlap projection; unit-test over representative rect layouts.
- [ ] 3.4 Bind `paneFocusLeft/Right/Up/Down` handlers in `useShellCommands` to move `focusedLeafId`; no-op when no neighbor.

## 4. Zoom / maximize pane

- [ ] 4.1 `zoomedLeafId` transient state in `PanelContent`; clear it when the zoomed leaf is reset/removed.
- [ ] 4.2 `PanelTree`: when zoom is set, render only that leaf full-area (short-circuit the group walk); persisted tree untouched.
- [ ] 4.3 Bind `paneZoomToggle` in `useShellCommands`; unit-test the render short-circuit and that geometry is not mutated.

## 5. Keybindings routed through xterm

- [ ] 5.1 Add action ids: `surfaceNew`, `paneFocusLeft/Right/Up/Down`, `paneZoomToggle` (`ids.ts`); definitions + default keys in `defs.ts`/`keybindings.ts` (split-right Cmd/Ctrl+D, split-down +Shift, close Cmd/Ctrl+W, new Cmd/Ctrl+T, nav Cmd/Ctrl+Alt+Arrow, zoom Cmd/Ctrl+Alt+Z).
- [ ] 5.2 `surfaceNew` handler: spawn into the focused empty leaf, else split the focused leaf and spawn (mirror `surfaceRunCommand` placement logic).
- [ ] 5.3 Expose `matchPaneShortcut(event): actionId | null` from the keybindings layer, scoped to the pane action set; unit-test matches and non-matches.
- [ ] 5.4 In `DesktopTerminalPane`/`useTerminalPaneExtras`, register `attachCustomKeyEventHandler`: matched pane shortcut → dispatch + return false (not written to PTY); unmatched → pass through.
- [ ] 5.5 Audit default keys against `keybindings.ts` presets and reserved xterm/find chords; adjust collisions.

## 6. Verify

- [ ] 6.1 Update/extend affected unit + e2e tests (panel close/exit lifecycle, focus, zoom, keybindings) to the new behavior; remove assertions on the old blocked-close/frozen-exit behavior.
- [ ] 6.2 `bun run check-types`, `bun run lint`, `bun test` (apps/ui) green; root `bun run verify` green.
