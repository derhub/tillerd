## Why

ADR-0036 de-abstracted storage into `entities/ + infra/ + app/`, but domain logic still leaks into `infra/`: stores and repos gatekeep business invariants, retention and precedence policies, initial-state and ordering decisions, and capability rules. That defeats the layering — the boundary rules now in flight (`entities-app-or-infra-only`, `infra-only-in-app`) push toward one model: **infra is a dumb raw API; `app/` owns every domain rule and is the sole integrator of entities + infra.** An audit of `infra/` found eight concrete leaks; this change moves them.

## What Changes

- Establish the rule: `infra/` SHALL contain only raw operations (sqlx execute/bind, column↔field mapping, socket I/O, wire encode/decode, fs read/write). All domain rules live in `app/` use-case handlers; `entities/` hold pure data + rules.
- Move six audited leaks — each strips infra to a bare getter/setter/deleter and lifts the rule into the app handler, keeping the existing bus-level behavior test green as the safety net:
  1. `config/setting.rs` `resolve`/`resolve_all` (project-overrides-global precedence + merge/sort) → `app/settings` handlers over raw `get`/`get_all`.
  2. `config/theme.rs` `discard` Prebuilt-immutable guard → `app/settings/discard_theme`; infra exposes raw `delete`.
  3. `notification.rs` `prune` retention cap → `app/notification` prune handler; infra runs the provided `DELETE`.
  4. `runtime/daemon.rs` `launch` `kind != Terminal` guard → `app/surface/spawn_surface`; **BREAKING (internal):** `SpawnRequest` drops `kind`, the daemon becomes a kind-agnostic raw "spawn a PTY". The duplicate-proxy guard stays in infra as raw resource integrity.
  5. `surface_repo.rs` initial status `Pending` + live-first `ORDER BY` → app passes the initial status and owns the sort.
  6. `project.rs` `name.trim()` normalization → `entities::Project::new` / `app/project` create.
  7. `config/keybinding.rs` precedence (`resolve` override-wins, `list` defaults+overrides merge, `resolve_action` default-chord shadowing) → `app` handlers over raw `defaults`/`overrides` reads. Exact mirror of move 1; leaving it while moving `setting.rs` precedence would be incoherent.
  8. `session.rs` `list` `ORDER BY pinned DESC, sort_order` (pinned-float) → app-owned ordering constant. Same semantic-precedence-ordering class as move 5's live-first sort.
- Restructure the surface runtime as a concrete raw API: rename `infra/runtime/` → `infra/daemon_pty_api/` and **de-abstract the `SurfaceRuntime` port** — drop the `Arc<dyn SurfaceRuntime>` trait + dynamic dispatch in favor of a concrete `DaemonPtyApi` (the real daemon-socket client) with static dispatch. `Ctx` holds an enum over the two concrete runtimes (`DaemonPtyApi` for prod, `FakeRuntime` for tests) so the existing bus tests keep their `RuntimeCall` assertions and stay green. `SurfaceEventSink` (the output port) is untouched. **BREAKING (internal):** `Ctx::runtime` returns the enum, not `&dyn SurfaceRuntime`; ~12 `test_util`/`boot` sites swap `Arc<dyn SurfaceRuntime>` for the enum's `Fake` variant.
- Reclassified as raw, left in place: `session.rs` status↔`archived_at` column mapping; deterministic alphabetical list sorts (`setting`/`theme`/`profile`/`keybinding` `list` by key/id — stable ordering, not semantic precedence); the `theme`/`profile` `get_active` pointer-then-load composition and `profile.duplicate` field copy (no rule check); the migration `Default`/`Unfiled` seeds (with the canonical id constants owned by `entities`); the **`daemon-pty-client` crate stays as-is** — it is already pure raw wire codec (frame encode/decode, `SessionFrame`, request encoders), the layer *below* infra that `daemon_pty_api`'s transport consumes; not in scope, not deleted.
- Record the decision as an ADR superseding the relevant parts of ADR-0036.

## Capabilities

### New Capabilities

- `infra-raw-boundary`: infra is a raw API and app owns all domain logic — the normative rule, the raw/domain split criteria, and its enforcement by the `entities-app-or-infra-only` / `infra-only-in-app` rules.

### Modified Capabilities

- `surface-runtime`: the "only a Terminal surface spawns a PTY" capability rule is enforced in the app spawn handler before any effect (no throwaway `pending`→`failed` row); the runtime spawns a PTY for any request it is given. The runtime is a concrete `DaemonPtyApi` raw client (no `SurfaceRuntime` port); the app depends on it by static dispatch.

## Impact

- `crates/orchestrator/src/infra/config/{setting,theme,keybinding}.rs`, `notification.rs`, `surface_repo.rs`, `session.rs`, `project.rs` — strip to raw operations.
- `crates/orchestrator/src/infra/runtime/` → `crates/orchestrator/src/infra/daemon_pty_api/` — directory rename; `daemon.rs`/`transport.rs`/`fake.rs` move; `mod.rs` drops the `SurfaceRuntime` trait + `SpawnRequest.kind`, exposes concrete `DaemonPtyApi` + the `Ctx`-side runtime enum.
- `crates/orchestrator/src/context.rs` — `Ctx` field/accessor switch from `Arc<dyn SurfaceRuntime>` to the concrete runtime enum.
- `crates/orchestrator/src/app/settings/{resolve_setting,resolve_settings,discard_theme}.rs`, `app/notification/*`, `app/surface/spawn_surface.rs`, `app/project/*`, the keybinding resolve/list handlers, and the session-list ordering handler — handlers gain the lifted rules.
- `crates/orchestrator/src/entities/{project,workspace}.rs` — normalization + canonical id constants.
- `boot.rs` + ~11 `app/*/test_util.rs` — swap the `FakeRuntime` injection from `Arc<dyn>` to the enum variant.
- `crates/daemon-pty-client/` — untouched (raw wire codec, below infra).
- `docs/adr/0038-*.md` — records the decision.
- Behavior is preserved; existing bus-level tests stay green (one refinement: non-Terminal spawn now returns an app validation error up front).
