# Launch Execution

## Why

The launch system models commands, worktrees, and placement, but the executor never acts on
them: it resolves a launch item's command only to discard it, always creates a bare login-shell
terminal, and ignores the item's target kind and worktree step. The command library and worktree
step are implemented and tested in isolation but never invoked.

The deeper problem is shape, not wiring. Surface creation is split into kind-specific entry
points (`open_terminal`, `open_agent`) that each bake in how to spawn and, for the agent, the
whole hook lifecycle. That is not extensible: every new surface kind needs a new entry point.
The launch layer should instead be thin — **parse a launch item, then hand off to the right
service** (the pseudo-terminal daemon to spawn a command; the gate adapter to drive an agent's
hooks; the worktree service to create a worktree). The kind of surface is a routing detail,
not a separate spawn path.

## What Changes

- The launch executor becomes a thin coordinator: validate + resolve each launch item, then
  hand off. It owns no spawning and no hook logic — it routes to services and records
  per-item outcomes (best-effort: a failed item is recorded, others continue).
- **BREAKING — REMOVE** the kind-specific entry points `open_terminal` (0.0.2) and `open_agent`
  (0.0.3). Spawning collapses to one generic operation: hand a resolved command
  (executable / args / env / cwd) to the pseudo-terminal daemon, which spawns it. Absent
  command keeps the login-shell default. The surface kind no longer changes how a surface is
  spawned.
- **Agent becomes a layered adapter, not a spawn path.** The agent lifecycle currently inside
  `open_agent` (subscribe to the gate before spawn, install hooks, route hook→status/content,
  interrupt) is extracted into an **agent adapter** the executor attaches when `target = agent`.
  The adapter wraps the generic spawn (subscribe → spawn → drain). A new agent or kind is a new
  adapter with zero launch-layer changes.
- **BREAKING — REMOVE** the launch fields on the agent definition (binary, argument template,
  binary resolution). The launch command comes from the command library item. `AgentDefinition`
  keeps only adapter semantics: hook→status/content parsing, interrupt sequence, version range,
  and hook install/teardown. This deletes the duplicated "how to launch the agent" that lived in
  both the agent definition and the command library.
- The executor wires the remaining item fields: dispatch on `target` to the correct surface
  kind, and run the `worktree` step (create → returns the cwd) when present, recording
  `worktree_id`. Auxiliary runners (e.g. a dev server) are ordinary launch items with
  `target = terminal` and a placement — their output streams to the placed pane and closing the
  pane leaves the process running (soft-delete keeps the PTY). Commands run under local trust —
  no sandboxing (decision #7); validation is launch-spec schema plus command resolution (fail
  fast on an unresolvable command).
- The workspace IPC exposes the remaining store operations: rename and archive for project and
  session, and get/delete for command-library entries (store methods exist; only the host
  handlers are missing).
- Correctness fixes: idempotent `seed_commands` via a single insert-or-ignore statement; the
  template→session spec copy runs in one transaction; the worktree step runs against an explicit
  repository root, not the process working directory; `set_launch_template_spec` returns
  not-found for an absent template.

## Capabilities

### New Capabilities

- `launch-execution` — the coordinator: per-item parse/validate, target dispatch, worktree
  handoff, best-effort continuation, placement.
- `surface-spawn` — one generic spawn: hand a resolved command to the pseudo-terminal daemon;
  kind-agnostic; absent command means login shell.

### Modified Capabilities

- `surface-runtime` — remove `open_terminal` / `open_agent`; a single generic spawn plus a
  kind-keyed adapter attach.
- `agent-adapter` — extracted into a layered adapter that wraps the generic spawn; sheds launch
  fields; keeps hook parse, interrupt, version range, and hook setup.
- `workspace-ipc` — add rename/archive (project, session) and get/delete (command) handlers.
- `command-library` — seeding is idempotent under concurrent open.

## Impact

- Refactor `crates/orchestrator/src/surface/runtime.rs` (remove the two `open_*` methods; add
  one generic spawn + adapter attach), `agent/definition.rs` (slim the definition),
  `surface/api.rs`, `launch/executor.rs` (the coordinator), `launch/worktree.rs`,
  `persistence/{sqlite,memory}.rs`.
- `apps/desktop/src-tauri`: `workspace_host.rs`, `lib.rs` (new handlers).
- Refactors backend code merged in 0.0.2 and 0.0.3 (the `open_*` split and the agent
  definition's launch fields). The agent UI pane and the hook parse/status/interrupt/setup
  logic are kept — they move into the adapter; behavior is preserved.
- `daemon-pty-client` spawn already carries `command`/`args` — no protocol change.
- Builds on the parked `feature/0-0-5-launch-system` scaffold (launch spec + lazy migration,
  command-library store, worktree step, base IPC, SDK types) — reused, not re-implemented.
- No data migration (pre-v1).
