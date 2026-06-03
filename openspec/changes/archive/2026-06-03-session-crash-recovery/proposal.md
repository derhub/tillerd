## Why

When a session exits unexpectedly there is no recovery path: the client sees the terminal go blank with no way to continue the agent's work. With a `crashed` status now available (from `exit-qualifier-taxonomy`), a supervised, client-initiated recovery flow can relaunch the agent and resume its conversation — without silent auto-respawn that could double-apply work.

## What Changes

- Add a `stop()` operation distinct from `kill()`. `stop()` marks a session intentionally stopped and ineligible for resume; `kill()` allows resume.
- Stopped-session state is durable (survives engine, server, and daemon restarts) via the session-persistence store.
- After a `crashed` status, a client may recover with `start({ resume: sessionId })`. Recovery routes through the spawn path (new process + hook re-wiring), never the live-session reattach path.
- Recovery restores conversation continuity via the agent's resume mechanism; the recovered terminal starts blank (pre-crash screen is not replayed).
- The engine never auto-respawns; recovery is always explicit.

## Capabilities

### New Capabilities

- `session-recovery`: The `stop()` operation, durable stop state, client-initiated spawn-based recovery, and the no-pre-crash-replay rule.

### Modified Capabilities

- `pty-daemon`: Adds a durable stopped-session set consulted on resume requests.
- `agent-session`: Adds `stop()` distinct from `kill()` with a `SessionStopped` guard on resume.

## Impact

- `@athing/sdk` — `stop` IPC frame, `stop` WebSocket message, `SessionStopped` typed error, `stop()` on the `AgentSession` interface
- `packages/daemon` — `stop` handler, durable stopped-session set in the session-persistence store, resume guard, sessionId re-registration handoff
- `packages/engine/src/daemon/proxy.ts` — `stop()` method; recovery routed through spawn-with-resume with per-session hook-token wiring
- `apps/server`, `apps/ui` — `stop` message handling; crash recovery prompt and resume/dismiss flow
- **Depends on `exit-qualifier-taxonomy`** (requires the `crashed` status and the qualifier contract).
