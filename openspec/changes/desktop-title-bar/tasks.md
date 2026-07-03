## 1. Feasibility spike (gates everything else)

- [ ] 1.1 Add `tauri-controls` + peer dep `@tauri-apps/plugin-os` to `apps/ui`; add `core:window` minimize/maximize/unmaximize/toggle-maximize permissions to `apps/desktop/src-tauri/capabilities/default.json`.
- [ ] 1.2 Render `<WindowControls>` in a throwaway spot in the desktop dev build; confirm the controls display and that minimize / maximize / close fire under Tauri 2.11, and that styling survives Tailwind v4.
- [ ] 1.3 Decide the control-callback path: use the library's built-in window calls if they fire, else wire the callbacks to our own `windows.ts` boundary while keeping the library's presentational components. Record the outcome on the decisions page. Only add `tauri_plugin_os` (crate + `os:` capability) if platform auto-detection requires it; prefer an explicit `platform` prop otherwise.

## 2. Window-op boundary

- [ ] 2.1 Add `minimizeSelf()`, `toggleMaximizeSelf()`, `startDrag()` to `app/lib/windows.ts`, each guarded via `currentWindow()`, mirroring `closeSelf()`.
- [ ] 2.2 Unit-test the wrappers no-op off the desktop host (browser build) and call the window methods on it.

## 3. Panel-visibility state

- [ ] 3.1 Add `leftPanelVisibleKey`, `rightPanelVisibleKey`, `bottomPanelVisibleKey` to `app/lib/settings/keys.ts`.
- [ ] 3.2 Add `usePanelVisible(side)` + `setPanelVisible(side, value)` in `app/lib/store.ts`, shaped like `useProjectExpanded`, backed by the settings store; defaults left=visible, right/bottom=hidden.
- [ ] 3.3 Test: toggling persists to settings; defaults apply when unset; `resetUiStore` hygiene covers the new keys if applicable.

## 4. Command/toggle actions

- [ ] 4.1 Add `panelToggleLeft`, `panelToggleRight`, `panelToggleBottom`, `commandToggle` action ids + titles to `app/lib/commands/ids.ts`.
- [ ] 4.2 Register the four actions (in `RootLayout` or a `useTitleBarCommands` hook) whose `run` calls the same setters used by the title bar buttons; `commandToggle` flips `commandCenterOpen`.
- [ ] 4.3 Test: invoking each action from the registry toggles the corresponding state.

## 5. Title bar component

- [ ] 5.1 Create `app/components/shell/TitleBar.tsx`: fixed-height row with `tauri-controls` window controls (per D1/D3 outcome), a `data-tauri-drag-region` drag area, and a toggle toolbar (left / right / bottom / command) using existing `ui/button` + lucide icons, each button reflecting its region's visibility.
- [ ] 5.2 Off the desktop host, omit OS controls and render the toolbar only; the drag region no-ops.
- [ ] 5.3 Test: buttons call the correct toggles; button active/inactive state tracks visibility; OS controls absent in browser build.

## 6. Shell layout restructure + dock regions

- [ ] 6.1 Restructure `ShellChrome` in `RootLayout.tsx` to `flex flex-col h-dvh`: `<TitleBar>` row + `flex-1 min-h-0` body; body = upper row (left sidebar | outlet | right dock) over the bottom dock.
- [ ] 6.2 Gate the left sidebar, right dock, and bottom dock on `usePanelVisible(side)`; hidden regions render `null` so the outlet reclaims the space.
- [ ] 6.3 Add right-dock and bottom-dock region components rendering labeled placeholder content.
- [ ] 6.4 Keep the existing bottom-right floating action cluster anchored to the content area.
- [ ] 6.5 Test (RootLayout render): each region present/absent per its visibility state; regions toggle independently.

## 7. Verify

- [ ] 7.1 `bun test` (apps/ui) green; `cargo nextest run` (apps/desktop) green; type-check + lint clean.
- [ ] 7.2 Manual desktop smoke: drag moves window; minimize/maximize/close work; all four toggles work from both title bar and command palette; visibility persists across restart.
