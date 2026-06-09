# 0016. Daemon becomes pty-only; hook ingress moves to the gate

- Status: accepted
- Date: 2026-06-07

## Context

ADR-0008 moved PTY ownership into a detached daemon and, at the time, co-located the hook ingress
there (a named Unix domain socket at `~/.tillerd/hooks.sock`, with the daemon relaying hook events to
subscribers). The system has since grown several long-running tools (PTY daemon, memory layer, MCP
gateway) plus thin orchestrators, and two independent hook ingresses emerged (the daemon's and the
memory layer's). There is no single trust boundary for agent-facing traffic, and the daemon — which
should own only file descriptors — carries downstream knowledge it does not need.

This change introduces a **gate**: an agent-facing router over composable middleware (v1 globals
`Observe` outermost, then `Auth`) that hosts the agent-hook route natively and a tool route for other
ingestors. The gate becomes the one chokepoint for agent-facing traffic.

## Decision

Amend the hook-ingress clause of ADR-0008. The daemon becomes **pty-only**: it owns PTY master fds,
raw byte streaming, and session control (spawn/kill/resize), and exposes a consumer-oblivious
**session-event subscription** (lifecycle: start/exit/status by session id). It carries no knowledge
of downstream consumers.

Hook ingress and hook fan-out (`relay_hook`, `hooks.sock`, the negotiated hook capability) move out
of the daemon to the gate. The installed agent hook posts to the gate's hook endpoint
(loopback HTTP, per-session token); the gate normalizes raw input to the canonical `HookEvent` via an
injected adapter and fans it out to per-session subscribers. PTY bytes never flow through the gate —
the hot path stays direct (daemon to engine/UI over the existing binary framing), so the gate adds
zero latency to interactive rendering.

The rest of ADR-0008 stays in force: PTY ownership lives in the detached daemon; sessions survive
server restarts; the "no orphans" obligation applies to daemon shutdown.

## Consequences

- The daemon's public surface is PTY bytes + session control + a versioned session-event
  subscription, and nothing else; a new consumer needs no daemon change.
- One ingress for agent hooks (the gate), replacing the two prior ingresses; one trust boundary
  (`Auth`) and one observability point (`Observe`) for all agent-facing traffic.
- Orchestrators register a session with the gate **before** spawn and inject `{gate URL, session,
token}` into the daemon spawn env; the daemon passes this through and stays oblivious.
- Future policy middleware (`validate`/`firewall`/`redact`) drops onto a gate route with no daemon
  change.
- Rollback (pre-v1): re-point the installed hook at the daemon's old ingress and restore the daemon
  relay; the step is independently revertible.
