# Tasks — Launch Execution

Test-first throughout (red → green → refactor). Each task traces to a spec requirement and honors
ADR-0024 (one proxy per surface; `surface_id` is the daemon key) and ADR-0027 (0.x is terminal-only;
the agent surface is deferred to 1.x). Built on the parked `feature/0-0-5-launch-system` scaffold.

## 1. Generic spawn (spec: surface-spawn)

- [x] 1.1 Add `ResolvedCommand { exe, args, env }` and a generic `spawn(surface_id, command: Option<ResolvedCommand>, cwd) -> (DaemonConnection, rx)` in the surface runtime that hands the command to the pseudo-terminal service; `None` spawns the login shell
- [x] 1.2 Test: a resolved command reaches the daemon with exe/args/env/cwd keyed by `surface_id`; `None` requests the login shell
- [x] 1.3 Test: the spawn yields the per-surface proxy — output bytes stream over the event sink tagged with `surface_id` (covered by the existing terminal streaming tests via `open_terminal`)

## 2. Adapter seam (spec: surface-runtime)

- [x] 2.1 Uniform `launch_surface(surface, kind, command, ...)` dispatch keyed by `SurfaceKind`; `launch_terminal` calls the generic spawn
- [x] 2.2 The runtime brings a surface to life only through `launch_surface` with no other kind branching
- [x] 2.3 Terminal adapter (`launch_terminal`) calls the generic spawn
- [x] 2.4 **Removed** `open_terminal`; rewired `create_terminal_surface` and every test call site to `launch_surface`
- [x] 2.5 Test: `launch_surface_rejects_unsupported_kind` (diff) + terminal dispatch covered by existing tests
- [x] 2.6 Existing terminal streaming/status/resize/reattach tests pass through the new seam

## 3. Remove the agent surface — terminal-only 0.x (ADR-0027)

- [x] 3.1 Orchestrator: delete the `agent` module (`definition`/`parse`/`setup`), `launch_agent`/`AgentProxy`/`resolve_agent_command`/`agent_def`, the `SurfaceKind::Agent` variant, `create_agent_surface`, and the `agent-cli` seed; `launch_surface` is terminal-only (diff → unsupported). The gate stays (shared hook-ingress/MCP infra; only the agent's gate subscription is removed)
- [x] 3.2 Desktop host: remove `surface_create_agent` + `agent_bootstrap` IPC commands and the agent bootstrap module; `SurfaceApi::new` (no gate socket)
- [x] 3.3 TS: delete the agent-adapter package and the retired `engine` / `platform-bun` packages; remove agent types from `packages/sdk`; remove the renderer agent path (`agent_bootstrap`, agent surface UI/transport); strip the deps from every package.json. (CLI install/uninstall + `apps/server/src/index.ts` were also retired in follow-up cleanup)
- [x] 3.4 Tests: `cargo test -p tillerd-orchestrator` green after removal; terminal streaming/status/resize/reattach unaffected
- [x] 3.5 Keep the gate, hook ingress, mcp-gateway, and memorya (shared infra) untouched

## 4. Launch executor (spec: launch-execution)

- [x] 4.1 Resolve each item's command (library reference → stored command; inline → as given; unknown → typed not-found, no surface). The executor hands the `ResolvedCommand` to a `SurfaceLauncher` trait (production impl dispatches to `launch_surface`; tests record). NOTE: the production launcher (runtime + agent config) and wiring the executor into session-creation are still pending — the executor has no production caller yet (pre-existing gap).
- [x] 4.2 Dispatch by `surface_kind_for(target)`; unsupported target → typed `UnsupportedSurfaceKind`
- [x] 4.3 Worktree step: run against an explicit repository root (not the process cwd), set the surface working directory and `worktree_id`; a failing step fails the item, no surface
- [x] 4.5 Record placement on the surface; run items in declared order; record best-effort per-item outcomes. Auxiliary runners (e.g. a dev server) are ordinary terminal items with a placement; closing the pane leaves the process running (soft-delete keeps the PTY) — no separate script concept
- [x] 4.6 Tests: order + best-effort; command resolution; target dispatch; placement; worktree (2 cases)

## 5. Template instantiation fixes (spec: launch-execution)

- [x] 5.1 Copy the template's launch spec + version into a new session in a single transaction (serialized by the connection lock; not-found on absent template)
- [x] 5.2 `set_launch_template_spec` returns a typed not-found for an absent template (checks rows affected; both stores)
- [x] 5.3 Tests: session created from a template carries a copy of the spec and diverges; updating an absent template is not-found

## 6. Workspace IPC completeness (spec: workspace-ipc)

- [x] 6.1 Added host handlers for project rename/archive, session rename/archive, and command get/delete; project/session create/list, layout get/set, command list/create present; all registered in the host command set
- [x] 6.2 Tests: each `do_*` handler reaches its store operation; an operation on an absent identifier returns a typed not-found

## 7. Command library (spec: command-library)

- [x] 7.1 `seed_commands` is a single `INSERT OR IGNORE` per prebuilt under one lock (no exists-check/release/re-insert window); `busy_timeout` added so concurrent opens serialize
- [x] 7.2 Tests: repeated open and concurrent open (4 threads) each leave one copy of every prebuilt command; create → get → list → delete round-trip

## 8. Verify + cleanup

- [x] 8.1 `cargo test -p tillerd-orchestrator` green; `cargo clippy -p tillerd-orchestrator --all-targets --locked -- -D warnings` clean
- [x] 8.2 `bun run verify` green (format / check-types / lint / test / e2e) — EXIT=0 after the agent purge
- [x] 8.3 Confirmed no dead code remains: the stub executor, the `open_*` methods, and the entire agent surface (orchestrator module, `SurfaceKind::Agent`, desktop IPC + bootstrap, TS agent + retired packages) are gone with no dangling references
