# 0026. Surface creation is a uniform adapter dispatch over a generic spawn; the launch executor is a thin router

- Status: proposed
- Date: 2026-06-12
- Supersedes: none

## Context

ADR-0021 makes a session an instance of a declarative launch spec. ADR-0024 puts one PTY proxy per
surface under the surface-runtime, names `surface_id` the daemon session key, and explicitly defers
"re-spawn from a launch spec" while noting the agent surface "reuses the same proxy and status path."

The first cut wired neither well. Surface creation grew kind-specific entry points — `open_terminal`
(0.0.2) and `open_agent` (0.0.3) — and the agent's launch command was hardcoded in the agent
definition, duplicating the command library. The launch executor that should instantiate items never
used the resolved command, the target kind, the worktree step, or the scripts. This shape does not
scale: every new surface kind needs a new entry point, and "how to launch the agent" lives in two
places.

A decision is needed for the mechanism beneath ADR-0024 — how a launch item becomes a live surface —
because it is the seam every surface kind and every future launch feature reuses.

## Decision

Surface creation is a **uniform adapter dispatch over a single generic spawn**, driven by a **thin
launch executor**.

- **The launch executor is a parse→handoff router.** Per item it owns only kind-agnostic
  orchestration: validate, resolve the command, run `pre` scripts, run the `worktree` step, persist
  the surface row, invoke the adapter, run `post` and `auto-spawn`, and record a best-effort outcome
  (a failed item is recorded; the rest continue). It holds no per-kind logic beyond selecting the
  adapter.
- **A surface kind is a `SurfaceAdapter`.** The executor dispatches through a registry keyed by the
  item's `target` and calls `adapter.launch(surface_id, command, ctx)`. A new kind is a new adapter
  with no executor change.
- **One generic spawn.** A shared spawn service hands a resolved command (`exe`/`args`/`env`/`cwd`) to
  the pseudo-terminal daemon; an absent command means the login shell (preserving 0.0.2 terminal
  behavior). Adapters compose this one spawn; there is no per-kind spawn path.
- **The agent is a layered adapter, not a spawn path.** The agent adapter subscribes to the gate by
  `surface_id` before spawn, installs hooks, calls the generic spawn, and drains the gate into
  status/content over the `EventSink` — returning the same per-surface proxy ADR-0024 defines.
  `open_terminal` and `open_agent` are removed.
- **The launch command comes from the launch item.** The command (command-library reference or
  inline) is supplied by the item. The agent definition keeps only adapter semantics — hook
  parsing, interrupt sequence, version range, hook install/teardown — and sheds its launch fields
  (binary, argument template, resolution).

This supersedes nothing. It refines surface creation beneath ADR-0024 while keeping its invariants
(one proxy per surface, `surface_id` as the daemon session key, raw bytes and status over the
`EventSink`), and it fulfills ADR-0024's deferred re-spawn-from-launch-spec. It is coherent with
ADR-0020/0021/0022/0023. Scripts run under local trust (roadmap decision #7); no sandbox.

## Consequences

- **Easier:** new surface kinds are additive — a new adapter, no executor edit; one spawn path for
  every kind; the agent's launch unifies with the command library, removing the duplicated source of
  truth; the executor carries zero kind-knowledge, so launch features (worktree, scripts, placement)
  are written once for all kinds.
- **Harder / costs:** refactors backend merged in 0.0.2 and 0.0.3 — `open_terminal`/`open_agent` are
  removed and the agent definition is slimmed; the host-facing `create_*_surface` API from 0.0.4
  changes (pre-v1, internal); a trait-object registry adds a small indirection for the current three
  kinds, which can collapse to an enum later without changing spawn's callers.
- **Neutral:** ADR-0024's proxy, byte-stream, and status contracts are unchanged — behavior moves,
  it does not change; `diff` stays deferred (decision #9) but the seam admits it as just another
  adapter; the local-trust script model (decision #7) is unchanged.
