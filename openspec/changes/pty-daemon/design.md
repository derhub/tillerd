## Context

The engine currently holds PTY master file descriptors in-process alongside the WebSocket server. A server restart closes those fds, sending SIGHUP to the agent process, destroying all session state. The hook ingress also binds to a random ephemeral port baked into each session's process environment at spawn time; a restart silently invalidates all hook delivery.

ADR-0007 requires no orphaned processes, graceful shutdown, and typed errors. ADR-0002 requires a single engine with transport as a per-session feature. ADR-0005 requires the engine to consume lifecycle only as `HookEvent`. All three constrain this design.

## Goals / Non-Goals

**Goals:**
- PTY sessions survive server process restarts
- Hook delivery survives server process restarts
- Engine public API (`AgentSession`, `Engine`) unchanged except two additive methods
- Server can lazily reconnect to existing sessions when a client presents a session id
- Replay buffer allows terminal renderers to restore visual state after reconnect

**Non-Goals:**
- Zero-downtime daemon binary upgrade (fd inheritance handoff) — Phase 2
- Daemon crash recovery — Phase 2
- Content event replay on reconnect (transcripts are on disk; callers re-read from offset)
- Windows support (Unix domain sockets are macOS/Linux only in v1)

## Decisions

### 1. Detached daemon as a separate binary

**Decision:** Introduce `packages/daemon` (`@athing/daemon`) as a standalone Bun binary that the engine spawns detached on first use and re-adopts on subsequent starts.

**Why:** The PTY master fd must outlive the engine host process. The only way to achieve this on Unix is to move fd ownership into a process that is not a child of the server. A detached sibling process with a known socket address satisfies this. Embedding the daemon logic as a thread or worker inside the engine process does not — the fd still closes when the process dies.

**Alternatives considered:**
- _OS-level supervisor (launchd/systemd)_: Reliable but requires platform-specific service registration outside the SDK; not portable for a dev tool.
- _Named pipe / shared memory_: More complex, no meaningful benefit over Unix domain sockets for this use case.

### 2. Two named Unix domain sockets with deterministic paths

**Decision:**
- `~/.athing/daemon.sock` — IPC control channel (NDJSON framed by newlines)
- `~/.athing/hooks.sock` — hook HTTP ingress (HTTP/1.1 over Unix domain socket)

Paths are derived from `$HOME` at runtime. No random ports. No manifest entries needed for socket paths.

**Why:** Deterministic paths mean the engine needs only a liveness check (manifest `pid`) to decide adopt-vs-spawn; it never has to discover where to connect. The hook socket path can be baked into session process environments without ever becoming stale across daemon restarts — the path is always the same.

**Why not a fixed TCP port for hooks:** Port conflicts are possible on shared machines; Unix domain sockets are always process-local and access-controlled by filesystem permissions.

### 3. Adapter parse functions stay in the engine; daemon relays raw payloads

**Decision:** The daemon holds no adapter knowledge. On hook arrival it verifies the session token and forwards the raw JSON payload over the IPC channel tagged with `{ ev: "hook", sessionId, payload }`. The engine's `AgentSessionProxy` calls `adapter.parseHook(payload)` locally and feeds `statusMapper`.

**Why:** Adapter parse functions are closures — they cannot be serialized across a process boundary. The daemon must remain adapter-agnostic to serve any future adapter without changes. Only the serializable parts of `AgentDefinition` (launch config, hook install spec) cross the IPC boundary.

**Consequence:** `statusMapper`, `transcriptReader`, and `sendQueue` remain local objects inside `AgentSessionProxy` in the engine, exactly as today. The proxy is the seam between "engine intelligence" and "daemon I/O".

### 4. IPC protocol: NDJSON over Unix domain socket

**Decision:** Messages are newline-delimited JSON objects. Binary data (PTY output, replay buffer) is encoded as arrays of unsigned integers (`number[]`). The protocol is symmetric: either side can send at any time once connected.

**Why:** NDJSON is trivially parseable in Bun without additional libraries, debuggable with standard tools, and sufficient for the expected throughput (single user, one PTY per session). A binary framing protocol (e.g., length-prefixed MessagePack) would reduce overhead for heavy PTY output but adds implementation complexity not justified for v1.

**Message taxonomy:**

Client → Daemon:
```
{ op: "spawn",      sessionId, launch: { command, args, flags }, hookSocketPath, token, cols, rows, cwd }
{ op: "kill",       sessionId }
{ op: "send",       sessionId, text }
{ op: "input",      sessionId, bytes: number[] }
{ op: "interrupt",  sessionId }
{ op: "resize",     sessionId, cols, rows }
{ op: "subscribe",  sessionId }
{ op: "unsubscribe", sessionId }
{ op: "list" }
```

Daemon → Client:
```
{ ev: "spawned",   sessionId }
{ ev: "data",      sessionId, bytes: number[] }
{ ev: "hook",      sessionId, payload: unknown }
{ ev: "exit",      sessionId, code: number|null, signal: string|null }
{ ev: "replay",    sessionId, chunks: number[][] }
{ ev: "sessions",  ids: string[] }
{ ev: "error",     sessionId?, kind: string, message: string }
```

### 5. Engine supervisor: adopt-or-spawn on first `start()` call

**Decision:** `createEngine()` remains synchronous. Daemon adoption is lazy — triggered on the first call to `engine.start()` or `engine.reconnect()`. The supervisor reads the manifest, checks `process.kill(pid, 0)`, connects to the socket, or spawns the daemon.

**Why:** Keeps the existing `createEngine()` API unchanged. A synchronous factory is simpler for callers. The first `start()` is already async, so the adoption latency is absorbed there.

### 6. Session store in `apps/server` using embedded SQL

**Decision:** `apps/server` maintains a database at `~/.athing/server.db` with a `sessions` table `(id TEXT PRIMARY KEY, cwd TEXT, created_at INTEGER)`. On WS open with `?id=<sessionId>` the server calls `engine.reconnect(id, adapter, { cwd })`. On startup, the server reconciles DB rows against `engine.listSessions()` and deletes stale rows.

**Why `~/.athing/server.db`:** Follows the user across project directory changes; consistent with manifest and socket paths. The sessions belong to the user's agent, not to a specific project checkout.

### 7. `reconnect` and `listSessions` added to `Engine` interface in `@athing/sdk`

**Decision:** Both methods are added to the public `Engine` interface in `@athing/sdk` (minor version bump). The non-daemon engine path implements `listSessions` as returning `[]` and `reconnect` as throwing `TransportClosed`.

**Why on the interface:** Any consumer of the SDK that holds an `Engine` can call these methods without knowing whether the engine is daemon-backed or direct. Future transport modes inherit the contract.

## Risks / Trade-offs

- **Daemon version drift** → Manifest includes a `version` field; engine refuses to adopt a daemon with an incompatible version and spawns a fresh one. The old daemon is SIGTERMed.
- **Stale socket file after daemon crash** → On adopt, if the socket file exists but the PID is dead, engine deletes the stale socket file before spawning. Checked by `process.kill(pid, 0)` in the supervisor.
- **Replay buffer memory per session** → Buffer is bounded at 64 KB per session. For a single-user tool with a handful of sessions, total memory is negligible.
- **NDJSON throughput ceiling** → For very high-bandwidth PTY output (e.g., `cat` of a large file), serialising bytes as JSON integer arrays adds ~3× overhead vs. raw binary. Acceptable for interactive agent use; revisit if profiling shows it as a bottleneck.
- **Daemon as single point of failure** → If the daemon crashes, all sessions are lost. This is accepted scope for Phase 1. Phase 2 adds daemon crash recovery.

## Migration Plan

1. Add `packages/daemon`; existing engine remains functional with direct PTY (no breaking change yet).
2. Add daemon supervisor + client + proxy to `packages/engine`; gated behind a `useDaemon: true` option on `createEngine()` for rollout safety.
3. Once validated, make `useDaemon: true` the default; deprecate direct PTY path.
4. Update `apps/server` to use session store and `?id=` reconnect endpoint.
5. Remove deprecated direct PTY path in a subsequent change.

**Rollback:** If the daemon path is gated behind `useDaemon`, reverting to the direct path requires only a config change.

## Open Questions

- ADR-0007 states "no orphans" at the engine level. With the daemon, the engine no longer kills sessions on shutdown — the daemon does. ADR-0008 (to be created by the `adr` step) should clarify that the no-orphans obligation now belongs to the daemon and that a graceful engine shutdown sends a "release" signal to the daemon rather than killing sessions outright.
- Should the daemon expose a HTTP management endpoint (e.g., `GET /sessions`) in addition to the Unix socket, for external observability tooling? Deferred to Phase 2.
