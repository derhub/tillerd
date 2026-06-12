# Design — Launch Execution

## Context

The launch system (ADR-0021) declares, per session, a list of launch items. The executor that
should instantiate them is a stub: it resolves a command then discards it and always creates a
bare terminal. Separately, surface creation is split into kind-specific entry points
(`open_terminal` from 0.0.2, `open_agent` from 0.0.3), and the agent's launch command is
hardcoded in the agent definition — duplicating what the command library already holds.

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
- Surface kind is a `SurfaceAdapter`; the executor dispatches uniformly through a registry — a new
  kind is a new adapter with zero executor change.
- The agent lifecycle (gate subscribe-before-spawn, hook install, drain→status/content, interrupt,
  teardown) is extracted into the agent adapter, reusing ADR-0024's proxy/status path.
- The launch command comes from the launch item (command library or inline); the agent definition
  keeps only adapter semantics.
- Complete the workspace IPC; ship the correctness fixes.

**Non-Goals**
- No sandboxing of pre/post/auto-spawn scripts (roadmap decision #7 — local trust).
- `diff` and other non-command kinds are not implemented here (decision #9, deferred) — only the
  uniform seam that admits them.
- No second agent adapter (0.2.0), no placement geometry beyond named regions (0.2.4), no web host.

## Decisions

### 1. The launch executor is a thin parse→handoff router
Per item it owns only the kind-agnostic orchestration: validate, resolve the command, run `pre`
scripts, run the `worktree` step, persist the surface row, call the adapter, run `post` +
`auto-spawn`, and record a best-effort outcome (a failed item is recorded; the rest continue,
roadmap decision #13). It contains no `match` on kind beyond selecting the adapter.
*Alternative:* a fat executor with per-kind branches — rejected: re-introduces the special-casing
this change exists to remove, and every new kind would edit the executor.

### 2. Surface kind is a `SurfaceAdapter`; dispatch is a uniform registry lookup
```
trait SurfaceAdapter { async fn launch(&self, surface_id, command: Option<ResolvedCommand>, ctx: &LaunchCtx) -> Result<Proxy>; }
```
The executor holds `registry: HashMap<target, Box<dyn SurfaceAdapter>>` and calls
`registry[item.target].launch(...)`. Terminal is the trivial adapter (just spawn); agent adds the
gate lifecycle; diff (later) ignores the command. *Alternative:* an `enum SurfaceKind` matched
inside the runtime — rejected: a new kind edits the match, losing the "zero executor change"
property that is the point.

### 3. One generic spawn service; adapters call it
`ctx.spawn(surface_id, command: Option<ResolvedCommand>, cwd, env) -> (DaemonConnection, rx)` hands
the command to the pseudo-terminal daemon (ADR-0016/0024). `None` ⇒ login shell (preserves 0.0.2
terminal behavior). Spawning lives in one place; adapters compose it. *Alternative:* per-kind spawn
paths — rejected (duplication; the daemon is already kind-agnostic).

### 4. The agent is a layered adapter, not a spawn path
`AgentAdapter::launch` = subscribe to the gate by `surface_id` (before spawn, so no hook event is
missed) → install hooks → `ctx.spawn(command)` → start the gate-drain task (hook → status/content
→ `EventSink`). It returns the existing `AgentProxy` (conn + read task + gate task), reusing
ADR-0024's proxy/status path verbatim. Interrupt and hook teardown move here too.
*Alternative:* keep the lifecycle in a bespoke `open_agent` — rejected (the duplication and
non-extensibility being removed).

### 5. The launch command comes from the launch item; the agent definition sheds launch fields
`ResolvedCommand { exe, args, env }` is produced by the executor from the item (`library_ref` →
command-library row, or `inline`). `AGENT_DEF`/`AgentDefinition` drops `binary`, `args_template`,
and `resolution`; it keeps hook parse, `interrupt_seq`, version range, and hook install/teardown.
The agent's command is the command-library "agent" preset, supplied by the launch item like any
other. *Alternative:* keep the launch fields on the definition — rejected (two sources of truth
for how to launch the agent).

### 6. Reuse the parked 0.0.5 scaffold
The launch spec + lazy migration, command-library store, worktree step, base IPC handlers, and SDK
types are sound and stay. This change adds the executor wiring, the adapter seam, the surface-API
collapse, the remaining IPC, and the correctness fixes on top.

## Risks / Trade-offs

- [Refactors backend merged in 0.0.2 + 0.0.3 — `open_terminal`/`open_agent`, `AGENT_DEF` launch
  fields] → behavior is preserved, not changed: the agent UI pane and hook parse/status/interrupt
  logic move into the adapter; existing terminal/agent tests are reused against the new seam, plus
  new tests assert the command reaches the daemon and the agent path still routes status/content.
- [Agent ordering: gate-subscribe must precede spawn] → enforced inside `AgentAdapter::launch`; the
  generic `spawn` is invoked only after the subscription is live. Tested with a fake gate + daemon.
- [Trait-object indirection for 3 kinds] → small and bounded; the gain (executor has zero
  kind-knowledge; new kinds are additive) outweighs it. If it ever costs more than it saves, the
  registry can collapse to an enum without touching callers of `spawn`.
- [Removing `open_*` changes the host-facing surface API (`create_terminal_surface` /
  `create_agent_surface` from 0.0.4)] → callers updated in this one branch; pre-v1, internal API.
- [Best-effort scripts under local trust] → accepted (decision #7); a hostile launch template is
  out of the threat model for a local single-user tool.

## Migration Plan

Pre-v1, no data migration. On the `feature/launch-execution` branch (descendant of main, which has
0.0.3 + 0.0.4): (1) introduce `ResolvedCommand` + the generic `spawn`; (2) extract `TerminalAdapter`
and `AgentAdapter` behind `SurfaceAdapter`; remove `open_terminal`/`open_agent`; rewire
`create_*_surface` and the daemon-host callers; (3) slim `AGENT_DEF`; (4) implement the executor
(dispatch, worktree, scripts, best-effort) on the registry; (5) add the missing IPC handlers; (6)
apply the four correctness fixes. Rollback = revert the branch; the parked
`feature/0-0-5-launch-system` scaffold is untouched.

## Open Questions

- `Box<dyn SurfaceAdapter>` registry vs a small enum: this design picks the trait+registry for the
  zero-executor-change property; revisit only if the indirection proves unjustified for the current
  three kinds. (Records as ADR-0026.)
- Where the registry is constructed/owned (surface-runtime vs the executor) — a wiring detail for
  the tasks step.
- `diff` is a non-command kind with no adapter yet (deferred): does the executor treat
  `target = diff` as a typed "unsupported kind" error for now, or skip it? Lean typed error so a
  template referencing diff fails loudly until the diff adapter lands.
- `pre`/`post`/`auto-spawn` runner: process spawn via `process-launch`, inheriting the resolved
  `cwd`/env; confirm it reuses the existing crate rather than a new one.
