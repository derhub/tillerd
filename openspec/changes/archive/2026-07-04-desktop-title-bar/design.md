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
- Persisting dock sizes across restart (the regions are drag-resizable via `react-resizable-panels`, but the chosen size is per-session, not saved — size persistence is a later change).
- Changes to the recursive terminal panel tree (`ui-shell`).
- Per-window (non-persisted) layout variants; visibility is a single durable setting shared across windows, matching `sidebarExpandedKey`.

### D6: Sidebar and docks are drag-resizable

The shell body is a nested `react-resizable-panels` layout: an outer vertical group whose top row is a horizontal group (sidebar | content | right dock) and whose bottom row is the full-width bottom dock, with resize handles between regions. The bottom dock spans the full window width (below the sidebar too), not just the content column. Sizes use unit strings (sidebar `224px`, right `256px`, bottom `200px`) with min/max bounds. A hidden region renders neither its panel nor its handle, so the remaining regions reclaim the space. Size is not persisted yet (see Non-Goals). Alternative rejected: fixed-width docks — the user wants to resize them; the panel library is already a dependency (terminal splits use it).

## Decisions

### D1: Keep the native OS window controls — no plugin, no library, no custom-drawn controls

The change originally chose the `tauri-controls` library (user directive). The **Task-1 feasibility spike disqualified every drawn/library approach and landed on Tauri's own documented standard**:

- `tauri-controls` 0.4.0 **crashes under React 19** — it bundles React 18's JSX runtime and reads `React.__SECRET_INTERNALS….ReactCurrentDispatcher`, removed in React 19; importing it white-screens the app. No React-19 release.
- The `tauri-plugin-decorum` / `tauri-plugin-frame` plugins (frame is the maintained 2026 fork) give native macOS traffic-light *inset* but require a Rust plugin, per-window `create_overlay_titlebar()` wiring, and extra config/perms — more than the goal needs.
- Hand-drawn traffic lights are not the OS's real controls.

Per the official Tauri window-customization guide, the standard is to keep the **OS-native controls** and build a custom titlebar around them. `decorations: false` is precisely what strips the native controls; leaving decorations on with `titleBarStyle: "Overlay"` (macOS) keeps the native traffic lights in their default top-left position while the webview draws behind them. So the decision is a **config change, not code**: the `TitleBar` renders no control buttons at all — the OS draws them.

### D2: No window-op wrappers, no extra window permissions

Because the OS owns minimize/maximize/close, there is no IPC to make: no `minimizeSelf`/`toggleMaximizeSelf` in `windows.ts`, and no `allow-minimize`/`allow-maximize`/etc. capability permissions. Only `core:window:allow-start-dragging` is kept, for the `data-tauri-drag-region` bar. The `TitleBar` reserves left padding on the macOS desktop host so the toolbar sits inline to the right of the native controls.

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
