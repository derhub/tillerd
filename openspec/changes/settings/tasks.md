## 1. Settings store (orchestrator)

- [x] 1.1 Add a `settings` module to `crates/orchestrator` with persistence over the existing `setting` table (no migration): scoped get/set/list and project-resolves-over-global. TDD all `settings-store` scenarios — round-trip (global + project), overwrite-replaces, restart survival, project-over-global, global fallback, unknown-key-absent, list-by-scope, and the "don't ask again" keyed-boolean get/set.
- [x] 1.2 Expose additive orchestrator API methods (`get_setting` / `set_setting` / `list_settings`, plus project-resolved read) and wire them into the orchestrator's transport-agnostic API surface. (Store trait is the API surface, exposed via `Orchestrator::store()`.)

## 2. Host-agnostic access (SDK + desktop bridge)

- [x] 2.1 Add a web-safe settings client to `packages/sdk` (typed get/set/list over the orchestrator API) satisfying the `settings-store` "host-agnostic access" requirement; unit-test the round-trip against a fake transport.
- [x] 2.2 Add `apps/desktop/src-tauri/src/settings_host.rs` Tauri bridge commands delegating to the orchestrator settings API; register them in the Tauri builder and add them to the `command_contract.rs` dynamic-ACL contract test.

## 3. Window state

- [x] 3.1 Add `tauri-plugin-window-state` (v2.4.1) to `apps/desktop/src-tauri/Cargo.toml` with target cfg `cfg(any(target_os = "macos", windows, target_os = "linux"))`; init Rust-only via the existing `.plugin(...)` chain in `lib.rs` (`Builder::default().build()`, desktop-gated — no JS bindings, default auto-save/restore); grant `"window-state:default"` in `capabilities/default.json`; default geometry stays the `tauri.conf.json` 800×800. DOCUMENTED GAP: the `window-state` scenarios (size/position/maximized restore) cannot be asserted via tauri-webdriver — native window geometry is unreachable from the webview DOM (see testing memory). Coverage = the official plugin's own test suite + the compile/registration check; manual verification on relaunch.

## 4. Settings panel UI

- [x] 4.1 Add a host-agnostic settings port adapter in `apps/ui/app/lib/transport` (desktop adapter over the new bridge), mirroring the `LogSource` shape; unit-test the adapter contract with an injected fake.
- [x] 4.2 Build the settings panel component + chrome affordance (gear in the bottom-right cluster, non-modal popover, renders instantly). Panel open/close interaction is portal/layout-dependent → e2e (per testing memory); control state covered by the hook tests.
- [x] 4.3 Make theme dynamic: drive `root.tsx` appearance class from the persisted theme setting applied before first paint (paint script + localStorage cache, durable value in the `setting` table); wire the panel's light/dark control. Apply + persist + cache covered by theme/hook unit tests; visual relaunch-restore → e2e.
- [x] 4.4 Add terminal color-scheme selection: apply the persisted scheme to terminal surfaces (`DesktopTerminalPane`, initial + live update via ref) and persist the choice; scheme resolution covered by unit tests; visual apply → e2e.
- [ ] 4.5 Sidebar expand state — DEFERRED: `SessionSidebar` has no expand/collapse UI in 0.0.x (the project tree expand/collapse is a 0.0.14 item, "persisted via 0.0.9"). The persistence mechanism (settings store + `SIDEBAR_EXPANDED_KEY`) is in place; the tree UI that consumes it lands with 0.0.14. Flagged for user.

## 5. Defaults and per-project overrides (DEFERRED to follow-up changes)

- [~] 5.1 Default command + default template resolution at creation — DEFERRED: the selection UI needs a template-list API (none exists) + an SDK command-list method. The settings store this builds on ships in this change.
- [~] 5.2 Per-project launch-template + plain-env override — DEFERRED: needs a project-scoped settings UI + launch-executor env injection (composing with ENV_ALLOWLIST).

## 6. Docs and verify gate

- [x] 6.1 Mark the ROADMAP.md 0.0.9 "Env secrets via the OS keychain" bullet deferred (plus the defaults/per-project/sidebar deferrals); CHANGELOG updated.
- [x] 6.2 Final fix-all gate: format + check-types + lint + full test suite (18/18) + desktop e2e (10/10) all green; shipped spec scenarios map to tests (settings-store, theme/scheme) or documented e2e/manual coverage (panel open/close, window geometry).
