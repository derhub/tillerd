# Design: MCP gateway daemon

## Context

Each MCP client today is configured with the full backend server list independently, duplicating
spawn commands, credentials, and supervision. This change introduces an aggregating gateway: one
MCP face over many backends, owned by a single supervisor.

The gateway must outlive the desktop UI (backends and warm state survive the UI closing), so it
follows the detached-daemon pattern already established for the PTY daemon in ADR-0008: a long-lived
process, a manifest under `~/.athing/`, reuse-or-spawn, and "no orphans / graceful shutdown" from
ADR-0007. It is a sibling of the PTY daemon, not a dependant: it shares the conventions, not the
code. The PTY daemon's Unix-socket framing (ADR-0009/0010/0011) is specific to PTY fd transport and
is intentionally not reused — the gateway speaks standard MCP.

Constraints in force: ADR-0007 reliability contract (graceful shutdown, bounded interactions, typed
errors, authenticated control plane, capability/version awareness); ADR-0008 detached-daemon
ownership and manifest pattern. Platform scope macOS/Linux for v1.

## Goals / Non-Goals

Goals:
- One MCP endpoint aggregating many stdio and remote backends, consumable by any standard MCP client.
- Survive the launching UI; explicit stop only.
- Correct full-protocol behavior, including reverse-direction requests (sampling, roots,
  elicitation) — not a tool-only proxy that silently drops half the protocol.
- Operable: per-backend health, targeted restart, and live config reload with graceful drain.

Non-Goals (v1): persistent disk cache; progress notifications; resource subscriptions; completion;
logging level forwarding; pagination cursors; OAuth for remote backends; secret keychain or env
interpolation; a stdio front face; upgrade-via-fd handoff. Multi-user/commercial deployment remains
out of scope (one subscription = one user).

## Decisions

### D1: Standalone detached daemon, not in-process

The gateway is its own process, launched detached and tracked by a manifest, mirroring ADR-0008.

- Alternative: run the gateway in-process inside the desktop app. Rejected — it cannot satisfy
  "survive the UI closing"; backends would die with the UI.
- Consequence: the desktop app is a client + launcher of two daemons (PTY and gateway), connecting
  to the gateway as an ordinary MCP client.

### D2: Standard MCP streamable-http front, no bespoke protocol

The only public surface is the MCP streamable HTTP transport on loopback, plus a small REST control
plane on the same server. There is no gateway-specific wire protocol.

- Alternative: a custom IPC like the PTY daemon's socket framing. Rejected — it would force every
  consumer to implement gateway-specific glue; the whole value is that any MCP client connects
  unchanged. The PTY daemon needs custom framing to pass fds; the gateway carries only JSON-RPC.
- Consequence: the desktop UI integrates as "point an MCP client at the endpoint with the token."

### D3: Rust crate built on the MCP Rust SDK

Implemented as `packages/athing-mcp-gateway-rs`, outside the Bun/turbo graph, like the PTY daemon.

- Alternative: a Bun/TypeScript daemon. Rejected for the delivery shape — this is a bundled,
  spawned, long-lived background process in a desktop app that already ships a Rust runtime; a Rust
  binary is one small artifact with a tiny idle footprint and one toolchain, whereas a Bun daemon
  means bundling and spawning a second runtime beside the existing Rust one.
- Trade-off: the SDK and bidirectional-proxy wiring are more verbose in Rust than in the more
  mature TypeScript SDK; accepted because delivery shape is a hard constraint and SDK maturity is a
  convenience.

### D4: Pure core, side-effecting edges (ports-and-adapters posture)

Namespacing/routing and the registry are pure over snapshot data (no I/O, unit-testable in
isolation); the supervisor and backend connections own all process and network I/O; the transport
layer is the only place that knows the wire. This mirrors the project's sdk/engine split.

- Consequence: namespace round-trip and registry behavior are tested without spawning anything.

### D5: In-memory index only; lazy via boot-index + idle-shutdown

The registry is in memory; there is no persistent cache. Lazy backends are indexed once at boot,
then their process is released; they respawn on first call.

- Alternative: a persistent disk cache so `list_tools` can answer without ever spawning lazy
  backends. Rejected for v1 — the daemon is long-lived and boots rarely, so the cache's payoff
  (faster reboot) is low while it adds staleness and invalidation complexity.
- Consequence: a (rare) daemon boot spawns every backend once to index, then releases lazy ones.

### D6: Full reverse-proxy relay

A real client handler per backend relays server-to-client requests (sampling, roots, elicitation)
to the front client and forwards roots/list-changed/cancellation notifications both ways.

- Alternative: a tool-only gateway that no-ops these. Rejected — it silently breaks any backend that
  uses sampling/elicitation/roots, which is invisible to users and corrupts behavior.

### D7: Management via REST, distinct from the tool surface

Health, status, targeted restart/stop/start, and reload are REST endpoints on the same loopback
server and token — never MCP tools.

- Rationale: if admin were a tool, the agent could restart backends mid-task; it also bloats the
  tool list this design works to keep lean. Health is unauthenticated (liveness + version only) so a
  launcher can probe before holding the token; everything else requires the token.

### D8: Loose backend parsing, strict root

Unknown keys inside a backend entry are tolerated and logged (so configs authored for other clients
paste unchanged); unknown top-level keys are rejected. `$schema` is permitted at the root. The
shipped `schema.json` is generated from the config types and guarded by a drift test.

- Alternative: strict everywhere (reject unknown backend keys). Rejected — it breaks paste
  compatibility, the primary config-ergonomics goal. Trade-off: a typo'd backend key is silently
  ignored rather than rejected; the schema + editor validation mitigate this.

### D9: Reload diff splits spawn-fields from policy-fields

Reload validates, then diffs per backend: spawn-affecting fields (command/args/env/url/headers)
respawn; policy-only fields (allowlist/lazy) apply in place. Disruptive changes drain in-flight
calls (new calls block until swap; in-flight wait up to a timeout, then force-cancel). Reload is
serialized and best-effort per backend.

- Rationale: respawning for an allowlist tweak is wasteful and disruptive; draining avoids killing
  calls mid-flight while keeping the swap bounded.

## Risks / Trade-offs

- [Single point of failure: the daemon crashing drops all backend connections] → capped-backoff
  restart and active-liveness healing recover individual backends; full daemon recovery is via
  reuse-or-spawn on next launch. Same accepted posture as ADR-0008 for the PTY daemon.
- [Aggregated tool list bloat at 20+ backends harms model tool selection] → per-backend
  `allowedTools` filtering trims the surface; documented as the first lever users reach for.
- [Lazy serves indexed primitives for a not-running backend; dynamic-tool backends can drift] →
  lazy defaults off, re-index on respawn emits list-changed, and a backend's own list-changed
  capability is treated as a "do not lazy" hint.
- [Loopback port is reachable by any local process] → bearer token (per launch) + loopback-origin
  allowlist + loopback-only bind; token discoverable only via the local manifest.
- [Detach done wrong leaves an orphan or a child that dies with its launcher] → reuse the PTY
  daemon's detach approach; manifest reuse-or-spawn prevents duplicate instances.
- [MCP Rust SDK API drift across minor versions] → pin the SDK minor; the pure core is insulated;
  signatures are settled at first compile.

## Migration Plan

Additive — no existing capability changes, no data migration. New files appear under `~/.athing/`
(`mcp.json` user-authored, `mcp-gateway.json` manifest, shipped `schema.json`). Rollout: build the
crate, ship the binary with the desktop app, have the app reuse-or-spawn it and connect as an MCP
client. Rollback: stop spawning the daemon; removing `mcp.json` and the manifest returns the system
to its prior state. The PTY daemon and its IPC are untouched throughout.

## Open Questions

- Idle-shutdown timeout and active-liveness ping interval/timeout defaults — pick conservative
  starting values, expose later if needed.
- Whether the desktop app or a user launch agent owns spawning the daemon at login (affects
  survive-reboot, not v1 correctness).
- No in-force ADR needs revisiting; the adr step should record new ADR(s) for the standalone-gateway
  daemon, the MCP-only front, and the Rust/SDK choice, continuing the 0008-0011 daemon series.
