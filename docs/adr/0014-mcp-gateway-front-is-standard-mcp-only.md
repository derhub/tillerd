# 0014. MCP gateway exposes a standard MCP front only

- Status: accepted
- Date: 2026-06-04
- Supersedes: none

## Context

The PTY daemon (ADR-0008) speaks a bespoke Unix-socket protocol with binary framing (ADR-0009) to
pass PTY master file descriptors (ADR-0010) and hand them off across upgrades (ADR-0011). That
custom wire exists because PTY fd transport demands it. The gateway carries no fds — only JSON-RPC.
Its value is that any MCP client can reach every backend without implementing gateway-specific glue.

## Decision

The gateway's only public surface is the standard MCP streamable HTTP transport, bound to the
loopback interface, plus a small REST control plane on the same server and token. There is no
gateway-specific wire protocol. Every consumer — the desktop UI included — connects as an ordinary
MCP client; the UI is not privileged with custom IPC. Access requires a per-launch bearer token
(discoverable via the manifest) and a loopback-origin check; the daemon binds loopback only.

Management operations (health, status, targeted restart/stop/start, reload) live on the REST control
plane, deliberately distinct from the MCP tool surface, so an agent calling tools cannot administer
the daemon. The health endpoint is unauthenticated (liveness and version only) so a launcher can
probe before holding the token.

This boundary is independent of the PTY daemon's: ADR-0009/0010/0011 govern PTY fd transport and do
not apply here, and the gateway does not reuse that framing.

## Consequences

- UI integration reduces to "point an MCP client at the endpoint with the token" — no athing-specific
  protocol code in any consumer.
- Third-party MCP clients on the same machine can use the gateway unchanged.
- The gateway owns no codec, framing, or version negotiation of its own; it inherits all of that from
  the MCP protocol and its SDK.
- Administration cannot be performed through the agent's tool channel, only through the authenticated
  control plane.
