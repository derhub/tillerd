## Why

When the server process restarts (crash or reload), all active PTY sessions die: the PTY master fd closes, the agent process receives SIGHUP and exits, and the hook ingress URL baked into each session's environment goes stale. Users lose work and must start over. The fix is to move PTY ownership out of the server process into a detached long-lived daemon.

## What Changes

- **New package `@athing/daemon`**: detached binary that owns all PTY master fds, the hook ingress, and per-session replay buffers. Survives server restarts.
- **Engine becomes a daemon client**: `@athing/engine` gains a daemon supervisor (adopt-or-spawn), a Unix socket IPC client, and a session proxy that implements `AgentSession` over IPC. Direct `PtyTransport` use moves to the daemon.
- **Hook ingress moves to daemon**: the loopback hook receiver now runs inside the daemon on a named Unix socket (`~/.athing/hooks.sock`), making the URL stable across server restarts. `ATHING_BRIDGE_URL` env var carries the socket path.
- **`Engine` interface gains two methods**: `reconnect(sessionId, adapter, opts)` to re-attach to a live daemon session, and `listSessions()` to enumerate sessions the daemon knows about.
- **`apps/server` gains session persistence**: a `bun:sqlite` database at `~/.athing/server.db` stores active sessions. On startup the server reconciles the DB against the daemon and reconnects lazily (when a WS client presents a session id via `?id=` query param).
- **ADR-0008**: documents that PTY ownership moves to the daemon; the "no orphans" obligation from ADR-0007 now applies to daemon shutdown, not server shutdown.

## Capabilities

### New Capabilities

- `pty-daemon`: a detached daemon process that owns PTY master fds, runs the hook ingress on a stable Unix socket, maintains per-session replay buffers, and exposes an IPC control channel for the engine.
- `session-persistence`: server-side SQLite storage of active session metadata and lazy reconnect from a WS client presenting a known session id.

### Modified Capabilities

- `pty-transport`: PTY drive plane moves from engine-internal to daemon-owned; engine communicates via IPC instead of holding the master fd directly.
- `hook-ingress`: loopback receiver moves from a random HTTP port in the engine to a named Unix socket in the daemon; `ATHING_BRIDGE_URL` is repurposed as a socket path.
- `agent-session`: `Engine` interface gains `reconnect` and `listSessions`; `AgentSession` is now implemented as a proxy over IPC rather than a direct in-process object.

## Impact

- **New package**: `packages/daemon` (`@athing/daemon`) — Bun binary, node-pty, Unix socket server, replay buffer.
- **`packages/engine`**: gains `daemon/supervisor.ts`, `daemon/client.ts`, `daemon/proxy.ts`; existing `PtyTransport` and `HookReceiver` are deprecated in favour of daemon delegation; `Engine` interface changes are additive but require SDK bump.
- **`packages/sdk`**: `Engine` interface gains two methods — minor version bump, non-breaking for existing callers who don't use the new methods.
- **`apps/server`**: adds `bun:sqlite` session store, `?id=` reconnect on WS endpoint.
- **Dependencies**: no new runtime deps beyond what is already in the repo; `node-pty` moves from `packages/engine` to `packages/daemon`.
- **Platform**: macOS/Linux only (Unix sockets); no Windows support in v1.
