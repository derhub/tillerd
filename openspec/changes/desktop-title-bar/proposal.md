## Why

The desktop window is configured with `decorations: false` and a transparent title bar, so the OS draws no chrome — today there is no way to move, minimize, maximize, or close the window from within the app, and no home for global layout controls. The shell also renders only a fixed left sidebar; there is no place to dock secondary or auxiliary content, and no affordance to hide chrome to reclaim space.

## What Changes

- Add a custom title bar row spanning the top of the shell window, keeping the **OS-native** window controls (minimize / maximize / close) via window config (overlay title bar) — the shell draws no control buttons — plus a draggable region for moving the window.
- Add a toolbar in the title bar with toggle buttons for: the left sidebar (existing), a new right dock region, a new bottom dock region, and the command palette (existing command center).
- Introduce two new collapsible shell regions — a right dock and a bottom dock — with placeholder content, each with persisted visibility.
- Make the existing left sidebar collapsible from the title bar, with persisted visibility.
- Use the native `data-tauri-drag-region` attribute for window dragging (covered by `core:window:allow-start-dragging`); no JS window-op wrappers are needed since the OS owns minimize/maximize/close.
- Register the panel and command toggles as command-center actions so they are also reachable from the palette and rebindable.
- Configure the desktop window for an overlay title bar (`titleBarStyle: "Overlay"`, decorations on, `hiddenTitle`) so the OS draws the controls; apply the same to child windows.

## Capabilities

### New Capabilities

- `desktop-title-bar`: The title bar chrome — OS window controls, drag region, the panel/command toggle toolbar, the left/right/bottom dock regions and their persisted visibility state, and the command-center actions that drive the toggles.

### Modified Capabilities

<!-- None. The title bar is new chrome above the existing shell. The `ui-shell` capability governs the recursive terminal panel tree in the content area, which is unchanged. Command-center actions are additive and specified in the new capability. -->

## Impact

- **apps/ui** (React frontend):
  - `app/components/shell/RootLayout.tsx` — restructure `ShellChrome` into a vertical `title-bar row` + `body row`; body hosts left sidebar, content outlet, right dock, bottom dock.
  - New `app/components/shell/TitleBar.tsx` (drag region + toggle toolbar; reserves space for the native controls, draws none).
  - New right/bottom dock region components (placeholder content).
  - `app/lib/store.ts` / `app/lib/settings/keys.ts` — panel-visibility state (durable settings, mirroring `sidebarExpandedKey`).
  - `app/lib/commands/ids.ts` / `defs.ts` + a `useTitleBarCommands` hook — new toggle commands tagged for the `titlebar` surface.
- **apps/desktop** (Rust / Tauri):
  - `src-tauri/tauri.conf.json` — main window `titleBarStyle: "Overlay"` + `decorations: true` + `hiddenTitle: true` so the OS draws native controls over the custom title bar. No new `core:window` permissions beyond the existing `allow-start-dragging`.
  - `src-tauri/src/window_host.rs` — apply the same overlay title bar to child windows (macOS).
- **Dependencies**: none. The native OS window controls are kept via window config (`titleBarStyle`), so no controls library or plugin is added. (The `tauri-controls` library was evaluated and rejected — it crashes under React 19.)
- **Risk**: low — the controls are OS-native; the main surface area is the shell layout restructure, covered by component tests and desktop e2e.
