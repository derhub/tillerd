## Context

This change builds on `exit-qualifier-taxonomy`, which provides the `crashed` status. It adds the recovery half: a way to intentionally stop a session (and keep it stopped durably) and a supervised path to recover a crashed session by relaunching the agent with conversation continuity. The daemon-survives-restart invariant constrains where durable state lives.

## Goals / Non-Goals

**Goals:**

- `stop()` is a first-class operation distinct from `kill()`, preventing resume.
- Stopped state survives engine, server, and daemon restarts.
- Recovery is explicit, spawn-based, and re-wires hooks for the new process.
- Recovery restores conversation continuity, not the dead terminal screen.

**Non-Goals:**

- Automatic re-spawn / crash-loop backoff (deferred).
- Terminal state snapshot (separate change `terminal-state-snapshot`).
- Exit classification (provided by `exit-qualifier-taxonomy`).

## Decisions

### Decision 1: No automatic re-spawn

The engine emits `crashed` and waits. Recovery requires an explicit `start({ resume: sessionId })` from the client. This avoids silent double-work (re-running a partially complete task) and gives the user control.

### Decision 2: Snapshot serves reconnect, not crash-recovery

Two scenarios must not be conflated. **Reconnect** (handled by `terminal-state-snapshot`): the process is alive; a snapshot restores the screen. **Crash-recovery** (here): the process is dead; recovery spawns a _new_ process whose terminal starts blank — there is no prior state to snapshot. Continuity comes from the agent's conversation-resume, not the rendered screen. Pre-crash scrollback is not restored — an accepted limitation; the value is continuing the work, not pixel-restoring a dead terminal.

### Decision 3: Crash-recovery routes through the spawn path, never reconnect

Two distinct resume routes exist: `reconnect()` (subscribe mode, attaches to a live daemon session, installs no hooks) and `start()` with a resume token (spawn mode, new process, installs the loopback hook ingress). After a crash the daemon session is gone, so `reconnect()` cannot apply. Recovery MUST route through spawn so a fresh process launches, the agent resumes its conversation via the resume flag, and the per-session hook token is re-wired. The engine caches per-adapter hook installation but wires a per-session token each spawn; on recovery the cached adapter install is correctly skipped, but the per-session token MUST still be wired for the new process. **Alternative considered:** auto-detect live vs dead behind one call — rejected for v1; the routes have different hook/process semantics, and an explicit choice keeps failure modes legible.

### Decision 4: Stopped-session state is durable via the session-persistence store

`stop()` sends a `stop` frame; the daemon kills the session, marks `killedByUser`, and records the session id. Holding this only in daemon memory survives engine/server restarts but not a daemon restart, which would resurrect resumability for an intentionally stopped session. Stopped identifiers SHALL therefore be written to the existing session-persistence store. On `spawn { resume }`, the daemon consults the durable store (authoritative) before spawning and rejects with `SessionStopped` if present; the in-memory set is a bounded cache over the durable record. This closes the requirement fully: stopped stays stopped across engine, server, and daemon restarts.

### Decision 5: sessionId reuse on recovery

Recovery spawns a new process under the same sessionId the daemon just evicted. The daemon registry SHALL accept re-registration of a just-evicted sessionId. Subscribers of the dead session (already sent `crashed` + exit) MUST NOT auto-reattach; the client drives a fresh subscribe against the recovered session.

## Risks / Trade-offs

- **Recovery hook wiring** -> The cached per-adapter install must not cause the per-session token wiring to be skipped on recovery. Mitigation: explicit test that hook callbacks reach the recovered process.
- **stoppedSessions growth** -> The in-memory cache grows unbounded over a long daemon lifetime. Mitigation: bound it (LRU/TTL) over the durable record so eviction never resurrects resumability.
- **Crash loop** -> A resumed session that immediately re-crashes produces a repeated prompt. v1 relies on the user noticing (no auto-respawn); a backoff/loop-guard is deferred.
- **Resume-after-crash edge** -> The agent's session state file may be mid-write at crash, so conversation resume could be imperfect. Mitigation: surface resume failure as a typed error rather than a silent blank session.

## Open Questions

- **Crash-loop guard:** whether to add a backoff or attempt cap after N consecutive crash-recover cycles. Deferred to a later change.
