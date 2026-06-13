# Launch Execution

## Why

The launch system models commands, worktrees, and placement, but the executor never acts on
them: it resolves a launch item's command only to discard it, always creates a bare login-shell
terminal, and ignores the item's target kind and worktree step. The command library and worktree
step are implemented and tested in isolation but never invoked.

The deeper problem is shape, not wiring. Surface creation is split into kind-specific entry
points (`open_terminal`, `open_agent`) that each bake in how to spawn. The launch layer should
instead be thin — **parse a launch item, then hand off to the right service** (the pseudo-terminal
daemon to spawn a command; the worktree service to create a worktree). Spawning is one generic
operation. In parallel, 0.x narrows to a terminal-only surface model: the agent surface is removed
and deferred to 1.x (ADR-0027), so the launch layer dispatches terminals without per-kind spawn
paths.

## What Changes

- The launch executor becomes a thin coordinator: validate + resolve each launch item, then
  hand off. It owns no spawning and no hook logic — it routes to services and records
  per-item outcomes (best-effort: a failed item is recorded, others continue).
- **BREAKING — REMOVE** the kind-specific entry points `open_terminal` (0.0.2) and `open_agent`
  (0.0.3). Spawning collapses to one generic operation: hand a resolved command
  (executable / args / env / cwd) to the pseudo-terminal daemon, which spawns it. Absent
  command keeps the login-shell default. The surface kind no longer changes how a surface is
  spawned.
- **BREAKING — REMOVE the agent surface entirely (terminal-only 0.x; agent deferred to 1.x,
  ADR-0027).** Delete the orchestrator agent module (`definition`/`parse`/`setup`), `launch_agent`,
  `AgentProxy`, the `SurfaceKind::Agent` variant, `create_agent_surface`, and the `agent-cli` seed;
  the desktop `surface_create_agent` + `agent_bootstrap` IPC and bootstrap; and the TS agent-adapter
  package plus the retired TS backend (`engine`, `platform-bun`). The **gate**,
  hook ingress, mcp-gateway, and memorya stay — they are shared infrastructure; only the agent
  surface's subscription to the gate is removed. 0.x surfaces are terminal (with a `diff` viewer
  stub); runnable surfaces are terminals.
- The executor wires the remaining item fields: dispatch on `target` to the correct surface
  kind, and run the `worktree` step (create -> returns the cwd) when present, recording
  `worktree_id`. Auxiliary runners (e.g. a dev server) are ordinary launch items with
  `target = terminal` and a placement — their output streams to the placed pane and closing the
  pane leaves the process running (soft-delete keeps the PTY). Commands run under local trust —
  no sandboxing (decision #7); validation is launch-spec schema plus command resolution (fail
  fast on an unresolvable command).
- The workspace IPC exposes the remaining store operations: rename and archive for project and
  session, and get/delete for command-library entries (store methods exist; only the host
  handlers are missing).
- Correctness fixes: idempotent `seed_commands` via a single insert-or-ignore statement; the
  template->session spec copy runs in one transaction; the worktree step runs against an explicit
  repository root, not the process working directory; `set_launch_template_spec` returns
  not-found for an absent template.

## Capabilities

### New Capabilities

- `launch-execution` — the coordinator: per-item parse/validate, target dispatch, worktree
  handoff, best-effort continuation, placement.
- `surface-spawn` — one generic spawn: hand a resolved command to the pseudo-terminal daemon;
  kind-agnostic; absent command means login shell.

### Modified Capabilities

- `surface-runtime` — remove `open_terminal` / `open_agent` and the agent surface entirely; a
  single generic spawn plus terminal dispatch (terminal-only 0.x).
- `workspace-ipc` — add rename/archive (project, session) and get/delete (command) handlers.
- `command-library` — seeding is idempotent under concurrent open.

## Impact

- Refactor `crates/orchestrator/src/surface/runtime.rs` (remove the two `open_*` methods + the
  agent surface; add one generic spawn + terminal dispatch), `surface/api.rs`,
  `launch/executor.rs` (the coordinator), `launch/worktree.rs`, `persistence/{sqlite,memory}.rs`.
- Delete the orchestrator `agent` module and the `apps/desktop/src-tauri` agent host
  (`surface_create_agent`, `agent_bootstrap`/bootstrap); `apps/desktop/src-tauri`:
  `workspace_host.rs`, `lib.rs` (new handlers).
- TS: delete the agent-adapter package and the retired `engine` / `platform-bun` packages, remove
  agent types from `packages/sdk`, and the renderer agent path (`apps/ui`); strip the deps from
  each package.json.
- **BREAKING** — reverses the agent surface merged in 0.0.3; the gate / hook ingress / mcp-gateway
  / memorya stay (shared infra). Agent returns in 1.x (ADR-0027).
- `daemon-pty-client` spawn already carries `command`/`args` — no protocol change.
- Builds on the parked `feature/0-0-5-launch-system` scaffold (launch spec + lazy migration,
  command-library store, worktree step, base IPC, SDK types) — reused, not re-implemented.
- No data migration (pre-v1).
