## Why

UI commands are declared in three disconnected places: ids and titles in `commands/ids.ts`, default keybindings in `commands/keybindings.ts`, and `run` handlers scattered across `RootLayout`, `useShellCommands`, and `PanelContent`. Adding one command means editing three files, there is no way to gate a command by context, no way to declare where a command surfaces (palette, title bar, context menu), no icon or toggle/checked state, and the palette shows every command regardless of whether it applies. The `command-center` spec already promises actions "available in context" and a palette that "lists every action currently available in context" — but nothing evaluates context today. This is the foundation the desktop title bar (PR #64) needs, so it lands first.

## What Changes

- Introduce a single **command contribution model**: each command is declared once as a `CommandDef` carrying `id`, `title`, `keywords`, `category`, `icon`, `surfaces` (where it appears), `group`, an optional `when` context expression, and an optional `toggle` (checked-state) selector. Handlers are registered by id, decoupled from declaration, and may close over live context.
- Add a **minimal `when` context system**: a context-key store plus a small evaluator (AND of named keys with negation). Gates palette visibility, keybinding activation, and command enablement. Seed the existing `terminalFocus`-style guard as a real context key.
- Add **surface tags**: commands declare which surfaces they appear in (`palette`, `titlebar`, `contextmenu`) plus a `group`, so the title bar toolbar and palette are data-driven off one table.
- Add **first-class toggle commands**: a `toggle` selector drives a checked/on-off state; the palette renders a check and title bar buttons render active/inactive from the same source.
- Keep and extend the existing keybinding layer — canonical `Accelerator` format, presets, per-action overrides, leader key, resolution, mac display — now sourced from the command table's `defaultKeys` instead of a separate `PRESETS` map, and activation gated by `when`. Single-chord only (sequences deferred).
- Migrate all existing commands (project/session/surface/panel/view/settings actions) onto the new model; the palette, global shortcut dispatch, and keybinding settings read from the single registry.

## Capabilities

### New Capabilities

- `ui-command-manager`: the command contribution model — `CommandDef` declaration, handler-by-id registration, the `when` context-key store and evaluator, surface tags/groups, toggle/checked state, and the single resolution path that feeds the palette, global shortcuts, and keybinding settings.

### Modified Capabilities

- `command-center`: the palette now filters by `when` context and surface tag, renders category/group and toggle-checked state, and sources bindings and actions from the command manager's single registry rather than the ad-hoc id/keybinding split. Leader key, presets, overrides, and persistence behavior are unchanged.

## Impact

- **apps/ui**:
  - New `app/lib/commands/registry` extended to the contribution model (`CommandDef`, `registerCommand`/handler-by-id, resolution).
  - New context-key store + `when` evaluator (`app/lib/commands/context.ts`, `when.ts`).
  - `commands/ids.ts` → becomes/feeds the central command table (metadata + `defaultKeys` + surfaces + `when` + toggle).
  - `commands/keybindings.ts` — presets sourced from the table; format/resolution retained.
  - `commands/useKeybindings.ts` — dispatch gated by `when`.
  - `command/CommandCenter.tsx` — filter by `when`+surface, render group/category + toggle check.
  - Migrate registration sites (`RootLayout`, `useShellCommands`, `PanelContent`) to `registerCommand(id, handler)`.
- **Dependencies**: none new (pure frontend refactor).
- **Downstream**: the desktop title bar (PR #64) consumes this — its panel/command toggles become toggle `CommandDef`s tagged `titlebar`. PR #64 is paused until this lands.
- **Risk**: broad migration touching every command registration; mitigated by keeping the `Accelerator` format and settings keys stable and migrating incrementally behind the existing `useCommands()` consumer contract.
