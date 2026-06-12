# Tasks — Launch Execution

Test-first throughout (red → green → refactor). Each task traces to a spec requirement and honors
ADR-0026 (uniform adapter dispatch over a generic spawn) and ADR-0024 (one proxy per surface;
`surface_id` is the daemon key). Built on the parked `feature/0-0-5-launch-system` scaffold.

## 1. Generic spawn (spec: surface-spawn)

- [ ] 1.1 Add `ResolvedCommand { exe, args, env }` and a generic `spawn(surface_id, command: Option<ResolvedCommand>, cwd) -> (DaemonConnection, rx)` in the surface runtime that hands the command to the pseudo-terminal service; `None` spawns the login shell
- [ ] 1.2 Test: a resolved command reaches the daemon with exe/args/env/cwd keyed by `surface_id`; `None` requests the login shell
- [ ] 1.3 Test: the spawn yields the per-surface proxy — output bytes stream over the event sink tagged with `surface_id`

## 2. Adapter seam (spec: surface-runtime)

- [ ] 2.1 Define `SurfaceAdapter { async fn launch(surface_id, command, ctx) -> Proxy }` and `LaunchCtx` (generic spawn, gate access, store, event sink)
- [ ] 2.2 Add a registry keyed by target kind; the runtime creates a surface by `registry[kind].launch(...)` with no other kind branching
- [ ] 2.3 Extract `TerminalAdapter` (calls the generic spawn) from `open_terminal`
- [ ] 2.4 **Remove** `open_terminal` and `open_agent`; rewire `create_terminal_surface` / `create_agent_surface` and the daemon-host callers to dispatch through the registry
- [ ] 2.5 Test: terminal kind → terminal adapter; agent kind → agent adapter; registering a new kind needs no change to the dispatch
- [ ] 2.6 Test: existing terminal behavior preserved (reuse the 0.0.2 streaming/status/resize/reattach tests against the new seam)

## 3. Agent adapter (spec: agent-adapter)

- [ ] 3.1 Extract `AgentAdapter::launch`: subscribe to the gate by `surface_id` before spawn → install hooks → generic spawn of the item's command → drain hooks into status/content; teardown (cancel subscription + uninstall hooks) on removal
- [ ] 3.2 Test: subscription precedes spawn; hook events become status/content tagged with `surface_id`; removal tears down; a refused subscription fails with a typed error and spawns nothing
- [ ] 3.3 Slim `AGENT_DEF` / `AgentDefinition`: remove `binary`, `args_template`, resolution; keep hook parse, `interrupt_seq`, version range, hook install/teardown
- [ ] 3.4 Test: the launch command comes from the item, not the definition; status mapping is still owned by the definition
- [ ] 3.5 Keep the agent UI pane and hook parse/status logic (relocated, behavior unchanged)

## 4. Launch executor (spec: launch-execution)

- [ ] 4.1 Resolve each item's command (library reference → stored command; inline → as given; unknown → typed not-found, no surface)
- [ ] 4.2 Dispatch by target kind via the registry; unsupported target → typed unsupported-kind error
- [ ] 4.3 Worktree step: run against an explicit repository root (not the process cwd), set the surface working directory and `worktree_id`; a failing step fails the item, no surface
- [ ] 4.4 Pre/post/auto-spawn scripts via the existing process-launch crate: pre before the surface (failure skips it, others continue), post after start, auto-spawn as background processes
- [ ] 4.5 Record placement on the surface; run items in declared order; record best-effort per-item outcomes
- [ ] 4.6 Tests: order + best-effort; command resolution (3 cases); target dispatch (2 cases); worktree (2 cases); scripts (pre-fail-skip, post-after-start); placement persisted

## 5. Template instantiation fixes (spec: launch-execution)

- [ ] 5.1 Copy the template's launch spec + version into a new session in a single transaction
- [ ] 5.2 `set_launch_template_spec` returns a typed not-found for an absent template
- [ ] 5.3 Tests: session created from a template carries a copy of the spec and diverges; updating an absent template is not-found

## 6. Workspace IPC completeness (spec: workspace-ipc)

- [ ] 6.1 Add host handlers for project rename/archive, session rename/archive, and command get/delete; confirm project/session create/list, layout get/set, and command list/create are present; register all in the host command set
- [ ] 6.2 Tests: each client call reaches its store operation; an operation on an absent identifier returns a typed not-found

## 7. Command library (spec: command-library)

- [ ] 7.1 Make `seed_commands` idempotent via a single insert-or-ignore statement (no lock-release-relock); safe under concurrent open
- [ ] 7.2 Tests: repeated open and concurrent open each leave one copy of every prebuilt command; create → get → list → delete round-trip

## 8. Verify + cleanup

- [ ] 8.1 `cargo test -p tillerd-orchestrator` green; `cargo clippy -p tillerd-orchestrator --all-targets --locked -- -D warnings` clean
- [ ] 8.2 `bun run verify` green (format / check-types / lint / test / e2e)
- [ ] 8.3 Confirm no dead code remains: the stub executor, the `open_*` methods, and the agent definition's launch fields are gone with no dangling references
