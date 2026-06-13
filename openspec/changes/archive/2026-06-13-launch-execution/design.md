# Design — Launch Execution

## Context

The launch system (ADR-0021) declares, per session, a list of launch items. The executor that
should instantiate them is a stub: it resolves a command then discards it and always creates a
bare terminal. Separately, surface creation is split into kind-specific entry points
(`open_terminal` from 0.0.2, `open_agent` from 0.0.3). This change collapses spawning to one
generic operation and, per ADR-0027, narrows 0.x to a terminal-only surface model — the agent
surface added in 0.0.3 is removed and deferred to 1.x.

In-force ADRs this design must stay coherent with: ADR-0020 (a session is a container of
surfaces; `surface_id` is the shared kernel), ADR-0021 (declarative launch spec), ADR-0022
(orchestrator owns the backend; renderer is UI+SDK), ADR-0023 (two-level id; `surface_id` is the
only id backends see), ADR-0024 (surface-runtime owns one PTY proxy per surface; `surface_id` is
the daemon session key; raw bytes + status flow over the `EventSink`). ADR-0024 explicitly left
"re-spawn from a launch spec" out of scope and noted the agent surface "reuses the same proxy and
status path." This change fills that gap and refines *how* a surface is brought to life beneath
ADR-0024's invariants, which it keeps.

## Goals / Non-Goals

**Goals**
- The launch executor is a thin coordinator: parse + validate an item, hand off to services,
  record per-item outcome. No spawning or hook logic in it.
- One generic spawn (hand a resolved command to the pseudo-terminal daemon); kind-agnostic.
- `launch_surface` dispatches by `SurfaceKind`; 0.x is terminal-only (a `diff` viewer stub is the
  only other kind, and it is an unsupported-launch error).
- The launch command comes from the launch item (command library or inline).
- **Remove the agent surface entirely (terminal-only 0.x; agent deferred to 1.x, ADR-0027).**
- Complete the workspace IPC; ship the correctness fixes.

**Non-Goals**
- No sandboxing of launched commands (roadmap decision #7 — local trust).
- No agent surface in 0.x (ADR-0027) — it returns in 1.x with the gate hook lifecycle.
- `diff` is not implemented here (decision #9, deferred) — `target = diff` is a typed
  unsupported-kind error.
- No placement geometry beyond named regions (0.2.4), no web host.

## Decisions

### 1. The launch executor is a thin parse->handoff router
Per item it owns only the kind-agnostic orchestration: validate, resolve the command, run the
`worktree` step, persist the surface row, call the adapter, and record a best-effort outcome (a
failed item is recorded; the rest continue, roadmap decision #13). It contains no `match` on kind
beyond selecting the adapter.
*Alternative:* a fat executor with per-kind branches — rejected: re-introduces the special-casing
this change exists to remove, and every new kind would edit the executor.

### 2. `launch_surface` dispatches by `SurfaceKind`; terminal-only in 0.x
`launch_surface(surface, kind, command, ...)` matches on `SurfaceKind`: `Terminal` calls the generic
spawn via `launch_terminal`; `Diff` is a typed unsupported-launch error (no adapter yet). With the
agent removed (decision #4), terminal is the only runnable kind, so a plain `match` is right — no
trait-object registry, no `async-trait` dep. *Alternative:* a `Box<dyn SurfaceAdapter>` registry —
deferred: it earns its keep only with a second runnable kind, which 0.x does not have.

### 3. One generic spawn service
`spawn(surface_id, command: Option<ResolvedCommand>, cwd, ...) -> (DaemonConnection, rx)` hands the
command to the pseudo-terminal daemon (ADR-0016/0024). `None` ⇒ login shell (preserves 0.0.2
terminal behavior). Spawning lives in one place. *Alternative:* per-kind spawn paths — rejected
(duplication; the daemon is already kind-agnostic).

### 4. Remove the agent surface — terminal-only 0.x (ADR-0027)
The agent surface (gate subscribe-before-spawn, hook install, drain->status/content, interrupt,
teardown, `AgentProxy`, `AGENT_DEF`) is deleted from the orchestrator, the desktop host
(`surface_create_agent`/`agent_bootstrap`), and the TS layer (the agent-adapter package plus the
retired `engine`/`platform-bun`). The **gate**, hook ingress, mcp-gateway, and memorya stay — they
are shared infrastructure; only the agent surface's subscription to the gate is removed. The agent
returns in 1.x with the gate lifecycle, where its launch command comes from the command library
like any other (no duplicated launch fields). *Alternative:* keep a slimmed agent adapter now —
rejected: 0.x ships terminal-first and the agent surface is not yet earning its complexity.

### 5. Reuse the parked 0.0.5 scaffold
The launch spec + lazy migration, command-library store, worktree step, base IPC handlers, and SDK
types are sound and stay. This change adds the executor wiring, the adapter seam, the surface-API
collapse, the remaining IPC, and the correctness fixes on top.

## Risks / Trade-offs

- [Removing the agent surface reverses 0.0.3] -> accepted product decision (ADR-0027): 0.x ships
  terminal-first. The deletion is broad (orchestrator agent module, `SurfaceKind::Agent`, desktop
  IPC + bootstrap, the TS agent-adapter package + retired `engine`/`platform-bun`) but the gate / hook
  ingress / mcp-gateway / memorya — the shared infra — are untouched; only the agent surface's gate
  subscription is removed. Pre-v1, no migration. The agent returns in 1.x.
- [Removing `open_*` changes the host-facing surface API (`create_terminal_surface` /
  `create_agent_surface` from 0.0.4)] -> `create_agent_surface` is deleted; callers updated in this
  one branch; pre-v1, internal API.
- [Launched commands run under local trust] -> accepted (decision #7); a hostile launch template is
  out of the threat model for a local single-user tool.

## Migration Plan

Pre-v1, no data migration. On the `feature/launch-execution` branch (descendant of main, which has
0.0.3 + 0.0.4): (1) introduce `ResolvedCommand` + the generic `spawn`; (2) make `launch_surface`
the single terminal entry; remove `open_terminal`/`open_agent`; rewire `create_terminal_surface`
and the daemon-host callers; (3) **remove the agent surface** (orchestrator module, `AgentProxy`,
`AGENT_DEF`, `SurfaceKind::Agent`, desktop `surface_create_agent`/`agent_bootstrap`, TS agent +
retired packages); (4) implement the executor (dispatch, worktree, best-effort); (5) add the
missing IPC handlers; (6) apply the correctness fixes. Rollback = revert the branch; the parked
`feature/0-0-5-launch-system` scaffold is untouched.

## Open Questions

- `diff` is a non-command kind with no adapter yet (deferred): the executor treats `target = diff`
  as a typed unsupported-kind error so a template referencing diff fails loudly until the diff
  viewer lands.
