## 1. ADR and SDK contracts

- [ ] 1.1 Write `docs/adr/0008-pty-ownership-moves-to-detached-daemon.md` — no-orphans obligation transfers to daemon; engine shutdown releases subscriptions, not sessions
- [ ] 1.2 Add `reconnect(sessionId, adapter, options?): Promise<AgentSession>` to `Engine` interface in `packages/sdk`
- [ ] 1.3 Add `listSessions(): Promise<string[]>` to `Engine` interface in `packages/sdk`
- [ ] 1.4 Bump `packages/sdk` minor version

## 2. IPC protocol schema

- [ ] 2.1 Define valibot schemas for all client→daemon ops (`spawn`, `kill`, `send`, `input`, `interrupt`, `resize`, `subscribe`, `unsubscribe`, `list`) in a shared `packages/daemon/src/protocol.ts`
- [ ] 2.2 Define valibot schemas for all daemon→client events (`spawned`, `data`, `hook`, `exit`, `replay`, `sessions`, `error`) in `packages/daemon/src/protocol.ts`
- [ ] 2.3 Implement NDJSON framing helpers (encode/decode, handle partial lines across chunks)

## 3. Daemon package scaffold

- [ ] 3.1 Create `packages/daemon/package.json` with `name: "@athing/daemon"`, Bun binary entry, and `node-pty` dependency moved from `packages/engine`
- [ ] 3.2 Add `packages/daemon` to `turbo.json` task graph
- [ ] 3.3 Create `packages/daemon/src/main.ts` — entry point: parse argv, write manifest, start server, handle SIGTERM cascade

## 4. Daemon: manifest

- [ ] 4.1 Implement `packages/daemon/src/manifest.ts` — atomic write of `~/.athing/daemon.json` with `{ pid, version }` on start; delete on graceful stop
- [ ] 4.2 Ensure manifest is cleaned up on unhandled exceptions and SIGTERM

## 5. Daemon: hook ingress

- [ ] 5.1 Implement `packages/daemon/src/hook-ingress.ts` — `Bun.serve({ unix: "~/.athing/hooks.sock" })` that verifies per-session token and relays raw payload to subscribed engine clients
- [ ] 5.2 Port token-verification logic from `packages/engine/src/ingress/receiver.ts`
- [ ] 5.3 Port idempotency key logic from `packages/engine/src/ingress/dispatcher.ts`

## 6. Daemon: PTY session and replay buffer

- [ ] 6.1 Implement `packages/daemon/src/replay-buffer.ts` — fixed-capacity ring buffer of `Uint8Array` chunks (64 KB total), evicts oldest on overflow
- [ ] 6.2 Implement `packages/daemon/src/pty-session.ts` — wraps `PtyTransport` (moved from engine), holds replay buffer, token, and subscriber set
- [ ] 6.3 Move `packages/engine/src/pty/transport.ts` to `packages/daemon/src/pty-transport.ts` (or re-export); remove `node-pty` from engine deps

## 7. Daemon: IPC server

- [ ] 7.1 Implement `packages/daemon/src/server.ts` — `Bun.listen({ unix: "~/.athing/daemon.sock" })`, NDJSON framing, dispatch ops to session registry
- [ ] 7.2 Handle `spawn` op: resolve binary, validate launch config, create `PtySession`, respond `spawned`
- [ ] 7.3 Handle `kill` op: escalating termination per ADR-0007, emit `exit` to subscribers
- [ ] 7.4 Handle `send`/`input`/`interrupt`/`resize` ops: delegate to `PtySession`
- [ ] 7.5 Handle `subscribe` op: register client socket, send `replay` chunks, then stream live events
- [ ] 7.6 Handle `unsubscribe` op and client disconnect: remove subscriber, do NOT kill session
- [ ] 7.7 Handle `list` op: return `{ ev: "sessions", ids: [...] }`
- [ ] 7.8 Graceful shutdown: on SIGTERM, kill all sessions with escalation, remove manifest and sockets, exit

## 8. Engine: daemon supervisor

- [ ] 8.1 Implement `packages/engine/src/daemon/supervisor.ts` — `adoptOrSpawn()`: read manifest, `process.kill(pid, 0)` liveness check, connect or spawn daemon binary
- [ ] 8.2 Handle stale socket file (PID dead but socket exists): delete socket before spawning
- [ ] 8.3 Handle daemon version mismatch: SIGTERM old daemon, spawn fresh
- [ ] 8.4 Expose `getDaemonClient(): Promise<DaemonClient>` for lazy init on first `start()`/`reconnect()` call

## 9. Engine: IPC client

- [ ] 9.1 Implement `packages/engine/src/daemon/client.ts` — `Bun.connect({ unix: "~/.athing/daemon.sock" })`, NDJSON framing, request/response correlation, event subscriptions
- [ ] 9.2 Implement `send(op): Promise<void>` and `subscribe(sessionId, handler): () => void`
- [ ] 9.3 Handle client disconnect: surface as `TransportClosed` typed error on all active subscriptions

## 10. Engine: session proxy

- [ ] 10.1 Implement `packages/engine/src/daemon/proxy.ts` — `AgentSessionProxy` implementing `AgentSession` over `DaemonClient`
- [ ] 10.2 Wire `hook` events from daemon through `adapter.parseHook` → `statusMapper.apply`
- [ ] 10.3 Wire `data` events to `dataBuf` replay and `dataHandlers`
- [ ] 10.4 Wire `exit` events to `exitHandlers`; cancel startup timer on first `hook` or `data` event
- [ ] 10.5 Implement `send`/`input`/`interrupt`/`resize`/`kill` by delegating to `DaemonClient`
- [ ] 10.6 Preserve `sendQueue` logic (ready-gating, bounded capacity, `QueueFull` error) in proxy

## 11. Engine: wire daemon path into Engine factory

- [ ] 11.1 Update `packages/engine/src/engine.ts` — add `useDaemon?: boolean` to `createEngine()` options; default `false` initially
- [ ] 11.2 When `useDaemon: true`: call `supervisor.adoptOrSpawn()` lazily on first `start()`/`reconnect()`; delegate session creation to proxy
- [ ] 11.3 Implement `engine.listSessions()` — calls `DaemonClient.list()`, returns `string[]`; returns `[]` when `useDaemon: false`
- [ ] 11.4 Implement `engine.reconnect(sessionId, adapter, opts)` — validates session exists in daemon, constructs `AgentSessionProxy` with `subscribe` (not `spawn`), delivers replay buffer; throws `TransportClosed` when `useDaemon: false`
- [ ] 11.5 Update `engine.shutdown()` — when daemon-backed, `unsubscribe` all sessions (do not kill them); daemon retains ownership per ADR-0008

## 12. Hook install: update ATHING_BRIDGE_URL

- [ ] 12.1 Update `packages/engine/src/ingress/install.ts` — `ATHING_BRIDGE_URL` env value is now the socket path (`~/.athing/hooks.sock`), not an HTTP URL
- [ ] 12.2 Update the notify script template in `install.ts` — `fetch("http://localhost", { unix: bridgeUrl, method: "POST", ... })`
- [ ] 12.3 Update `packages/engine/src/session/session.ts` — pass hook socket path from daemon client, not from `HookReceiver`

## 13. Remove now-redundant engine-internal hooks infrastructure

- [ ] 13.1 Deprecate `packages/engine/src/ingress/receiver.ts` and `dispatcher.ts` (keep for non-daemon path until rollout)
- [ ] 13.2 Remove `HookReceiver` from `EngineImpl` when `useDaemon: true`

## 14. apps/server: session persistence

- [ ] 14.1 Add `bun:sqlite` session store initialization to `apps/server/src/index.ts` — open `~/.athing/server.db`, `CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, cwd TEXT, created_at INTEGER)`
- [ ] 14.2 On WS `open`: insert session row; on `exit` event: delete row
- [ ] 14.3 On server startup: call `engine.listSessions()`, delete DB rows absent from daemon, log count of reconnectable sessions
- [ ] 14.4 Extend WS upgrade handler: if `?id=` query param present, call `engine.reconnect(id, adapter, { cwd })` instead of `engine.start`; send `{ type: "session_resume", sessionId }` to client
- [ ] 14.5 Return `{ type: "error", kind: "TransportClosed" }` and close WS if reconnect fails (unknown or dead session)
- [ ] 14.6 Enable `useDaemon: true` in `createEngine()` call in server

## 15. Tests

- [ ] 15.1 Unit tests for NDJSON framing helpers (encode/decode, partial-line handling)
- [ ] 15.2 Unit tests for replay buffer (capacity eviction, replay order)
- [ ] 15.3 Unit tests for `AgentSessionProxy` — mock `DaemonClient`, verify hook→status pipeline, sendQueue gating, kill delegation
- [ ] 15.4 Unit tests for `DaemonSupervisor` — mock filesystem and process, verify adopt/spawn/stale-socket paths
- [ ] 15.5 Integration test: daemon spawns, engine connects, session starts, server restarts, engine reconnects, session still alive
- [ ] 15.6 Integration test: hook delivery continues after engine host process restart
- [ ] 15.7 Integration test: `?id=` WS reconnect delivers replay buffer and resumes event stream
