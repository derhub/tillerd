## 1. Protocol & SDK Types

- [x] 1.1 Add `stop` frame type to the daemon↔engine IPC wire schema: `{ type: "stop", sessionId }`
- [x] 1.2 Add `SessionStopped` to the typed error taxonomy in `@athing/sdk`
- [x] 1.3 Add `stop` WebSocket message type to the apps/server↔apps/ui protocol schema
- [x] 1.4 Add `stop()` to the `AgentSession` public interface in `@athing/sdk`

## 2. Durable Stop Persistence — Daemon

- [x] 2.1 Persist stopped-session ids to the durable session-persistence store; keep a bounded in-memory cache (LRU/TTL) over the durable record
- [x] 2.2 Handle `stop` frame: kill the session (same as kill frame), set `killedByUser = true`, write the session id to the durable stopped set
- [x] 2.3 On `spawn` with `resume: sessionId`, consult the durable store (authoritative, not just the cache); if present, reject with a `SessionStopped` typed error frame
- [x] 2.4 Unit test: daemon receives `stop` frame → subsequent `spawn { resume }` is rejected with `SessionStopped`
- [x] 2.5 Integration test: stop → daemon cold restart → `spawn { resume }` still rejected (durable across daemon restart)
- [x] 2.6 Unit test: in-memory cache eviction does NOT resurrect resumability (durable record still rejects)

## 3. sessionId Re-registration — Daemon

- [x] 3.1 Allow the registry to accept re-registration of a just-evicted sessionId (for recovery under the same id)
- [x] 3.2 Ensure subscribers of the dead session do not auto-reattach to the re-registered session
- [x] 3.3 Integration test: evict on exit → spawn same id → registry accepts; stale subscribers do not auto-reattach

## 4. Stop Operation — Engine

- [x] 4.1 Add `stop()` method to `AgentSessionProxy`: sends `stop` frame to daemon (daemon handles kill + durable stopped registration)
- [x] 4.2 Unit test: `stop()` → daemon receives `stop` frame → subsequent `start({ resume })` rejected with `SessionStopped`
- [x] 4.3 Unit test: `kill()` → daemon receives `kill` frame → subsequent `start({ resume })` is not rejected

## 5. Crash-Recovery Routing — Engine

- [x] 5.1 Ensure crash-recovery (`start({ resume: sessionId })` after a `crashed` status) routes through the SPAWN path (new process), NOT the `reconnect()`/subscribe path which requires a live daemon session
- [x] 5.2 On recovery spawn, perform the per-session hook-token wiring for the new process even when the per-adapter hook install is already cached in `installedAdapters`
- [x] 5.3 Surface resume failure (e.g. agent session state unreadable after crash) as a typed error rather than a silent blank session
- [x] 5.4 Integration test: crashed session → `start({ resume })` spawns a fresh process, per-session hook token is wired (hook callbacks reach the new process), agent conversation resumes
- [x] 5.5 Integration test: recovered session's terminal begins blank (no pre-crash screen replay), repopulated only by resumed agent output

## 6. apps/server — WebSocket Bridge

- [x] 6.1 Add `stop` message type handler in the WebSocket server: call `session.stop()` on the engine session
- [x] 6.2 Unit test: WebSocket client sends `stop` message → engine `stop()` called → daemon receives `stop` frame

## 7. UI — Recovery Flow

- [x] 7.1 Handle `crashed` status in `TerminalPane`: display an inline recovery prompt ("Session ended unexpectedly — resume?")
- [x] 7.2 On user confirmation, send `{ type: "spawn", resume: sessionId }` WebSocket message (routes to engine spawn-with-resume) and reconnect terminal to the new session
- [x] 7.3 On user dismissal, send `{ type: "stop" }` WebSocket message to mark the session as intentionally stopped and hide the prompt
- [x] 7.4 Recovered terminal starts blank by design — no special UI handling for pre-crash content

## 8. Observability & Regression

- [x] 8.1 Emit session-correlated logs for `stop` handling and `SessionStopped` rejections
- [x] 8.2 End-to-end test: session crashes → UI shows recovery prompt → user resumes → new session spawns with correct session identity and resumed conversation
- [x] 8.3 End-to-end test: server process restart → stop state preserved → stopped session still rejected after restart
- [x] 8.4 Regression test: `stop()` followed by attempt to resume produces `SessionStopped` error in UI
