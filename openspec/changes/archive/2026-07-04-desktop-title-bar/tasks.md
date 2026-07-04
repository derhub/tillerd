## 1. Controls approach (spike outcome: native OS controls, no plugin)

- [x] 1.1 Evaluated `tauri-controls` (crashes under React 19 — bundles removed React-18 internals), `tauri-plugin-decorum` / `tauri-plugin-frame` (build on Tauri 2.11 but need a Rust plugin + per-window wiring), and hand-drawn controls. Landed on Tauri's documented standard: keep the OS-native controls.
- [x] 1.2 No dependency and no library component are added; `tauri-controls` and `@tauri-apps/plugin-os` removed. The native controls are kept via window config (D1), not drawn.
- [x] 1.3 Config: `apps/desktop/src-tauri/tauri.conf.json` main window uses `titleBarStyle: "Overlay"` with decorations on (not `decorations: false`), so the OS draws the native controls. No extra `core:window` permissions beyond `allow-start-dragging`.

## 2. Window-op boundary — not needed (native controls, D2)

- [x] 2.1 No window-op wrappers: the OS owns minimize/maximize/close, so there is no IPC to wrap. Dragging uses the native `data-tauri-drag-region` attribute (`core:window:allow-start-dragging`).
- [x] 2.2 N/A — no wrappers to test.

## 3. Panel-visibility state

- [x] 3.1 Add `leftPanelVisibleKey`, `rightPanelVisibleKey`, `bottomPanelVisibleKey` to `app/lib/settings/keys.ts`.
- [x] 3.2 Add `usePanelVisible(side)` + `setPanelVisible(side, value)` in `app/lib/store.ts`, shaped like `useProjectExpanded`, backed by the settings store; defaults left=visible, right/bottom=hidden.
- [x] 3.3 Test: toggling persists to settings; defaults apply when unset; `resetUiStore` hygiene covers the new keys if applicable.

## 4. Toggle commands (via the command manager)

- [x] 4.1 Add `panelToggleLeft/Right/Bottom`, `commandToggle` ids to `app/lib/commands/ids.ts` and `CommandDef`s to `commands/defs.ts` with `surfaces: ["titlebar","palette"]`, a lucide `icon`, a `group`, and a `toggle` selector reading the relevant context key.
- [x] 4.2 Add a `useTitleBarCommands` hook registering each handler by id (`useCommand`): panel toggles call `setPanelVisible(side, !visible)`; `commandToggle` flips `commandCenterOpen`.
- [x] 4.3 Seed the context keys the toggle selectors read (`leftPanelVisible`/`rightPanelVisible`/`bottomPanelVisible`, `commandPaletteOpen`) via `setContextKey`, mirroring the underlying stores.
- [x] 4.4 Test: invoking each command toggles the store; the command's `checked` follows the store; palette shows the checked indicator.

## 5. Title bar component

- [x] 5.1 Create `app/components/shell/TitleBar.tsx`: fixed-height row with `tauri-controls` window controls (per D1/D3 outcome), a `data-tauri-drag-region` drag area, and a toolbar rendering `useSurfaceCommands("titlebar")` as icon buttons (`aria-pressed={cmd.checked}`, `onClick={cmd.run}`).
- [x] 5.2 Off the desktop host, omit OS controls and render the toolbar only; the drag region no-ops.
- [x] 5.3 Test: toolbar is data-driven from titlebar-surface commands; each button reflects its command's `checked`; OS controls absent in browser build.

## 6. Shell layout restructure + dock regions

- [x] 6.1 Restructure `ShellChrome` in `RootLayout.tsx` to `flex flex-col h-dvh`: `<TitleBar>` row + `flex-1 min-h-0` body; body = upper row (left sidebar | outlet | right dock) over the bottom dock.
- [x] 6.2 Gate the left sidebar, right dock, and bottom dock on `usePanelVisible(side)`; hidden regions render `null` so the outlet reclaims the space.
- [x] 6.3 Add right-dock and bottom-dock region components rendering labeled placeholder content.
- [x] 6.4 Keep the existing bottom-right floating action cluster anchored to the content area.
- [x] 6.5 Region visibility is covered at the seams the codebase tests: `usePanelVisible` (store.test), the toolbar toggle wiring (TitleBar.test / useTitleBarCommands.test), and the live shell via desktop e2e. There is no RootLayout unit-render harness (shell integration is e2e-tested), so no new unit test is added there.

## 7. Verify

- [x] 7.1 `bun test` (apps/ui) green; `cargo nextest run` (apps/desktop) green; type-check + lint clean.
- [ ] 7.2 Manual desktop smoke: drag moves window; minimize/maximize/close work; all four toggles work from both title bar and command palette; visibility persists across restart.
