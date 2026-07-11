# Proposal: surface-lifecycle

## Why

The desktop panel tree cannot be used as a daily terminal multiplexer. Closing a
terminal is blocked whenever it is the only pane, closing otherwise deletes the whole
pane, and a process that exits freezes as a dead pane with no path back to a spawnable
surface. The empty-surface picker and the backend close/detach operations already exist;
this change wires a coherent close/exit lifecycle around them and adds the pane focus,
navigation, and zoom controls a multiplexer needs. Nailing surface management now makes
every later mountable surface kind trivial to add.

## What Changes

- **Close = kill then empty picker in place.** Closing a terminal pane kills its PTY and
  resets that pane to the empty "New surface" picker; the pane stays in the layout. This
  works on the only/last pane (the current `leaves.length <= 1` block is removed).
- **Empty pane close = remove.** Closing an already-empty pane removes that leaf and
  collapses the split. The sole/root pane can never be removed: when it is the only leaf
  and empty, the close control is hidden. There is always at least one pane.
- **Clean exit holds output with a restart bar.** When a shell exits cleanly the pane
  keeps its final scrollback and shows an inline bar `exited(<code>) · Restart · New
  surface`. Restart respawns a PTY at the same placement; New surface resets the pane to
  the empty picker. Distinct from close — output is preserved until the user acts.
- **Unclean exit** keeps the existing failure overlay (Resume / Dismiss); Dismiss resets
  the pane to the empty picker. Clean vs unclean stays classified by `exit-classification`.
- **Confirm only when running.** The close-confirm dialog fires only when the surface
  still has a live process. Closing an exited or empty pane never prompts. The existing
  "don't ask again" preference is unchanged.
- **Active-pane focus + directional navigation.** The focused leaf is tracked and shown
  with a focus ring; `Cmd+Alt+Arrows` moves focus between panes by geometry.
- **Zoom / maximize pane.** `Cmd+Alt+Z` toggles the focused pane to fill the whole panel
  area (siblings hidden) and back, without mutating the persisted split tree.
- **Pane keybindings** for split-right, split-down, close-surface, new-surface — dispatched
  through xterm's own key handler so they fire while a terminal holds focus.

Out of scope: the diff panel (`ui-diff-panel`), any new surface kind, and any
daemon/orchestrator change. This change is UI-only.

## Capabilities

### New Capabilities

- `surface-lifecycle`: the pane content lifecycle — close resets a terminal pane to the
  empty picker and removes an empty pane, the always-one-pane guarantee, clean-exit hold
  with restart/new-surface, unclean-exit dismiss-to-empty, and confirm-only-when-running.
- `panel-multiplexer-nav`: active-pane focus tracking and focus ring, directional
  keyboard pane navigation, zoom/maximize-pane toggle, and pane keybindings routed through
  the terminal's key handler.

### Modified Capabilities

- `ui-panel-model`: the "Panel content type assignment" requirement gains close semantics
  — closing a bound (terminal) leaf unbinds it back to empty rather than removing it;
  closing an empty leaf removes it; the tree always retains at least one leaf.
- `ui-terminal-pane`: adds a terminal exit-lifecycle requirement (clean-exit restart bar,
  restart respawns at the same placement, new-surface resets to empty).

## Impact

- UI only, `apps/ui`: `lib/panelTree.ts` (reset-to-empty vs remove op, last-pane guard),
  `components/shell/PanelContent.tsx` (close routing, exit handling, confirm gate),
  `components/shell/PanelTree.tsx` and `Panel.tsx` (close-button logic, focus ring, zoom),
  `components/terminal/DesktopTerminalPane.tsx` (exit bar, restart), a new terminal exit-bar
  component, `components/shell/hooks/useShellCommands.ts` (close semantics, focus nav, zoom),
  `lib/commands/{ids,defs,keybindings}.ts` (new actions + default keys), plus focus-nav and
  zoom state in the panel-tree hook.
- No changes to the daemon, orchestrator, or wire protocol — `surfaceClose`/`surfaceDetach`
  and `(session, placement)` resolution already provide the needed backend behavior.
- Verify: `bun run check-types`, `bun run lint`, `bun test`; root `bun run verify`.
