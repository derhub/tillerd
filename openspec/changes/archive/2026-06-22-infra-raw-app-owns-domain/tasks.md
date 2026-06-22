Each move: strip infra to raw, lift the rule into the app handler/entity, keep the existing bus-level test green (adjust only a test that asserts a relocated error). Moves are independent.

## 1. Settings precedence

- [x] 1.1 `config/setting.rs`: keep raw `get(scope,key)` + `get_all(scope)`; delete `resolve`/`resolve_all`. Move the project-over-global cascade into `app/settings/resolve_setting`; the merge+sort into `resolve_settings`. Existing resolve tests stay green.

## 2. Theme immutability

- [x] 2.1 `config/theme.rs`: replace `discard` with raw `delete(id)`. Move the `Prebuilt` guard into `app/settings/discard_theme` (get → reject → delete). `discard_prebuilt_theme_returns_error` stays green.

## 3. Notification retention

- [x] 3.1 `notification.rs`: `prune` runs the given `DELETE` only; the retention count moves to an `app/notification` handler. Behavior test stays green.

## 4. Daemon kind-agnostic

- [x] 4.1 `runtime/mod.rs`: drop `kind` from `SpawnRequest`. `runtime/daemon.rs`: remove the `kind != Terminal` guard; keep the duplicate-proxy guard. `app/surface/spawn_surface`: reject non-`Terminal` with a validation error before persisting. Update both `SpawnRequest` construction sites for the dropped field — `spawn_surface.rs` and `reconcile_surfaces.rs:40`. Update the unsupported-kind test to assert the app validation error.

## 5. Surface initial state + ordering

- [x] 5.1 `surface_repo.rs`: `create` takes the initial status (no hardcoded `Pending`); remove the live-first `CASE` from the list query. `app/surface`: pass the initial status and apply the live-first ordering via a single named constant. List/spawn tests stay green.

## 6. Project name normalization

- [x] 6.1 Move `name.trim()` out of `infra/project.rs` into `entities::Project::new` (or `app/project` create); stored and returned values now match. Create/rename tests stay green.

## 7. Keybinding precedence

- [x] 7.1 `config/keybinding.rs`: keep raw `defaults()` / `overrides()` reads; delete `resolve`, the `list` merge, and `resolve_action`. Move the override-wins cascade + defaults/overrides merge + default-chord shadowing into the `app` keybinding resolve/list handlers (mirror of task 1). Existing keybinding tests stay green.

## 8. Session list ordering

- [x] 8.1 `session.rs`: drop the `ORDER BY pinned DESC, sort_order ASC` from the `list` query; the handler applies the pinned-float order via a single named ordering constant (mirror of task 5's live-first move). List tests stay green.

## 9. Daemon-PTY API restructure (de-abstract the port)

Builds on task 4 (kind already dropped from `SpawnRequest`). Mechanical + type-driven; the compiler lists every call site. `daemon-pty-client` crate is NOT touched.

- [x] 9.1 Rename dir `infra/runtime/` → `infra/daemon_pty_api/`; update `infra/mod.rs` module path. `daemon.rs`/`transport.rs`/`fake.rs` move verbatim.
- [x] 9.2 `daemon_pty_api/mod.rs`: delete the `SurfaceRuntime` trait. Promote `DaemonRuntime` → concrete `DaemonPtyApi` with the same eight inherent methods (spawn/stop/close/list/input/resize/attach/detach). Keep `SurfaceEventSink`, `SpawnRequest`, `Geometry`. Define a `Runtime` enum `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }` with inherent methods delegating to each variant (static dispatch).
- [x] 9.3 `context.rs`: `Ctx` holds `Runtime` (not `Arc<dyn SurfaceRuntime>`); `runtime()` returns `&Runtime`. Drop the `dyn` import.
- [x] 9.4 Swap the ~12 `FakeRuntime` injection sites (`boot.rs`, `app/*/test_util.rs`) from `Arc::new(FakeRuntime::new()) as Arc<dyn SurfaceRuntime>` to `Runtime::Fake(FakeRuntime::new())`. `FakeRuntime`'s `RuntimeCall` log and existing assertions are unchanged; bus tests stay green.

## 10. Verify gate

- [x] 10.1 Fix-all: `bun run verify` green (format, types, clippy, orchestrator 437 + desktop 55 + all crates); `sg scan`/`sg test` green (0 `entities-app-or-infra-only`/`infra-only-in-app` violations). The desktop-host `SurfaceEventSink`→`SurfaceEvents` migration landed alongside. `bun run e2e` deferred to CI (needs bundle). Flip of the `*-only-in-app` rules to `error` left as a follow-up.
