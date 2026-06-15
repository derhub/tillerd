## Context

The app's chrome actions are wired inline at their controls: the sidebar's new-session button
(`SessionSidebar.tsx`), the panel toolbar's split/close/detach (`AppShell.tsx` `renderLeaf`), the
native `View > Logs` menu (`apps/desktop/src-tauri/src/lib.rs` → `app.emit("menu:navigate", …)`,
listened for in `AppShell.tsx`). There is no shared registry and no keyboard activation. Settings
already provide a host-agnostic, reactive global store (`apps/ui/app/lib/settings/context.tsx`,
`useGlobalSetting`) over the orchestrator `setting` table (0.0.9). The desktop host already builds
a native menu and routes menu events to the renderer over Tauri events — the proven pattern this
change reuses for the leader key.

Architecture is frozen at 0.0.6: later 0.x versions are additive on the seams. This change touches
no backend, data model, or wire protocol — it is renderer plus an additive native-menu accelerator.

## Goals / Non-Goals

**Goals:**

- A single leader key, native on desktop so it fires over terminal focus, opens a fuzzy command
  palette of all available actions.
- A central action registry: actions defined once, consumed by the palette (and, later, 0.0.14
  accelerators) — wiring existing controls through it without changing their behavior.
- Configurable, persisted keybindings: preset baseline + per-action overrides in global settings.
- Built-in presets `default` (full), `vim` / `vscode` / `tmux` (mapped for wired actions).

**Non-Goals:**

- Per-action native accelerators firing over terminal focus — only the leader key is native in
  0.0.13; the rest fire in-renderer only when no terminal holds focus. Full set is 0.0.14.
- New feature actions beyond those already shipped; orchestrator / backend / wire changes; a
  server/web leader-key adapter (left behind the port); agent actions (0.x is terminal-only);
  preset entries for actions that do not yet exist.

## Decisions

### Action registry

A renderer module exposes a typed list of actions: stable string `id`, human `title`, optional
`keywords` (fuzzy aliases), a `run()` thunk, and an `isAvailable(ctx)` predicate. Existing inline
handlers are lifted into registry entries; the original controls call the same registry `run()` so
behavior is identical and there is one source of truth. Action ids are stable, kebab-dotted:
`project.new`, `session.new`, `surface.spawn`, `surface.close`, `panel.split-h`, `panel.split-v`,
`surface.detach`, `surface.reattach`, `project.open-new-window`, `session.switch`, `view.logs`,
`app.settings`. The registry is assembled where the handlers live (shell context) so `run()` closes
over the live app callbacks; availability is derived from current context (e.g. detach needs a live
surface). New actions register additively — adding an entry is the only step to surface it.

_Alternative considered:_ a global event bus dispatched by id. Rejected — the thunk-closure registry
keeps handlers colocated with their state and avoids an indirection layer with no current consumer.

### Keybinding resolution

A pure module merges two layers into a resolved `Map<actionId, binding>`: preset baseline →
per-action overrides. A binding is a single accelerator chord (e.g. `CmdOrCtrl+Shift+N`); parse/format
helpers canonicalize a chord and `eventToAccelerator` maps a `KeyboardEvent` to the same canonical
form for matching. Multi-key sequences (vim-style `g s`) are out of scope — each preset, including
`vim`/`tmux`, assigns single chords. Resolution is pure and unit-tested; the palette and the
in-renderer shortcut listener both read the resolved map. Clearing an override falls back to the
preset; an action absent from the active preset has no binding.

### Active leaf for panel-scoped actions

Split / close / detach / spawn act on a specific panel leaf, but the panel tree tracks no active
leaf. Add a renderer-only `activeLeafId` in the shell (additive runtime state, like `detached` —
never written to the frozen `layout_json`), set on pointer-down within a leaf and defaulting to the
sole/first leaf. The registry's panel-scoped commands close over it. `surface.reattach` is excluded
from the palette — it is a child-window-only control with no general main-window meaning.

### Registry as a context

Commands are contributed through a `CommandRegistryProvider`: each owning component calls
`useRegisterCommands(commands)` with its own context-bound thunks (the shell registers panel/surface
commands closed over `activeLeafId`; the sidebar registers project/session commands closed over its
handlers and the live session list for `session.switch`), and the palette reads the merged set via
`useCommands()`. Thunks stay colocated with the state they close over; the registry is the single
read surface and new actions register additively.

### Leader key: host-agnostic port + desktop adapter

A `LeaderKeyPort` exposes `onActivate(handler)` and `setBinding(binding)`. The desktop adapter
registers a native menu accelerator (extending the existing menu build in `lib.rs`) and, on the
menu event, emits `command-center:open`; the renderer listens (mirroring the `menu:navigate`
wiring) and opens the overlay. The port keeps the renderer host-agnostic; a future server/web
adapter (document keydown) lands additively without touching the overlay or registry. The leader
binding is read from settings and pushed to the adapter; on desktop, `setBinding` invokes a Tauri
command that updates the leader menu item's accelerator (the item handle is held in managed state).
The menu event-id → emitted-event mapping is extracted to a pure function so the Rust host test can
assert `command_center → command-center:open` without a running app.

_Alternative considered:_ renderer-only `window` keydown for the leader. Rejected — xterm captures
keys, so a leader that must fire over terminal focus has to be native (the roadmap's explicit
requirement); the port preserves host-agnosticism without giving that up.

### Persistence

Two additive global keys in `apps/ui/app/lib/settings/keys.ts`: `keybindings.preset` (preset name)
and `keybindings.overrides` (JSON map `actionId → binding`). Read/written through the existing
reactive `SettingsProvider` / `useGlobalSetting`, so changes apply live and persist via the 0.0.9
`setting` table. No migration, no schema change.

### Palette UI

`cmdk` (new npm dep in `apps/ui`) backs a shadcn `Command` overlay (`components/ui/command.tsx`),
matching the project's shadcn primitive system and design tokens. cmdk owns fuzzy filtering and
keyboard list navigation; the overlay reads the registry (filtered by availability) and the resolved
bindings for the trailing key hint. Escape / outside-click dismiss.

### Testing

Per the testing-memory layering:

- **Unit (bun:test + happy-dom):** registry assembly + availability; keybinding resolution (preset →
  override merge, clear-to-fallback); binding parse/format; preset table shape.
- **Component (happy-dom — non-layout):** overlay lists actions, filters on query, Enter invokes the
  handler + closes, Escape closes without invoking, binding hint renders.
- **Desktop e2e (tauri-webdriver):** the native accelerator is unreachable via WebDriver (native menu
  is out of the webview), so drive palette-open by emitting `command-center:open` (the established
  emit-the-same-event pattern), then assert fuzzy filter, action invocation, and that an override
  persists across reload.
- **Rust host test:** the leader accelerator is registered on the native menu and its event id maps
  to `command-center:open`.

_Documented gap:_ a native key physically firing over real terminal focus is not e2e-driveable
(WebDriver cannot inject a native keystroke into the native menu). It is covered by the Rust
registration test plus manual verification.

## Risks / Trade-offs

- **Native accelerator not e2e-driveable** → cover the registration in a Rust host test and the
  renderer half via the emitted event; accept manual verification for the physical key-over-terminal
  path (recorded above).
- **Leader default `CmdOrCtrl+K` collides with an app/terminal shortcut** → it is rebindable; the
  default is the common palette convention and is registered natively so it pre-empts the terminal.
- **Partial presets feel incomplete** (`vim`/`vscode`/`tmux` only map wired actions) → intended:
  presets extend as actions are added; `default` is fully populated now.
- **Registry drift** (a control bypasses the registry) → existing controls are migrated to call
  registry `run()` in this change; new actions add an entry. One source of truth.

## Migration Plan

Additive only. New deps and settings keys; no data migration. Rollback = revert the change; the
unused `keybindings.*` setting rows are inert.

## Open Questions

None — all readiness-gate decisions resolved at PROPOSE.
