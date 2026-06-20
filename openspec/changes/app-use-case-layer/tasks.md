## 1. Promote `create_session` into `app/` (R2)

- [x] 1.1 Add `crates/orchestrator/src/app/` (`mod.rs` + `session.rs`); register `pub mod app;` in `lib.rs`. Move `create_session` verbatim from `store/storage.rs` into `app/session.rs`; drop its `store/mod.rs` re-export and re-export it as `app::create_session`. Update the two callers' imports (`surface/api.rs` test helper, `tests/store_architecture.rs`). Suite stays green.

## 2. `open_session` use case over a `SessionActivator` port (R3 + R4)

- [x] 2.1 Define the `SessionActivator` port in `app/` (RPITIT `fn activate(&self, &SessionId) -> impl Future<Output = Result<()>>`, mirroring `SurfaceLauncher`). Write the `open_session` unit tests first (red): persists + invokes the activator once; activation error is non-fatal, session still returned (fake activator, no daemon).
- [x] 2.2 Implement `app::open_session(draft, &LaunchTemplates, &Sessions, &impl SessionActivator)`: `create_session` then best-effort `activator.activate` (error logged via `tracing::warn`, swallowed), return the session. Green.
- [x] 2.3 Implement `SessionActivator` for `SurfaceApi` by delegating to its existing `launch_session` (discard the per-item result vec).

## 3. Reduce the host controller to a shim (R3/R4 host)

- [x] 3.1 Rewire `workspace_host::session_create` to build the draft and delegate to `app::open_session(draft, &storage.launch_templates, &storage.sessions, &surfaces.api)`; remove the inline `create_session` + `launch_session` + `eprintln` block. Keep the frozen command signature + `SessionResponse` return.

## 4. Verify gate

- [x] 4.1 Run `bun run verify` (format:check + check-types + lint + test + e2e); fix all failures until green.
