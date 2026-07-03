## Context

The desktop window runs with `decorations: false`, `hiddenTitle: true`, `titleBarStyle: "Transparent"` (`apps/desktop/src-tauri/tauri.conf.json`), so the OS draws no chrome. The app has no title bar, no window controls, and no drag region — the window is effectively immovable from within the renderer. The shell (`ShellChrome` in `RootLayout.tsx`) is a single horizontal flex row: a fixed `w-56` left sidebar plus a content outlet. There is no right or bottom dock.

Existing infrastructure to build on:
- **Tauri window boundary**: `app/lib/tauriEvents.ts` exposes `currentWindow()` (dynamic-imports `getCurrentWindow()`, guarded by `isDesktopHost()`); `app/lib/windows.ts` builds `closeSelf`/`focusSelf` on top. Capabilities already grant `core:window:allow-start-dragging`, `allow-close`, `allow-destroy`, `allow-set-focus`.
- **UI state**: `app/lib/store.ts` (`@tanstack/react-store` `uiStore`) holds ephemeral window-scoped state (`commandCenterOpen`); durable UI state uses the settings store via `setGlobalSetting` + keys in `app/lib/settings/keys.ts` (e.g. `sidebarExpandedKey`).
- **Command registry**: `Command = {id,title,keywords?,group?,run}` registered via `<RegisterCommands>` / `useRegisterCommands`; palette is `CommandCenter.tsx`, opened via `useCommandCenterOpen()`.

The user explicitly chose the `tauri-controls` library for the window controls, and chose to build all three dock regions.

## Goals / Non-Goals

**Goals:**

- A title bar row above the shell body with `tauri-controls` OS window controls, a drag region, and a toggle toolbar.
- Left sidebar becomes hideable; new right and bottom dock regions, all with persisted visibility.
- Toggles wired both to title bar buttons and to command-center actions.
- Browser build degrades gracefully (toolbar renders, OS controls omitted, drag no-ops).

**Non-Goals:**

- Real content for the right/bottom docks (placeholder only; content is a later change).
- Resizable/draggable dock sizing beyond a fixed default width/height (react-resizable-panels wiring is out of scope).
- Changes to the recursive terminal panel tree (`ui-shell`).
- Per-window (non-persisted) layout variants; visibility is a single durable setting shared across windows, matching `sidebarExpandedKey`.

## Decisions

### D1: Use `tauri-controls` (not a custom control set)

User directive. `tauri-controls` provides `WindowControls`/`WindowTitlebar` React components with platform-aware styling and a `data-tauri-drag-region` affordance. Alternative considered and rejected by the user: a custom control set over the existing `currentWindow()` boundary (fewer deps, guaranteed Tauri-2.11 compatibility). Because the library is at 0.4.0 (~2 years old) and documents the deprecated `tauri-plugin-window`, **Task 1 is a feasibility spike**: install it, render `<WindowControls>` in the dev desktop build, confirm minimize/maximize/close fire under Tauri 2.11 and that styling survives Tailwind v4. If the library's controls do not fire, the spike wires the control callbacks to our own `currentWindow()` window-boundary methods while keeping the library's presentational components — recorded as a decision, not a silent pivot.

### D2: Window-op wrappers live in `windows.ts`

Add `minimizeSelf()`, `toggleMaximizeSelf()`, `startDrag()` to `app/lib/windows.ts`, each `(await currentWindow())?.method()`, mirroring `closeSelf()`. This keeps all `@tauri-apps/api/window` access behind the one boundary and gives the title bar host-agnostic callbacks. `close` reuses the existing `closeSelf()`. Alternative: let `tauri-controls` call the window API directly — rejected because it would bypass the `isDesktopHost()` guard and scatter Tauri imports.

### D3: Visibility state is durable settings, not ephemeral `uiStore`

Panel visibility should survive restart, so it follows the `sidebarExpandedKey` pattern: new keys in `settings/keys.ts` (`leftPanelVisibleKey`, `rightPanelVisibleKey`, `bottomPanelVisibleKey`) read/written through `settingsStore` / `setGlobalSetting`, exposed via `usePanelVisible(side)` hooks in `store.ts` shaped like `useProjectExpanded`. Defaults: left sidebar visible, right/bottom docks hidden (least surprise — new empty regions start collapsed). The command palette toggle stays on the existing ephemeral `commandCenterOpen`. Alternative: `uiStore` booleans (ephemeral) — rejected because visibility persistence is expected shell behavior.

### D4: Layout restructure — vertical shell

`ShellChrome` becomes `flex flex-col h-dvh`: row 1 = `<TitleBar>` (fixed height), row 2 = `flex-1 min-h-0` body. The body is `flex flex-col`: an upper `flex` row (left sidebar | content outlet | right dock) and the bottom dock beneath it. Hidden regions render `null` (not zero-width) so they consume no space and the outlet reclaims it — satisfies the "reclaims space" scenarios. The existing bottom-right floating action cluster (`NotificationIndicator`/`SettingsPanel`/`ServiceHealthIndicator`) stays anchored to the content area.

### D5: Toggles are `titlebar`-tagged toggle commands (via the shipped command manager)

The command manager (`ui-command-manager`, ADR-0045) now provides exactly the primitives this needs, so the toolbar is a projection of the command table rather than hand-wired buttons:

- Add four `CommandDef`s (`panelToggleLeft/Right/Bottom`, `commandToggle`) to `commands/defs.ts`, each with `surfaces: ["titlebar", "palette"]`, an `icon` (lucide), a `group`, and a `toggle` selector that reads its checked state from context (`c => c.leftPanelVisible`, `commandToggle`: `c => c.commandPaletteOpen`).
- Register their handlers by id via `useCommand` (in a `useTitleBarCommands` hook): each `run` flips the underlying store — `setPanelVisible(side, !visible)` / `setCommandCenterOpen(!open)`.
- Seed the context keys the selectors read via `setContextKey`: mirror panel visibility (`leftPanelVisible`/`rightPanelVisible`/`bottomPanelVisible`) and `commandPaletteOpen` into context wherever those stores change. This is the command manager's first real use of context keys and toggle commands.
- The title bar toolbar renders `useSurfaceCommands("titlebar")`: each command is a button showing its `icon`, `aria-pressed={command.checked}`, `onClick={command.run}`. Adding/removing a toolbar button is a defs edit, not a component edit. The same commands appear in the palette (checked state rendered there already).

Keybindings via `defaultKeys` are optional and can be added later without spec change.

## Risks / Trade-offs

- **`tauri-controls` incompatible with Tauri 2.11 / Tailwind v4** → Task 1 spike gates all downstream work; fallback wires library presentation to our own window boundary (D1). If the library is unusable even presentationally, escalate to Gate 2 as an overturned decision (custom controls).
- **Added deps (`tauri-controls`, `@tauri-apps/plugin-os`, possibly `tauri-plugin-os` crate)** → keep the crate optional; only add the OS plugin + `os:` capability if the library's platform auto-detection requires it. Prefer passing an explicit `platform` prop to avoid the OS plugin entirely if that path works.
- **Missing window permissions cause "not allowed by scope" at runtime** → add `allow-minimize`/`allow-maximize`/`allow-unmaximize`/`allow-toggle-maximize` to `capabilities/default.json` in the same change; verify in the spike.
- **Empty docks look broken** → render a labeled placeholder so the region is visibly intentional, and default them hidden (D3).
- **Layout regression to the existing sidebar/outlet** → cover the restructured layout with a RootLayout render test asserting region presence/absence by visibility state.

## Migration Plan

Additive; no data migration. Rollback = revert the change (new dep, new component, new settings keys, capability additions). Persisted visibility keys are new and default-safe, so a rollback leaves no orphaned required state.

## Open Questions

- Does `tauri-controls` 0.4.0 render and fire correctly under Tauri 2.11 + Tailwind v4? Resolved by Task 1; the answer is recorded on the decisions page.
- Final placement of the toggle toolbar relative to the OS controls (platform-dependent: macOS controls sit left, so the toolbar sits right; Windows/Linux the reverse). Inferred default: toolbar opposite the controls; adjust after the spike shows real placement.
