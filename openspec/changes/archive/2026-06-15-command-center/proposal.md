## Why

Chrome actions (new project / session / surface, close, split, detach, switch session, logs,
settings) are reachable only by mouse through scattered sidebar and panel-toolbar buttons.
There is no keyboard path to them, and none can fire while the terminal holds focus. 0.0.13
adds a leader-key–activated command palette and a configurable keybinding layer so every action
has a fast, discoverable, rebindable keyboard entry point — the foundation the 0.0.14 minimal
accelerators build on.

## What Changes

- **Leader key** — a configurable key, registered as a native Tauri menu accelerator so it fires
  even when the terminal (xterm) has keyboard focus. Activating it opens the command center
  overlay. Default: `CmdOrCtrl+K`.
- **Command center overlay** — a fuzzy-searchable palette listing all currently-available actions.
  Selecting an action invokes the same handler as its existing toolbar / sidebar / menu control.
  Backed by a new central action registry so 0.0.14 actions register additively.
- **Configurable bindings** — every action has a rebindable key. The selected preset and the
  per-action override map persist in the 0.0.9 global `setting` table (additive use, no migration).
- **Preset profiles** — built-in `default` (fully populated), plus `vim`, `vscode`, `tmux` mapped
  for the actions wired in this milestone and extensible as actions are added. A preset is the
  baseline; individual bindings override it.

Non-goals: per-action native accelerators that fire over terminal focus (only the leader key is
native in 0.0.13 — full set deferred to 0.0.14); new feature actions beyond those that already
exist; any orchestrator / backend / wire-protocol change; a server/web leader-key adapter (left
behind the host-agnostic port); agent actions (0.x is terminal-only).

## Capabilities

### New Capabilities

- `command-center`: leader-key activation (host-agnostic port + desktop Tauri accelerator), the
  action registry, the fuzzy command palette overlay, and the keybinding layer (preset baselines
  + per-action overrides persisted in global settings).

### Modified Capabilities

<!-- None. No existing spec's requirements change; the settings store is used additively. -->

## Impact

- **New (renderer, `apps/ui`):** action registry module; keybinding resolution module (preset +
  override merge, binding parse/format); command palette overlay component (shadcn `Command`);
  host-agnostic leader-key port + desktop adapter; preset tables (`default`/`vim`/`vscode`/`tmux`).
- **New dependency:** `cmdk` (npm, `apps/ui`) — the shadcn `Command` backing for the fuzzy palette;
  generates `components/ui/command.tsx`. No new crate.
- **Settings (additive):** new global keys `keybindings.preset` and `keybindings.overrides`
  (`apps/ui/app/lib/settings/keys.ts`), read/written through the existing reactive `SettingsProvider`.
- **Desktop host (`apps/desktop/src-tauri/src/lib.rs`):** register the leader-key native accelerator
  and emit a `command-center:open` event, alongside the existing `View > Logs` menu wiring. Additive;
  no frozen-seam change.
- **Wiring:** existing inline action handlers (sidebar new-session, panel split/close/detach, etc.)
  are exposed through the registry; their behavior is unchanged.
- **Tests:** unit (action registry, binding resolution, parse/format); component (palette render /
  filter / invoke / dismiss); desktop e2e (palette-open via the emitted event, fuzzy filter, action
  invocation, override persistence); Rust host test for accelerator registration.
