# 0027. 0.x is terminal-only; the agent surface is deferred to 1.x

- Status: accepted
- Date: 2026-06-12
- Amends: ADR-0026 (the agent adapter and the adapter registry it describes are deferred)

## Context

0.0.3 added an agent surface kind: a PTY surface that also subscribes to the gate by `surface_id`,
installs the agent's hooks, and drains hook events into a status/content model (`AgentProxy`,
`AGENT_DEF`/`AgentDefinition`, the orchestrator `agent` module, the desktop `surface_create_agent`
and `agent_bootstrap` IPC, and the TS agent-adapter package plus the retired TS backend `engine` /
`platform-bun`). ADR-0026 then proposed generalizing surface creation into a uniform
`Box<dyn SurfaceAdapter>` registry, with the agent as one adapter, to make new kinds additive.

In practice 0.x has exactly one runnable surface kind worth shipping: the terminal. The agent
surface carries the most complexity (gate subscription ordering, hook install/teardown, a status
model, binary resolution) for a capability that is not yet the product's focus, and the adapter
registry it motivates is indirection with a single implementor. A decision is needed on whether 0.x
ships the agent surface or narrows to terminals.

## Decision

**0.x is terminal-only. The agent surface is removed entirely and deferred to 1.x.**

- Delete the orchestrator `agent` module (`definition`/`parse`/`setup`), `launch_agent`,
  `AgentProxy`, `resolve_agent_command`/`agent_def`, the `SurfaceKind::Agent` variant,
  `create_agent_surface`, and the `agent-cli` seed. `launch_surface` dispatches by `SurfaceKind`
  through a plain match: `Terminal` spawns; `Diff` (a non-command viewer stub) is a typed
  unsupported-launch error. No `Box<dyn SurfaceAdapter>` registry, no `async-trait` dependency.
- Delete the desktop `surface_create_agent` + `agent_bootstrap` IPC commands and the bootstrap
  module; the surface host constructs `SurfaceApi::new` without a gate socket.
- Delete the TS agent-adapter package and the retired TS backend (`engine`, `platform-bun`), and
  remove agent types from the SDK and the agent path from the renderer.
- **The gate, hook ingress, mcp-gateway, and memorya stay.** The gate is shared infrastructure
  (ADR-0016/0018) consumed by mcp-gateway and memorya's hook capture; only the agent surface's
  subscription to it is removed.

This amends ADR-0026: its thin launch executor, single generic spawn, and item-supplied commands
stand; its agent adapter and adapter registry are deferred until 1.x reintroduces the agent surface.
It keeps ADR-0024's invariants (one proxy per surface, `surface_id` as the daemon session key, bytes
and status over the `EventSink`). Pre-v1, no data migration.

## Consequences

- **Easier:** the surface runtime, the desktop host, and the TS workspace shed a large, partially
  wired subsystem; `launch_surface` is a two-arm match with no trait objects; one spawn path; the
  command library is the single source of a surface's command.
- **Harder / costs:** reverses the agent surface shipped in 0.0.3 — a broad deletion across the
  orchestrator, the desktop host, and the TS packages. Reintroducing the agent in 1.x rebuilds the
  gate-drain lifecycle (the gate wire contract and hook parsing are preserved in the gate itself).
- **Neutral:** the gate / hook-ingress / mcp-gateway / memorya stack is untouched and keeps running;
  `diff` remains deferred (decision #9) and surfaces as a typed unsupported-kind error until a viewer
  lands.
