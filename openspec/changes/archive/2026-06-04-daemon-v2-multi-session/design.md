## Context

The current daemon (`packages/daemon`) is a single-process NDJSON relay. It owns PTY master fds directly in its main event loop and communicates over plain newline-delimited JSON. This works for a single session but has three structural problems at 3-5 concurrent sessions:

1. **Event loop contention** — a session producing high-bandwidth output (e.g. a long build) saturates the Node/Bun event loop, delaying input and output for every other session.
2. **No backpressure** — if an engine client falls behind consuming output the daemon accumulates unbounded data in memory.
3. **No upgrade path** — any daemon restart kills all live PTY sessions.

The engine client (`packages/engine/src/daemon/`) is also coupled to the NDJSON wire format via a simple readline parser, leaving no room for binary payloads or version evolution.

## Goals / Non-Goals

**Goals:**

- Replace in-process PTY ownership with a subprocess-per-session model so one session cannot stall others.
- Replace NDJSON framing with length-prefixed binary frames for efficiency and binary-payload support.
- Add `hello`/`hello-ack` version negotiation so daemon and client can evolve independently.
- Add ack-based flow control so a slow WebSocket client applies real backpressure rather than accumulating unbounded buffers.
- Add fd-handoff upgrade so the daemon binary can be replaced without interrupting live sessions.

**Non-Goals:**

- Multi-user or multi-tenant support (one subscription = one user).
- Windows support (macOS/Linux for v1).
- Persistent sessions across machine reboots (daemon restart without handoff still loses sessions).
- Network transport (Unix socket only for v1).

## Decisions

### D1: Binary framing format

**Choice**: 4-byte big-endian uint32 length prefix + UTF-8 JSON metadata + optional newline separator + raw binary body.

**Why**: PTY output is already binary (raw bytes, no re-encoding). Wrapping it in JSON (base64 or escape) wastes ~33% bandwidth and adds CPU. Length-prefix framing gives clean message boundaries without scanning for newline characters, which matters when binary payloads may contain `0x0A`.

**Alternatives considered**:

- Keep NDJSON, base64-encode PTY output — simple but ~33% overhead and CPU cost at 5 active sessions.
- MessagePack — efficient but adds a binary serialisation dependency; JSON header + binary body gives the same win with no extra dep.

### D2: Daemon holds PTY master fds directly

**Choice**: daemon owns all PTY master file descriptors in its main process. node-pty fires async `onData` callbacks via Bun's event loop (liburing/kqueue). No subprocess per session.

**Why**: macOS has no `/proc` filesystem — there is no portable way to transfer a file descriptor between processes without `SCM_RIGHTS`. With fd-handoff as a primary goal (D4), the master fd must live in the daemon process. Subprocess isolation would require `SCM_RIGHTS` to move fds from child to parent, which node-pty does not support and which adds significant complexity with no net benefit at 3–5 sessions. Bun's async I/O multiplexes N fds on a single event loop without blocking; flow control (D3) bounds per-session work so no session can starve others. Minimum memory footprint: no extra Bun runtime instances per session (~20–30 MB each).

**Alternatives considered**:

- Subprocess per session with socket-based IPC — clean isolation but upgrade stall of 1–3s while successor reconnects to child sockets; two encode/decode cycles per chunk; N node-pty native addon instances; startup latency ~100–200ms per session.
- Worker threads — shares the event loop for I/O, native crash kills daemon. Rejected.

### D3: Flow control via ack credits

**Choice**: each subscription starts with a 64 KB credit window. Client sends `ack(n)` after consuming `n` bytes. Daemon pauses reading from the PTY master fd when credit hits zero for all subscribers; resumes when any subscriber sends ack.

**Why**: pausing the PTY master fd read causes the slave-side write to block when the kernel PTY buffer fills — real kernel-level back-pressure with no data loss. The per-subscription model means one slow client cannot slow down a fast client on the same session.

**Credit window 64 KB**: matches the current replay buffer size, so a freshly-subscribing client that does not ack will drain exactly the replay buffer before pausing — a natural fit.

**Alternatives considered**:

- TCP-style sliding window — more granular but complex to implement correctly; fixed credit window is sufficient for this use case.
- Drop output on slow client — simple but violates the raw-bytes-end-to-end contract (ADR-0001).

### D4: Fd-handoff upgrade protocol

**Choice**: predecessor serialises live sessions to a snapshot file (atomic rename), spawns successor with PTY master fds at `process.stdio[4..N]` and an IPC channel at fd 3, waits up to 10 s for `upgrade-ack`, then exits.

**Why**: inheriting file descriptors across `fork`/`exec` is the only OS-level way to transfer an open PTY master fd without disrupting the slave-side process. The IPC channel serialises the handoff — predecessor cannot exit before successor is fully bound and serving.

**Snapshot format**: NDJSON (one JSON object per line) — simple, human-readable, no dependencies. Carries: `{ sessionId, pid, cwd, cols, rows, fdIndex, replayBuffer: "<base64>" }`. Base64 for replay buffer because snapshot is text-mode NDJSON.

**Alternatives considered**:

- `/proc/PID/fd` fd transfer via SCM_RIGHTS (Unix socket ancillary data) — works but requires an additional socket connection between predecessor and successor; stdio inheritance is simpler.
- Forking rather than exec — would share memory but inherits the wrong event loop state; exec gives a clean start.

### D5: Protocol versioning strategy

**Choice**: `hello` frame carries an array of supported integer version numbers. Daemon picks the highest mutually supported version, replies with `hello-ack`. Current version: 1.

**Why**: integer version numbers are simple to compare and extend. Array allows a client to offer backward-compatible versions during a rolling upgrade window. Version 1 covers all frame types in this change.

**Alternatives considered**:

- Semver strings — expressive but adds string comparison logic; integer sequence is sufficient.
- No versioning — rejected; makes it impossible to evolve the wire format without hard-coupling client and daemon.

### D6: Wire protocol for child IPC

**Choice**: same binary framing format as the client wire protocol (D1). Reuse the `FrameEncoder` / `FrameDecoder` types in both daemon and child.

**Why**: one codec to test and maintain. No handshake needed on the child channel (daemon owns both ends); the frame types are a strict subset of the client protocol (`spawn`, `input`, `resize`, `interrupt`, `data`, `exit`).

## Risks / Trade-offs

- **[Risk] node-pty `_fd` is a private field** → Mitigation: access is already demonstrated in production (Superset). Pin node-pty to 1.1.0 (already done). Add a runtime assert that `_fd` is a number on spawn so we fail fast if the API changes.

- **[Risk] Snapshot write races with in-flight PTY output** → Mitigation: snapshot is written while the predecessor is still running and serving output; the replay buffer recorded in the snapshot may be slightly behind the successor's first live output, but the successor continues collecting new data immediately. The gap is covered by the replay buffer window. No data loss — some bytes may be replayed twice; the client is responsible for idempotent rendering.

- **[Risk] Upgrade-ack timeout leaves predecessor alive** → Mitigation: predecessor continues normally; user retries or re-deploys. The supervisor logs the failure and does not retry automatically to avoid infinite churn.

- **[Risk] Subprocess crash before first byte** → Mitigation: child sends a `spawn-ack` or `error` frame immediately after PTY open; daemon propagates `error` to the subscribing client and removes the session from the registry cleanly.

- **[Risk] Credit window too small for replay on reconnect** → Mitigation: on subscribe the daemon pre-fills the client's credit from the replay buffer size + initial window, so a full replay never stalls mid-delivery.

## Migration Plan

1. Implement new `packages/daemon` alongside existing one (feature branch).
2. Update `packages/engine/src/daemon/client.ts` to speak the new framing + handshake.
3. Update supervisor to detect version mismatch and trigger upgrade.
4. Integration tests cover: multi-session isolation, flow-control pause/resume, upgrade with live sessions.
5. Cut over: replace `packages/daemon` entry point; bump `DAEMON_VERSION` in manifest.
6. Rollback: supervisor falls back to killing the daemon and spawning the old binary if handshake fails (EVERSION).

## Open Questions

- ADR-0008 states the daemon is "the new single point of failure — if the daemon crashes, all sessions are lost (accepted scope for Phase 1; recovery is Phase 2)." The fd-handoff upgrade path in this change partially addresses Phase 2 recovery for planned restarts but not for unplanned crashes. A new ADR should supersede this statement and narrow the scope: crashes still lose sessions; only planned upgrades are covered.
- The `daemon-wire-protocol` spec defines a frame catalogue but does not specify retry or reconnect semantics for the client. The engine's `supervisor.ts` handles reconnection at the process level; an ADR should record that the client wire protocol is stateless (no in-band reconnect) and reconnect is a supervisor concern.
