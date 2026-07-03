## Why

The desktop window is configured with `decorations: false` and a transparent title bar, so the OS draws no chrome — today there is no way to move, minimize, maximize, or close the window from within the app, and no home for global layout controls. The shell also renders only a fixed left sidebar; there is no place to dock secondary or auxiliary content, and no affordance to hide chrome to reclaim space.

## What Changes

- Add a custom title bar row spanning the top of the shell window, hosting native-looking OS window controls (minimize / maximize / close) via the `tauri-controls` library, and a draggable region for moving the window.
- Add a toolbar in the title bar with toggle buttons for: the left sidebar (existing), a new right dock region, a new bottom dock region, and the command palette (existing command center).
- Introduce two new collapsible shell regions — a right dock and a bottom dock — with placeholder content, each with persisted visibility.
- Make the existing left sidebar collapsible from the title bar, with persisted visibility.
- Expose window-control and drag operations through the existing Tauri window boundary (`windows.ts` / `tauriEvents.ts`), guarded by the desktop-host check so the browser build no-ops.
- Register the panel and command toggles as command-center actions so they are also reachable from the palette and rebindable.
- Grant the desktop capability the window permissions the controls require (`minimize`, `maximize`/`unmaximize`/`toggle-maximize`), and register `tauri-plugin-os` if `tauri-controls` requires it for platform detection.

## Capabilities

### New Capabilities

- `desktop-title-bar`: The title bar chrome — OS window controls, drag region, the panel/command toggle toolbar, the left/right/bottom dock regions and their persisted visibility state, and the command-center actions that drive the toggles.

### Modified Capabilities

<!-- None. The title bar is new chrome above the existing shell. The `ui-shell` capability governs the recursive terminal panel tree in the content area, which is unchanged. Command-center actions are additive and specified in the new capability. -->

## Impact

- **apps/ui** (React frontend):
  - `app/components/shell/RootLayout.tsx` — restructure `ShellChrome` into a vertical `title-bar row` + `body row`; body hosts left sidebar, content outlet, right dock, bottom dock.
  - New `app/components/shell/TitleBar.tsx` (title bar + `tauri-controls` window controls + toggle toolbar).
  - New right/bottom dock region components (placeholder content).
  - `app/lib/store.ts` / `app/lib/settings/keys.ts` — panel-visibility state (durable settings, mirroring `sidebarExpandedKey`).
  - `app/lib/windows.ts` / `app/lib/tauriEvents.ts` — `minimizeSelf` / `toggleMaximizeSelf` / `startDrag` wrappers.
  - `app/lib/commands/ids.ts` + registrations — new toggle actions.
- **apps/desktop** (Rust / Tauri):
  - `src-tauri/capabilities/default.json` — add `core:window:allow-minimize`, `allow-maximize`, `allow-unmaximize`, `allow-toggle-maximize` (and `os:` permissions if the OS plugin is added).
  - `src-tauri/src/lib.rs` — register `tauri_plugin_os` only if `tauri-controls` requires it.
- **Dependencies**: none. The native OS window controls are kept via window config (`titleBarStyle`), so no controls library or plugin is added. (The `tauri-controls` library was evaluated and rejected — it crashes under React 19.)
- **Risk**: low — the controls are OS-native; the main surface area is the shell layout restructure, covered by component tests and desktop e2e.
