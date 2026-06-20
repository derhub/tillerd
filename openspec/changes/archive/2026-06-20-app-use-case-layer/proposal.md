## Why

ADR-0035's relayering (entities/ + infra/ + store/) left cross-aggregate coordination misplaced: `create_session` is parked at the store root (`store/storage.rs`), its own doc-comment admitting it is application work, not repository work; and the two-phase session-open flow (create-then-activate) is assembled inside the tauri controller `workspace_host::session_create`, which calls `create_session` then `SurfaceApi::launch_session`. That cross-aggregate sequencing leaking through a controller is the anemic-controller anti-pattern design D7 named. R2/R3/R4 of ADR-0035 close this by formalizing an `app/` use-case layer.

## What Changes

- Add an `app/` use-case layer to `crates/orchestrator` that owns cross-aggregate orchestration.
- Move `create_session` verbatim from `store/storage.rs` into `app/` (R2). **BREAKING** (pre-v1, internal): the re-export path moves from `store::create_session` to `app::create_session`, no back-compat alias; the single caller is updated.
- Add an `app::open_session` use case sequencing create-then-activate: `create_session` followed by best-effort surface activation (R3 surface + R4 launch).
- Rewire `workspace_host::session_create` to delegate to `app::open_session`, reducing the tauri controller to a pure IPC shim.
- Behavior-preserving: no observable change, no new dependency, no new crate. Existing unit + integration + e2e suites are the guard.

Non-goals: postgres backend; relocating `SurfaceApi`'s internal cross-aggregate spec methods (it stays the surface-runtime port); moving `launch/spec.rs` or `launch/executor.rs` (they stay as domain utilities).

## Capabilities

### New Capabilities

- `app-use-case-layer`: cross-aggregate session/surface/launch coordination lives in a host-agnostic `app/` use-case layer; hosts (tauri, future server) delegate to it rather than assembling the sequence themselves.

### Modified Capabilities

<!-- none — behavior is preserved; this is an internal relayering with no requirement changes -->

## Impact

- `crates/orchestrator/src/app/` (new module), `store/storage.rs` + `store/mod.rs` (remove `create_session` + its re-export), `apps/desktop/src-tauri/src/workspace_host.rs` (controller becomes a shim).
- Test import updates: `crates/orchestrator/tests/store_architecture.rs`, `surface/api.rs` test helper.
- Frozen seam preserved: tauri command contract (names/args/returns) unchanged.
