## Why

The app has no way to configure itself: theme is hardcoded (`className="dark"` in `apps/ui/app/root.tsx`), there is no terminal color-scheme choice, no per-project launch/env overrides, and window size/position resets every launch. Roadmap 0.0.9 adds the settings layer the working app needs, and its preference storage is a prerequisite for 0.0.10 (notification "don't ask again") and 0.0.14 (UX polish). The data model already reserves a `setting` table (frozen at 0.0.6, currently unused), so this lands additively on a seam designed for it.

OS-keychain env-secrets are **deferred** from 0.0.9 (user directive): per-project env in this change is plain env vars only; the `secret_ref` table stays unused and no keychain dependency is added.

## What Changes

- **Settings store** — a host-agnostic, scoped (global / project) key→value store backed by the orchestrator `setting` table (additive INSERT/SELECT; no migration). New orchestrator `settings` module, additive orchestrator API (`get_setting` / `set_setting` / `list_settings`), a typed web-safe SDK client, and a desktop Tauri bridge (`settings_host.rs`) reached through a host-agnostic settings port (mirrors the 0.0.7 `LogSource` port). A future server/web adapter satisfies the same port unchanged.
- **Settings panel** — a non-modal panel opened from the app-shell chrome (bottom-right cluster, beside the 0.0.8 health indicator):
  - Theme light / dark — `root.tsx` `.dark` class becomes dynamic, driven by the stored setting (light/dark token sheets already exist).
  - Terminal color scheme — user-selectable; stored and applied to the xterm instance (full DESIGN.md `terminal-*` token remap stays 0.0.14).
  - "Don't ask again" preference — generic keyed boolean storage in the settings store, consumed by 0.0.10.
- **Window state** — window size / position / maximized persisted and restored across relaunch via the official `tauri-plugin-window-state` (new dependency; native storage, not the `setting` table).
- **Roadmap** — mark the 0.0.9 "Env secrets via the OS keychain" bullet deferred, and the items below deferred to follow-up changes.

**Deferred from this change** (each needs plumbing not budgeted by this slice; the settings store they build on ships here):
- Default command / template selection UI — requires a new template-list API (none exists) and an SDK command-list method.
- Per-project overrides (launch template + plain env) — requires a project-scoped settings UI and launch-executor env injection.
- Sidebar expand-state restore — `SessionSidebar` has no expand/collapse UI; the project tree is a 0.0.14 item ("persisted via 0.0.9"). The persistence mechanism is in place.

No breaking changes: every API, command, and SDK addition is additive on seams frozen at 0.0.6.

## Capabilities

### New Capabilities

- `settings-store`: scoped (global / project) key→value settings persisted in the orchestrator `setting` table, exposed through a host-agnostic port — orchestrator API, web-safe SDK client, and desktop Tauri bridge. Project scope falls back to global on miss. Includes generic "don't ask again" keyed-boolean storage.
- `settings-panel`: the app-shell settings UI — open/close affordance, theme switch, terminal color-scheme selection, default command-library/template selection, per-project launch-template and plain-env overrides, and persisted sidebar expand state. Renders instantly (progressive UI; no blocking screen).
- `window-state`: window size / position / maximized persisted natively via `tauri-plugin-window-state` and restored on relaunch.

### Modified Capabilities

<!-- None: the settings entry point, dynamic theme, and terminal-scheme application are net-new behavior owned by the new capabilities above; no existing spec's requirements change. -->

## Impact

- **Backend** — `crates/orchestrator`: new `settings` module + additive API methods over the existing `setting` table; no schema migration.
- **SDK** — `packages/sdk`: new typed, web-safe settings client (orchestrator-client capability).
- **Desktop** — `apps/desktop/src-tauri`: new `settings_host.rs` Tauri bridge commands, registered and added to the `command_contract.rs` dynamic-ACL contract test; `tauri-plugin-window-state` wired into the Tauri builder; window config (`tauri.conf.json`).
- **UI** — `apps/ui`: settings panel component + host-agnostic settings port adapter; dynamic theme in `root.tsx`; terminal-scheme application in `DesktopTerminalPane`; sidebar expand-state persistence in `ui-session-sidebar`.
- **Dependencies** — adds `tauri-plugin-window-state` (official). No `keyring` / keychain dependency (secrets deferred).
- **Docs** — `ROADMAP.md` 0.0.9 secrets bullet marked deferred.
- **Frozen seams** — `setting` table, orchestrator API, SDK contract, Tauri command surface all touched **additively** only (data model frozen at 0.0.6); `secret_ref` left unused.
