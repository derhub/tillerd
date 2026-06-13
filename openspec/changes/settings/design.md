## Context

Backend is the Rust orchestrator (ADR-0022/0023); TS is UI + SDK; desktop host is Tauri v2. The architecture froze at 0.0.6 — later 0.x versions are additive on the data model, orchestrator API, SDK contract, and command surface. Host-facing renderer features must be host-agnostic (port + adapters), because a web/server host is expected before v1; the 0.0.7 log viewer established the pattern (`LogSource` port, `apps/ui/app/lib/transport/log-source.ts`).

Current state relevant to 0.0.9:
- The `setting` table (`scope, project_id, key, value_json`, PK `(scope, project_id, key)`) and `secret_ref` table already exist in `crates/orchestrator/src/persistence/schema.rs` `migration_v1`, both **unused**.
- A separate desktop-local JSON store (`apps/desktop/src-tauri/src/store.rs`, `pref_*`/`registry_*`) backs the session→cwd registry only.
- Theme is hardcoded `className="dark"` in `apps/ui/app/root.tsx`; light/dark token sheets exist (`app.css`).
- The Tauri window is static 800×800; no window-state persistence.

User directive: OS-keychain env-secrets are deferred from 0.0.9.

## Goals / Non-Goals

**Goals:**

- A scoped (global/project) settings store on the frozen `setting` table, reached host-agnostically (orchestrator API → SDK client → desktop bridge), additive only.
- A settings panel: theme, terminal color scheme, default command/template, per-project launch-template + plain-env overrides, persisted sidebar state.
- Native window size/position/maximized persistence across relaunch.
- Generic "don't ask again" storage for 0.0.10.

**Non-Goals:**

- OS keychain, `secret_ref` usage, env-secret references (deferred; plain env only).
- Full DESIGN.md `terminal-*` token remap (0.0.14) — only store + apply the chosen scheme.
- Retry/restart recovery (frozen, read-only supervision seam).
- Removing the desktop-local `pref_*` store (still backs the registry).
- New schema migration (the tables already exist).

## Decisions

### D1 — Settings live in the orchestrator `setting` table, not the desktop-local store

Cross-host settings persist via the orchestrator over the existing `setting` table. **Why:** host-agnostic by mandate — a future server/web host reads the same settings unchanged; the table was reserved for exactly this and is frozen. **Alternative considered:** the desktop-local `desktop-store.json` `pref_*` map — rejected because it is Tauri-only and would not survive the host-agnostic requirement, forcing a migration later. The desktop-local store stays for the session registry only.

### D2 — A `settings` module inside `crates/orchestrator`, not a new crate

The store logic is a new module of the orchestrator (`crates/orchestrator/src/settings/`), with additive public API methods (`get_setting` / `set_setting` / `list_setting(s)`, project-resolves-over-global). **Why:** the layout preference is to add modules to existing crates, not new crates; settings are orchestrator-owned domain. **Alternative:** a standalone settings crate — rejected (no independent reuse or build boundary justifies the friction).

### D3 — Host-agnostic settings port mirrors `LogSource`

The renderer reaches settings through a `SettingsSource`-style port: a web-safe SDK client method set, with a desktop adapter implemented as Tauri bridge commands in a new `apps/desktop/src-tauri/src/settings_host.rs`. New commands are registered in the Tauri builder and added to the `command_contract.rs` dynamic-ACL contract test. **Why:** identical seam to 0.0.7; a later server adapter satisfies the same port. **Trade-off:** more layers than a direct Tauri call, but it is the established and mandated pattern.

### D4 — Window geometry via `tauri-plugin-window-state`, separate from settings

Window size/position/maximized use the official `tauri-plugin-window-state` plugin (its own native storage), not the `setting` table. **Why:** window geometry is a desktop-window concern, meaningless on a server host, and the official plugin is the canonical zero-maintenance solution. **Alternative:** hand-rolled save/restore on Tauri window events through the desktop store — rejected as more code to own for no benefit. New dependency, flagged and approved.

Verified against current docs (2026-06-14): plugin **2.4.1** (Tauri v2; `tauri = "2"` compatible). Wire **Rust-only** — no JS guest bindings — via the existing `.plugin(...)` chain in `apps/desktop/src-tauri/src/lib.rs` (same pattern as `tauri-plugin-webdriver`): `.plugin(tauri_plugin_window_state::Builder::default().build())`, desktop-gated. The default `Builder` auto-saves on exit and auto-restores on launch (`StateFlags::all()` — size/position/maximized), so no manual save/restore calls are needed. Add the dep with the desktop target cfg `cfg(any(target_os = "macos", windows, target_os = "linux"))`, and grant `"window-state:default"` in `apps/desktop/src-tauri/capabilities/default.json` (commands are blocked until permitted).

### D5 — Theme application becomes dynamic and read-before-paint

`root.tsx` stops hardcoding `.dark`; the appearance class is driven by the persisted theme setting and applied before first paint to avoid a flash. The terminal scheme is applied to the terminal surface from the persisted setting. **Why:** progressive/optimistic UI — instant render, no blocking screen. The setting read is fast and local; on miss the default (dark) applies.

### D6 — Defaults and per-project overrides reuse existing domain

"Default command/template" and per-project overrides are stored as settings keys resolved at session/surface creation. Per-project launch-template override and plain-env are project-scoped settings consumed by the existing launch/executor path; env values are injected into the launched process environment alongside the existing env allowlist. **Why:** keeps the launch contract intact; settings only supply values it already accepts.

## Risks / Trade-offs

- **Theme flash on load** → apply the persisted appearance class before first paint (inline at document root), not after hydration.
- **Per-project env vs the env allowlist** → project env override must compose with the orchestrator's existing ENV_ALLOWLIST rules, not bypass them; treat overrides as additive user env, not a replacement of the controlled vars.
- **New dependency (`tauri-plugin-window-state`)** → official Tauri plugin, low risk; pin and add to the workspace per the every-crate-package.json/turbo conventions where applicable (desktop app already has a package.json).
- **command_contract.rs ACL test** → new bridge commands must be added there or the contract test fails; this is the known trigger for that file. Cover the new commands.
- **happy-dom has no layout** → theme/scheme application and persistence round-trip that depend on real rendering go to desktop e2e (tauri-webdriver); panel logic/state unit-tests under bun:test + happy-dom.
- **Live cross-component propagation** → settings are held in one reactive provider (`SettingsProvider`), hydrated once via `listSettings`, so the panel and every mounted terminal read the same state and re-render together on a change. Per-component setting reads (independent local state) do NOT propagate live — a scheme change would miss already-mounted terminals until remount; the e2e (`settings-terminal-scheme`) guards this by asserting a mounted terminal's background flips on change.
