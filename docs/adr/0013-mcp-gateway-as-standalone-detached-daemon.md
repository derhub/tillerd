# 0013. MCP gateway is a standalone detached daemon

- Status: accepted
- Date: 2026-06-04
- Supersedes: none

## Context

Users configure many MCP servers, and every MCP client must hold the full list independently,
duplicating spawn commands, credentials, and supervision. An aggregating gateway removes that
duplication: one MCP face over many backends with a single supervisor. The gateway must keep
background servers and their warm state alive after the desktop UI that opened it closes — the same
"survive the host process" pressure that ADR-0008 resolved for PTY ownership by moving it into a
detached daemon.

## Decision

Run the MCP gateway as a standalone, long-lived, detached daemon — a sibling of the PTY daemon, not
a component of the desktop app and not a dependant of the PTY daemon. It follows the ADR-0008
lifecycle pattern: detach from its launcher's session, write a manifest at `~/.athing/mcp-gateway.json`
(`pid`/`port`/`token`/`version`) atomically, support reuse-or-spawn so a launcher connects to a
running matching-version instance instead of starting a second one, and remove the manifest on clean
stop. It honors the ADR-0007 reliability contract: graceful shutdown, bounded interactions, typed
errors, and an authenticated control plane.

The daemon stops only on explicit request or system shutdown, never because a client disconnected.
The desktop app becomes a launcher and client of two daemons (PTY and gateway). The gateway shares
the PTY daemon's lifecycle conventions but none of its code.

## Consequences

- Backends survive the UI closing; the gateway and its spawned servers run independently of any
  client.
- The gateway is a new single point of failure: if it crashes, backend connections drop. Per-backend
  capped-backoff restart and active-liveness healing recover individual backends; full recovery is
  reuse-or-spawn on next launch. This matches the accepted posture of ADR-0008.
- The application gains a second background daemon to supervise and reconnect to.
- "No orphans / graceful shutdown" from ADR-0007 applies to gateway shutdown: it cancels all backend
  connections and waits for cleanup.
